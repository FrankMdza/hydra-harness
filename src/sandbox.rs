//! macOS seatbelt profile for recorded launches, map ticket 62.
//!
//! On macOS every recorded launch wraps the vendor binary in
//! `/usr/bin/sandbox-exec` under the same user and the same `HOME`. The gate
//! still decides capabilities; the kernel refuses writes to Hydra's directories
//! and reads of the corpus under every spelling the gate's substring floor misses.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::policy::{self, PolicySet, Spawn};
use crate::provider::Provider;

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// Whether recorded launches on this machine are wrapped in the seatbelt.
pub fn wraps_launches() -> bool {
    cfg!(target_os = "macos")
}

/// Whether [`SANDBOX_EXEC`] is present. On macOS a missing binary refuses the
/// launch rather than running bare.
pub fn exec_available() -> bool {
    wraps_launches() && Path::new(SANDBOX_EXEC).is_file()
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    /// `sandbox_check(3)`: with a null operation, whether `pid` is sandboxed.
    fn sandbox_check(
        pid: libc::pid_t,
        operation: *const libc::c_char,
        kind: libc::c_int,
        ...
    ) -> libc::c_int;
}

/// Whether this process is itself inside a seatbelt, which on this machine
/// means Hydra put it around the run this process descends from.
///
/// A run under `standard` has shell and shell reaches the binary. The marker
/// check of build ticket 57 is a variable the run can unset; the seatbelt is
/// inherited and cannot be shed. So this is the one fact about its ancestry a
/// run cannot hide, and `hydra run` reads it before it honours `HYDRA_ROOT`.
pub fn inside_seatbelt() -> bool {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: a null operation asks only whether the pid is sandboxed;
        // the call reads no memory of ours.
        unsafe { sandbox_check(libc::getpid(), std::ptr::null(), 0) == 1 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// The profile path beside the gate in one spawn directory.
pub fn profile_path(spawn_dir: &Path) -> PathBuf {
    spawn_dir.join("sandbox.sb")
}

/// Composes and writes the profile, returning its sha256 for `run_start.sandbox`.
pub fn write_profile(set: &PolicySet, spawn: &Spawn<'_>, path: &Path) -> Result<String> {
    let text = compose(set, spawn);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, &text).with_context(|| format!("writing {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(text.as_bytes())))
}

/// The SBPL text: allow default, deny writes to Hydra's paths, deny corpus reads,
/// then allow writes to what this spawn owns.
pub fn compose(set: &PolicySet, spawn: &Spawn<'_>) -> String {
    let mut lines = vec!["(version 1)".to_string(), "(allow default)".to_string()];
    // Build ticket 137. In a gated interactive launch the provider under this
    // profile holds the human's terminal, and TIOCSTI would let it type into
    // that terminal's input queue. No process in a run has a reason to.
    lines.push("(deny file-ioctl (ioctl-command TIOCSTI))".to_string());
    for path in write_denies(set, spawn) {
        lines.push(format!(
            r#"(deny file-write* (subpath "{}"))"#,
            escape(&path)
        ));
    }
    let corpus = canonical(spawn.corpus);
    lines.push(format!(
        r#"(deny file-read* (subpath "{}"))"#,
        escape(&corpus)
    ));
    // Every spawn directory holds its run's `launch.json`, which carries the
    // task string and the whole `run_start`, so a sibling's is the corpus by
    // another door. Denied to reads whole, then this run's own re-allowed
    // below: the provider reads its hook config, its MCP config and the gate
    // itself out of there, and the gate script stays denied to writes.
    lines.push(format!(
        r#"(deny file-read* (subpath "{}"))"#,
        escape(&canonical(&spawn.layout.spawns()))
    ));
    lines.push(format!(
        r#"(deny file-read* (subpath "{}"))"#,
        escape(&canonical(&spawn.layout.pruned()))
    ));
    // Build ticket 140. The read denial above does not stop `connect`, and a
    // run could reach any socket in a sibling's spawn directory, measured
    // 2026-09-28. So every run's profile denies connecting to a socket under
    // the spawns directory and then allows its own `seats.sock`, which a gated
    // interactive launcher listens on. Not every unix socket:
    // `/var/run/mDNSResponder` is DNS.
    lines.push(format!(
        r#"(deny network-outbound (remote unix-socket (subpath "{}")))"#,
        escape(&canonical(&spawn.layout.spawns()))
    ));
    if let Some(own_spawn) = spawn.gate_script.parent() {
        lines.push(format!(
            r#"(allow network-outbound (remote unix-socket (path-literal "{}")))"#,
            escape(&canonical(&own_spawn.join(crate::seats::SOCKET)))
        ));
    }
    let bounded = bounded_shell(set, spawn);
    if bounded {
        for path in bounded_read_denies(spawn) {
            lines.push(format!(
                r#"(deny file-read-data (subpath "{}"))"#,
                escape(&path)
            ));
        }
    }
    // A later `(allow file-read* ..)` does not lift an earlier, narrower
    // `(deny file-read-data ..)`: the kernel matches the operation, and only an
    // allow naming `file-read-data` itself reopens it. Measured 2026-09-28 with
    // `sandbox-exec` on hand-written profiles, and pinned by
    // `tests/sandbox_read_only.rs`.
    let reopen = |lines: &mut Vec<String>, path: &str| {
        lines.push(format!(
            r#"(allow file-read* (subpath "{}"))"#,
            escape(path)
        ));
        if bounded {
            lines.push(format!(
                r#"(allow file-read-data (subpath "{}"))"#,
                escape(path)
            ));
        }
    };
    if let Some(own_spawn) = spawn.gate_script.parent() {
        reopen(&mut lines, &canonical(own_spawn));
    }
    if bounded {
        for path in bounded_read_allows(spawn) {
            reopen(&mut lines, &path);
        }
    }
    // Git resolves `GIT_OBJECT_DIRECTORY` one component at a time, and the
    // `lstat` of the spawn parent itself was refused: "Invalid path
    // '<root>/spawn': Operation not permitted", measured 2026-09-24. Map ticket
    // 74 puts a run's object store under its spawn directory, so the parent's
    // own metadata is readable. Its listing and every sibling stay denied.
    if spawn.parent_git_dir.is_some() {
        lines.push(format!(
            r#"(allow file-read-metadata (literal "{}"))"#,
            escape(&canonical(&spawn.layout.spawns()))
        ));
    }
    // A bounded shell's seat writes nothing, so nothing it owns is carved out
    // of the write denials: not the worktree, not its index, not its objects.
    let own_writes = if bounded {
        Vec::new()
    } else {
        policy::own_write_paths(spawn)
    };
    for path in own_writes {
        let own = canonical(&path);
        lines.push(format!(
            r#"(allow file-write* (subpath "{}"))"#,
            escape(&own)
        ));
    }
    if let Some(common) = spawn.parent_git_dir
        && let Some(run_id) = spawn.gate_script.parent().and_then(|p| p.file_name())
    {
        for name in ["HEAD", "commondir", "gitdir"] {
            let path = common.join("worktrees").join(run_id).join(name);
            lines.push(format!(
                r#"(deny file-write* (subpath "{}"))"#,
                escape(&canonical(&path))
            ));
        }
    }
    // The vendor's own data root under the spawn directory, `cursor-data/` on
    // `cursor-agent`, is a read carve-out at the gate and has to be writable at
    // the kernel: the binary creates it before the model runs anything, and
    // under the parent's deny its first `mkdir` failed with EPERM, measured
    // 2026-09-21 on run 01a0c6a5-be8c. The gate script beside it stays denied,
    // because the allow names the data root and not the spawn directory.
    for path in policy::own_read_paths(spawn) {
        let own = canonical(&path);
        lines.push(format!(
            r#"(allow file-write* (subpath "{}"))"#,
            escape(&own)
        ));
    }
    // `codex` reads skills by shell, so they are copied into the worktree rather
    // than linked into the snapshot. SBPL takes the last matching rule, so this
    // deny after the worktree allow keeps the copy read-only at the kernel.
    if spawn.provider == Provider::Codex {
        let skills = canonical(&spawn.worktree.join(Provider::Codex.skill_root()));
        lines.push(format!(
            r#"(deny file-write* (subpath "{}"))"#,
            escape(&skills)
        ));
    }
    // Last, so no earlier allow hands the worktree back. The parent denial on
    // `worktrees/` already covers it; this names it, and covers a worktree
    // that sits anywhere else.
    if bounded {
        lines.push(format!(
            r#"(deny file-write* (subpath "{}"))"#,
            escape(&canonical(spawn.worktree))
        ));
    }
    format!("{}\n", lines.join("\n"))
}

/// Whether the spawn's policy bounds its shell to a list, build ticket 139.
///
/// Map 82 answer 3 makes the list the first filter and this profile the
/// guarantee. A policy the set does not name composes the ordinary profile,
/// and the launch fails on that name before it runs anything.
fn bounded_shell(set: &PolicySet, spawn: &Spawn<'_>) -> bool {
    set.get(spawn.policy)
        .is_ok_and(|policy| policy.shell() == policy::ShellGrant::Bounded)
}

/// What a bounded shell's seat may not read: Hydra's whole tree, the layout
/// and the checkout the binary was built from, which on a real machine are
/// both `~/hydra`. The sibling worktrees, the store, the snapshots and the
/// config all sit inside.
///
/// `file-read-data` and not `file-read*`: the worktree sits under these, and
/// `realpath` and `getcwd` `lstat` every ancestor on the way down. The data
/// denial refuses a file's contents and a directory's listing and leaves an
/// ancestor's metadata alone.
fn bounded_read_denies(spawn: &Spawn<'_>) -> Vec<String> {
    let mut out = Vec::new();
    push_unique(&mut out, canonical(spawn.layout.root()));
    push_unique(&mut out, canonical(&crate::paths::hydra_home()));
    out
}

/// What the seat reads again after [`bounded_read_denies`]: its worktree, its
/// skill snapshot, the parent repo's git directory `git log` reads through the
/// worktree's `.git` file, and the vendor's own data root.
fn bounded_read_allows(spawn: &Spawn<'_>) -> Vec<String> {
    let mut out = Vec::new();
    push_unique(&mut out, canonical(spawn.worktree));
    push_unique(&mut out, canonical(spawn.skill_snapshot));
    if let Some(git_dir) = spawn.parent_git_dir {
        push_unique(&mut out, canonical(git_dir));
    }
    for path in policy::own_read_paths(spawn) {
        push_unique(&mut out, canonical(&path));
    }
    out
}

/// Wraps a launch argv for the seatbelt when `profile` is set.
pub fn wrap_command(binary: &str, argv: &[String], profile: &Path) -> (&'static str, Vec<String>) {
    let mut wrapped = vec![
        "-f".to_string(),
        profile.to_string_lossy().into_owned(),
        binary.to_string(),
    ];
    wrapped.extend(argv.iter().cloned());
    (SANDBOX_EXEC, wrapped)
}

/// Every denied path, parents of own paths included. SBPL takes the last
/// matching rule, measured 2026-09-21: a deny on `worktrees/` followed by an
/// allow on `worktrees/<own>` leaves the own tree writable and a sibling
/// refused. Skipping the parent because it holds an own path, which the first
/// draft did, would have left every sibling worktree open to the kernel layer.
fn write_denies(set: &PolicySet, spawn: &Spawn<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for path in policy::denied_write_paths(set, spawn) {
        push_unique(&mut out, canonical(&path));
    }
    out
}

fn push_unique(out: &mut Vec<String>, path: String) {
    if !path.is_empty() && !out.contains(&path) {
        out.push(path);
    }
}

fn escape(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Canonical absolute path, the spelling macOS seatbelt matches.
fn canonical(path: &Path) -> String {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path.to_path_buf();
    loop {
        if let Ok(base) = std::fs::canonicalize(&cursor) {
            let mut out = base;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return out.to_string_lossy().into_owned();
        }
        match (cursor.file_name(), cursor.parent()) {
            (Some(name), Some(parent)) if !parent.as_os_str().is_empty() => {
                tail.push(name.to_os_string());
                cursor = parent.to_path_buf();
            }
            _ => return path.to_string_lossy().into_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Layout;
    use crate::policy::PolicySet;
    use crate::provider::Provider;

    fn scratch_layout() -> (Layout, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "hydra-sandbox-unit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let layout = Layout::at(&root);
        layout.ensure().unwrap();
        (layout, root)
    }

    #[test]
    fn the_profile_names_denied_paths_and_not_the_worktree() {
        let (layout, root) = scratch_layout();
        let set = PolicySet::discover().unwrap();
        let worktree = root.join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let gate = layout.gate_script("r1");
        let spawn = Spawn {
            provider: Provider::Claude,
            policy: "standard",
            layout: &layout,
            worktree: &worktree,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.skill_snapshot_scope("r1", None),
            skill_checkout: None,
            parent_git_dir: None,
            gate_script: &gate,
            floor_lifted: false,
        };
        let text = compose(&set, &spawn);
        let worktree_canon = canonical(&worktree).to_lowercase();
        for path in policy::denied_write_paths(&set, &spawn) {
            let resolved = canonical(&path).to_lowercase();
            if policy::own_write_paths(&spawn)
                .iter()
                .any(|own| resolved == canonical(own).to_lowercase())
            {
                continue;
            }
            assert!(
                text.to_lowercase().contains(&resolved),
                "profile does not deny {resolved}"
            );
        }
        assert!(
            !text.to_lowercase().contains(&format!(
                r#"(deny file-write* (subpath "{}"))"#,
                worktree_canon
            )),
            "profile denies the run's own worktree"
        );
    }
}
