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
//! default, destructive only when a human typed the flag. The rungs spec's
//! section 4.11 already expects the leftover to be removed with `git branch -D`;
//! the flag is that sentence, typed through the command.
//!
//! **A curate pass's checkout of the skill store is swept the same way, and it
//! is the only other directory here a model writes into.** Map ticket 40's
//! decision 5, on 2026-09-06: reap used to take the worktree, the spawn directory
//! and the snapshot and leave `~/hydra/skill-worktrees/<run-id>/` behind with the
//! proposal it held. Nothing else swept it, because `hydra curate` only removes
//! the checkout of a pass it watched to the end. The commit is what makes
//! removing it safe: whatever the curator wrote goes to
//! `rungs/curate/<run-id>` in the store first, which is the branch the pass
//! would have committed to itself. Neither the store's working tree nor any run
//! record is written on the way.
//!
//! **Nothing in this crate calls into here except `main`.** Harness spec 7.5 is
//! explicit that the gate does not keep a run away from `hydra reap`, because the
//! binary is reached through `shell` and any rule against it is an argv match with
//! the pattern floor's failure mode. What keeps the rungs away is that no
//! rungs code path calls it, which is a property of Hydra's own source, and
//! `docs/rungs-spec.md`'s `evidence_is_read_only` checklist item is what holds
//! it true under a self-edit.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::live::{RunView, State};
use crate::paths::Layout;
use crate::worktree::{CheckoutGit, GitLocation, commit_everything, git};

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
    /// The run's own checkout of the parent repo, `None` when there was none.
    /// A run outside a repo never had one, and a reaped run no longer has one.
    pub worktree: Option<Checkout>,
    /// A curate pass's checkout of the skill store, `None` when there was none.
    /// Every run but a curate pass leaves none.
    pub store_checkout: Option<Checkout>,
    pub spawn_removed: bool,
}

/// What committing and removing one git worktree did.
///
/// **Two directories have this shape and the fields say the same thing for
/// both**: the run's checkout of the parent repo, whose branch is
/// `hydra/<run-id>`, and a curate pass's checkout of the skill store, whose
/// branch is `rungs/curate/<run-id>`.
pub struct Checkout {
    /// The commit reap made, or `None` when the directory was already clean.
    pub committed: Option<String>,
    /// The branch the directory was on, which is what the commit landed on.
    pub branch: String,
    pub branch_removed: bool,
    /// Why the branch survived, in git's own words where git decided it.
    ///
    /// A boolean here told the human "it holds commits" whatever the reason, and
    /// the two reasons are not alike: a branch checked out in another worktree is
    /// refused by `-D` as well, so the flag would not help and the message would
    /// have been wrong twice.
    pub branch_kept: Option<String>,
    pub removed: bool,
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
        // Build ticket 39. A full uuid reaches `one` untouched, so reaping a run
        // whose record was pruned still works; a shorter string is resolved and
        // a typo is an error rather than a silent "nothing to do".
        let run_id = crate::resolver::resolve(layout, run_id)?;
        return Ok(vec![one(layout, &views, &run_id, delete_branch)]);
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

    // A checkout can outlive both directories above. A pass whose run outlived
    // the call leaves all three, and a reap made before this build took the
    // worktree and the spawn directory and left the checkout, which is what
    // ticket 40 measured. Neither list above holds such an id, so the checkouts
    // are walked on their own.
    for orphan in orphan_checkouts(layout, &views, grace)? {
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

fn reap_git_context(run_id: &str, checkout: &Path, start: &Start) -> Result<CheckoutGit> {
    // A pruned run's record moved to `pruned/`, and it still names the parent.
    let parent = match start {
        Start::Read(line) => line
            .hydra
            .scopes
            .as_ref()
            .and_then(|scopes| scopes.parent_git_dir.as_deref()),
        Start::Missing | Start::Unreadable => None,
    };
    CheckoutGit::for_run_from(run_id, checkout, parent)
}

/// Line 0 as reap reads it: the three fields it acts on, typed.
///
/// The third hiring panel's H18. These used to be `serde_json::Value` paths,
/// and on a `Value` a field of the wrong shape reads exactly like a field that
/// is absent: a `skills.linked` that was not a list, or a provider this build
/// cannot name, came out as "nothing was injected", and reap committed what it
/// should have held back. A typed read tells the two apart. An absent field is
/// an answer; a field that does not parse makes the whole line one reap could
/// not read, which it treats as no line at all.
#[derive(serde::Deserialize)]
struct LineZero {
    provider: Option<String>,
    hydra: StartFields,
}

#[derive(serde::Deserialize)]
struct StartFields {
    #[serde(rename = "type")]
    kind: String,
    /// Contract 5.3. Absent when nothing was injected, never null on a record
    /// this build writes.
    skills: Option<SkillsFields>,
    /// Contract 5.1. Absent on a record written before build ticket 15.
    scopes: Option<ScopesFields>,
}

#[derive(serde::Deserialize)]
struct SkillsFields {
    linked: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ScopesFields {
    parent_git_dir: Option<PathBuf>,
}

/// What reading a run's line 0 found.
enum Start {
    /// No record under `runs/` or `pruned/`, or one with no line in it.
    Missing,
    /// A line 0 that is not a `run_start` in the shape [`LineZero`] names.
    Unreadable,
    Read(LineZero),
}

/// Line 0 from `runs/`, or from `pruned/` for a pruned run.
fn line_zero(layout: &Layout, run_id: &str) -> Start {
    let Some(first) = [layout.run_file(run_id), layout.pruned_file(run_id)]
        .iter()
        .find_map(|path| crate::live::first_line(path).ok().flatten())
    else {
        return Start::Missing;
    };
    match serde_json::from_str::<LineZero>(&first) {
        Ok(line) if line.hydra.kind == "run_start" => Start::Read(line),
        _ => Start::Unreadable,
    }
}

fn one(layout: &Layout, views: &[RunView], run_id: &str, delete_branch: bool) -> Reaped {
    let worktree = layout.worktree(run_id);
    let checkout = layout.skill_checkout(run_id);

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
    // reap. `hydra reap <run-id>` typed from somewhere else does it. Both
    // directories count: a human reading a proposal is standing in the checkout.
    if let Ok(cwd) = std::env::current_dir()
        && let Some(inside) = [&worktree, &checkout]
            .into_iter()
            .find(|path| cwd.starts_with(path))
    {
        return Reaped::Left {
            run_id: run_id.to_string(),
            why: format!("this shell is inside {}", inside.display()),
        };
    }

    let mut done = Done {
        run_id: run_id.to_string(),
        worktree: None,
        store_checkout: None,
        spawn_removed: false,
    };

    let start = line_zero(layout, run_id);
    let injected = injected_skills(&start);

    if worktree.is_dir() {
        match commit_then_remove(
            &worktree,
            reap_git_context(run_id, &worktree, &start),
            Some(&layout.run_objects(run_id)),
            &injected,
            &format!("hydra/{run_id}"),
            &format!("hydra reap: what run {run_id} left uncommitted"),
            delete_branch,
        ) {
            Ok(swept) => done.worktree = Some(swept),
            Err(why) => {
                return Reaped::Failed {
                    run_id: run_id.to_string(),
                    why: format!("{why:#}"),
                };
            }
        }
    }

    // Between the worktree and the spawn directory, because a failure here has
    // to stop the spawn directory the way a failure above does: the pair goes
    // together or not at all, and a proposal reap could not commit is work.
    if checkout.is_dir() {
        // `skills/` exists empty since build 366, and `rev-parse` there would
        // walk up to Hydra's own repository. So the store needs its own `.git`.
        let store = layout.skills();
        match commit_then_remove(
            &checkout,
            store
                .join(".git")
                .exists()
                .then(|| crate::worktree::git_common_dir(&store))
                .flatten()
                .ok_or_else(|| anyhow::anyhow!("skill store has no Git directory"))
                .and_then(|common| CheckoutGit::new(&checkout, &common)),
            None,
            &injected,
            &format!("rungs/curate/{run_id}"),
            &format!("hydra reap: what curate run {run_id} left uncommitted"),
            delete_branch,
        ) {
            Ok(swept) => done.store_checkout = Some(swept),
            Err(why) => {
                return Reaped::Failed {
                    run_id: run_id.to_string(),
                    why: format!("the skill store checkout: {why:#}"),
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

/// What Hydra put under a run's skill root, as checkout-relative paths.
///
/// Map ticket 71: no skill Hydra injected reaches a branch, and nothing else
/// under a skill root is Hydra's business. A skill the run wrote there is its
/// work and is committed like any other file.
struct InjectedSkills {
    /// `<root>/<name>` for each name the record lists as linked, whether the
    /// launch made a link or a copy. Excluded from the commit and removed.
    entries: Vec<String>,
    /// `<root>/.gitignore`, removed before staging when it is still the one
    /// Hydra wrote.
    ignores: Vec<String>,
}

/// Reads the injected names off the run's own `run_start`, contract 5.3's
/// `skills.linked`, from `runs/` or from `pruned/`.
///
/// **The record is the one source a run cannot forge.** The gate and the
/// seatbelt both deny a run writes under `~/hydra`, while anything in the
/// checkout is the run's to rewrite. `launch.json` carries the same line 0 but
/// is not read to interpret a run, so it is not read here either.
///
/// **No record means the names are unknown**, which happens only when line 0
/// was never written or the human pruned with `--delete`. A line 0 that does
/// not read as [`LineZero`], or a `skills` block under a provider this build
/// cannot name, is the same case. Then every skill root is held back whole, the rule before this narrowing, because leaking an
/// injected skill onto a branch is the failure map ticket 71 exists to stop.
fn injected_skills(start: &Start) -> InjectedSkills {
    let every_root = || InjectedSkills {
        entries: [
            crate::provider::Provider::Claude,
            crate::provider::Provider::CursorAgent,
            crate::provider::Provider::Agy,
            crate::provider::Provider::Codex,
        ]
        .iter()
        .map(|provider| provider.skill_root().to_string())
        .collect(),
        ignores: Vec::new(),
    };
    let Start::Read(line) = start else {
        return every_root();
    };
    let Some(skills) = &line.hydra.skills else {
        // No `skills` block: nothing was injected, so nothing is held back.
        return InjectedSkills {
            entries: Vec::new(),
            ignores: Vec::new(),
        };
    };
    // Something was injected under a root this build cannot name, so the
    // names do not say where to look.
    let Some(provider) = line
        .provider
        .as_deref()
        .and_then(|name| name.parse::<crate::provider::Provider>().ok())
    else {
        return every_root();
    };
    let root = provider.skill_root();
    let entries = skills
        .linked
        .iter()
        // A name is one path component. Anything else was not written by
        // `inject`, and joined onto the root it could name the root itself.
        .filter(|name| {
            let mut parts = Path::new(name).components();
            matches!(parts.next(), Some(std::path::Component::Normal(_))) && parts.next().is_none()
        })
        .map(|name| format!("{root}/{name}"))
        .collect();
    InjectedSkills {
        entries,
        ignores: vec![format!("{root}/.gitignore")],
    }
}

/// Removes `<root>/.gitignore` when it is still the one `skills::inject`
/// wrote: untracked, and holding exactly `*`. A repo's own ignore file there,
/// or one the run rewrote, is left to be committed or kept like any other.
fn drop_hydras_ignore(checkout: &Path, context: &CheckoutGit, ignore: &str) -> Result<()> {
    // A root reached through a link the run made would put the removal
    // outside the checkout.
    if !crate::worktree::inside_checkout(checkout, ignore) {
        return Ok(());
    }
    let path = checkout.join(ignore);
    let is_file = std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_file());
    if !is_file || std::fs::read_to_string(&path).ok().as_deref() != Some("*\n") {
        return Ok(());
    }
    if !git(context, &["ls-files", "--", ignore])?.trim().is_empty() {
        return Ok(());
    }
    std::fs::remove_file(&path)?;
    Ok(())
}

/// Removes untracked settings mount points left by bubblewrap.
fn drop_empty_settings(checkout: &Path, context: &CheckoutGit) -> Result<()> {
    for name in [".claude/settings.json", ".claude/settings.local.json"] {
        if !crate::worktree::inside_checkout(checkout, name) {
            continue;
        }
        let path = checkout.join(name);
        if !std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_file()) {
            continue;
        }
        let content = std::fs::read(&path)?;
        if !matches!(content.as_slice(), b"" | b"{}") {
            continue;
        }
        if git(context, &["ls-files", "--", name])?.trim().is_empty() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Commits what one checkout holds, then removes it and maybe its branch.
///
/// The order is the guarantee. Nothing below the commit runs if the commit did
/// not, and `git worktree remove` without `--force` is what enforces that: it
/// refuses a checkout with modified or untracked files, so a commit this function
/// only claimed to make would stop the removal rather than precede it.
///
/// **One sequence for the two directories a model writes into.** The run's
/// checkout of the parent repo and a curate pass's checkout of the skill store
/// differ in the repository they came off, the branch Hydra made for them and the
/// commit message, so those are the arguments. `hydra_branch` is
/// `hydra/<run-id>` for the one and `rungs/curate/<run-id>` for the other, and
/// the Git metadata comes from trusted launch data or the skill store.
///
/// `objects` is the run's own object directory, map ticket 74, and `None` for
/// a curate checkout, whose private index lives in the store's denied `.git`
/// and so holds nothing the curator staged. What git verifies in it goes into
/// the parent before the commit, and an index entry whose object did not
/// arrive is staged again from the checkout.
fn commit_then_remove(
    checkout: &Path,
    context: Result<CheckoutGit>,
    objects: Option<&Path>,
    injected: &InjectedSkills,
    hydra_branch: &str,
    message: &str,
    delete_branch: bool,
) -> Result<Checkout> {
    let context = context?;
    let main = main_worktree(&context)?;

    // A commit on a detached HEAD is reachable from nothing once the checkout is
    // gone, which is the one shape where committing first would not save the work
    // it claims to. Reap leaves it and says so.
    let Some(branch) = current_branch(&context)? else {
        anyhow::bail!(
            "{} is on a detached HEAD, so a commit would be reachable from nothing. \
             Give it a branch and reap it again.",
            checkout.display()
        );
    };
    if branch != hydra_branch {
        anyhow::bail!(
            "{} is on branch {branch}, expected {hydra_branch}; leaving the checkout and spawn directory",
            checkout.display()
        );
    }
    refuse_nested_repositories(checkout, &context)?;
    context.restore_pointer()?;
    let mut swept = Checkout {
        committed: None,
        branch: branch.clone(),
        branch_removed: false,
        branch_kept: None,
        removed: false,
    };

    // Hydra's `*` ignore file would hide a skill the run authored beside the
    // injected ones, and `git worktree remove` deletes ignored files without a
    // word. So it goes before staging, and the injected entries are held back
    // by name instead.
    for ignore in &injected.ignores {
        drop_hydras_ignore(checkout, &context, ignore)?;
    }
    drop_empty_settings(checkout, &context)?;
    if let Some(store) = objects {
        // A store git would not verify moves nothing, and the entries that
        // needed it are hashed again from the checkout below.
        if let Err(err) = crate::worktree::import_run_objects(&main, store) {
            eprintln!("hydra: reap moved none of the run's objects: {err:#}");
        }
        crate::worktree::unstage_missing_objects(&context)?;
    }
    let mut exclude = crate::provider::own_worktree_paths();
    exclude.extend(injected.entries.iter().map(String::as_str));
    swept.committed = commit_everything(&context, message, &exclude)?;

    git(&main, &["worktree", "remove", &checkout.to_string_lossy()])?;
    let saved = checkout.with_extension("git-context.json");
    if saved.is_file() {
        std::fs::remove_file(saved)?;
    }
    swept.removed = true;

    match drop_branch(&main, &branch, false) {
        Ok(()) => swept.branch_removed = true,
        Err(_) if delete_branch => match drop_branch(&main, &branch, true) {
            Ok(()) => swept.branch_removed = true,
            Err(forced) => swept.branch_kept = Some(forced),
        },
        Err(gentle) => swept.branch_kept = Some(gentle),
    }
    Ok(swept)
}

/// Refuse before rewriting any checkout metadata or importing run objects.
/// Gitlinks include legitimate submodules: neither they nor an unstaged nested
/// repository can be flattened without a separate decision from the human.
fn refuse_nested_repositories(checkout: &Path, context: &CheckoutGit) -> Result<()> {
    use std::collections::BTreeSet;
    use std::os::unix::ffi::OsStrExt;

    let listing = context
        .command()
        .args(["ls-files", "--stage", "-z"])
        .output()?;
    anyhow::ensure!(
        listing.status.success(),
        "git ls-files --stage failed: {}",
        String::from_utf8_lossy(&listing.stderr).trim()
    );
    let mut paths = BTreeSet::new();
    for entry in listing.stdout.split(|byte| *byte == 0) {
        if entry.starts_with(b"160000 ")
            && let Some(tab) = entry.iter().position(|byte| *byte == b'\t')
        {
            paths.insert(PathBuf::from(std::ffi::OsStr::from_bytes(
                &entry[tab + 1..],
            )));
        }
    }

    // Include ignored/untracked paths, but never follow directory symlinks or
    // descend into Git metadata. The checkout's own pointer is expected.
    let mut directories = vec![PathBuf::new()];
    while let Some(relative) = directories.pop() {
        for entry in std::fs::read_dir(checkout.join(&relative))? {
            let entry = entry?;
            if entry.file_name() == ".git" {
                if !relative.as_os_str().is_empty() {
                    paths.insert(relative.clone());
                }
            } else if entry.file_type()?.is_dir() {
                directories.push(relative.join(entry.file_name()));
            }
        }
    }
    anyhow::ensure!(
        paths.is_empty(),
        "refusing to reap nested repositories or gitlinks; leaving the checkout untouched:\n{}",
        paths
            .iter()
            .map(|path| format!("  {:?}", path))
            .collect::<Vec<_>>()
            .join("\n")
    );
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
    let out = crate::worktree::GitLocation::command(main)
        .args(["branch", flag, branch])
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
fn main_worktree(path: &CheckoutGit) -> Result<PathBuf> {
    let listing = git(path, &["worktree", "list", "--porcelain"])?;
    listing
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("checkout is not a git worktree"))
}

/// The branch `path` has checked out, or `None` on a detached HEAD.
fn current_branch(path: &CheckoutGit) -> Result<Option<String>> {
    let name = git(path, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();
    Ok((name != "HEAD").then_some(name))
}

/// What the snapshot sweep did to one sha.
///
/// **Keyed by a sha and not by a run id**, which is why this is its own type
/// rather than a fourth [`Reaped`]. Harness spec 5 gives the snapshot a different
/// lifetime from the other three rows: created per sha, shared between every run
/// that pinned it, and reaped when no live run references it.
pub enum SnapshotSweep {
    Removed {
        sha: String,
    },
    Failed {
        sha: String,
        why: String,
    },
    /// Nothing was swept at all, and why.
    Deferred {
        why: String,
    },
}

/// Removes every skills snapshot no live run is pinned to.
///
/// **Which command does this is a choice, and harness spec 5 names none.** The
/// table says a snapshot is reaped "when no live run references it" and stops
/// there. `hydra reap` is the command that already removes a run's scratch and is
/// already typed by a human, so it is the one that got this, and
/// `IMPLEMENTATION.md` records that as a choice rather than a reading.
///
/// **The refcount is the corpus.** A snapshot is referenced while a live run's
/// `run_start` names its sha, which [`crate::skills::referenced_shas`] answers off
/// the same scan `hydra status` uses. No counter file, so there is nothing to fall
/// out of step with the runs it counts.
///
/// **A launch in flight stands the whole sweep down.** The snapshot for a sha that
/// already exists is reused untouched, so a launch that is between creating its
/// checkout and writing line 0 references a snapshot that looks old and is named
/// by no record yet. Age cannot see that one, and removing it would leave a
/// running child with dangling links and no warning. A checkout younger than the
/// grace whose record has not spoken is the same evidence
/// [`crate::status::abandoned_worktrees`] reads, pointed the other way.
pub fn snapshots(layout: &Layout, grace: std::time::Duration) -> Result<Vec<SnapshotSweep>> {
    let dir = layout.snapshots();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    let views = crate::live::scan(layout)?;
    if let Some(run_id) = launching(layout, &views, grace)? {
        return Ok(vec![SnapshotSweep::Deferred {
            why: format!(
                "run {run_id} is still starting, and a launch reuses a snapshot \
                 without touching it"
            ),
        }]);
    }

    let referenced = crate::skills::referenced_shas(&views);
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    paths.sort();

    let mut swept = Vec::new();
    for path in paths {
        let sha = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if referenced.contains(&sha) || crate::status::just_created(&path, grace) {
            continue;
        }
        match crate::skills::remove_snapshot(&layout.skills(), &path) {
            Ok(()) => swept.push(SnapshotSweep::Removed { sha }),
            Err(err) => swept.push(SnapshotSweep::Failed {
                sha,
                why: format!("{err:#}"),
            }),
        }
    }
    Ok(swept)
}

/// The run id of a checkout that exists, has not spoken, and is younger than the
/// grace. That is a launch between its worktree and its line 0.
fn launching(
    layout: &Layout,
    views: &[RunView],
    grace: std::time::Duration,
) -> Result<Option<String>> {
    let entries = match std::fs::read_dir(layout.worktrees()) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    Ok(entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| crate::status::just_created(path, grace))
        .filter_map(|path| path.file_name().map(|n| n.to_string_lossy().to_string()))
        .find(|run_id| {
            !views
                .iter()
                .any(|view| view.run_id == *run_id && view.launched())
        }))
}

/// What a human reads back from the snapshot sweep. Silence for a snapshot still
/// in use, because a line per live run's snapshot is noise around the removals.
pub fn snapshot_report(swept: &[SnapshotSweep]) -> String {
    let mut out = String::new();
    for item in swept {
        match item {
            SnapshotSweep::Removed { sha } => {
                out.push_str(&format!("snapshot {} removed\n", short(sha)));
            }
            SnapshotSweep::Failed { sha, why } => {
                out.push_str(&format!("snapshot {} failed: {why}\n", short(sha)));
            }
            SnapshotSweep::Deferred { why } => {
                out.push_str(&format!("snapshots left alone: {why}\n"));
            }
        }
    }
    out
}

/// The first twelve characters, counted in characters.
///
/// A snapshot's name is a directory read off disk rather than a sha git handed
/// over, so byte-slicing it panics the day one is not hex.
fn short(sha: &str) -> String {
    sha.chars().take(12).collect()
}

/// How many snapshots were tried and would not go through. The sibling of
/// [`failures`], which counts the same thing for runs.
pub fn snapshot_failures(swept: &[SnapshotSweep]) -> usize {
    swept
        .iter()
        .filter(|item| matches!(item, SnapshotSweep::Failed { .. }))
        .count()
}

/// How many snapshots the sweep actually acted on. A deferral is the sweep
/// declining to act, so it belongs in neither half of "n of m".
pub fn snapshots_acted_on(swept: &[SnapshotSweep]) -> usize {
    swept
        .iter()
        .filter(|item| !matches!(item, SnapshotSweep::Deferred { .. }))
        .count()
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

/// Checkouts of the skill store with no live run behind them.
///
/// The same three filters [`orphan_spawns`] applies, for the same reasons: a
/// directory, older than the launch grace, and no live run to pull it out from
/// under. A pass creates its checkout before the run that writes into it exists,
/// so the grace is what keeps a sweep off a pass that is starting.
fn orphan_checkouts(
    layout: &Layout,
    views: &[RunView],
    grace: std::time::Duration,
) -> Result<Vec<String>> {
    let entries = match std::fs::read_dir(layout.skill_worktrees()) {
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
                if let Some(swept) = &done.worktree {
                    checkout_lines(&mut out, "worktree", swept);
                }
                if let Some(swept) = &done.store_checkout {
                    checkout_lines(&mut out, "skill store checkout", swept);
                }
                if done.spawn_removed {
                    out.push_str("  spawn directory removed\n");
                }
            }
            Reaped::Left { run_id, why } => out.push_str(&format!("left {run_id}: {why}\n")),
            Reaped::Failed { run_id, why } => out.push_str(&format!("failed {run_id}: {why}\n")),
        }
    }
    out
}

/// What became of one checkout, three lines at most.
///
/// `what` names which of the two directories this was. A curate pass leaves both,
/// and "the checkout" would name either of them to a human reading that pass back.
fn checkout_lines(out: &mut String, what: &str, swept: &Checkout) {
    if let Some(sha) = &swept.committed {
        out.push_str(&format!(
            "  {what} committed {} to {}\n",
            short(sha),
            swept.branch
        ));
    }
    if swept.removed {
        out.push_str(&format!("  {what} removed\n"));
    }
    if swept.branch_removed {
        out.push_str(&format!("  branch {} removed\n", swept.branch));
    } else if let Some(why) = &swept.branch_kept {
        out.push_str(&format!("  branch {} kept: {why}\n", swept.branch));
    }
}

/// How many entries were tried and would not go through.
pub fn failures(reaped: &[Reaped]) -> usize {
    reaped
        .iter()
        .filter(|item| matches!(item, Reaped::Failed { .. }))
        .count()
}
