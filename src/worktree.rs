//! One git worktree per recorded run, harness spec 6.3.
//!
//! Branch named from the run id, when the cwd is a repo. The spawn seeds it with
//! the parent's uncommitted changes, because a clean checkout would hide exactly
//! the work in progress most delegations are about. Two children editing one file
//! with neither aware of the other is the most likely way delegation produces
//! garbage, and this is what prevents it.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub struct Worktree {
    pub path: PathBuf,
    pub branch: String,
}

/// The repository root containing `dir`, or `None` when `dir` is not in one.
pub fn repo_root(dir: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!root.is_empty()).then(|| PathBuf::from(root))
}

/// Creates the run's worktree and seeds it from the parent's working tree.
pub fn create(repo: &Path, dest: &Path, branch: &str) -> Result<Worktree> {
    if !head_exists(repo) {
        bail!(
            "{} has no commits, so there is nothing to branch a worktree from",
            repo.display()
        );
    }

    git(
        repo,
        &[
            "worktree",
            "add",
            "-b",
            branch,
            &dest.to_string_lossy(),
            "HEAD",
        ],
    )
    .with_context(|| format!("creating a worktree at {}", dest.display()))?;

    seed(repo, dest).with_context(|| "seeding the worktree with uncommitted changes")?;

    Ok(Worktree {
        path: dest.to_path_buf(),
        branch: branch.to_string(),
    })
}

/// Tracked changes as a patch, then the untracked files git would report.
///
/// Ignored files are left behind on purpose: they are `target/`, `runs/` and the
/// rest of what a build produces, and copying them would put a parent's run records
/// inside a child's checkout.
fn seed(repo: &Path, dest: &Path) -> Result<()> {
    let patch = git_bytes(repo, &["diff", "HEAD", "--binary"])?;
    if !patch.is_empty() {
        apply_patch(dest, &patch)?;
    }

    let listing = git_bytes(repo, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    for name in listing.split(|b| *b == 0) {
        if name.is_empty() {
            continue;
        }
        let relative = PathBuf::from(String::from_utf8_lossy(name).to_string());
        let from = repo.join(&relative);
        let to = dest.join(&relative);
        if !from.is_file() {
            continue;
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&from, &to)
            .with_context(|| format!("copying untracked {}", relative.display()))?;
    }
    Ok(())
}

fn apply_patch(dest: &Path, patch: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("git")
        .args(["apply", "--whitespace=nowarn", "-"])
        .current_dir(dest)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(patch)?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "git apply refused the parent's uncommitted changes: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn head_exists(repo: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repo)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git").args(args).current_dir(repo).output()?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn git_bytes(repo: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("git").args(args).current_dir(repo).output()?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(out.stdout)
}
