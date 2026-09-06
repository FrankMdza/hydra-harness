//! `hydra reap`, harness spec 10 and 17.
//!
//! One worktree and one spawn directory go together, and the uncommitted work in
//! the checkout is committed to the child's own branch before either is touched.
//!
//! **The commit-first rule is enforced by git rather than by this module.** That
//! matters more than it sounds. Harness spec 10 calls reap the Hermes lesson
//! turned on our own tooling: Hermes told its model archiving was recoverable
//! while the tool underneath called `shutil.rmtree`. So the sequence here is built
//! so that a step which did not happen stops the next one. `git commit` leaves the
//! checkout clean, and only a clean checkout is one `git worktree remove` will
//! take without `--force`, which this never passes. A failed commit therefore
//! cannot be followed by a removal.
//!
//! **The branch survives whatever it carries, and the same guarantee is what
//! decides that.** `git branch -d` refuses a branch that is not fully merged, so
//! the commit reap just made is exactly what keeps the branch alive. A branch with
//! nothing on it is litter and goes; a branch holding a child's work stays and
//! `hydra status` stops mentioning its checkout. `--delete-branch` is the way past
//! that, and it is the same shape `hydra prune` gives deletion: recoverable by
//! default, destructive only when a human typed the flag. The Ouroboros spec's
//! section 4.11 already expects the leftover to be removed with `git branch -D`;
//! the flag is that sentence, typed through the command.
//!
//! **Nothing in this crate calls into here except `main`.** Harness spec 7.5 is
//! explicit that the gate does not keep a run away from `hydra reap`, because the
//! binary is reached through `shell` and any rule against it is an argv match with
//! the pattern floor's failure mode. What keeps Ouroboros away is that no
//! Ouroboros code path calls it, which is a property of Hydra's own source, and
//! `docs/ouroboros-spec.md`'s `evidence_is_read_only` checklist item is what holds
//! it true under a self-edit.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;

use crate::live::{RunView, State};
use crate::paths::Layout;
use crate::worktree::git;

/// What one run's reap did.
pub enum Reaped {
    /// The checkout, the branch and the spawn directory, as they were left.
    Done(Done),
    /// Nothing was touched on purpose. A live run is the ordinary reason.
    Left { run_id: String, why: String },
    /// Something was tried and would not go through. Nothing was lost: every step
    /// that can fail runs before the one that removes anything.
    Failed { run_id: String, why: String },
}

pub struct Done {
    pub run_id: String,
    /// The commit reap made, or `None` when the checkout was already clean.
    pub committed: Option<String>,
    /// The branch the commit landed on, and whether it is still there.
    pub branch: Option<String>,
    pub branch_removed: bool,
    /// Why the branch survived, in git's own words where git decided it.
    ///
    /// A boolean here told the human "it holds commits" whatever the reason, and
    /// the two reasons are not alike: a branch checked out in another worktree is
    /// refused by `-D` as well, so the flag would not help and the message would
    /// have been wrong twice.
    pub branch_kept: Option<String>,
    pub worktree_removed: bool,
    pub spawn_removed: bool,
}

/// Reaps one run, or every checkout no live run is using.
///
/// **The `None` case reads `hydra status`'s own list rather than the directory.**
/// [`crate::status::abandoned_worktrees`] derives it from the pid in each record,
/// and the one thing reap must never touch is a checkout whose run is still going.
///
/// **`grace` is an argument for the same reason it is one there.** A checkout is
/// created and seeded before its run file exists, so for seconds on a large repo a
/// launch in progress has no record and looks exactly like a run whose record was
/// pruned. `main` passes [`crate::run::LAUNCH_BUDGET`], which is what the launcher
/// itself waits for line 0, so the two cannot disagree. A reaper that passed zero
/// would eventually remove the checkout of a run that was about to start, and a
/// test that wants the race can build it.
///
/// **Naming a run id skips the grace entirely.** It is a launch window, and a
/// human who typed the id has answered the question it exists to ask.
pub fn reap(
    layout: &Layout,
    run_id: Option<&str>,
    delete_branch: bool,
    grace: std::time::Duration,
) -> Result<Vec<Reaped>> {
    let views = crate::live::scan(layout)?;

    if let Some(run_id) = run_id {
        return Ok(vec![one(layout, &views, run_id, delete_branch)]);
    }

    let mut ids: Vec<String> = crate::status::abandoned_worktrees(layout, &views, grace)?
        .iter()
        .filter_map(|path| path.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect();

    // A spawn directory whose checkout has gone, or which never had one. Harness
    // spec 5 reaps the pair, so a sweep that only walked `worktrees/` would leave
    // half of it behind: a run outside a repo gets a spawn directory and no
    // checkout at all. The same grace applies, because `hydra run` writes the
    // spawn directory before the detached half writes line 0.
    for orphan in orphan_spawns(layout, &views, grace)? {
        if !ids.contains(&orphan) {
            ids.push(orphan);
        }
    }
    ids.sort();

    Ok(ids
        .iter()
        .map(|id| one(layout, &views, id, delete_branch))
        .collect())
}

fn one(layout: &Layout, views: &[RunView], run_id: &str, delete_branch: bool) -> Reaped {
    let worktree = layout.worktree(run_id);

    if let Some(view) = views.iter().find(|view| view.run_id == run_id)
        && view.state == State::Live
    {
        return Reaped::Left {
            run_id: run_id.to_string(),
            why: match view.pid {
                Some(pid) => format!("the run is still going, pid {pid}"),
                None => "the run is still going".to_string(),
            },
        };
    }

    // Removing the directory this process is standing in leaves the shell in a
    // path that no longer exists, and the human reads the confusion as a bug in
    // reap. `hydra reap <run-id>` typed from somewhere else does it.
    if std::env::current_dir().is_ok_and(|cwd| cwd.starts_with(&worktree)) {
        return Reaped::Left {
            run_id: run_id.to_string(),
            why: format!("this shell is inside {}", worktree.display()),
        };
    }

    let mut done = Done {
        run_id: run_id.to_string(),
        committed: None,
        branch: None,
        branch_removed: false,
        branch_kept: None,
        worktree_removed: false,
        spawn_removed: false,
    };

    if worktree.is_dir() {
        match remove_checkout(&worktree, run_id, delete_branch, &mut done) {
            Ok(()) => {}
            Err(why) => {
                return Reaped::Failed {
                    run_id: run_id.to_string(),
                    why: format!("{why:#}"),
                };
            }
        }
    }

    // Harness spec 5: the two are reaped together, so this runs whether or not
    // there was a checkout. The three oldest runs on this machine predate the
    // spawn directory and have no pair; a run outside a repo has the pair the
    // other way round.
    let spawn = layout.spawn(run_id);
    if spawn.is_dir() {
        if let Err(err) = std::fs::remove_dir_all(&spawn) {
            return Reaped::Failed {
                run_id: run_id.to_string(),
                why: format!("removing {}: {err}", spawn.display()),
            };
        }
        done.spawn_removed = true;
    }

    Reaped::Done(done)
}

/// Commits what the checkout holds, then removes it and maybe its branch.
///
/// The order is the guarantee. Nothing below the commit runs if the commit did
/// not, and `git worktree remove` without `--force` is what enforces that: it
/// refuses a checkout with modified or untracked files, so a commit this function
/// only claimed to make would stop the removal rather than precede it.
fn remove_checkout(
    worktree: &Path,
    run_id: &str,
    delete_branch: bool,
    done: &mut Done,
) -> Result<()> {
    let main = main_worktree(worktree)?;
    let branch = current_branch(worktree)?;

    // A commit on a detached HEAD is reachable from nothing once the checkout is
    // gone, which is the one shape where committing first would not save the work
    // it claims to. Reap leaves it and says so.
    let Some(branch) = branch else {
        anyhow::bail!(
            "{} is on a detached HEAD, so a commit would be reachable from nothing. \
             Give it a branch and reap it again.",
            worktree.display()
        );
    };
    done.branch = Some(branch.clone());

    if !git(worktree, &["status", "--porcelain"])?.trim().is_empty() {
        git(worktree, &["add", "-A"])?;
        // `--no-verify`, because a pre-commit hook that refuses would leave reap
        // unable to keep its promise, and this commit is a snapshot of what a
        // checkout held rather than a contribution anyone reviews.
        git(
            worktree,
            &[
                "commit",
                "-q",
                "--no-verify",
                "-m",
                &format!("hydra reap: what run {run_id} left uncommitted"),
            ],
        )?;
        done.committed = Some(git(worktree, &["rev-parse", "HEAD"])?.trim().to_string());
    }

    git(&main, &["worktree", "remove", &worktree.to_string_lossy()])?;
    done.worktree_removed = true;

    // Only the branch Hydra made for this run. A child that checked out one of the
    // human's own branches gets its work committed there and the branch left
    // alone, which is the difference between cleaning up after a run and deleting
    // somebody's work.
    if branch != format!("hydra/{run_id}") {
        done.branch_kept = Some(format!(
            "{branch} is not the branch Hydra made for this run"
        ));
        return Ok(());
    }
    match drop_branch(&main, &branch, false) {
        Ok(()) => done.branch_removed = true,
        Err(_) if delete_branch => match drop_branch(&main, &branch, true) {
            Ok(()) => done.branch_removed = true,
            Err(forced) => done.branch_kept = Some(forced),
        },
        Err(gentle) => done.branch_kept = Some(gentle),
    }
    Ok(())
}

/// `git branch -d`, or `-D` when the human typed the flag, with git's reason.
///
/// **The reason is the return value and not a boolean.** `-d` refuses for two very
/// different causes: the branch holds commits nothing else has, or it is checked
/// out in another worktree. The second is refused by `-D` too, so a report that
/// collapsed both into "it holds commits" told the human the wrong thing and left
/// them typing a flag that cannot help. `hint:` lines are dropped, because git's
/// hint for the first case is the flag this function already offers.
fn drop_branch(main: &Path, branch: &str, force: bool) -> Result<(), String> {
    let flag = if force { "-D" } else { "-d" };
    let out = Command::new("git")
        .args(["branch", flag, branch])
        .current_dir(main)
        .output()
        .map_err(|err| format!("git branch {flag} would not run: {err}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&out.stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("hint:"))
        .unwrap_or("git refused and said nothing")
        .trim_start_matches("error: ")
        .to_string())
}

/// The main worktree of the repository `path` is a checkout of.
///
/// `git worktree list` puts it first, always. Reap needs it because
/// `git worktree remove` and `git branch -d` both want to run somewhere that is
/// not the directory about to disappear.
fn main_worktree(path: &Path) -> Result<PathBuf> {
    let listing = git(path, &["worktree", "list", "--porcelain"])?;
    listing
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("{} is not a git worktree", path.display()))
}

/// The branch `path` has checked out, or `None` on a detached HEAD.
fn current_branch(path: &Path) -> Result<Option<String>> {
    let name = git(path, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();
    Ok((name != "HEAD").then_some(name))
}

/// Spawn directories with no checkout beside them and no live run behind them.
fn orphan_spawns(
    layout: &Layout,
    views: &[RunView],
    grace: std::time::Duration,
) -> Result<Vec<String>> {
    let entries = match std::fs::read_dir(layout.spawns()) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    Ok(entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| !crate::status::just_created(path, grace))
        .filter_map(|path| path.file_name().map(|n| n.to_string_lossy().to_string()))
        .filter(|run_id| !layout.worktree(run_id).exists())
        .filter(|run_id| {
            !views
                .iter()
                .any(|view| view.run_id == *run_id && view.state == State::Live)
        })
        .collect())
}

/// What a human reads back. One line per run, and the commit sha where there is
/// one, because that sha is the whole of the promise reap makes.
pub fn report(reaped: &[Reaped]) -> String {
    if reaped.is_empty() {
        return "nothing to reap\n".to_string();
    }
    let mut out = String::new();
    for item in reaped {
        match item {
            Reaped::Done(done) => {
                out.push_str(&format!("reaped {}\n", done.run_id));
                if let Some(sha) = &done.committed {
                    let branch = done.branch.as_deref().unwrap_or("its branch");
                    out.push_str(&format!(
                        "  committed {} to {branch}\n",
                        &sha[..12.min(sha.len())]
                    ));
                }
                if done.worktree_removed {
                    out.push_str("  worktree removed\n");
                }
                if done.spawn_removed {
                    out.push_str("  spawn directory removed\n");
                }
                match (&done.branch, &done.branch_kept) {
                    (Some(branch), _) if done.branch_removed => {
                        out.push_str(&format!("  branch {branch} removed\n"));
                    }
                    (Some(branch), Some(why)) => {
                        out.push_str(&format!("  branch {branch} kept: {why}\n"));
                    }
                    _ => {}
                }
            }
            Reaped::Left { run_id, why } => out.push_str(&format!("left {run_id}: {why}\n")),
            Reaped::Failed { run_id, why } => out.push_str(&format!("failed {run_id}: {why}\n")),
        }
    }
    out
}

/// How many entries were tried and would not go through.
pub fn failures(reaped: &[Reaped]) -> usize {
    reaped
        .iter()
        .filter(|item| matches!(item, Reaped::Failed { .. }))
        .count()
}
