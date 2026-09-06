//! The `~/hydra` layout.
//!
//! Harness spec section 5 names four places Hydra writes. Three exist here: the
//! run file, the worktree and the spawn directory. The fourth is
//! `~/hydra/skills-snapshots/<sha>/`, which needs the skill store (ticket 07).
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

use anyhow::{Context, Result};

pub struct Layout {
    root: PathBuf,
}

impl Layout {
    /// `~/hydra`, from the real home rather than from `HOME`.
    ///
    /// Harness spec 6.2: a wrong `HOME` logs three providers out silently, so
    /// nothing here trusts the variable. [`crate::env::real_home`] reads the
    /// passwd entry.
    pub fn discover() -> Result<Self> {
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

    /// `~/hydra/skills/`, the `skill_store` scope. Nothing creates it yet: the
    /// store is ticket 07. The gate still needs the path, because a policy that
    /// scopes a write to a store that does not exist must deny rather than
    /// resolve to something else.
    pub fn skills(&self) -> PathBuf {
        self.root.join("skills")
    }

    /// Where a curator pass checks the skill store out, one directory per run.
    ///
    /// **A sixth place, and the design names four.** Ouroboros spec 3.6 gives the
    /// curator a git worktree of the store rather than the store's own working
    /// tree, and 9.1 leaves where it lives open. It is a sibling of `worktrees/`
    /// rather than a file in the spawn directory because `hydra reap` removes that
    /// directory whole: a pass whose run outlived its call leaves an uncommitted
    /// proposal here, and work is not something a cleanup command should be able
    /// to take. Nothing but `hydra curate` creates or removes one.
    pub fn skill_worktrees(&self) -> PathBuf {
        self.root.join("skill-worktrees")
    }

    pub fn skill_worktree(&self, run_id: &str) -> PathBuf {
        self.skill_worktrees().join(run_id)
    }

    /// Where one run's `skill_store` scope points.
    ///
    /// A curate pass writes into its own checkout of the store (ouroboros spec
    /// 3.6) and every other run gets the store itself, which no policy but
    /// `curate` grants anything on.
    ///
    /// **It is a function of the run id and the policy name so that a reader can
    /// recompute it.** Contract 7.7 re-evaluates the run's policy against every
    /// recorded call, and a scope that lived only in the compiled gate would make
    /// every curate run read back as a run whose gate leaked on its own writes.
    /// No field on the record carries a scope, and inventing one is a contract
    /// change; a rule both sides derive costs nothing and cannot drift.
    pub fn skill_store_scope(&self, run_id: &str, policy: &str) -> PathBuf {
        match policy {
            "curate" => self.skill_worktree(run_id),
            _ => self.skills(),
        }
    }

    pub fn spawns(&self) -> PathBuf {
        self.root.join("spawn")
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

    /// What `hydra run` hands the detached half of the launch, harness spec 9.
    ///
    /// It is read once, before the record exists, and every fact it carries is
    /// copied onto line 0. Nothing reads it to interpret a run, which is the
    /// test harness spec 5 puts on anything living in this directory.
    pub fn launch_spec(&self, run_id: &str) -> PathBuf {
        self.spawn(run_id).join("launch.json")
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

    /// Creates the directories this slice writes into. Never the worktree itself,
    /// which `git worktree add` insists on creating.
    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(self.runs())
            .with_context(|| format!("creating {}", self.runs().display()))?;
        std::fs::create_dir_all(self.worktrees())
            .with_context(|| format!("creating {}", self.worktrees().display()))?;
        std::fs::create_dir_all(self.spawns())
            .with_context(|| format!("creating {}", self.spawns().display()))?;
        Ok(())
    }
}
