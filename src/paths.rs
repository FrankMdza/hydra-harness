//! The `~/hydra` layout.
//!
//! Harness spec section 5 names four places Hydra writes and all four are here:
//! the run file, the worktree, the spawn directory and the skills snapshot.
//! `~/hydra/pruned/` is a fifth that section 5 does not name and
//! `~/hydra/skill-worktrees/` is a sixth; [`Layout::pruned`] and
//! [`Layout::skill_worktrees`] say why each is a choice rather than a reading.
//!
//! **The spawn directory is a sibling of `runs/` and not a subdirectory of it.**
//! Section 5 is explicit: ticket 13 derives liveness from a run file's opening
//! envelope and forbids anything but the owning process from appending to it, and
//! a run that is a directory invites a second writer.
//!
//! The worktree location is a choice this slice makes rather than one the design
//! settled. No closed ticket names it, the same gap harness spec 19.1 flags for
//! the filenames inside the spawn directory. `~/hydra/worktrees/<run-id>/` keeps
//! D9 true for any target repo: Hydra writes only under a path it created, even
//! when the repo being worked on lives somewhere else entirely.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// The environment variable that overrides where `policies/` and `config/` are read from.
pub const HYDRA_HOME_VAR: &str = "HYDRA_HOME";

/// Where the shipped `policies/` and `config/` directories live.
///
/// Reads [`HYDRA_HOME_VAR`] when set, otherwise the compile-time
/// `CARGO_MANIFEST_DIR`. The compile-time path stays the default because
/// `cargo run` and the test suite both find those directories beside the source
/// tree with no setup, and because the binary has run from its own checkout
/// since the first slice. Build ticket 50 gave it an override; nothing gave it
/// an install path, and `README.md` says the binary is not relocatable away
/// from a tree that holds both directories.
pub fn hydra_home() -> PathBuf {
    hydra_home_from(std::env::var_os(HYDRA_HOME_VAR))
}

/// The same resolution with the variable's value passed in.
///
/// Split out so a test can drive the order without setting a process-wide
/// variable: the unit tests in this crate run in one binary and in parallel, and
/// `PolicySet::discover` in `src/policy/mod.rs` calls [`hydra_home`] from tests
/// of its own, which a mutation here would break at random.
fn hydra_home_from(named: Option<std::ffi::OsString>) -> PathBuf {
    match named {
        Some(home) if !home.is_empty() => PathBuf::from(home),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    }
}

/// Whether every byte of a path is ASCII once symlinks are resolved.
///
/// Map ticket 68: the gate folds case in ASCII only, and macOS folds non-ASCII
/// letters in paths too, so a root whose path holds such a letter would miss
/// the other case on every deny entry and scope compiled from it. The compiler
/// folds what `resolved_path` returns, not what the user typed, so an ASCII
/// symlink to a non-ASCII directory has to be judged by its target.
pub fn path_is_ascii(path: &Path) -> bool {
    crate::policy::resolved_path(path)
        .as_os_str()
        .as_encoded_bytes()
        .is_ascii()
}

/// Map ticket 68, build ticket 78. Refuse when either root the compiler folds
/// into denials and scopes is not ASCII-only.
///
/// Known residual: the gate folds a third path, the launching repo's `.git`
/// (`Spawn::parent_git_dir`, which `src/policy/mod.rs` adds to the denied
/// write paths and the scopes), and it is not checked here. Map ticket 68
/// decided the two Hydra roots. Whether a launch from a repo under a non-ASCII
/// path is refused is still open.
pub fn refuse_non_ascii_roots(layout: &Layout, checkout: &Path) -> Result<()> {
    refuse_non_ascii_root("data root", layout.root())?;
    refuse_non_ascii_root("checkout", checkout)?;
    Ok(())
}

fn refuse_non_ascii_root(label: &str, path: &Path) -> Result<()> {
    if path_is_ascii(path) {
        return Ok(());
    }
    bail!(
        "the {label} {path} contains a byte outside ASCII, and the gate folds case in ASCII only \
         (map ticket 68). Every deny entry and scope under that root would miss the other case on \
         macOS.",
        path = path.display()
    );
}

pub struct Layout {
    root: PathBuf,
}

/// Whether a string is a git object name and therefore safe to `join`.
///
/// Hex and nothing else, any length git might use: 40 today, 64 the day a
/// repository is sha256. It rules out `/`, `..` and every other spelling that
/// escapes the directory the caller meant to stay inside.
fn is_object_name(sha: &str) -> bool {
    !sha.is_empty() && sha.chars().all(|c| c.is_ascii_hexdigit())
}

impl Layout {
    /// `~/hydra`, from the real home rather than from `HOME`.
    ///
    /// Harness spec 6.2: a wrong `HOME` logs a provider out silently, so
    /// nothing here trusts the variable. [`crate::env::real_home`] reads the
    /// passwd entry.
    ///
    /// **`HYDRA_ROOT` overrides it, and it exists for one test file.** Every other
    /// test builds a [`Layout::at`] a temporary directory and calls the library
    /// directly; `tests/face.rs` cannot, because it drives the built binary as a
    /// subprocess and the binary is what calls this. The variable is read here and
    /// nowhere else, it never reaches a child provider, because 6.2 builds that
    /// environment from an allowlist this name is not on, and a human who sets it
    /// has moved their whole `~/hydra` rather than half of it.
    pub fn discover() -> Result<Self> {
        if let Some(root) = std::env::var_os("HYDRA_ROOT") {
            return Ok(Self::at(PathBuf::from(root)));
        }
        let home = crate::env::real_home().context("cannot locate the home directory")?;
        Ok(Self::at(home.join("hydra")))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn runs(&self) -> PathBuf {
        self.root.join("runs")
    }

    pub fn run_file(&self, run_id: &str) -> PathBuf {
        self.runs().join(format!("{run_id}.jsonl"))
    }

    pub fn worktrees(&self) -> PathBuf {
        self.root.join("worktrees")
    }

    pub fn worktree(&self, run_id: &str) -> PathBuf {
        self.worktrees().join(run_id)
    }

    /// `~/hydra/skills/`, the `skill_store` scope and the canonical store.
    ///
    /// [`crate::skills::Store::open`] creates it as a git repo of its own, living
    /// inside the `~/hydra` working tree and ignored by it (6.4).
    ///
    /// **The `skill_store` scope on every spawn, and a path no policy writes.**
    /// Harness spec 7.5 denies it under all five since ticket 40, so its only
    /// writers are [`crate::skills::Store::pin`] and a human. A curator proposes
    /// into [`Layout::skill_checkout`] instead.
    pub fn skills(&self) -> PathBuf {
        self.root.join("skills")
    }

    /// `~/hydra/skills-snapshots/`, harness spec 5's second row.
    pub fn snapshots(&self) -> PathBuf {
        self.root.join("skills-snapshots")
    }

    /// One immutable checkout of the store, keyed by the sha a run pinned.
    ///
    /// **Keyed by sha rather than by run id**, which is what makes it shared: two
    /// runs on one sha see identical bytes because they are looking at the same
    /// bytes. It is reaped when no live run references it, and
    /// [`crate::skills::referenced_shas`] is how that is asked.
    pub fn snapshot(&self, sha: &str) -> PathBuf {
        self.snapshots().join(sha)
    }

    /// Where a curator pass checks the skill store out, one directory per run.
    ///
    /// **A sixth place, and the design names four.** Rungs spec 3.6 gives the
    /// curator a git worktree of the store rather than the store's own working
    /// tree, and 9.1 leaves where it lives open. It is a sibling of `worktrees/`
    /// rather than a file in the spawn directory because `hydra reap` removes that
    /// directory whole: a pass whose run outlived its call leaves an uncommitted
    /// proposal here, and work is not something a cleanup command should be able
    /// to take. `hydra curate` creates one, and `hydra reap` removes one after
    /// committing what it holds to `rungs/curate/<run-id>` (ticket 40).
    pub fn skill_worktrees(&self) -> PathBuf {
        self.root.join("skill-worktrees")
    }

    /// Where one run's `skill_checkout` scope points, rungs spec 3.6.
    ///
    /// A curate pass writes into its own checkout of the store and every other
    /// policy is given none, which is what makes the carve-out of harness spec
    /// 7.5 per spawn rather than per policy name.
    ///
    /// **It is a function of the run id so that a reader can recompute it.**
    /// Contract 7.7 re-evaluates the run's policy against every recorded call,
    /// and a scope that lived only in the compiled gate would make every curate
    /// run read back as a run whose gate leaked on its own writes. No field on
    /// the record carries a scope, and inventing one is a contract change; a
    /// rule both sides derive costs nothing and cannot drift.
    pub fn skill_checkout(&self, run_id: &str) -> PathBuf {
        self.skill_worktrees().join(run_id)
    }

    /// Where one run's `skill_snapshot` scope points, harness spec 6.4 and 7.2.
    ///
    /// The snapshot the run pinned, which is what its injected links resolve to,
    /// so a read through a link lands inside the scope. Keyed by sha rather than
    /// by run id for the same reason [`Layout::snapshot`] is.
    ///
    /// **A run with no store sha gets a path nothing creates**, the way a run
    /// with no repo gets `no-worktree` for its `worktree` scope. Pointing at
    /// `skills-snapshots/` instead would hand the run read access to every other
    /// run's snapshot, and a run that was linked nothing has nothing to read.
    ///
    /// **It is a function of the run id and the sha so that a reader can
    /// recompute it.** Contract 7.7 re-evaluates the run's policy against every
    /// recorded call, and `hydra.skills.store_sha` is the field it reads. No
    /// field on the record carries a scope, and inventing one is a contract
    /// change; a rule both sides derive costs nothing and cannot drift.
    ///
    /// **A sha that is not a sha resolves to no snapshot at all.** On the read
    /// side the value comes off a file, the validator only checks that it is a
    /// string, and `join` with an absolute path replaces everything before it:
    /// a record carrying `/` would compile a replay gate whose read scope is the
    /// whole filesystem, so every denial in it would re-derive as an allow and
    /// a real leak would go unreported. Anything but hex therefore falls back to
    /// the same path an unpinned run gets, which fails toward a run that could
    /// read nothing rather than one that could read everything.
    pub fn skill_snapshot_scope(&self, run_id: &str, store_sha: Option<&str>) -> PathBuf {
        match store_sha {
            Some(sha) if is_object_name(sha) => self.snapshot(sha),
            _ => self.spawn(run_id).join("no-snapshot"),
        }
    }

    pub fn spawns(&self) -> PathBuf {
        self.root.join("spawn")
    }

    /// `~/hydra/approvals/`, where a seat request no shape names waits for
    /// `hydra approve`, map ticket 82 answer 4 and build ticket 141.
    ///
    /// **Denied to every gated run, reads and writes.** A run that writes here
    /// approves its own request, and one that reads here reads other sessions'
    /// tasks. The launcher, outside the seatbelt, writes a request in; the
    /// human's `hydra approve` renames it; nothing else touches it.
    pub fn approvals(&self) -> PathBuf {
        self.root.join("approvals")
    }

    /// `~/hydra/puppets/`, where Master of Puppets keeps one append-only JSONL
    /// per goal, map ticket 76 decision B.
    ///
    /// **Outside every worktree and denied to every gated run.** A run that can
    /// rewrite its own assignment or approval walks around the mode, so the log
    /// is data a run may read if its scope reaches it and never write.
    pub fn puppets(&self) -> PathBuf {
        self.root.join("puppets")
    }

    /// Where `hydra prune` moves a run file to.
    ///
    /// **A fifth place, and the design names four.** Harness spec 5's table says
    /// prune "moves rather than deletes" and stops there, so the destination is
    /// this build's choice, the same gap 19.1 leaves for the filenames inside the
    /// spawn directory. A sibling of `runs/` keeps the move a rename on one
    /// filesystem and keeps D9 true, and the run id stays on the file, so putting
    /// a record back is a `mv` and nothing else.
    pub fn pruned(&self) -> PathBuf {
        self.root.join("pruned")
    }

    pub fn pruned_file(&self, run_id: &str) -> PathBuf {
        self.pruned().join(format!("{run_id}.jsonl"))
    }

    /// Where a pending launch holds its slot under the cap, build ticket 81.
    ///
    /// A sibling of `spawn/` rather than a child of it. `hydra reap` reads every
    /// directory under `spawn/` as a run id and removes the ones with no
    /// checkout, which would take pending reservations and their lock with it.
    pub fn reservations(&self) -> PathBuf {
        self.root.join("reservations")
    }

    /// `~/hydra/spawn/<run-id>/`, created per run and reaped with the worktree.
    pub fn spawn(&self, run_id: &str) -> PathBuf {
        self.spawns().join(run_id)
    }

    /// The compiled gate, 7.5. Outside the worktree it guards, which is what
    /// gives `permissive` and `curate` a base case.
    pub fn gate_script(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("gate")
    }

    /// Where the seed snapshot of 6.3 stages, so the run's own index is left
    /// alone. It holds a git index and never a path the run is told about.
    ///
    /// It is in the spawn directory because that is what ticket 06 reaps beside
    /// the worktree, and the gate protects only the gate script inside it. Under
    /// four of the five policies writes are scoped to the worktree and a run
    /// cannot reach this; under `permissive` it can, and a `permissive` run that
    /// rewrites its own index makes the diff stat wrong and nothing else. The
    /// record is what carries the truth about what a run wrote.
    pub fn seed_index(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("seed-index")
    }

    /// The scratch index the closing diff stat stages into. A second file rather
    /// than a reuse of [`Layout::seed_index`], which staging would overwrite.
    pub fn stat_index(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("stat-index")
    }

    /// The one git object directory a run with a checkout may write, map
    /// ticket 74. The run's own git is pointed here by environment and reads
    /// the parent's store as an alternate; `hydra reap` moves what git verifies
    /// out of it into the parent, and it goes with the spawn directory.
    pub fn run_objects(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("objects")
    }

    /// What `hydra run` hands the detached half of the launch, harness spec 9.
    ///
    /// It is read once, before the record exists, and every fact it carries is
    /// copied onto line 0. Nothing reads it to interpret a run, which is the
    /// test harness spec 5 puts on anything living in this directory.
    pub fn launch_spec(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("launch.json")
    }

    /// The marker `hydra stop` writes before it signals a run, build ticket 169.
    ///
    /// In the spawn directory beside [`Layout::launch_spec`], because that is
    /// the one path the watcher knows, and because every policy denies writes
    /// under `spawn/`, so a run cannot mark itself or a sibling stopped.
    pub fn stop_marker(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join(crate::watch::STOP_MARKER)
    }

    /// Where the detached half's own stdout and stderr go.
    ///
    /// Diagnostics, never a source. Anything about the run that matters is on the
    /// record; this is where a message from Hydra's own code lands when there is
    /// no terminal left to print it to.
    pub fn watcher_log(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("watcher.log")
    }

    /// The hook config `--settings` points at, 6.6.
    ///
    /// The filename is this build's choice. Harness spec 19.1 leaves the names
    /// inside the spawn directory open, and 6.6's table writes
    /// `~/hydra/spawn/<run-id>/settings.json`, so the name comes from the table
    /// rather than from a ticket.
    pub fn settings_file(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("settings.json")
    }

    /// The MCP config `--mcp-config` points at, 6.5.
    ///
    /// The name comes from 6.5's own table rather than from a ticket, the same
    /// way `settings.json` comes from 6.6's. It is a `claude` path: the other two
    /// providers read their MCP config out of the worktree, which is 6.5's
    /// deliberate asymmetry rather than an oversight here.
    pub fn mcp_file(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("mcp.json")
    }

    /// Cursor's data root for one run, `CURSOR_DATA_DIR`, 6.2 and 6.5.
    ///
    /// A subdirectory of the spawn directory rather than the directory itself,
    /// which harness spec 19.1 flags as a refinement over ticket 29: the vendor
    /// writes `projects/<slug>/` under this root, and keeping that out of the
    /// directory that holds the gate script keeps "the script lives where
    /// nothing else writes" true at no cost. Reaped with the rest of the spawn
    /// directory.
    pub fn cursor_data_dir(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("cursor-data")
    }

    /// Creates the directories this slice writes into. Never the worktree itself,
    /// which `git worktree add` insists on creating.
    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(self.runs())
            .with_context(|| format!("creating {}", self.runs().display()))?;
        std::fs::create_dir_all(self.worktrees())
            .with_context(|| format!("creating {}", self.worktrees().display()))?;
        std::fs::create_dir_all(self.spawns())
            .with_context(|| format!("creating {}", self.spawns().display()))?;
        std::fs::create_dir_all(self.reservations())
            .with_context(|| format!("creating {}", self.reservations().display()))?;
        for directory in [
            self.pruned(),
            self.puppets(),
            self.approvals(),
            self.skills(),
            self.snapshots(),
            self.skill_worktrees(),
        ] {
            std::fs::create_dir_all(&directory)
                .with_context(|| format!("creating {}", directory.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydra_home_reads_the_environment_variable_before_the_compile_time_path() {
        let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            hydra_home_from(Some("/somewhere/else".into())),
            PathBuf::from("/somewhere/else")
        );
        assert_eq!(hydra_home_from(None), checkout);
        // An empty value is a variable somebody exported and never set, and
        // resolving it would name the filesystem root. It reads as unset.
        assert_eq!(hydra_home_from(Some("".into())), checkout);
    }

    #[cfg(unix)]
    #[test]
    fn an_ascii_symlink_to_a_non_ascii_directory_is_refused() {
        let scratch = std::env::temp_dir().join(format!(
            "hydra-ascii-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let target = scratch.join("Café");
        std::fs::create_dir_all(&target).unwrap();
        let link = scratch.join("hydra-link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(link.as_os_str().as_encoded_bytes().is_ascii());
        assert!(!path_is_ascii(&link));
        let err = refuse_non_ascii_roots(&Layout::at(&link), Path::new("/tmp/checkout"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("data root"), "{err}");
        assert!(err.contains("map ticket 68"), "{err}");

        std::fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn a_path_outside_ascii_is_not_ascii() {
        assert!(path_is_ascii(Path::new("/tmp/hydra")));
        assert!(!path_is_ascii(Path::new("/tmp/Café")));
        assert!(!path_is_ascii(Path::new("/tmp/Straße")));
    }

    #[test]
    fn a_non_ascii_data_root_is_refused_and_an_ascii_one_is_not() {
        let ascii = Layout::at("/tmp/hydra-ascii-root");
        refuse_non_ascii_roots(&ascii, Path::new("/tmp/checkout")).unwrap();

        let non_ascii = Layout::at("/tmp/Café");
        let err = refuse_non_ascii_roots(&non_ascii, Path::new("/tmp/checkout")).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("data root"), "{message}");
        assert!(message.contains("/tmp/Café"), "{message}");
        assert!(message.contains("map ticket 68"), "{message}");
    }

    #[test]
    fn a_non_ascii_checkout_is_refused_and_an_ascii_one_is_not() {
        let layout = Layout::at("/tmp/hydra");
        refuse_non_ascii_roots(&layout, Path::new("/tmp/checkout")).unwrap();

        let err = refuse_non_ascii_roots(&layout, Path::new("/tmp/Café")).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("checkout"), "{message}");
        assert!(message.contains("/tmp/Café"), "{message}");
        assert!(message.contains("map ticket 68"), "{message}");
    }

    #[test]
    fn a_store_sha_that_is_not_a_sha_resolves_to_no_snapshot() {
        // Contract 7.7 reads this value off a record, and `join` with an
        // absolute path replaces everything before it. A replay gate whose read
        // scope is `/` re-derives every denial as an allow.
        let layout = Layout::at("/tmp/hydra-test");
        let real = "1f0dcafe1f0dcafe1f0dcafe1f0dcafe1f0dcafe";
        assert_eq!(
            layout.skill_snapshot_scope("r1", Some(real)),
            layout.snapshot(real)
        );
        let none = layout.skill_snapshot_scope("r1", None);
        for bad in ["/", "..", "../../etc", "a/b", "", "not-hex"] {
            assert_eq!(
                layout.skill_snapshot_scope("r1", Some(bad)),
                none,
                "{bad:?} reached the filesystem as a path component"
            );
        }
    }
}
