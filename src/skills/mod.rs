//! The skill store, the snapshot and per-run injection, harness spec 6.4.
//!
//! One canonical store at `~/hydra/skills/<name>/SKILL.md`, flat, its own git repo
//! inside the `~/hydra` working tree and ignored by it. Beside each `SKILL.md` sits
//! an optional `hydra.toml` carrying `origin`, `pinned` and `providers`, which no
//! provider opens.
//!
//! **Injection is per run into the worktree, never into the user's home.** The
//! symlink farm survives as a mechanism and dies as a location. A run with no
//! checkout therefore gets no injection at all, and its record says so by leaving
//! `skills` absent.
//!
//! **The links point into a snapshot, not into the store.** Without that, versioning
//! the store is decorative: a symlink resolves to the live working tree, so an edit
//! mid-session would reach a session already running. `~/hydra/skills-snapshots/<sha>/`
//! is a git worktree of the store checked out at the sha the run pinned, shared
//! between runs on the same sha because two runs looking at the same bytes is the
//! point rather than a saving.
//!
//! **A dirty store is committed before pinning.** Launching from `HEAD` and ignoring
//! the working tree would mean editing a skill, running, and watching nothing happen.
//! Committing is the only option where the sha in the run record is always true.
//!
//! **Nothing here counts references in a file.** A snapshot is referenced when a
//! live run's record names its sha, which [`crate::live::scan`] answers, so the
//! corpus is the refcount and there is no second source of truth to fall out of
//! step with it. [`crate::reap`] is what acts on the answer.

pub mod collision;
pub mod frontmatter;
pub mod index;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};

use serde::Deserialize;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::paths::Layout;
use crate::provider::Provider;
use crate::worktree::git;

/// The canonical store, `~/hydra/skills/`.
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// Opens the store, creating it as a git repo the first time.
    ///
    /// **The first commit is empty and deliberate.** A repo with no commits has no
    /// sha, so the first run would have nothing to pin and `skills.store_sha`
    /// would be null on a store that exists. An empty root commit costs one object
    /// and makes the field true from the start.
    pub fn open(layout: &Layout) -> Result<Self> {
        let dir = layout.skills();
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let store = Self { dir };
        if !store.dir.join(".git").exists() {
            git(&store.dir, &["init", "-q"])
                .with_context(|| format!("making {} a git repo", store.dir.display()))?;
            store.commit("hydra: the skill store begins", None)?;
        }
        Ok(store)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Commits the store if it is dirty, then answers with the sha to pin.
    ///
    /// **Authorship is a trailer and names the run that pinned it** (6.4). Hydra
    /// cannot know which run dirtied the store: a `curate` child writes into it
    /// with its own tool calls and is long gone by the time the next launch finds
    /// the change. What Hydra does know is which run turned that working tree into
    /// a commit, and that is what the trailer says.
    ///
    /// The committer is Hydra rather than the human whose git config happens to be
    /// on the machine. The commit is the harness's act, and passing the identity on
    /// the command line also means a machine with no global git identity can still
    /// launch a run.
    /// **Two launches racing on a dirty store is handled the way the snapshot
    /// race is.** `git add` and `git commit` take the index lock, so the launch
    /// that loses gets a git error rather than a wrong answer. If the store is
    /// clean by the time that error arrives, the other launch committed the very
    /// change this one was about to, and its sha is the right one to pin.
    pub fn pin(&self, run_id: &str) -> Result<String> {
        if self.is_dirty()? {
            let subject =
                "hydra: the store was dirty at launch, so it was committed before pinning";
            let committed =
                git(&self.dir, &["add", "-A"]).and_then(|_| self.commit(subject, Some(run_id)));
            if let Err(err) = committed
                && self.is_dirty()?
            {
                return Err(err).context("committing the store before pinning it");
            }
        }
        Ok(git(&self.dir, &["rev-parse", "HEAD"])?.trim().to_string())
    }

    fn is_dirty(&self) -> Result<bool> {
        Ok(!git(&self.dir, &["status", "--porcelain"])?
            .trim()
            .is_empty())
    }

    fn commit(&self, subject: &str, run_id: Option<&str>) -> Result<()> {
        let message = match run_id {
            Some(run_id) => format!("{subject}\n\nHydra-Pinned-By: {run_id}\n"),
            None => format!("{subject}\n"),
        };
        git(
            &self.dir,
            &[
                "-c",
                "user.name=hydra",
                "-c",
                "user.email=hydra@localhost",
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                &message,
            ],
        )?;
        Ok(())
    }

    /// The immutable checkout of one sha, created once and shared after that.
    ///
    /// **An existing snapshot is reused rather than rebuilt.** Snapshots are keyed
    /// by sha and a git worktree at a sha holds the same bytes whoever asked for
    /// it, so two runs on one sha are looking at one directory on purpose.
    ///
    /// **The checkout is made beside the target and renamed into place.** Build
    /// ticket 47 watched three launches in one second on a fresh sha: one made the
    /// snapshot and two died, one on "already exists" and one on an `index.lock`
    /// inside the store's own bookkeeping. `git worktree add` straight into the
    /// target is a check-then-create with a window a whole checkout wide. So each
    /// launch checks out into its own `<sha>.tmp-<pid>-<n>` and asks git to move
    /// it onto `<sha>`. The move is one `rename(2)`, and a directory cannot be
    /// renamed onto a non-empty one, so exactly one launch wins. A loser removes
    /// its own checkout and takes the winner's, which holds the bytes it wanted.
    /// A launch that dies mid-checkout leaves a staging directory and never a
    /// half-made `<sha>`, and the reap sweep removes it like any other snapshot
    /// nobody references.
    ///
    /// **The rename was not enough on its own, so git runs under `flock(2)`.**
    /// Every `git worktree` command begins by reading every entry under the
    /// store's `.git/worktrees/`, and an entry another launch is in the middle of
    /// making has no `commondir` yet, which git reads as fatal. Four threads on
    /// unique staging paths hit that one time in four. The lock is
    /// `skills-snapshots/.lock`, held for the few git calls this function makes
    /// and by [`remove_snapshot`] for its one. The kernel drops it with the file
    /// descriptor, so a launch that dies holding it wedges nobody, and the file
    /// itself carries nothing and is never read.
    ///
    /// **An existing directory is verified before it is used.** `HEAD` in it has
    /// to be `sha` and the store has to list it. A directory the store does not
    /// list is what a rename behind git's back leaves, and `git worktree repair`
    /// puts the listing right. A directory at the wrong sha is refused, because
    /// removing it under a run that might be reading it is not this function's
    /// call to make; the error says what to remove by hand.
    pub fn snapshot(&self, layout: &Layout, sha: &str) -> Result<PathBuf> {
        let path = layout.snapshot(sha);
        std::fs::create_dir_all(layout.snapshots())
            .with_context(|| format!("creating {}", layout.snapshots().display()))?;
        let _held = lock_snapshots(&layout.snapshots())?;
        if path.exists() {
            return self.verify_snapshot(&path, sha).map(|()| path);
        }

        let staging = layout.snapshots().join(format!(
            "{sha}.tmp-{}-{}",
            std::process::id(),
            SNAPSHOT_ATTEMPT.fetch_add(1, Ordering::Relaxed)
        ));
        git(
            &self.dir,
            &[
                "worktree",
                "add",
                "--detach",
                "-q",
                &staging.to_string_lossy(),
                sha,
            ],
        )
        .with_context(|| format!("checking the store out at {sha} into {}", staging.display()))?;

        let moved = git(
            &self.dir,
            &[
                "worktree",
                "move",
                &staging.to_string_lossy(),
                &path.to_string_lossy(),
            ],
        );
        match moved {
            Ok(_) => self.verify_snapshot(&path, sha).map(|()| path),
            Err(err) => {
                // Whatever happened, the staging checkout is nobody's snapshot.
                // Through git so the store stops listing it, with `remove_dir_all`
                // and a prune behind that in case git will not.
                if git(
                    &self.dir,
                    &["worktree", "remove", "--force", &staging.to_string_lossy()],
                )
                .is_err()
                {
                    let _ = std::fs::remove_dir_all(&staging);
                    let _ = git(&self.dir, &["worktree", "prune"]);
                }
                if path.exists() {
                    // A concurrent launch on the same sha got there first. The
                    // directory it made is the one this run wants, so losing the
                    // race is not a failure.
                    return self.verify_snapshot(&path, sha).map(|()| path);
                }
                Err(err).with_context(|| {
                    format!(
                        "moving the checkout of {sha} into place at {}",
                        path.display()
                    )
                })
            }
        }
    }

    /// Whether `path` is this store's worktree checked out at `sha`.
    ///
    /// The listing check is what catches a snapshot the store has lost track of,
    /// and `git worktree repair` is the answer when the directory itself is sound.
    fn verify_snapshot(&self, path: &Path, sha: &str) -> Result<()> {
        let head = git(path, &["rev-parse", "--verify", "HEAD"])
            .with_context(|| {
                format!(
                    "{} exists but is not a checkout of the store; remove it by hand and \
                     run `git -C {} worktree prune`",
                    path.display(),
                    self.dir.display()
                )
            })?
            .trim()
            .to_string();
        if head != sha {
            bail!(
                "{} is checked out at {head} and not at {sha}; a snapshot is immutable, so \
                 somebody moved it, and it has to be removed by hand",
                path.display()
            );
        }
        if !self.lists_worktree(path)? {
            git(&self.dir, &["worktree", "repair", &path.to_string_lossy()])
                .with_context(|| format!("repairing the store's listing of {}", path.display()))?;
        }
        Ok(())
    }

    fn lists_worktree(&self, path: &Path) -> Result<bool> {
        let wanted = std::fs::canonicalize(path)?;
        let listed = git(&self.dir, &["worktree", "list", "--porcelain"])?;
        Ok(listed
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .filter_map(|entry| std::fs::canonicalize(entry).ok())
            .any(|entry| entry == wanted))
    }
}

/// Distinguishes staging checkouts made by one process in the same second.
static SNAPSHOT_ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// Takes `<snapshots>/.lock` with `flock(2)`, released when the handle drops.
///
/// Advisory and kernel-held: the file existing means nothing, only an open
/// descriptor holding it does, so there is no stale lock to clear after a crash.
fn lock_snapshots(snapshots: &Path) -> Result<std::fs::File> {
    use std::os::fd::AsRawFd;
    let path = snapshots.join(".lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    loop {
        // SAFETY: `flock` takes a descriptor this handle owns and touches no
        // memory of ours.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return Ok(file);
        }
        let err = std::io::Error::last_os_error();
        if err.kind() != std::io::ErrorKind::Interrupted {
            return Err(err).with_context(|| format!("locking {}", path.display()));
        }
    }
}

/// `hydra.toml`, the sidecar no provider opens.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Sidecar {
    /// Where the skill came from, when it came from somewhere.
    #[serde(default)]
    pub origin: Option<String>,
    /// A skill Hydra may not propose an edit to. Rung two reads it.
    #[serde(default)]
    pub pinned: bool,
    /// Which providers this skill is linked for. Absent admits all of them.
    #[serde(default)]
    pub providers: Option<Vec<String>>,
}

impl Sidecar {
    fn read(dir: &Path) -> Result<Self> {
        let file = dir.join("hydra.toml");
        let Ok(text) = std::fs::read_to_string(&file) else {
            return Ok(Self::default());
        };
        toml::from_str(&text).with_context(|| format!("parsing {}", file.display()))
    }

    /// Whether this skill is linked for a run on `provider`.
    fn admits(&self, provider: Provider) -> bool {
        match &self.providers {
            None => true,
            Some(providers) => providers.iter().any(|name| name == provider.binary()),
        }
    }
}

/// One skill in a snapshot, with its sidecar read.
pub struct StoreSkill {
    pub name: String,
    pub path: PathBuf,
    pub sidecar: Sidecar,
}

/// Every skill in a snapshot, name-ordered, with the invalid ones left out.
///
/// **An unusable skill is skipped and said out loud rather than failing the run.**
/// The store is a directory a human and a curator both write to, and one bad file
/// in it must not be able to stop every launch on the machine. The write-time
/// validator is where a bad file is supposed to be caught; this is the backstop
/// for one that got in another way.
pub fn skills_in(snapshot: &Path) -> Result<Vec<StoreSkill>> {
    let dir = match std::fs::read_dir(snapshot) {
        Ok(dir) => dir,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", snapshot.display())),
    };

    let mut paths: Vec<PathBuf> = dir
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("SKILL.md").is_file())
        .collect();
    paths.sort();

    let mut skills = Vec::new();
    for path in paths {
        // The sidecar is read inside the same fallible step as the frontmatter,
        // so a `hydra.toml` that will not parse skips its skill instead of
        // failing every launch on the machine. That is what the note above
        // promises, and reading it with `?` would have broken the promise.
        let read = frontmatter::read(&path).and_then(|skill| {
            Ok(StoreSkill {
                name: skill.name,
                sidecar: Sidecar::read(&path)?,
                path: path.clone(),
            })
        });
        match read {
            Ok(skill) => skills.push(skill),
            Err(err) => eprintln!(
                "hydra: the store is not linking {}: {err:#}",
                path.display()
            ),
        }
    }
    Ok(skills)
}

/// What one run's injection put in front of the provider.
pub struct Injected {
    /// Contract 5.3's `skills.store_sha`.
    pub store_sha: String,
    /// Contract 5.3's `skills.linked`, in the order the links were made.
    pub linked: Vec<String>,
    /// Store skills whose name the repo's own project scope already held, so no
    /// link was made. Not part of the record: `linked` says what Hydra linked, and
    /// these are what it did not. [`collision::check`] is what reports them.
    pub blocked: Vec<String>,
    /// The directory the links went into, `<worktree>/.claude/skills/` on `claude`.
    pub root: PathBuf,
}

/// The note the watcher adds to `run_end.error` when a `codex` skill copy no
/// longer matches the snapshot it was taken from. It is appended after any tee
/// note, so it need not start the field. The outcome derivation never reads it:
/// a `codex` run that reached `turn.completed` has a terminator, which makes the
/// field a fidelity note, and one that did not is `failed` with or without it.
pub const COPY_MISMATCH_NOTE: &str = "the codex skill copy no longer matches the snapshot";

/// Links the snapshot's admitted skills into the run's checkout.
///
/// **The directory ignores itself.** A `.gitignore` holding `*` goes in first, so
/// `git status` reports nothing and `git add -A` stages nothing, verified on all
/// three builds by ticket 24. A linked worktree has no private `info/exclude`, so
/// hiding the links through git's exclude machinery would hide them in the user's
/// own working tree too. This needs no write outside a directory Hydra created.
///
/// **An existing `.gitignore` is left alone.** A repo that tracks its own
/// `<repo>/.claude/skills/.gitignore` would see the overwrite as a modification to
/// the checkout, which is a change to the human's work rather than to Hydra's. The
/// cost is that Hydra's links then show up in `git status`, which is visible rather
/// than silent.
pub fn inject(
    worktree: &Path,
    provider: Provider,
    snapshot: &Path,
    store_sha: &str,
) -> Result<Injected> {
    let root = worktree.join(provider.skill_root());
    std::fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;

    let ignore = root.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, "*\n").with_context(|| format!("writing {}", ignore.display()))?;
    }

    let mut linked = Vec::new();
    let mut blocked = Vec::new();
    for skill in skills_in(snapshot)? {
        if !skill.sidecar.admits(provider) {
            continue;
        }
        let link = root.join(&skill.name);
        // `symlink_metadata` rather than `exists`, which follows the link. A
        // checkout seeded from a parent carrying a dangling symlink of this name
        // would read as absent and then fail `symlink` with `EEXIST`, killing the
        // launch instead of reporting a name that is taken.
        if std::fs::symlink_metadata(&link).is_ok() {
            // Something in the checkout already holds this name. Overwriting it
            // would edit the human's work, so no link is made and the collision
            // check is what reports it.
            blocked.push(skill.name);
            continue;
        }
        if provider.reads_skills_by_shell() {
            // `linked` still names the skills Hydra put in front of the provider,
            // whether they arrived as links or as copies of the snapshot.
            copy_skill_tree(&skill.path, &link)
                .with_context(|| format!("copying {} into {}", skill.name, root.display()))?;
        } else {
            std::os::unix::fs::symlink(&skill.path, &link)
                .with_context(|| format!("linking {} into {}", skill.name, root.display()))?;
        }
        linked.push(skill.name);
    }

    Ok(Injected {
        store_sha: store_sha.to_string(),
        linked,
        blocked,
        root,
    })
}

/// The write-time collision check, harness spec 6.4's first of two.
///
/// **Nothing in this build calls it, and that is a gap rather than a decision.**
/// 6.4 puts this check on the curator: "at write, the curator refuses to create or
/// rename a store skill to a name the index already holds, so no collision Hydra
/// causes reaches a run". No code path in this build writes to the store. The
/// curator writes through its own gated tool calls, and the step that turns those
/// writes into a store commit belongs to rung one. So this is the check with its
/// caller missing, in the shape `record::Refusal` was written in before any of its
/// four checks existed: one home for the rule, so the ticket that acquires the
/// caller does not invent a second one.
///
/// **The launch check is not a substitute.** It fails the run, which 6.4 calls
/// harsh and justifies by what this check does to the odds. Without this, every
/// name a curator picks badly costs a refused launch instead of a refused write.
pub fn admit(store_dir: &Path, name: &str, user: &index::Index, home: &Path) -> Result<()> {
    let skill = frontmatter::read(&store_dir.join(name))?;
    let taken: Vec<String> = index::user_roots(home)
        .iter()
        .flat_map(|root| user.holding(&skill.name, std::slice::from_ref(root)))
        .map(|entry| entry.path.display().to_string())
        .collect();
    if !taken.is_empty() {
        bail!(
            "{name} cannot go in the store: {} already holds that name, and a \
             launch would refuse the whole run over it",
            taken.join(", ")
        );
    }
    Ok(())
}

/// Recursively copies one admitted skill from the snapshot into the checkout.
///
/// `hydra.toml` is skipped because no provider opens it. Files are 0444 and
/// directories 0555 so a run on an unwrapped launch cannot edit in place without
/// `chmod`, which the seatbelt also refuses on macOS. `hydra reap` gives the
/// write bit back before it removes the checkout.
///
/// A symlink in the snapshot is copied as a symlink with the same target, never
/// followed, so a link to a file outside the store does not become a copy of
/// that file's bytes and a link loop cannot recurse forever.
fn copy_skill_tree(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("creating {}", dst.display()))?;
    for entry in std::fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "hydra.toml" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            let target = std::fs::read_link(&from)
                .with_context(|| format!("reading the link {}", from.display()))?;
            std::os::unix::fs::symlink(&target, &to)
                .with_context(|| format!("linking {}", to.display()))?;
        } else if kind.is_dir() {
            copy_skill_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)
                .with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
            set_read_only_file(&to)?;
        }
    }
    set_read_only_dir(dst)?;
    Ok(())
}

#[cfg(unix)]
fn set_read_only_file(path: &Path) -> Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444))
        .with_context(|| format!("chmod 0444 on {}", path.display()))
}

#[cfg(not(unix))]
fn set_read_only_file(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_read_only_dir(path: &Path) -> Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o555))
        .with_context(|| format!("chmod 0555 on {}", path.display()))
}

#[cfg(not(unix))]
fn set_read_only_dir(_path: &Path) -> Result<()> {
    Ok(())
}

/// Compares a `codex` skill copy with the snapshot it came from.
///
/// Returns text for `run_end.error` when a linked skill differs, is missing, or
/// when the copy gained a file the snapshot does not carry, checked from both
/// sides. `None` means the trees still match on every file the copy was given.
/// `hydra.toml` is never copied, so one found in the copy counts as added.
///
/// Every byte of every copied skill is read twice, once per side, at exit and
/// on `codex` only, so the cost grows with the admitted skills and not with the
/// whole store.
pub fn copy_mismatch(copy_root: &Path, snapshot: &Path, linked: &[String]) -> Option<String> {
    let mut problems = Vec::new();
    for name in linked {
        let copy_skill = copy_root.join(name);
        let snap_skill = snapshot.join(name);
        match std::fs::symlink_metadata(&copy_skill) {
            Err(_) => {
                problems.push(format!("{name} is missing from the copy"));
                continue;
            }
            // A link here would be read through, and could point back at the
            // snapshot and compare equal to it.
            Ok(meta) if !meta.is_dir() => {
                problems.push(format!("{name} is no longer a directory in the copy"));
                continue;
            }
            Ok(_) => {}
        }
        if !snap_skill.exists() {
            problems.push(format!("{name} is missing from the snapshot"));
            continue;
        }
        diff_skill_trees(&copy_skill, &snap_skill, name, &mut problems);
    }
    if let Ok(entries) = std::fs::read_dir(copy_root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name == ".gitignore" {
                continue;
            }
            let name = name.to_string_lossy();
            if linked.iter().any(|linked| linked == name.as_ref()) {
                continue;
            }
            problems.push(format!("{name} was added to the copy"));
        }
    }
    if problems.is_empty() {
        None
    } else {
        Some(format!("{}: {}", COPY_MISMATCH_NOTE, problems.join("; ")))
    }
}

fn diff_skill_trees(copy: &Path, snap: &Path, prefix: &str, problems: &mut Vec<String>) {
    let copy_entries = match std::fs::read_dir(copy) {
        Ok(entries) => entries.flatten().collect::<Vec<_>>(),
        Err(err) => {
            problems.push(format!("{prefix}: cannot read copy ({err})"));
            return;
        }
    };
    let snap_entries = match std::fs::read_dir(snap) {
        Ok(entries) => entries.flatten().collect::<Vec<_>>(),
        Err(err) => {
            problems.push(format!("{prefix}: cannot read snapshot ({err})"));
            return;
        }
    };
    for entry in copy_entries {
        let name = entry.file_name();
        let rel = format!("{prefix}/{}", name.to_string_lossy());
        let snap_path = snap.join(&name);
        // The copy never carries `hydra.toml`, so one found there was added.
        // Nothing here follows a link: a link is compared by its target.
        let Ok(snap_meta) = std::fs::symlink_metadata(&snap_path) else {
            problems.push(format!("{rel} was added to the copy"));
            continue;
        };
        if name == "hydra.toml" {
            problems.push(format!("{rel} was added to the copy"));
            continue;
        }
        let copy_path = entry.path();
        let Ok(copy_meta) = std::fs::symlink_metadata(&copy_path) else {
            problems.push(format!("{rel}: cannot read copy"));
            continue;
        };
        let (copy_kind, snap_kind) = (copy_meta.file_type(), snap_meta.file_type());
        if copy_kind.is_symlink() != snap_kind.is_symlink()
            || copy_kind.is_dir() != snap_kind.is_dir()
        {
            problems.push(format!("{rel} changed type"));
            continue;
        }
        if copy_kind.is_symlink() {
            if std::fs::read_link(&copy_path).ok() != std::fs::read_link(&snap_path).ok() {
                problems.push(format!("{rel} points somewhere else"));
            }
        } else if copy_kind.is_dir() {
            diff_skill_trees(&copy_path, &snap_path, &rel, problems);
        } else if file_bytes(&copy_path) != file_bytes(&snap_path) {
            problems.push(format!("{rel} differs"));
        }
    }
    for entry in snap_entries {
        let name = entry.file_name();
        if name == "hydra.toml" {
            continue;
        }
        if std::fs::symlink_metadata(copy.join(&name)).is_err() {
            problems.push(format!(
                "{prefix}/{} is missing from the copy",
                name.to_string_lossy()
            ));
        }
    }
}

fn file_bytes(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// Contract 5.3's `skills` object, or nothing when there was no checkout to
/// inject into.
///
/// `resolved` is `null` rather than `[]` when the provider could not be asked.
/// Contract 5.3 already requires a reader to handle its absence rather than
/// treating it as an empty listing, and an empty array would claim the model saw
/// no skills at all.
pub fn record(injected: &Injected, resolved: Option<&[String]>) -> serde_json::Value {
    serde_json::json!({
        "store_sha": injected.store_sha,
        "linked": injected.linked,
        "resolved": resolved,
    })
}

/// Every store sha a live run is currently pinned to.
///
/// This is the refcount, read off the corpus. A run that has ended releases its
/// snapshot by definition, because the links inside its checkout are reaped with
/// the checkout.
pub fn referenced_shas(views: &[crate::live::RunView]) -> Vec<String> {
    let mut shas: Vec<String> = views
        .iter()
        .filter(|view| view.is_live())
        .filter_map(|view| view.skills_sha.clone())
        .collect();
    shas.sort();
    shas.dedup();
    shas
}

/// Removes one snapshot, through git so the store stops listing it.
///
/// **Through git rather than `remove_dir_all`**, so the store's own
/// `.git/worktrees/` entry goes with the directory. A snapshot removed behind
/// git's back stays registered, and `git worktree add` then refuses to make the
/// same sha again until somebody prunes it by hand.
///
/// `git worktree remove` refuses a checkout with modifications, which is the same
/// guarantee `hydra reap` leans on everywhere else. A snapshot is immutable by
/// construction, so a refusal here means somebody edited one and it is worth
/// hearing about rather than overruling with `--force`.
///
/// Under the same lock [`Store::snapshot`] takes, for the reason given there: a
/// `git worktree` command reading a sibling entry a launch is mid-way through
/// making dies on it.
pub fn remove_snapshot(store_dir: &Path, snapshot: &Path) -> Result<()> {
    if !snapshot.exists() {
        bail!("{} is not there", snapshot.display());
    }
    if !store_dir.join(".git").exists() {
        bail!(
            "{} is not a git repo, so {} cannot be removed through it",
            store_dir.display(),
            snapshot.display()
        );
    }
    let _held = snapshot.parent().map(lock_snapshots).transpose()?;
    git(
        store_dir,
        &["worktree", "remove", &snapshot.to_string_lossy()],
    )?;
    Ok(())
}
