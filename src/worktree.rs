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
    /// The tree the checkout held once seeding finished, [`seed_tree`]'s answer.
    ///
    /// **Contract 17's diff stat is measured against this and not against
    /// `HEAD`.** The seed puts the parent's uncommitted work in the checkout, so
    /// `git diff HEAD` inside it counts the human's work in progress as the
    /// child's. `None` means `git write-tree` would not answer, which costs the
    /// diff stat and nothing else.
    pub seed_tree: Option<String>,
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
///
/// `index` is a scratch path for the seed snapshot, outside the checkout.
pub fn create(repo: &Path, dest: &Path, branch: &str, index: &Path) -> Result<Worktree> {
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

    // A snapshot that will not write costs the diff stat and nothing else, so it
    // does not refuse the run. It says so rather than losing the field quietly.
    let seed_tree = match seed_tree(dest, index) {
        Ok(tree) => Some(tree),
        Err(err) => {
            eprintln!("hydra: no diff stat for this run, the seed snapshot failed: {err:#}");
            None
        }
    };

    Ok(Worktree {
        path: dest.to_path_buf(),
        branch: branch.to_string(),
        seed_tree,
    })
}

/// The tree the worktree holds right now, written into the object database.
///
/// **It stages into an index of the caller's choosing and never the worktree's
/// own.** `git add -A` against `.git/index` would leave the child looking at a
/// fully staged checkout, and a child asked to finish work in progress would see
/// a tree that does not match what the parent has. The snapshot has to be
/// invisible to the run it measures.
///
/// Ignored files are outside it, because `add -A` honours `.gitignore` and the
/// seed skipped them for the same reason: they are `target/` and the rest of what
/// a build produces.
pub fn seed_tree(worktree: &Path, index: &Path) -> Result<String> {
    if let Some(parent) = index.parent() {
        std::fs::create_dir_all(parent)?;
    }
    git_with_index(worktree, index, &["add", "-A"])?;
    Ok(git_with_index(worktree, index, &["write-tree"])?
        .trim()
        .to_string())
}

/// What changed in the worktree since `base`, contract 17's diff stat.
///
/// The comparison is tree against tree rather than a working-tree diff, which is
/// what lets it exclude the seed: `base` is what the checkout held before the
/// child touched it.
///
/// The index is a scratch file the caller names, and it has to be a different one
/// from the seed's: staging into that path would destroy the base this measures
/// against. So it starts cold and rehashes every file in the checkout. Measured on
/// Hydra's own repo, 123 tracked files, that is 10 to 50ms. A repo big enough for
/// it to matter can be handed a copy of the seed index instead, which carries the
/// stat cache, and nothing here has to change for that.
pub fn diff_stat(worktree: &Path, index: &Path, base: &str) -> Result<Stat> {
    let head = seed_tree(worktree, index)?;
    let numstat = git(worktree, &["diff-tree", "-r", "--numstat", base, &head])?;
    Ok(Stat::parse(&numstat))
}

/// Three numbers off `git diff-tree -r --numstat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stat {
    pub files_changed: usize,
    pub insertions: u64,
    pub deletions: u64,
}

impl Stat {
    /// One `<added>\t<deleted>\t<path>` line per file.
    ///
    /// **A binary file reports `-` for both counts and still counts as a file.**
    /// Dropping the row would report a run that replaced an image as having
    /// changed nothing, and inventing a line count for it would be worse.
    fn parse(numstat: &str) -> Self {
        let mut stat = Self::default();
        for line in numstat.lines() {
            let mut fields = line.split('\t');
            let (Some(added), Some(deleted)) = (fields.next(), fields.next()) else {
                continue;
            };
            if fields.next().is_none() {
                continue;
            }
            stat.files_changed += 1;
            stat.insertions += added.parse::<u64>().unwrap_or(0);
            stat.deletions += deleted.parse::<u64>().unwrap_or(0);
        }
        stat
    }
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

/// `git` with `GIT_INDEX_FILE` pointed somewhere the run cannot see.
fn git_with_index(repo: &Path, index: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_INDEX_FILE", index)
        .output()?;
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
