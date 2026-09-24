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

/// A checkout bound to metadata supplied by Hydra, never its writable git file.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CheckoutGit {
    path: PathBuf,
    common: PathBuf,
    private: PathBuf,
}

impl CheckoutGit {
    pub fn common_dir(&self) -> &Path {
        &self.common
    }

    /// Replace the writable pointer without following a symlink planted there.
    /// Git's parent-side worktree removal validates this pointer too.
    pub fn restore_pointer(&self) -> Result<()> {
        use std::io::Write;
        let path = self.path.join(".git");
        match std::fs::remove_file(&path) {
            Ok(()) => (),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
            Err(err) => return Err(err.into()),
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        writeln!(file, "gitdir: {}", self.private.display())?;
        Ok(())
    }

    /// Remove the checkout's private `logs/` before a commit made outside the
    /// seatbelt. A checkout launched before build ticket 95 could write there,
    /// and a link planted as `logs` or inside it would take the reflog line to
    /// any file it names. The link is removed, never followed; the reflog of a
    /// checkout about to be removed is worth nothing.
    pub fn clear_private_logs(&self) -> Result<()> {
        let logs = self.private.join("logs");
        match std::fs::symlink_metadata(&logs) {
            Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(&logs)?,
            Ok(_) => std::fs::remove_file(&logs)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
            Err(err) => return Err(err.into()),
        }
        Ok(())
    }

    pub fn new(path: &Path, common: &Path) -> Result<Self> {
        let name = path.file_name().context("checkout has no directory name")?;
        Ok(Self {
            path: path.to_path_buf(),
            common: common.to_path_buf(),
            private: common.join("worktrees").join(name),
        })
    }

    /// Launch data beside the checkout survives record and spawn pruning and is
    /// outside the run's writable directory.
    pub fn for_run(run_id: &str, path: &Path, start: Option<&serde_json::Value>) -> Result<Self> {
        if let Some(common) = start.and_then(|s| s["scopes"]["parent_git_dir"].as_str()) {
            return Self::new(path, Path::new(common));
        }
        let saved = path.with_extension("git-context.json");
        let context: Self = serde_json::from_slice(&std::fs::read(&saved).with_context(|| {
            format!(
                "no trusted git directory for {run_id}: its record has no \
                     `scopes.parent_git_dir` and {} is missing, so Hydra will not run git \
                     in a checkout whose own `.git` file the run could write. Inspect {} by \
                     hand, then remove it with `git -C <parent repo> worktree remove`",
                saved.display(),
                path.display()
            )
        })?)?;
        anyhow::ensure!(
            context.path == path,
            "saved Git checkout path does not match"
        );
        Ok(context)
    }
}

/// Parent repositories may discover metadata and keep the human's own hooks.
/// Run checkouts use CheckoutGit.
///
/// **Neither reads global or system configuration.** `~/.gitconfig` is a file
/// a `standard` run can write by shell, and the system file is
/// `/opt/homebrew/etc/gitconfig` on this Mac, owned by the same user. A filter,
/// an external diff or a `core.hooksPath` defined there would run from every
/// git Hydra starts outside the seatbelt, reap's `git add` included: the third
/// hiring panel's H5. So [`GitLocation::command`] sets
/// `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`. The repository's
/// own `config` is still read, and a run cannot write it, because the shared
/// git directory is denied. The one value Hydra still takes from the wider
/// configuration is who commits, read as data by [`identity`] and passed with
/// `-c` by [`git_as_author`].
pub trait GitLocation {
    fn path(&self) -> &Path;

    /// `git` here reading every level of configuration. Only [`identity`]
    /// uses it, and `git config --get` runs no program a value names.
    fn configured(&self) -> Command {
        let mut command = Command::new("git");
        command.current_dir(self.path());
        command
    }

    fn command(&self) -> Command {
        let mut command = self.configured();
        command
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1");
        command
    }
}

impl GitLocation for Path {
    fn path(&self) -> &Path {
        self
    }
}

impl GitLocation for PathBuf {
    fn path(&self) -> &Path {
        self
    }
}

impl GitLocation for CheckoutGit {
    fn path(&self) -> &Path {
        &self.path
    }
    fn configured(&self) -> Command {
        let mut command = self.path.as_path().configured();
        // A belt on top of the pinned directories: nothing a run could have
        // configured runs a hook or a monitor from a git Hydra starts.
        command
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
            ])
            .env("GIT_DIR", &self.private)
            .env("GIT_COMMON_DIR", &self.common)
            .env("GIT_WORK_TREE", &self.path);
        command
    }
}

/// `user.name` and `user.email` as git would resolve them at `repo` with every
/// level of configuration read, as `-c` arguments. Empty when neither is set,
/// and git then falls back to its own guess, as it did before.
///
/// **Read as data, never as configuration.** A run that rewrites `~/.gitconfig`
/// can change the name on a commit Hydra makes of that run's work, and nothing
/// else.
pub fn identity<G: GitLocation + ?Sized>(repo: &G) -> Vec<String> {
    let mut args = Vec::new();
    for key in ["user.name", "user.email"] {
        let Ok(out) = repo.configured().args(["config", "--get", key]).output() else {
            continue;
        };
        let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && !value.is_empty() {
            args.push("-c".to_string());
            args.push(format!("{key}={value}"));
        }
    }
    args
}

/// [`git`] for a command that writes a commit, with [`identity`] in front of
/// it, because the configuration the commit would have read it from is
/// ignored.
pub(crate) fn git_as_author<G: GitLocation + ?Sized>(repo: &G, args: &[&str]) -> Result<String> {
    let identity = identity(repo);
    let mut full: Vec<&str> = identity.iter().map(String::as_str).collect();
    full.extend(args);
    git(repo, &full)
}

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
    let out = dir
        .command()
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!root.is_empty()).then(|| PathBuf::from(root))
}

/// The `.git` directory the repository containing `dir` keeps its refs in, or
/// `None` when `dir` is not in a repository or is no longer there.
///
/// **Not `<repo>/.git`.** Inside a linked worktree that name is a file holding a
/// pointer, and the directory it points at is the one every checkout of the repo
/// shares. Every run's branch is in it, and on a machine where the parent is
/// Hydra's own checkout so is Hydra's history. That shared directory is what
/// harness spec 7.5 denies, and it is what `--git-common-dir` answers with from
/// either side. From a main checkout the two are the same path.
pub fn git_common_dir(dir: &Path) -> Option<PathBuf> {
    let out = dir
        .command()
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let common = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!common.is_empty()).then(|| PathBuf::from(common))
}

/// Commits everything a checkout holds, and answers with the commit it made.
///
/// `None` when the tree was already clean, which is what tells a checkout
/// holding work from one holding none.
///
/// `exclude` is relative paths that are never committed. `hydra reap` and the
/// curate-store sweep pass Hydra's own files, the ones `provider::own_worktree_paths`
/// lists; any other caller passes an empty list and this is `git add -A` as it was.
///
/// **Excluding is done after staging, not with a pathspec.** `git add -A -- .
/// ':(exclude)<path>'` refuses the whole command when one of the named paths is
/// ignored, with "The following paths are ignored by one of your .gitignore
/// files", and on a real worktree they always are: Hydra drops the ignore file
/// that names them. Measured on 2026-09-08 on the first two reaps after build
/// ticket 30 landed. So the sequence is `git add -A`, then `git reset -- <path>...`
/// to take Hydra's own files back out of the index, which exits 0 whether or not
/// a path exists. The emptiness check lists every untracked file by name, since
/// the default porcelain collapses an untracked directory to one line that no
/// path list can match.
///
/// **Untracked files at those paths are then removed**, so a caller that runs
/// `git worktree remove` without `--force` is not stopped by files it was told
/// not to commit. A tracked file at one of those paths, a repo's own
/// `.cursor/.gitignore`, is left alone; so is an ignored one, since `git worktree
/// remove` does not count it as dirt.
///
/// **`--no-verify`**, because a pre-commit hook that refuses would leave the
/// caller unable to keep the work the checkout holds, and this commit is a
/// snapshot of what a directory held rather than a contribution anyone reviews.
///
/// Two callers, and both remove the checkout afterwards: `hydra reap` and the
/// curate pass's own `propose`. The removal is what differs between them, so it
/// stays with each.
pub fn commit_everything<G: GitLocation + ?Sized>(
    checkout: &G,
    message: &str,
    exclude: &[&str],
) -> Result<Option<String>> {
    if !dirty_outside(checkout, exclude)? {
        remove_untracked(checkout, exclude)?;
        return Ok(None);
    }
    git(checkout, &["add", "-A"])?;
    if !exclude.is_empty() {
        let mut args = vec!["reset", "-q", "--"];
        args.extend(exclude);
        git(checkout, &args)?;
    }
    git_as_author(checkout, &["commit", "-q", "--no-verify", "-m", message])?;
    remove_untracked(checkout, exclude)?;
    Ok(Some(
        git(checkout, &["rev-parse", "HEAD"])?.trim().to_string(),
    ))
}

/// Whether `git status` reports anything at a path not in `exclude`.
fn dirty_outside<G: GitLocation + ?Sized>(checkout: &G, exclude: &[&str]) -> Result<bool> {
    let status = git(
        checkout,
        &["status", "--porcelain", "-z", "--untracked-files=all"],
    )?;
    // `-z`: entries are NUL-terminated and never C-quoted, and a rename is
    // `XY new\0old\0`, two entries, where the new name is what would be
    // committed and the old one is skipped.
    let mut entries = status.split('\0').filter(|entry| !entry.is_empty());
    while let Some(entry) = entries.next() {
        let path = entry.get(3..).unwrap_or("");
        if entry.starts_with('R') || entry.starts_with('C') {
            entries.next();
        }
        if !excluded(path, exclude) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether a path `git status` printed is one of `exclude` or lies under one.
/// An injected skill copy is a directory, and with `--untracked-files=all`
/// status names the files inside it rather than the directory.
fn excluded(path: &str, exclude: &[&str]) -> bool {
    exclude.iter().any(|root| {
        path == *root
            || path
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Removes the untracked files at `exclude`, and only those: a path `git
/// ls-files` knows is the repo's own and stays.
///
/// **Every path is checked before anything is removed.** An entry that is empty,
/// `.`, absolute or holds `..` is skipped, since `git clean -fd -- .` would clean
/// the whole checkout. So is one reached through a symlink the run made, for
/// example `.codex` pointed at a directory outside the checkout: removing through
/// it would delete something reap was never told about. A symlink at the path
/// itself is removed as a link, never followed.
fn remove_untracked<G: GitLocation + ?Sized>(checkout: &G, exclude: &[&str]) -> Result<()> {
    for path in exclude {
        if !inside_checkout(checkout.path(), path) {
            continue;
        }
        let file = checkout.path().join(path);
        let Ok(meta) = std::fs::symlink_metadata(&file) else {
            continue;
        };
        if meta.is_dir() {
            // An injected `codex` skill is a directory, and so is a whole skill
            // root when reap could not read which skills were injected. `git
            // clean` without `-x` drops only what is untracked and not ignored,
            // and only under this path, so a tracked file there stays. The copy
            // is 0555 directories, and neither `git clean` here nor `git
            // worktree remove` after can unlink inside one, so the owner's write
            // bit comes back first.
            restore_write_bits(&file)?;
            git(checkout, &["clean", "-fd", "--", path])
                .with_context(|| format!("cleaning untracked files under {}", file.display()))?;
        } else if git(checkout, &["ls-files", "--", path])?.trim().is_empty() {
            std::fs::remove_file(&file).with_context(|| format!("removing {}", file.display()))?;
        }
    }
    Ok(())
}

/// Whether `path` names something strictly below `checkout` without passing
/// through a symlink on the way.
pub(crate) fn inside_checkout(checkout: &Path, path: &str) -> bool {
    let relative = Path::new(path);
    let mut parts = relative.components().peekable();
    if parts.peek().is_none() {
        return false;
    }
    let mut cursor = checkout.to_path_buf();
    while let Some(part) = parts.next() {
        let std::path::Component::Normal(name) = part else {
            return false;
        };
        cursor.push(name);
        // The last component may be a link, which is removed as one. Any link
        // before it would carry the removal somewhere else.
        if parts.peek().is_some()
            && std::fs::symlink_metadata(&cursor).is_ok_and(|meta| meta.file_type().is_symlink())
        {
            return false;
        }
    }
    true
}

/// Gives the owner write permission back on `dir` and every directory under
/// it, without following symlinks, so the entries inside can be unlinked.
#[cfg(unix)]
fn restore_write_bits(dir: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let meta = std::fs::symlink_metadata(dir)?;
    if !meta.is_dir() {
        return Ok(());
    }
    let mode = meta.permissions().mode();
    if mode & 0o200 == 0 {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode | 0o700))
            .with_context(|| format!("restoring write permission on {}", dir.display()))?;
    }
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        restore_write_bits(&entry?.path())?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn restore_write_bits(_dir: &Path) -> Result<()> {
    Ok(())
}

/// Creates the run's worktree and seeds it from the parent's working tree.
///
/// `index` is a scratch path for the seed snapshot, outside the checkout.
///
/// **A failure after `git worktree add` takes the checkout back out.** The
/// launch writes no record until this returns, so a checkout and a
/// `hydra/<run-id>` branch left behind would belong to no record, and only
/// `reap::orphan_checkouts` would ever find them: the third hiring panel's H39.
/// If the rollback fails as well, the error names both failures.
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

    let context = match bind_and_seed(repo, dest) {
        Ok(context) => context,
        Err(err) => {
            return Err(match roll_back(repo, dest, branch) {
                Ok(()) => err,
                Err(undo) => err.context(format!(
                    "and the checkout at {} and branch {branch} could not be removed: {undo:#}",
                    dest.display()
                )),
            });
        }
    };

    // A snapshot that will not write costs the diff stat and nothing else, so it
    // does not refuse the run. It says so rather than losing the field quietly.
    let seed_tree = match seed_tree(&context, index) {
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

/// Every fallible step [`create`] takes after `git worktree add`.
fn bind_and_seed(repo: &Path, dest: &Path) -> Result<CheckoutGit> {
    let common = git_common_dir(repo).context("parent has no Git directory")?;
    let context = CheckoutGit::new(dest, &common)?;
    std::fs::write(
        dest.with_extension("git-context.json"),
        serde_json::to_vec(&context)?,
    )?;
    seed(repo, &context).with_context(|| "seeding the worktree with uncommitted changes")?;
    Ok(context)
}

/// Undoes `git worktree add -b` and the context file beside it. Nothing has
/// run in the checkout yet, so `--force` discards only what the seed copied.
fn roll_back(repo: &Path, dest: &Path, branch: &str) -> Result<()> {
    git(
        repo,
        &["worktree", "remove", "--force", &dest.to_string_lossy()],
    )?;
    git(repo, &["branch", "-D", branch])?;
    let saved = dest.with_extension("git-context.json");
    match std::fs::remove_file(&saved) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("removing {}", saved.display())),
    }
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
pub fn seed_tree<G: GitLocation + ?Sized>(worktree: &G, index: &Path) -> Result<String> {
    if let Some(parent) = index.parent() {
        std::fs::create_dir_all(parent)?;
    }
    git_with_index(worktree, index, &["add", "-A"])?;
    Ok(git_with_index(worktree, index, &["write-tree"])?
        .trim()
        .to_string())
}

/// The tree an index already holds, without staging anything into it.
///
/// **This is how a run recovers its own diff-stat base after the fact.** The sha
/// [`create`] returned lives in one process's memory, and a detached run returns
/// to a caller that never had it. `~/hydra/spawn/<run-id>/seed-index` is the index
/// the seed snapshot staged into and nothing writes to it again, so `write-tree`
/// against it answers with the same sha `seed_tree` answered with at launch.
/// Persisting the sha beside it would be a second source of truth for a fact the
/// index already carries.
pub fn tree_of_index<G: GitLocation + ?Sized>(worktree: &G, index: &Path) -> Result<String> {
    if !index.is_file() {
        bail!("{} does not exist", index.display());
    }
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
pub fn diff_stat<G: GitLocation + ?Sized>(worktree: &G, index: &Path, base: &str) -> Result<Stat> {
    let head = seed_tree(worktree, index)?;
    diff_tree_stat(worktree, base, &head)
}

/// Measures two captured trees without staging the worktree again.
pub fn diff_tree_stat<G: GitLocation + ?Sized>(
    worktree: &G,
    base: &str,
    head: &str,
) -> Result<Stat> {
    let numstat = git(worktree, &["diff-tree", "-r", "--numstat", base, head])?;
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
fn seed(repo: &Path, dest: &CheckoutGit) -> Result<()> {
    // No external diff and no textconv: either turns the patch into something
    // `git apply` cannot read, and both are programs a configuration names.
    let patch = git_bytes(
        repo,
        &["diff", "--no-ext-diff", "--no-textconv", "HEAD", "--binary"],
    )?;
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
        let to = dest.path().join(&relative);
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

fn apply_patch(dest: &CheckoutGit, patch: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = dest
        .command()
        .args(["apply", "--whitespace=nowarn", "-"])
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
    repo.command()
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// `git` in `repo`, stdout on success and stderr in the error.
///
/// Crate-visible because `hydra reap` runs the same three-line shape and a second
/// copy of it would be a second place to get the stderr handling wrong.
pub(crate) fn git<G: GitLocation + ?Sized>(repo: &G, args: &[&str]) -> Result<String> {
    let out = repo.command().args(args).output()?;
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
fn git_with_index<G: GitLocation + ?Sized>(
    repo: &G,
    index: &Path,
    args: &[&str],
) -> Result<String> {
    let out = repo
        .command()
        .args(args)
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

fn git_bytes<G: GitLocation + ?Sized>(repo: &G, args: &[&str]) -> Result<Vec<u8>> {
    let out = repo.command().args(args).output()?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(out.stdout)
}

#[cfg(test)]
mod tests {
    use super::inside_checkout;

    #[test]
    fn an_exclude_entry_that_names_the_checkout_or_leaves_it_is_skipped() {
        let checkout = std::env::temp_dir().join(format!("hydra-inside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&checkout);
        std::fs::create_dir_all(checkout.join(".claude")).unwrap();
        for path in ["", ".", "./", "..", "../x", "/etc", ".claude/../.."] {
            assert!(
                !inside_checkout(&checkout, path),
                "{path:?} was let through"
            );
        }
        assert!(inside_checkout(&checkout, ".claude/skills"));

        // A run that points `.codex` outside the checkout takes `.codex/skills`
        // out of reach, and the link itself is still a path reap may remove.
        let outside = checkout.with_extension("outside");
        std::fs::create_dir_all(outside.join("skills")).unwrap();
        std::os::unix::fs::symlink(&outside, checkout.join(".codex")).unwrap();
        assert!(!inside_checkout(&checkout, ".codex/skills"));
        assert!(inside_checkout(&checkout, ".codex"));

        std::fs::remove_dir_all(&checkout).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }
}
