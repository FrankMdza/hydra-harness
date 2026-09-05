//! The `~/hydra` layout.
//!
//! Harness spec section 5 names four places Hydra writes. This slice creates two:
//! the run file and the worktree. There is no spawn directory, because a spawn
//! directory holds a gate script, an MCP config and Cursor's data root, and this
//! slice writes none of the three.
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

    /// Creates the directories this slice writes into. Never the worktree itself,
    /// which `git worktree add` insists on creating.
    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(self.runs())
            .with_context(|| format!("creating {}", self.runs().display()))?;
        std::fs::create_dir_all(self.worktrees())
            .with_context(|| format!("creating {}", self.worktrees().display()))?;
        Ok(())
    }
}
