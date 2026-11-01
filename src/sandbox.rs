//! Launch wrapper for recorded launches, map ticket 62 and build ticket 318.
//!
//! On macOS every recorded launch wraps the vendor binary in
//! `/usr/bin/sandbox-exec` under the same user and the same `HOME`. On Linux
//! the same rule table becomes bubblewrap mounts. The gate still decides
//! capabilities; the kernel refuses writes to Hydra's directories and reads of
//! the corpus under every spelling the gate's substring floor misses.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::hex::hex_lower;
use crate::policy::{self, PolicySet, Spawn};
use crate::provider::Provider;

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// The `sandbox_kind` a record names on macOS.
pub const KIND_SEATBELT: &str = "seatbelt";
/// The `sandbox_kind` a record names on Linux.
pub const KIND_BUBBLEWRAP: &str = "bubblewrap";
/// The file beside a bubblewrap profile that holds the launcher's mount namespace.
pub const LAUNCHER_MNT_FILE: &str = "launcher.mnt";

/// The rule every profile carries third, and the line the copilot's block of
/// build ticket 250 goes in after.
const TIOCSTI_DENY: &str = "(deny file-ioctl (ioctl-command TIOCSTI))";

/// What `claude` keeps under `~/.claude` that a gated interactive session may
/// write, build ticket 250. Its transcripts are not here: the run's own
/// `projects/<slug>` directory is a write carve-out of every `claude` profile,
/// and it comes after the denials. Absent on purpose are `settings.json`,
/// `hooks/`, `skills/`, `plugins/`, `commands/`, `agents/` and `CLAUDE.md`,
/// which the human's next ungated session reads as instructions or runs as
/// code. Taken from the entries running sessions had modified on 2026-10-03,
/// not from a trace of one launch under the profile.
const CLAUDE_STATE_DIRS: &[&str] = &[
    "backups",
    "debug",
    "file-history",
    "paste-cache",
    "plans",
    "session-env",
    "sessions",
    "shell-snapshots",
    "statsig",
    "tasks",
    "telemetry",
    "todos",
];

/// The launch wrapper this host uses, which is the `sandbox_kind` a record names.
pub fn kind() -> &'static str {
    if cfg!(target_os = "linux") {
        KIND_BUBBLEWRAP
    } else {
        KIND_SEATBELT
    }
}

/// Whether recorded launches on this machine are wrapped. macOS always wraps.
/// Linux wraps only when bubblewrap can run here.
pub fn wraps_launches() -> bool {
    if cfg!(target_os = "macos") {
        return true;
    }
    cfg!(target_os = "linux") && linux_bwrap_ready()
}

/// Whether the launch wrapper is present. On macOS a missing `sandbox-exec`
/// refuses the launch rather than running bare.
pub fn exec_available() -> bool {
    if cfg!(target_os = "macos") {
        Path::new(SANDBOX_EXEC).is_file()
    } else if cfg!(target_os = "linux") {
        linux_bwrap_ready()
    } else {
        false
    }
}

/// The first executable file named `bwrap` in `path_var`, searched in order.
pub fn bwrap_in_path(path_var: Option<&OsStr>) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(path_var?)
        .map(|dir| dir.join("bwrap"))
        .find(|candidate| {
            std::fs::metadata(candidate)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// Whether the kernel allows unprivileged user namespaces, which bwrap needs.
///
/// `proc_sys` is `/proc/sys` on a real host. Three switches close user
/// namespaces, and a `0` or `1` in each one refuses them. Without any of the
/// three files, a `kernel` directory means the switches are open.
pub fn linux_userns_enabled_in(proc_sys: &Path) -> bool {
    let read = |rel: &str| {
        std::fs::read_to_string(proc_sys.join(rel))
            .ok()
            .map(|text| text.trim().to_string())
    };
    if read("kernel/unprivileged_userns_clone").as_deref() == Some("0")
        || read("user/max_user_namespaces").as_deref() == Some("0")
        || read("kernel/apparmor_restrict_unprivileged_userns").as_deref() == Some("1")
    {
        return false;
    }
    proc_sys.join("kernel").is_dir()
}

/// Whether this Linux host can run bwrap for a launch.
pub fn linux_bwrap_ready() -> bool {
    bwrap_in_path(std::env::var_os("PATH").as_deref()).is_some()
        && linux_userns_enabled_in(Path::new("/proc/sys"))
}

/// Why a launch under `policy` must refuse on this host, or `None` when it may run.
///
/// Decision 1 of build ticket 318. A Linux host without bwrap runs `judge`
/// bare, and refuses `standard` and `permissive` rather than fall back to hooks.
pub fn launch_refusal_for_policy_on(
    os: &str,
    policy: &str,
    sandbox_exec_present: bool,
    bwrap_ready: bool,
) -> Option<String> {
    match os {
        "macos" if !sandbox_exec_present => Some(
            "/usr/bin/sandbox-exec is missing on this Mac, so Hydra refuses to launch a recorded run without the seatbelt rather than run bare"
                .to_string(),
        ),
        "linux" if !bwrap_ready && matches!(policy, "standard" | "permissive") => Some(format!(
            "bwrap on PATH with unprivileged user namespaces enabled is unavailable on this Linux host, so Hydra refuses to launch under {policy} rather than fall back to hook-only"
        )),
        _ => None,
    }
}

/// [`launch_refusal_for_policy_on`] for this host.
pub fn launch_refusal_for_policy(policy: &str) -> Option<String> {
    launch_refusal_for_policy_on(
        std::env::consts::OS,
        policy,
        Path::new(SANDBOX_EXEC).is_file(),
        linux_bwrap_ready(),
    )
}

/// Whether a provider's own sandbox must stay off for a launch under `kind`.
///
/// Decision 2 of build ticket 318. Neither `sandbox-exec` nor bwrap lets Codex's
/// `workspace-write` user namespace nest inside Hydra's wrapper. bwrap locks the
/// parent mounts, and the worktree's gitdir and object store sit outside it.
pub fn forbids_vendor_sandbox_for(kind: Option<&str>) -> bool {
    matches!(kind, Some(KIND_SEATBELT | KIND_BUBBLEWRAP))
}

/// [`forbids_vendor_sandbox_for`] for the wrapper this launch uses.
pub fn forbids_vendor_sandbox() -> bool {
    wraps_launches() && forbids_vendor_sandbox_for(Some(kind()))
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
///
/// On Linux the same question asks whether this process runs inside bwrap.
pub fn inside_seatbelt() -> bool {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: a null operation asks only whether the pid is sandboxed;
        // the call reads no memory of ours.
        unsafe { sandbox_check(libc::getpid(), std::ptr::null(), 0) == 1 }
    }
    #[cfg(target_os = "linux")]
    {
        let Ok(ns) = std::fs::read_link("/proc/self/ns/mnt") else {
            return false;
        };
        let Ok(mountinfo) = std::fs::read_to_string("/proc/self/mountinfo") else {
            return false;
        };
        linux_inside_bwrap_from(&ns.to_string_lossy(), &mountinfo)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

/// Whether this process runs inside a bwrap that the launcher did not start.
///
/// Decision 3 of build ticket 318. The launcher writes its mount namespace to
/// `launcher.mnt` beside the profile, and bwrap binds that spawn directory at a
/// mount point of its own. A mount point whose witness names another namespace
/// than `current_mnt_ns` is one this process sees through a new namespace.
pub fn linux_inside_bwrap_from(current_mnt_ns: &str, mountinfo: &str) -> bool {
    let current = current_mnt_ns.trim();
    if current.is_empty() {
        return false;
    }
    mountinfo
        .lines()
        .filter_map(|line| line.split_whitespace().nth(4))
        .map(unescape_mountinfo)
        .any(|mount_point| {
            std::fs::read_to_string(Path::new(&mount_point).join(LAUNCHER_MNT_FILE)).is_ok_and(
                |launcher| {
                    let launcher_ns = launcher.trim();
                    !launcher_ns.is_empty() && launcher_ns != current
                },
            )
        })
}

/// Undoes the octal escapes `/proc/self/mountinfo` writes, `\040` for a space.
fn unescape_mountinfo(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut rest = field;
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        let octal = rest
            .get(at + 1..at + 4)
            .and_then(|digits| u8::from_str_radix(digits, 8).ok());
        match octal {
            Some(byte) => {
                out.push(char::from(byte));
                rest = &rest[at + 4..];
            }
            None => {
                out.push('\\');
                rest = &rest[at + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The profile path beside the gate in one spawn directory.
pub fn profile_path(spawn_dir: &Path) -> PathBuf {
    if cfg!(target_os = "linux") {
        spawn_dir.join("sandbox.bwrap")
    } else {
        spawn_dir.join("sandbox.sb")
    }
}

/// Whether `path` names a bubblewrap profile, by its extension.
fn is_bwrap(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("bwrap")
}

/// Composes and writes the profile, returning its sha256 for `run_start.sandbox`.
/// A `.bwrap` path takes the bubblewrap form and the launcher's mount namespace.
pub fn write_profile(set: &PolicySet, spawn: &Spawn<'_>, path: &Path) -> Result<String> {
    write_profile_for_launch(set, spawn, path, false)
}

/// Writes a profile with an explicit gated interactive seat socket allowance.
pub fn write_profile_for_launch(
    set: &PolicySet,
    spawn: &Spawn<'_>,
    path: &Path,
    gated_interactive: bool,
) -> Result<String> {
    if is_bwrap(path) {
        let launcher_mnt = std::fs::read_link("/proc/self/ns/mnt")
            .ok()
            .map(|ns| ns.to_string_lossy().into_owned());
        return write_bwrap_profile_for_launch(
            set,
            spawn,
            path,
            launcher_mnt.as_deref(),
            gated_interactive,
        );
    }
    let text = compose(set, spawn);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, &text).with_context(|| format!("writing {}", path.display()))?;
    Ok(hex_lower(&Sha256::digest(text.as_bytes())))
}

/// Composes and writes a bubblewrap profile: one argument per line. With a
/// launcher namespace it also writes [`LAUNCHER_MNT_FILE`] beside the profile.
/// Returns the sha256 of the profile text.
pub fn write_bwrap_profile(
    set: &PolicySet,
    spawn: &Spawn<'_>,
    path: &Path,
    launcher_mnt_ns: Option<&str>,
) -> Result<String> {
    write_bwrap_profile_for_launch(set, spawn, path, launcher_mnt_ns, false)
}

fn write_bwrap_profile_for_launch(
    set: &PolicySet,
    spawn: &Spawn<'_>,
    path: &Path,
    launcher_mnt_ns: Option<&str>,
    gated_interactive: bool,
) -> Result<String> {
    let refusals = spawn.gate_script.with_file_name("gate-refusals.jsonl");
    if let Some(parent) = refusals.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&refusals)
        .with_context(|| format!("creating {}", refusals.display()))?;
    for directory in policy::own_read_paths(spawn) {
        std::fs::create_dir_all(&directory)
            .with_context(|| format!("creating {}", directory.display()))?;
    }
    for directory in [".claude/skills", ".agents/plugins", ".codex/skills"] {
        create_inside(spawn.worktree, directory)?;
    }
    // Build ticket 370, measured on a fresh Linux: the home side of the plugin
    // denials, `~/.claude/plugins` and `~/.claude/skills`, did not exist, and
    // compose refused every launch. Created empty here and bound read-only like
    // the worktree's. A symlinked `~/.claude` is followed, because a dotfiles
    // manager links it and the human chose that, unlike a link a seed brought.
    for directory in policy::plugin_write_paths(spawn.worktree) {
        if !directory.starts_with(spawn.worktree) {
            std::fs::create_dir_all(&directory)
                .with_context(|| format!("creating {}", directory.display()))?;
        }
    }
    let empty_settings = spawn.gate_script.with_file_name("empty-settings.json");
    std::fs::write(&empty_settings, "{}")
        .with_context(|| format!("writing {}", empty_settings.display()))?;
    let args = compose_bwrap_for_launch(set, spawn, gated_interactive)?;
    let text = format!("{}\n", args.join("\n"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, &text).with_context(|| format!("writing {}", path.display()))?;
    if let Some(ns) = launcher_mnt_ns {
        let witness = path.with_file_name(LAUNCHER_MNT_FILE);
        std::fs::write(&witness, format!("{}\n", ns.trim()))
            .with_context(|| format!("writing {}", witness.display()))?;
    }
    Ok(hex_lower(&Sha256::digest(text.as_bytes())))
}

/// The SBPL text: allow default, deny writes to Hydra's paths, deny corpus reads,
/// then allow writes to what this spawn owns.
/// Creates `relative` under `worktree` one component at a time, and refuses
/// the launch at a component that is a symlink, build ticket 371.
/// `create_dir_all` follows a link the seed copied into the worktree and
/// creates the directory wherever it points.
fn create_inside(worktree: &Path, relative: &str) -> Result<()> {
    std::fs::create_dir_all(worktree)
        .with_context(|| format!("creating {}", worktree.display()))?;
    let mut path = worktree.to_path_buf();
    for component in Path::new(relative).components() {
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => anyhow::bail!(
                "{} is a symlink, so Hydra will not create the skill root through it. \
                 Remove it from the working tree and launch again",
                path.display()
            ),
            Ok(_) => (),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&path)
                    .with_context(|| format!("creating {}", path.display()))?;
            }
            Err(err) => {
                return Err(err).with_context(|| format!("reading {}", path.display()));
            }
        }
    }
    Ok(())
}

pub fn compose(set: &PolicySet, spawn: &Spawn<'_>) -> String {
    render_sbpl(&rules_of(set, spawn))
}

/// One line of the seatbelt profile, or one bubblewrap mount. The variants follow
/// the numbered rules of [`compose`], and [`rules_of`] emits them in its order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Rule 1. Denies the TIOCSTI ioctl on a headless pty run.
    TtyTiocstiDeny,
    /// Rule 1 on an interactive TUI run, where `--new-session` would detach the
    /// controlling terminal. [`rules_of`] never emits it, because [`compose`]
    /// always writes the TIOCSTI line.
    InteractiveTtyTiocstiDeny { legacy_tiocsti_disabled: bool },
    /// Rule 2. A write deny from [`write_denies`].
    WriteDenySubpath(String),
    /// Rule 3. A read deny on the corpus, the spawns, the pruned set or the approvals.
    ReadDenySubpath(String),
    /// Rule 4. Denies connecting to a socket under the spawns directory, then
    /// allows this run's own `seats.sock`.
    UnixSocketConnectDeny {
        spawns_dir: String,
        allow_seat_socket: Option<String>,
    },
    /// Rule 5. Denies file data under a bounded shell's read-denied roots.
    BoundedReadDataDeny(String),
    /// Rule 6. A read allow. Under a bounded shell it also reopens file data.
    ReadAllowSubpath { path: String, bounded: bool },
    /// Rule 7. Metadata only, on the spawns directory of a run with a parent git dir.
    ReadMetadataAllowLiteral(String),
    /// Rule 8. A write allow on a subpath the run owns.
    WriteAllowSubpath(String),
    /// Rule 8. A write allow on one file, the gate's refusal log.
    WriteAllowLiteral(String),
    /// Rule 9. A write deny on one of the worktree's private git files.
    GitAdminWriteDeny(String),
    /// Rule 10. A write deny on the codex skills root copied into the worktree.
    CodexSkillsWriteDeny(String),
    /// Rule 11. A write deny on a plugin path under the worktree or the home.
    PluginWriteDenySubpath(String),
    /// Rule 11. A write deny on a plugin manifest directory, matched by regex.
    /// `component` is already regex-escaped, and `suffix` is `$` or `/`.
    PluginManifestRegexDeny {
        component: String,
        suffix: &'static str,
    },
    /// Rule 12. The worktree's write deny under a bounded shell, last in the profile.
    BoundedWorktreeWriteDeny(String),
}

/// The rules [`compose`] emits, in its order. [`render_sbpl`] and
/// [`compose_bwrap`] both read this table, so the two composers cannot disagree
/// about which rule comes first.
pub fn rules_of(set: &PolicySet, spawn: &Spawn<'_>) -> Vec<Rule> {
    // Build ticket 137. In a gated interactive launch the provider under this
    // profile holds the human's terminal, and TIOCSTI would let it type into
    // that terminal's input queue. No process in a run has a reason to.
    let mut rules = vec![Rule::TtyTiocstiDeny];
    for path in write_denies(set, spawn) {
        rules.push(Rule::WriteDenySubpath(path));
    }
    rules.push(Rule::ReadDenySubpath(canonical(spawn.corpus)));
    // Every spawn directory holds its run's `launch.json`, which carries the
    // task string and the whole `run_start`, so a sibling's is the corpus by
    // another door. Denied to reads whole, then this run's own re-allowed
    // below: the provider reads its hook config, its MCP config and the gate
    // itself out of there, and the gate script stays denied to writes.
    rules.push(Rule::ReadDenySubpath(canonical(&spawn.layout.spawns())));
    rules.push(Rule::ReadDenySubpath(canonical(&spawn.layout.pruned())));
    // Build ticket 141. A waiting seat request carries its session's task, so
    // the approval store is the corpus by another door too.
    rules.push(Rule::ReadDenySubpath(canonical(&spawn.layout.approvals())));
    // Build ticket 140. The read denial above does not stop `connect`, and a
    // run could reach any socket in a sibling's spawn directory, measured
    // 2026-09-28. So every run's profile denies connecting to a socket under
    // the spawns directory and then allows its own `seats.sock`, which a gated
    // interactive launcher listens on. Not every unix socket:
    // `/var/run/mDNSResponder` is DNS.
    rules.push(Rule::UnixSocketConnectDeny {
        spawns_dir: canonical(&spawn.layout.spawns()),
        allow_seat_socket: spawn
            .gate_script
            .parent()
            .map(|own_spawn| canonical(&own_spawn.join(crate::seats::SOCKET))),
    });
    let bounded = bounded_shell(set, spawn);
    if bounded {
        for path in bounded_read_denies(spawn) {
            rules.push(Rule::BoundedReadDataDeny(path));
        }
    }
    // A later `(allow file-read* ..)` does not lift an earlier, narrower
    // `(deny file-read-data ..)`: the kernel matches the operation, and only an
    // allow naming `file-read-data` itself reopens it. Measured 2026-09-28 with
    // `sandbox-exec` on hand-written profiles, and pinned by
    // `tests/sandbox_read_only.rs`.
    if let Some(own_spawn) = spawn.gate_script.parent() {
        rules.push(Rule::ReadAllowSubpath {
            path: canonical(own_spawn),
            bounded,
        });
    }
    if bounded {
        for path in bounded_read_allows(spawn) {
            rules.push(Rule::ReadAllowSubpath {
                path,
                bounded: true,
            });
        }
    }
    // Git resolves `GIT_OBJECT_DIRECTORY` one component at a time, and the
    // `lstat` of the spawn parent itself was refused: "Invalid path
    // '<root>/spawn': Operation not permitted", measured 2026-09-24. Map ticket
    // 74 puts a run's object store under its spawn directory, so the parent's
    // own metadata is readable. Its listing and every sibling stay denied.
    if spawn.parent_git_dir.is_some() {
        rules.push(Rule::ReadMetadataAllowLiteral(canonical(
            &spawn.layout.spawns(),
        )));
    }
    // A bounded shell's seat writes nothing, so nothing it owns is carved out
    // of the write denials: not the worktree, not its index, not its objects.
    let own_writes = if bounded {
        Vec::new()
    } else {
        policy::own_write_paths(spawn)
    };
    for path in own_writes {
        rules.push(Rule::WriteAllowSubpath(canonical(&path)));
    }
    if let Some(common) = spawn.parent_git_dir
        && let Some(run_id) = spawn.gate_script.parent().and_then(|p| p.file_name())
    {
        for name in ["HEAD", "commondir", "gitdir"] {
            let path = common.join("worktrees").join(run_id).join(name);
            rules.push(Rule::GitAdminWriteDeny(canonical(&path)));
        }
    }
    // The vendor's own data root under the spawn directory, `cursor-data/` on
    // `cursor-agent`, is a read carve-out at the gate and has to be writable at
    // the kernel: the binary creates it before the model runs anything, and
    // under the parent's deny its first `mkdir` failed with EPERM, measured
    // 2026-09-21 on run 01a0c6a5-be8c. The gate script beside it stays denied,
    // because the allow names the data root and not the spawn directory.
    // The live hook appends evidence here even under a bounded shell policy.
    // The gate still denies tool calls naming its spawn directory.
    rules.push(Rule::WriteAllowLiteral(canonical(
        &spawn.gate_script.with_file_name("gate-refusals.jsonl"),
    )));
    for path in policy::own_read_paths(spawn) {
        rules.push(Rule::WriteAllowSubpath(canonical(&path)));
    }
    // `codex` reads skills by shell, so they are copied into the worktree rather
    // than linked into the snapshot. SBPL takes the last matching rule, so this
    // deny after the worktree allow keeps the copy read-only at the kernel.
    if spawn.provider == Provider::Codex {
        rules.push(Rule::CodexSkillsWriteDeny(canonical(
            &spawn.worktree.join(Provider::Codex.skill_root()),
        )));
    }
    // Build ticket 257. Where a later launch loads a plugin from, after the
    // worktree allow so the kernel refuses a shell `mkdir` there too, under
    // every policy and in both launch modes: the copilot's block goes in
    // before these and cannot hand them back.
    for path in policy::plugin_write_paths(spawn.worktree) {
        rules.push(Rule::PluginWriteDenySubpath(canonical(&path)));
    }
    // The manifest directory anywhere: the directory itself, and what is in it.
    let component = regex_escape(policy::PLUGIN_MANIFEST_DIR);
    for suffix in ["$", "/"] {
        rules.push(Rule::PluginManifestRegexDeny {
            component: component.clone(),
            suffix,
        });
    }
    // Last, so no earlier allow hands the worktree back. The parent denial on
    // `worktrees/` already covers it; this names it, and covers a worktree
    // that sits anywhere else.
    if bounded {
        rules.push(Rule::BoundedWorktreeWriteDeny(canonical(spawn.worktree)));
    }
    rules
}

/// The SBPL text for `rules`: the two header lines, then one line per rule.
pub fn render_sbpl(rules: &[Rule]) -> String {
    let mut lines = vec!["(version 1)".to_string(), "(allow default)".to_string()];
    for rule in rules {
        match rule {
            Rule::TtyTiocstiDeny | Rule::InteractiveTtyTiocstiDeny { .. } => {
                lines.push(TIOCSTI_DENY.to_string());
            }
            Rule::WriteDenySubpath(path)
            | Rule::GitAdminWriteDeny(path)
            | Rule::CodexSkillsWriteDeny(path)
            | Rule::PluginWriteDenySubpath(path)
            | Rule::BoundedWorktreeWriteDeny(path) => {
                lines.push(format!(
                    r#"(deny file-write* (subpath "{}"))"#,
                    escape(path)
                ));
            }
            Rule::ReadDenySubpath(path) => {
                lines.push(format!(r#"(deny file-read* (subpath "{}"))"#, escape(path)));
            }
            Rule::BoundedReadDataDeny(path) => {
                lines.push(format!(
                    r#"(deny file-read-data (subpath "{}"))"#,
                    escape(path)
                ));
            }
            Rule::UnixSocketConnectDeny {
                spawns_dir,
                allow_seat_socket,
            } => {
                lines.push(format!(
                    r#"(deny network-outbound (remote unix-socket (subpath "{}")))"#,
                    escape(spawns_dir)
                ));
                if let Some(socket) = allow_seat_socket {
                    lines.push(format!(
                        r#"(allow network-outbound (remote unix-socket (path-literal "{}")))"#,
                        escape(socket)
                    ));
                }
            }
            Rule::ReadAllowSubpath { path, bounded } => {
                lines.push(format!(
                    r#"(allow file-read* (subpath "{}"))"#,
                    escape(path)
                ));
                if *bounded {
                    lines.push(format!(
                        r#"(allow file-read-data (subpath "{}"))"#,
                        escape(path)
                    ));
                }
            }
            Rule::ReadMetadataAllowLiteral(path) => {
                lines.push(format!(
                    r#"(allow file-read-metadata (literal "{}"))"#,
                    escape(path)
                ));
            }
            Rule::WriteAllowSubpath(path) => {
                lines.push(format!(
                    r#"(allow file-write* (subpath "{}"))"#,
                    escape(path)
                ));
            }
            Rule::WriteAllowLiteral(path) => {
                lines.push(format!(
                    r#"(allow file-write* (literal "{}"))"#,
                    escape(path)
                ));
            }
            Rule::PluginManifestRegexDeny { component, suffix } => {
                lines.push(format!(
                    r##"(deny file-write* (regex #"/{component}{suffix}"))"##
                ));
            }
        }
    }
    format!("{}\n", lines.join("\n"))
}

/// The bubblewrap arguments for the same rules [`compose`] writes. Bubblewrap
/// mounts the host read-only or read-write under `--dev-bind / /`, and each rule
/// becomes the mount that narrows that view.
pub fn compose_bwrap(set: &PolicySet, spawn: &Spawn<'_>) -> Result<Vec<String>> {
    compose_bwrap_for_launch(set, spawn, false)
}

fn compose_bwrap_for_launch(
    set: &PolicySet,
    spawn: &Spawn<'_>,
    gated_interactive: bool,
) -> Result<Vec<String>> {
    let mut args = vec![
        "--dev-bind".to_string(),
        "/".to_string(),
        "/".to_string(),
        "--unshare-pid".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--die-with-parent".to_string(),
    ];
    let private = spawn.parent_git_dir.and_then(|common| {
        spawn
            .gate_script
            .parent()?
            .file_name()
            .map(|id| common.join("worktrees").join(id))
    });
    let mut private_mounted = false;
    let config = canonical(&crate::paths::hydra_home().join("config"));
    let mut config_mounted = false;
    for mut rule in rules_of(set, spawn) {
        if let Rule::WriteDenySubpath(path) = &rule
            && Path::new(path).starts_with(&config)
        {
            if config_mounted {
                continue;
            }
            config_mounted = true;
            rule = Rule::WriteDenySubpath(config.clone());
        }
        // Its concrete worktree and home paths already come through
        // `PluginWriteDenySubpath`, so the regex has nothing to add here.
        if matches!(rule, Rule::PluginManifestRegexDeny { .. }) {
            continue;
        }
        if let Rule::UnixSocketConnectDeny {
            allow_seat_socket, ..
        } = &mut rule
            && !gated_interactive
        {
            *allow_seat_socket = None;
        }
        if let Rule::WriteAllowSubpath(path) = &rule
            && let Some(private) = &private
            && [private.join("index"), private.join("index.lock")]
                .iter()
                .any(|file| canonical(file) == *path)
        {
            if !private_mounted {
                // Git renames index.lock over index. A rename across a mount
                // point fails, so mount the directory and protect its metadata.
                let path = canonical(private);
                args.extend(["--bind".to_string(), path.clone(), path]);
                for name in ["HEAD", "commondir", "gitdir", "logs"] {
                    let file = private.join(name);
                    if file.exists() {
                        let path = canonical(&file);
                        args.extend(["--ro-bind".to_string(), path.clone(), path]);
                    }
                }
                private_mounted = true;
            }
            continue;
        }
        if let Rule::WriteDenySubpath(path)
        | Rule::GitAdminWriteDeny(path)
        | Rule::CodexSkillsWriteDeny(path)
        | Rule::PluginWriteDenySubpath(path)
        | Rule::BoundedWorktreeWriteDeny(path) = &rule
            && !Path::new(path).exists()
        {
            if [".claude/settings.json", ".claude/settings.local.json"]
                .iter()
                .any(|name| canonical(&spawn.worktree.join(name)) == *path)
            {
                let source = spawn.gate_script.with_file_name("empty-settings.json");
                if !source.is_file() {
                    anyhow::bail!(
                        "missing bubblewrap deny source {} for {path}",
                        source.display()
                    );
                }
                args.extend(["--ro-bind".to_string(), canonical(&source), path.clone()]);
                continue;
            }
            anyhow::bail!("missing bubblewrap deny source {path}");
        }
        args.extend(translate_rule_to_bwrap(&rule).map_err(anyhow::Error::msg)?);
    }
    Ok(args)
}

/// The bubblewrap arguments for one rule. A rule with no mount equivalent is an
/// error that names it, so a profile can never lose a rule without saying so.
pub fn translate_rule_to_bwrap(rule: &Rule) -> std::result::Result<Vec<String>, String> {
    match rule {
        Rule::TtyTiocstiDeny => Ok(vec!["--new-session".to_string()]),
        Rule::InteractiveTtyTiocstiDeny {
            legacy_tiocsti_disabled: true,
        } => Ok(Vec::new()),
        Rule::InteractiveTtyTiocstiDeny {
            legacy_tiocsti_disabled: false,
        } => Err(
            "rule 1 (interactive TIOCSTI ioctl deny) has no bubblewrap mount equivalent when dev.tty.legacy_tiocsti is not 0"
                .to_string(),
        ),
        Rule::WriteDenySubpath(path)
        | Rule::GitAdminWriteDeny(path)
        | Rule::CodexSkillsWriteDeny(path)
        | Rule::PluginWriteDenySubpath(path)
        | Rule::BoundedWorktreeWriteDeny(path) => {
            Ok(vec!["--ro-bind".to_string(), path.clone(), path.clone()])
        }
        Rule::ReadDenySubpath(path) | Rule::BoundedReadDataDeny(path) => {
            Ok(vec!["--tmpfs".to_string(), path.clone()])
        }
        Rule::UnixSocketConnectDeny {
            spawns_dir,
            allow_seat_socket,
        } => {
            let mut out = vec!["--tmpfs".to_string(), spawns_dir.clone()];
            if let Some(socket) = allow_seat_socket
                && Path::new(socket).exists()
            {
                out.extend(["--bind".to_string(), socket.clone(), socket.clone()]);
            }
            Ok(out)
        }
        Rule::ReadAllowSubpath { path, .. } => {
            Ok(vec!["--ro-bind".to_string(), path.clone(), path.clone()])
        }
        Rule::ReadMetadataAllowLiteral(path) => Ok(vec!["--dir".to_string(), path.clone()]),
        Rule::WriteAllowSubpath(path) | Rule::WriteAllowLiteral(path) => {
            Ok(vec!["--bind".to_string(), path.clone(), path.clone()])
        }
        Rule::PluginManifestRegexDeny { component, suffix } => Err(format!(
            "rule 11 (plugin manifest regex /{component}{suffix}) has no bubblewrap mount equivalent"
        )),
    }
}

/// Narrows a gated interactive launch's profile, build ticket 250, and returns
/// the new sha256 for `run_start.sandbox`.
///
/// Map ticket 98: under `(allow default)` a gated session wrote
/// `~/prueba-fuera.txt` with a shell `printf`, because the gate does not read
/// shell and the profile denied only Hydra's own paths. The session a human
/// types into gets every write under the home directory denied, then handed
/// back where it works and where `claude` keeps the state it needs to run.
/// Headless launches keep the profile [`compose`] writes.
pub fn narrow_for_copilot(profile: &Path, worktree: &Path) -> Result<String> {
    // Build ticket 378. The regex allows this refusal first named are gone,
    // and the refusal stays. The block below is seatbelt text spliced after the
    // seatbelt TIOCSTI line, which a bubblewrap profile does not have. That
    // profile also opens with `--new-session`, which detaches the terminal a
    // human types into.
    if is_bwrap(profile) {
        anyhow::bail!(
            "the copilot block has only a seatbelt form; gated interactive launch is not supported under bubblewrap"
        );
    }
    let home = crate::env::real_home()?;
    let spawn_dir = profile
        .parent()
        .context("a profile path with no spawn directory")?;
    let block = copilot_block(&home, worktree, spawn_dir)?;
    let text = std::fs::read_to_string(profile)
        .with_context(|| format!("reading {}", profile.display()))?;
    let text = insert_after_tiocsti(&text, &block)?;
    std::fs::write(profile, &text).with_context(|| format!("writing {}", profile.display()))?;
    Ok(hex_lower(&Sha256::digest(text.as_bytes())))
}

/// The deny on the home directory and the allows after it.
///
/// It goes before every rule [`compose`] writes after the TIOCSTI line. SBPL
/// takes the last matching rule, so Hydra's own denials still win over these
/// allows: the spawn directory allowed here stays denied whole except what
/// [`compose`] carves out of it again, and the worktree's private git files
/// stay as they were.
pub fn copilot_block(home: &Path, worktree: &Path, spawn_dir: &Path) -> Result<Vec<String>> {
    let home = canonical(home);
    if home.contains('"') {
        anyhow::bail!("a home directory with a quote in its name: {home}");
    }
    let mut lines = vec![format!(
        r#"(deny file-write* (subpath "{}"))"#,
        escape(&home)
    )];
    let mut allowed = Vec::new();
    push_unique(&mut allowed, canonical(worktree));
    // `claude` and `git` write scratch files here. All three sit outside the
    // home directory on a stock Mac, so these allow nothing the deny took,
    // unless `TMPDIR` points under home.
    push_unique(&mut allowed, canonical(&std::env::temp_dir()));
    push_unique(&mut allowed, "/private/tmp".to_string());
    push_unique(&mut allowed, "/private/var/folders".to_string());
    push_unique(&mut allowed, canonical(spawn_dir));
    for dir in CLAUDE_STATE_DIRS {
        push_unique(&mut allowed, format!("{home}/.claude/{dir}"));
    }
    push_unique(
        &mut allowed,
        format!("{home}/Library/Caches/claude-cli-nodejs"),
    );
    for path in allowed {
        lines.push(format!(
            r#"(allow file-write* (subpath "{}"))"#,
            escape(&path)
        ));
    }
    // Build ticket 378, map 99. `~/.claude.json` and `~/.claude/history.jsonl`
    // stay under the home deny. A session that writes `~/.claude.json` can add a
    // user-scope MCP server, and the human's next ungated `claude` starts it.
    // `claude` 2.1.312 ran with both denied on 2026-11-01.
    Ok(lines)
}

fn insert_after_tiocsti(text: &str, block: &[String]) -> Result<String> {
    let mut lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|line| *line == TIOCSTI_DENY)
        .context("the profile has no TIOCSTI rule to put the copilot's block after")?;
    for (offset, line) in block.iter().enumerate() {
        lines.insert(at + 1 + offset, line);
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn regex_escape(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if r"\.^$|?*+()[]{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
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

/// Wraps a launch argv for the launch wrapper that `profile` names.
pub fn wrap_command(binary: &str, argv: &[String], profile: &Path) -> (&'static str, Vec<String>) {
    if is_bwrap(profile) {
        return wrap_bwrap_command(binary, argv, profile);
    }
    let mut wrapped = vec![
        "-f".to_string(),
        profile.to_string_lossy().into_owned(),
        binary.to_string(),
    ];
    wrapped.extend(argv.iter().cloned());
    (SANDBOX_EXEC, wrapped)
}

/// Wraps a launch argv for bwrap: the profile's arguments, `--`, then the command.
///
/// The profile was written just before the launch, so a read failure here is a
/// bug. It panics rather than return a command with no mounts.
pub fn wrap_bwrap_command(
    binary: &str,
    argv: &[String],
    profile: &Path,
) -> (&'static str, Vec<String>) {
    let text = std::fs::read_to_string(profile)
        .unwrap_or_else(|e| panic!("reading {}: {e}", profile.display()));
    let mut wrapped: Vec<String> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    wrapped.push("--".to_string());
    wrapped.push(binary.to_string());
    wrapped.extend(argv.iter().cloned());
    ("bwrap", wrapped)
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
    fn layout_ensure_creates_deny_directories() {
        let (layout, root) = scratch_layout();
        for path in [
            layout.pruned(),
            layout.puppets(),
            layout.approvals(),
            layout.skills(),
            layout.snapshots(),
            layout.skill_worktrees(),
        ] {
            assert!(path.is_dir(), "{}", path.display());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    fn prepare_bwrap_fixture(set: &PolicySet, spawn: &Spawn<'_>) {
        std::fs::create_dir_all(spawn.gate_script.parent().unwrap()).unwrap();
        std::fs::write(spawn.gate_script, "fixture").unwrap();
        if let Some(common) = spawn.parent_git_dir {
            let id = spawn.gate_script.parent().unwrap().file_name().unwrap();
            let private = common.join("worktrees").join(id);
            std::fs::create_dir_all(&private).unwrap();
            for name in ["HEAD", "commondir", "gitdir"] {
                std::fs::write(private.join(name), "fixture").unwrap();
            }
        }
        write_bwrap_profile(
            set,
            spawn,
            &spawn.gate_script.with_file_name("fixture.bwrap"),
            None,
        )
        .unwrap();
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

    /// Build ticket 371, rule 4. `create_dir_all` follows a link the seed put
    /// in the worktree and creates `skills` wherever it points.
    #[test]
    fn a_seeded_skill_root_link_refuses_the_launch_by_name() {
        for link in [".claude", ".claude/skills", ".codex"] {
            let (layout, root) = scratch_layout();
            let set = PolicySet::discover().unwrap();
            let worktree = root.join("worktree");
            let outside = root.join("outside");
            std::fs::create_dir_all(&outside).unwrap();
            let at = worktree.join(link);
            std::fs::create_dir_all(at.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&outside, &at).unwrap();
            let gate = layout.gate_script("r1");
            std::fs::create_dir_all(gate.parent().unwrap()).unwrap();
            std::fs::write(&gate, "fixture").unwrap();
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
            let profile = gate.with_file_name("sandbox.bwrap");
            let err = write_bwrap_profile(&set, &spawn, &profile, None)
                .unwrap_err()
                .to_string();
            assert!(err.contains(&*at.to_string_lossy()), "{link}: {err}");
            assert_eq!(
                std::fs::read_dir(&outside).unwrap().count(),
                0,
                "{link}: something was created through the link"
            );
            assert!(!profile.exists(), "{link}");
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    /// The sha of `file` as the system's own tool prints it, which shares no code
    /// with `sha2` or with [`crate::hex::hex_lower`].
    fn system_sha256(file: &Path) -> String {
        for (tool, args) in [("sha256sum", &[][..]), ("shasum", &["-a", "256"][..])] {
            let Ok(out) = std::process::Command::new(tool)
                .args(args)
                .arg(file)
                .output()
            else {
                continue;
            };
            if out.status.success() {
                let text = String::from_utf8(out.stdout).unwrap();
                return text.split_whitespace().next().unwrap().to_string();
            }
        }
        panic!("neither sha256sum nor shasum ran on this machine");
    }

    /// Build ticket 337. `GateMark` compares the `gate_sha` an old record carries
    /// with the one compiled today, and `run_start.sandbox` is the profile's sha.
    /// Both must stay the lowercase hex the system tool prints for the bytes on
    /// disk, whatever `sha2` version formats them, or every old record reads back
    /// `Foreign`.
    #[test]
    fn gate_sha_and_profile_sha_are_the_system_tools_hex_of_the_files_written() {
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
        policy::write_gate(&set, &spawn).unwrap();
        let gate_sha = policy::gate_sha(&set, &spawn).unwrap();
        assert_eq!(gate_sha, system_sha256(&gate));

        let profile = profile_path(&layout.spawn("r1"));
        let profile_sha = write_profile(&set, &spawn, &profile).unwrap();
        assert_eq!(profile_sha, system_sha256(&profile));

        for sha in [&gate_sha, &profile_sha] {
            assert_eq!(sha.len(), 64, "{sha}");
            assert!(
                sha.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "{sha}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A policy set whose `read-only` grants a bounded shell. No shipped policy
    /// bounds its shell, so the bounded tests copy the shipped set and swap in the
    /// fixture `read-only.json`, which does.
    fn bounded_set(root: &Path) -> PolicySet {
        let shipped = PolicySet::discover().unwrap();
        let dir = root.join("policies");
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(shipped.dir()).unwrap() {
            let path = entry.unwrap().path();
            std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/policies/read-only.json");
        std::fs::copy(fixture, dir.join("read-only.json")).unwrap();
        PolicySet::load(dir).unwrap()
    }

    /// The kind of each rule, with a run of one kind written once.
    fn kinds(rules: &[Rule]) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for rule in rules {
            let kind = match rule {
                Rule::TtyTiocstiDeny => "tiocsti",
                Rule::InteractiveTtyTiocstiDeny { .. } => "interactive-tiocsti",
                Rule::WriteDenySubpath(_) => "write-deny",
                Rule::ReadDenySubpath(_) => "read-deny",
                Rule::UnixSocketConnectDeny { .. } => "socket",
                Rule::BoundedReadDataDeny(_) => "bounded-read-data",
                Rule::ReadAllowSubpath { .. } => "read-allow",
                Rule::ReadMetadataAllowLiteral(_) => "metadata",
                Rule::WriteAllowSubpath(_) => "write-allow",
                Rule::WriteAllowLiteral(_) => "write-allow-literal",
                Rule::GitAdminWriteDeny(_) => "git-admin",
                Rule::CodexSkillsWriteDeny(_) => "codex-skills",
                Rule::PluginWriteDenySubpath(_) => "plugin-write",
                Rule::PluginManifestRegexDeny { .. } => "plugin-manifest",
                Rule::BoundedWorktreeWriteDeny(_) => "bounded-worktree",
            };
            if out.last() != Some(&kind) {
                out.push(kind);
            }
        }
        out
    }

    /// The index where `want` first appears in `args` at or after `from`.
    fn find_from(args: &[String], from: usize, want: &[&str]) -> usize {
        (from..args.len())
            .find(|&at| {
                args.get(at..at + want.len())
                    .is_some_and(|slice| slice.iter().map(String::as_str).eq(want.iter().copied()))
            })
            .unwrap_or_else(|| panic!("{want:?} is not in the arguments after {from}"))
    }

    /// Ticket 364 checks allow sources against a real git worktree.
    #[test]
    fn bwrap_spawn_preparation_and_private_git_mounts() {
        use std::os::unix::net::UnixListener;
        let root = PathBuf::from("/tmp").join(format!(
            "hydra-364-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let layout = Layout::at(&root);
        layout.ensure().unwrap();
        let set = PolicySet::discover().unwrap();
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .current_dir(&repo)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init"]);
        git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "seed",
        ]);
        let common = repo.join(".git");
        for provider in [Provider::Claude, Provider::CursorAgent, Provider::Codex] {
            let id = match provider {
                Provider::Claude => "claude",
                Provider::CursorAgent => "cursor-agent",
                _ => "codex",
            };
            let worktree = root.join(id);
            git(&["worktree", "add", "-b", id, worktree.to_str().unwrap()]);
            let gate = layout.gate_script(id);
            std::fs::create_dir_all(gate.parent().unwrap()).unwrap();
            std::fs::write(&gate, "fixture").unwrap();
            std::fs::create_dir_all(layout.run_objects(id)).unwrap();
            let spawn = Spawn {
                provider,
                policy: "standard",
                layout: &layout,
                worktree: &worktree,
                skill_store: &layout.skills(),
                corpus: &layout.runs(),
                skill_snapshot: &layout.skill_snapshot_scope(id, None),
                skill_checkout: None,
                parent_git_dir: Some(&common),
                gate_script: &gate,
                floor_lifted: false,
            };
            let sbpl = compose(&set, &spawn);
            let created: Vec<_> = policy::own_read_paths(&spawn)
                .into_iter()
                .filter(|path| !path.exists())
                .collect();
            write_profile(&set, &spawn, &gate.with_file_name("sandbox.sb")).unwrap();
            let refusals = gate.with_file_name("gate-refusals.jsonl");
            assert!(!refusals.exists());
            assert!(created.iter().all(|path| !path.exists()));
            write_bwrap_profile(&set, &spawn, &gate.with_file_name("sandbox.bwrap"), None).unwrap();
            assert_eq!(std::fs::read_to_string(&refusals).unwrap(), "");
            std::fs::write(&refusals, "existing evidence\n").unwrap();
            write_bwrap_profile(&set, &spawn, &gate.with_file_name("sandbox.bwrap"), None).unwrap();
            assert_eq!(
                std::fs::read_to_string(&refusals).unwrap(),
                "existing evidence\n"
            );
            assert_eq!(sbpl, compose(&set, &spawn));
            let args = compose_bwrap(&set, &spawn).unwrap();
            let config = canonical(&crate::paths::hydra_home().join("config"));
            assert_eq!(
                args.windows(3)
                    .filter(|mount| mount == &["--ro-bind", &config, &config])
                    .count(),
                1
            );
            for directory in [".claude/skills", ".agents/plugins", ".codex/skills"] {
                assert!(worktree.join(directory).is_dir());
            }
            for settings in [".claude/settings.json", ".claude/settings.local.json"] {
                let destination = canonical(&worktree.join(settings));
                let at = args
                    .windows(3)
                    .position(|mount| mount[0] == "--ro-bind" && mount[2] == destination)
                    .unwrap();
                assert_eq!(std::fs::read_to_string(&args[at + 1]).unwrap(), "{}");
                assert!(!worktree.join(settings).exists());
                let existing = worktree.join(settings);
                std::fs::write(&existing, "{\"hooks\":{}}").unwrap();
                let existing_args = compose_bwrap(&set, &spawn).unwrap();
                find_from(
                    &existing_args,
                    0,
                    &["--ro-bind", &destination, &destination],
                );
                assert_eq!(
                    std::fs::read_to_string(&existing).unwrap(),
                    "{\"hooks\":{}}"
                );
                std::fs::remove_file(existing).unwrap();
            }
            for mount in args
                .windows(3)
                .filter(|mount| matches!(mount[0].as_str(), "--bind" | "--ro-bind" | "--dev-bind"))
            {
                assert!(Path::new(&mount[1]).exists(), "{id}: {mount:?}");
            }
            let private = canonical(&common.join("worktrees").join(id));
            let at = find_from(&args, 0, &["--bind", &private, &private]);
            for name in ["HEAD", "commondir", "gitdir", "logs"] {
                let path = format!("{private}/{name}");
                if Path::new(&path).exists() {
                    find_from(&args, at + 3, &["--ro-bind", &path, &path]);
                }
            }
            for name in ["index", "index.lock"] {
                assert!(!args.contains(&format!("{private}/{name}")));
            }
            let socket = gate.with_file_name(crate::seats::SOCKET);
            assert!(!args.contains(&canonical(&socket)));
            assert!(
                !compose_bwrap_for_launch(&set, &spawn, true)
                    .unwrap()
                    .contains(&canonical(&socket))
            );
            let _listener = UnixListener::bind(&socket).unwrap();
            let args = compose_bwrap(&set, &spawn).unwrap();
            assert!(!args.contains(&canonical(&socket)));
            let args = compose_bwrap_for_launch(&set, &spawn, true).unwrap();
            find_from(
                &args,
                0,
                &["--bind", &canonical(&socket), &canonical(&socket)],
            );
            for directory in created {
                std::fs::remove_dir_all(directory).unwrap();
            }
            let missing = layout.approvals();
            std::fs::remove_dir(&missing).unwrap();
            let profile = gate.with_file_name("missing.bwrap");
            let error = write_bwrap_profile(&set, &spawn, &profile, None).unwrap_err();
            assert!(error.to_string().contains(missing.to_str().unwrap()));
            assert!(!profile.exists());
            std::fs::create_dir(&missing).unwrap();
        }
        let _ = std::fs::remove_dir_all(root);
    }

    /// Build ticket 318a. `render_sbpl` and `compose_bwrap` read one rule table, so
    /// the profile text is that table's SBPL for every provider and policy, and the
    /// bubblewrap arguments open with the same head.
    #[test]
    fn compose_and_compose_bwrap_share_one_rule_table_across_policies_and_providers() {
        let (layout, root) = scratch_layout();
        let sets = [PolicySet::discover().unwrap(), bounded_set(&root)];
        let worktree = root.join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let gate = layout.gate_script("r1");
        let common = root.join("parent.git");
        for set in &sets {
            for name in policy::NAMES {
                for provider in [
                    Provider::Claude,
                    Provider::CursorAgent,
                    Provider::Agy,
                    Provider::Codex,
                ] {
                    for parent_git_dir in [None, Some(common.as_path())] {
                        let spawn = Spawn {
                            provider,
                            policy: name,
                            layout: &layout,
                            worktree: &worktree,
                            skill_store: &layout.skills(),
                            corpus: &layout.runs(),
                            skill_snapshot: &layout.skill_snapshot_scope("r1", None),
                            skill_checkout: None,
                            parent_git_dir,
                            gate_script: &gate,
                            floor_lifted: false,
                        };
                        let case = format!(
                            "{name} {provider:?} parent git {}",
                            parent_git_dir.is_some()
                        );
                        assert_eq!(
                            render_sbpl(&rules_of(set, &spawn)),
                            compose(set, &spawn),
                            "{case}"
                        );
                        prepare_bwrap_fixture(set, &spawn);
                        let args = compose_bwrap(set, &spawn).unwrap();
                        assert_eq!(
                            args[..8],
                            [
                                "--dev-bind",
                                "/",
                                "/",
                                "--unshare-pid",
                                "--proc",
                                "/proc",
                                "--die-with-parent",
                                "--new-session"
                            ],
                            "{case}"
                        );
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Build ticket 318a. Each rule kind comes out in `compose`'s order, and
    /// bubblewrap carries it as the mount the ticket names.
    #[test]
    fn compose_bwrap_renders_each_rule_kind_in_compose_order() {
        let (layout, root) = scratch_layout();
        let set = PolicySet::discover().unwrap();
        let bounded_policies = bounded_set(&root);
        let worktree = root.join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let gate = layout.gate_script("r1");
        let common = root.join("parent.git");
        let own = canonical(gate.parent().unwrap());
        let spawns = canonical(&layout.spawns());
        let layout_root = canonical(layout.root());
        let wt = canonical(&worktree);
        let head = canonical(&common.join("worktrees").join("r1").join("HEAD"));
        let skills = canonical(&worktree.join(Provider::Codex.skill_root()));
        let refusals = canonical(&gate.with_file_name("gate-refusals.jsonl"));

        let standard = Spawn {
            provider: Provider::Codex,
            policy: "standard",
            layout: &layout,
            worktree: &worktree,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.skill_snapshot_scope("r1", None),
            skill_checkout: None,
            parent_git_dir: Some(&common),
            gate_script: &gate,
            floor_lifted: false,
        };
        assert_eq!(
            kinds(&rules_of(&set, &standard)),
            [
                "tiocsti",
                "write-deny",
                "read-deny",
                "socket",
                "read-allow",
                "metadata",
                "write-allow",
                "git-admin",
                "write-allow-literal",
                "codex-skills",
                "plugin-write",
                "plugin-manifest",
            ]
        );
        prepare_bwrap_fixture(&set, &standard);
        let args = compose_bwrap(&set, &standard).unwrap();
        let chain: [Vec<&str>; 6] = [
            vec!["--ro-bind", &own, &own],
            vec!["--dir", &spawns],
            vec!["--bind", &wt, &wt],
            vec!["--ro-bind", &head, &head],
            vec!["--bind", &refusals, &refusals],
            vec!["--ro-bind", &skills, &skills],
        ];
        let mut at = 0;
        for want in &chain {
            at = find_from(&args, at, want) + want.len();
        }

        let bounded = Spawn {
            provider: Provider::Codex,
            policy: "read-only",
            layout: &layout,
            worktree: &worktree,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.skill_snapshot_scope("r1", None),
            skill_checkout: None,
            parent_git_dir: Some(&common),
            gate_script: &gate,
            floor_lifted: false,
        };
        assert_eq!(
            kinds(&rules_of(&bounded_policies, &bounded)),
            [
                "tiocsti",
                "write-deny",
                "read-deny",
                "socket",
                "bounded-read-data",
                "read-allow",
                "metadata",
                "git-admin",
                "write-allow-literal",
                "codex-skills",
                "plugin-write",
                "plugin-manifest",
                "bounded-worktree",
            ]
        );
        let args = compose_bwrap(&bounded_policies, &bounded).unwrap();
        let chain: [Vec<&str>; 5] = [
            vec!["--tmpfs", &spawns],
            vec!["--tmpfs", &layout_root],
            vec!["--ro-bind", &head, &head],
            vec!["--bind", &refusals, &refusals],
            vec!["--ro-bind", &skills, &skills],
        ];
        let mut at = 0;
        for want in &chain {
            at = find_from(&args, at, want) + want.len();
        }
        assert_eq!(
            args[args.len() - 3..],
            ["--ro-bind", wt.as_str(), wt.as_str()]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Build ticket 318a. Bubblewrap has no mount for two rules, and the
    /// translation names each one rather than dropping it.
    #[test]
    fn translate_rule_to_bwrap_refuses_untranslatable_rules_naming_each_rule() {
        assert_eq!(
            translate_rule_to_bwrap(&Rule::TtyTiocstiDeny),
            Ok(vec!["--new-session".to_string()])
        );
        assert_eq!(
            translate_rule_to_bwrap(&Rule::InteractiveTtyTiocstiDeny {
                legacy_tiocsti_disabled: true
            }),
            Ok(Vec::new())
        );
        let plugin = Rule::PluginManifestRegexDeny {
            component: regex_escape(policy::PLUGIN_MANIFEST_DIR),
            suffix: "$",
        };
        assert_eq!(
            translate_rule_to_bwrap(&plugin),
            Err(r"rule 11 (plugin manifest regex /\.claude-plugin$) has no bubblewrap mount equivalent"
                .to_string())
        );
        let interactive = Rule::InteractiveTtyTiocstiDeny {
            legacy_tiocsti_disabled: false,
        };
        assert_eq!(
            translate_rule_to_bwrap(&interactive),
            Err("rule 1 (interactive TIOCSTI ioctl deny) has no bubblewrap mount equivalent when dev.tty.legacy_tiocsti is not 0".to_string())
        );
    }

    /// Build ticket 318b. `bwrap` counts only as an executable file on `PATH`.
    #[test]
    fn bwrap_in_path_finds_an_executable_file_named_bwrap() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("hydra-bwrap-path-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (plain, tool) = (root.join("plain"), root.join("tool"));
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::create_dir_all(&tool).unwrap();
        std::fs::write(plain.join("bwrap"), "").unwrap();
        std::fs::set_permissions(plain.join("bwrap"), std::fs::Permissions::from_mode(0o644))
            .unwrap();
        std::fs::write(tool.join("bwrap"), "").unwrap();
        std::fs::set_permissions(tool.join("bwrap"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        let both = std::env::join_paths([&plain, &tool]).unwrap();
        assert_eq!(
            bwrap_in_path(Some(both.as_os_str())),
            Some(tool.join("bwrap"))
        );
        let plain_only = std::env::join_paths([&plain]).unwrap();
        assert_eq!(bwrap_in_path(Some(plain_only.as_os_str())), None);
        assert_eq!(bwrap_in_path(None), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Build ticket 318b. The kernel switches that close user namespaces, read
    /// from a fake `/proc/sys`. A missing `kernel` directory, a `0` in either
    /// sysctl, or AppArmor's restriction at `1` each refuse bubblewrap.
    #[test]
    fn linux_userns_check_reads_unprivileged_userns_max_userns_and_apparmor() {
        let root = std::env::temp_dir().join(format!("hydra-userns-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert!(!linux_userns_enabled_in(&root), "no kernel directory");
        std::fs::create_dir_all(root.join("kernel")).unwrap();
        assert!(linux_userns_enabled_in(&root), "kernel directory alone");
        let put = |rel: &str, text: &str| {
            std::fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
            std::fs::write(root.join(rel), text).unwrap();
        };
        put("kernel/unprivileged_userns_clone", "0\n");
        assert!(!linux_userns_enabled_in(&root), "clone switch off");
        put("kernel/unprivileged_userns_clone", "1\n");
        put("user/max_user_namespaces", "0\n");
        assert!(!linux_userns_enabled_in(&root), "max_user_namespaces is 0");
        put("user/max_user_namespaces", "15000\n");
        put("kernel/apparmor_restrict_unprivileged_userns", "1\n");
        assert!(!linux_userns_enabled_in(&root), "AppArmor restricts");
        put("kernel/apparmor_restrict_unprivileged_userns", "0\n");
        assert!(linux_userns_enabled_in(&root), "every switch open");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Build ticket 318b. Decision 1. On Linux without bwrap, `standard` and
    /// `permissive` refuse and `judge` launches bare. On macOS a missing
    /// `sandbox-exec` refuses every policy.
    #[test]
    fn launch_refusal_for_policy_refuses_standard_and_permissive_on_linux_without_bwrap_and_allows_judge()
     {
        for policy in ["standard", "permissive"] {
            let refusal = launch_refusal_for_policy_on("linux", policy, false, false)
                .unwrap_or_else(|| panic!("{policy} launches without bwrap"));
            assert!(refusal.contains("bwrap"), "{refusal}");
            assert!(refusal.contains(policy), "{refusal}");
            assert_eq!(
                launch_refusal_for_policy_on("linux", policy, false, true),
                None
            );
        }
        assert_eq!(
            launch_refusal_for_policy_on("linux", "judge", false, false),
            None
        );
        let missing = launch_refusal_for_policy_on("macos", "judge", false, true).unwrap();
        assert!(missing.contains("sandbox-exec"), "{missing}");
        assert_eq!(
            launch_refusal_for_policy_on("macos", "standard", true, false),
            None
        );
    }

    /// Build ticket 318b. Decision 2. Codex's own sandbox stays off under both wrappers.
    #[test]
    fn codex_vendor_sandbox_is_forbidden_under_both_seatbelt_and_bubblewrap() {
        assert!(forbids_vendor_sandbox_for(Some(KIND_SEATBELT)));
        assert!(forbids_vendor_sandbox_for(Some(KIND_BUBBLEWRAP)));
        assert!(!forbids_vendor_sandbox_for(None));
        assert!(!forbids_vendor_sandbox_for(Some("hook-only")));
    }

    /// Build ticket 318b. Decision 3. A `launcher.mnt` beside a mount point, whose
    /// namespace differs from this process's, means this process runs inside bwrap.
    /// The mount point carries a space, which mountinfo writes as `\040`.
    #[test]
    fn linux_inside_bwrap_witness_compares_launcher_mnt_against_current_mnt() {
        let root = std::env::temp_dir().join(format!("hydra-mnt-witness-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let spawn = root.join("spawn with space");
        std::fs::create_dir_all(&spawn).unwrap();
        std::fs::write(spawn.join(LAUNCHER_MNT_FILE), "mnt:[111]\n").unwrap();
        let escaped = spawn.display().to_string().replace(' ', r"\040");
        let mountinfo = format!(
            "1 0 8:1 / / rw - ext4 /dev/sda1 rw\n36 1 0:31 / {escaped} rw - tmpfs tmpfs rw\n"
        );
        assert!(linux_inside_bwrap_from("mnt:[222]", &mountinfo));
        assert!(
            !linux_inside_bwrap_from("mnt:[111]\n", &mountinfo),
            "the launcher's own namespace"
        );
        assert!(
            !linux_inside_bwrap_from("", &mountinfo),
            "no namespace read"
        );
        assert!(
            !linux_inside_bwrap_from("mnt:[222]", "1 0 8:1 / / rw - ext4 /dev/sda1 rw\n"),
            "no mount holds a witness"
        );
        std::fs::write(spawn.join(LAUNCHER_MNT_FILE), "\n").unwrap();
        assert!(
            !linux_inside_bwrap_from("mnt:[222]", &mountinfo),
            "an empty witness"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Build ticket 318b. The bwrap profile is one argument per line, the launcher's
    /// namespace lands beside it, and `wrap_command` hands a `.bwrap` profile to bwrap
    /// while a `.sb` profile keeps `sandbox-exec`.
    #[test]
    fn write_bwrap_profile_and_wrap_bwrap_command_round_trip() {
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
        let profile = layout.spawn("r1").join("sandbox.bwrap");
        prepare_bwrap_fixture(&set, &spawn);
        let sha = write_bwrap_profile(&set, &spawn, &profile, Some("mnt:[4026531841]\n")).unwrap();
        assert_eq!(sha, system_sha256(&profile));
        let args = compose_bwrap(&set, &spawn).unwrap();
        assert_eq!(
            std::fs::read_to_string(&profile).unwrap(),
            format!("{}\n", args.join("\n"))
        );
        assert_eq!(
            std::fs::read_to_string(layout.spawn("r1").join(LAUNCHER_MNT_FILE)).unwrap(),
            "mnt:[4026531841]\n"
        );

        let argv = vec!["-p".to_string(), "hi".to_string()];
        let (program, wrapped) = wrap_command("claude", &argv, &profile);
        assert_eq!(program, "bwrap");
        let mut want = args.clone();
        want.extend(["--", "claude", "-p", "hi"].map(String::from));
        assert_eq!(wrapped, want);
        let sb = layout.spawn("r1").join("sandbox.sb");
        assert_eq!(wrap_command("claude", &argv, &sb).0, SANDBOX_EXEC);

        let other = layout.spawn("r2").join("sandbox.bwrap");
        write_bwrap_profile(&set, &spawn, &other, None).unwrap();
        assert!(!layout.spawn("r2").join(LAUNCHER_MNT_FILE).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Build tickets 318b and 378. The copilot block is seatbelt text only, so a
    /// bwrap profile is refused by name rather than narrowed.
    #[test]
    fn narrow_for_copilot_refuses_a_bwrap_profile() {
        let root = std::env::temp_dir().join(format!("hydra-copilot-bwrap-{}", std::process::id()));
        let profile = root.join("sandbox.bwrap");
        let err = narrow_for_copilot(&profile, &root).unwrap_err().to_string();
        assert!(
            err.contains("the copilot block has only a seatbelt form"),
            "{err}"
        );
        assert!(
            err.contains("gated interactive launch is not supported under bubblewrap"),
            "{err}"
        );
    }

    /// Build ticket 378, map 99. No line of the copilot block hands back
    /// `~/.claude.json` or `~/.claude/history.jsonl`, by regex or by path.
    #[test]
    fn the_copilot_block_allows_neither_claude_json_nor_history() {
        let block = copilot_block(Path::new("/h"), Path::new("/w"), Path::new("/s")).unwrap();
        assert_eq!(block[0], r#"(deny file-write* (subpath "/h"))"#);
        for line in &block {
            assert!(!line.contains("regex"), "{line}");
            assert!(!line.contains(".claude.json"), "{line}");
            assert!(!line.contains("history.jsonl"), "{line}");
        }
    }
}
