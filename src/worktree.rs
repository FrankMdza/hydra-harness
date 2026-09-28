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
        let common = start.and_then(|s| s["scopes"]["parent_git_dir"].as_str());
        Self::for_run_from(run_id, path, common.map(Path::new))
    }

    /// The same, with the record's `scopes.parent_git_dir` already read, for a
    /// caller that reads line 0 through a typed view.
    pub fn for_run_from(run_id: &str, path: &Path, parent_git_dir: Option<&Path>) -> Result<Self> {
        if let Some(common) = parent_git_dir {
            return Self::new(path, common);
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
/// git directory is denied. Hydra still takes three things from the wider
/// configuration, each read as data and passed with `-c`: who commits, through
/// [`identity`] and [`git_as_author`], and the excludes file and every
/// `safe.directory`, through [`pinned_config`].
pub trait GitLocation {
    fn path(&self) -> &Path;

    /// `git` here reading every level of configuration. Only the reads behind
    /// [`identity`] and [`pinned_config`] use it, and `git config --get-regexp`
    /// runs no program a value names.
    ///
    /// **Never a run's object directory.** The watcher starts from the child's
    /// environment, which carries the two variables of [`run_object_env`], and
    /// a git that inherited them would read a pack the run forged and write
    /// Hydra's own objects where the run can rewrite them. So every git Hydra
    /// starts drops both and reads the parent's store alone.
    fn configured(&self) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(self.path())
            .env_remove("GIT_OBJECT_DIRECTORY")
            .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES");
        command
    }

    /// `git` here reading only the repository's own configuration, with the
    /// values of [`pinned_config`] passed on the command line.
    fn command(&self) -> Command {
        let pinned = pinned_config(self);
        let mut command = self.configured();
        command
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(pinned);
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

/// How long Hydra waits for `git config` to read the wider configuration.
///
/// `~/.gitconfig` is a file a run can write, and an `[include]` naming a fifo
/// makes git block on opening it until something writes the other end. A read
/// that does not answer in time counts as finding nothing, so the git Hydra
/// runs next reads no global value rather than hanging reap: the review of
/// build ticket 99, item 5.
pub const CONFIG_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// `git config -z --get-regexp <pattern>` at `repo` with every level of
/// configuration read, as `(key, value)` pairs in the order git lists them.
/// Empty when nothing matches. `None` when git could not start, failed, or did
/// not answer within [`CONFIG_READ_TIMEOUT`].
fn read_config<G: GitLocation + ?Sized>(repo: &G, pattern: &str) -> Option<Vec<(String, String)>> {
    use std::io::Read;
    use std::process::Stdio;

    let mut child = repo
        .configured()
        .args(["config", "-z", "--get-regexp", pattern])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let deadline = std::time::Instant::now() + CONFIG_READ_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                eprintln!(
                    "hydra: git config at {} did not answer within {}s, so the git that \
                     follows gets no value from global configuration",
                    repo.path().display(),
                    CONFIG_READ_TIMEOUT.as_secs()
                );
                return None;
            }
        }
    };
    let bytes = reader.join().ok()?;
    // Exit 1 is "no key matched", which is an answer.
    if !matches!(status.code(), Some(0) | Some(1)) {
        return None;
    }
    Some(
        bytes
            .split(|b| *b == 0)
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                let entry = String::from_utf8_lossy(entry);
                match entry.split_once('\n') {
                    Some((key, value)) => (key.to_string(), value.to_string()),
                    None => (entry.to_string(), String::new()),
                }
            })
            .collect(),
    )
}

/// `user.name` and `user.email` as git would resolve them at `repo` with every
/// level of configuration read, as `-c` arguments. Empty when neither is set,
/// and git then falls back to its own guess, as it did before.
///
/// **Read as data, never as configuration.** A run that rewrites `~/.gitconfig`
/// can change the name on a commit Hydra makes of that run's work, and nothing
/// else. The read gives up after [`CONFIG_READ_TIMEOUT`].
///
/// Nothing else a commit reads from global configuration reaches it, so a
/// human's `commit.gpgSign` does not either: Hydra's commits are unsigned.
pub fn identity<G: GitLocation + ?Sized>(repo: &G) -> Vec<String> {
    let values = read_config(repo, r"^user\.(name|email)$").unwrap_or_default();
    let mut args = Vec::new();
    for key in ["user.name", "user.email"] {
        // The last value git lists is the one it would use.
        if let Some((_, value)) = values.iter().rev().find(|(name, _)| name == key)
            && !value.trim().is_empty()
        {
            args.push("-c".to_string());
            args.push(format!("{key}={}", value.trim()));
        }
    }
    args
}

/// The `-c` arguments every git Hydra starts carries: the review of build
/// ticket 99, items 1, 2 and 5. Each value is either fixed here or read as data
/// through [`GitLocation::configured`], never taken from configuration git
/// reads on its own.
///
/// - `core.attributesFile=/dev/null`. When that key is unset git reads
///   `$XDG_CONFIG_HOME/git/attributes`, with or without global configuration,
///   and a run can write that file. An attribute there changes the bytes reap
///   commits: `*.txt text` turns a CRLF file into LF on `git add`.
/// - `core.excludesFile`, the file the human's own git reads: the configured
///   one, or git's default `$XDG_CONFIG_HOME/git/ignore`. Build ticket 99
///   stopped reading a custom one set in `~/.gitconfig`, so the seed copied
///   files the human ignores only there into the run's checkout, and reap's
///   `add -A` committed them. The value names a file, not a program. Anything
///   but a regular file becomes `/dev/null`, so a fifo there cannot hang git.
/// - every `safe.directory`. Git honours it only from system, global or
///   command-line configuration, so without it a repository that needs one
///   failed [`repo_root`], and `hydra run` launched with no worktree.
pub fn pinned_config<G: GitLocation + ?Sized>(repo: &G) -> Vec<String> {
    let values = read_config(repo, r"^(core\.excludesfile|safe\.directory)$").unwrap_or_default();
    let mut args = vec![
        "-c".to_string(),
        "core.attributesFile=/dev/null".to_string(),
    ];
    let excludes = match values
        .iter()
        .rev()
        .find(|(key, _)| key == "core.excludesfile")
    {
        Some((_, value)) => expand_home(value),
        None => default_excludes_file(),
    };
    let excludes = match excludes {
        Some(path) if std::fs::metadata(&path).is_ok_and(|meta| meta.is_file()) => {
            path.display().to_string()
        }
        _ => "/dev/null".to_string(),
    };
    args.push("-c".to_string());
    args.push(format!("core.excludesFile={excludes}"));
    for (_, value) in values.iter().filter(|(key, _)| key == "safe.directory") {
        args.push("-c".to_string());
        args.push(format!("safe.directory={value}"));
    }
    args
}

/// `~/` expanded against `HOME`, the one form of a configured path this reads.
/// `None` for an empty value, which git treats as unset.
fn expand_home(value: &str) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }
    match value.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(rest)),
        None => Some(PathBuf::from(value)),
    }
}

/// Where git looks for an excludes file when `core.excludesFile` is unset.
fn default_excludes_file() -> Option<PathBuf> {
    match std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        Some(config) => Some(PathBuf::from(config).join("git/ignore")),
        None => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/git/ignore")),
    }
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

/// What a run's own git is told about objects, map ticket 74: write into
/// `store`, the run's directory under its spawn directory, and read the
/// parent's store as an alternate.
///
/// **Environment, because git has no other way to move a checkout's object
/// directory.** No configuration key names one, and a private `commondir`
/// would move the refs with it. A run can unset either variable; its git then
/// writes into the parent's `objects/`, which the seatbelt and the gate both
/// deny, so `git add` fails rather than landing anywhere shared. The cost is
/// that git inside the run, in any repository, reads and writes this store.
pub fn run_object_env(store: &Path, common: &Path) -> [(&'static str, String); 2] {
    [
        ("GIT_OBJECT_DIRECTORY", store.display().to_string()),
        (
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            common.join("objects").display().to_string(),
        ),
    ]
}

/// Creates the run's object directory beside its gate and answers
/// [`run_object_env`] for it, or nothing when `repo` has no git directory.
pub fn prepare_run_objects(
    layout: &crate::paths::Layout,
    run_id: &str,
    repo: &Path,
) -> Result<Vec<(&'static str, String)>> {
    let Some(common) = git_common_dir(repo) else {
        return Ok(Vec::new());
    };
    let store = layout.run_objects(run_id);
    for dir in [store.join("info"), store.join("pack")] {
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    Ok(run_object_env(&store, &common).to_vec())
}

/// The most bytes of a run's store reap imports, 1 GiB. Past it the run's work
/// is committed by hashing the checkout again, which costs time and nothing
/// else.
pub const RUN_STORE_IMPORT_CAP: u64 = 1 << 30;

/// [`import_run_objects_within`] with [`RUN_STORE_IMPORT_CAP`].
pub fn import_run_objects(parent: &Path, store: &Path) -> Result<usize> {
    import_run_objects_within(parent, store, RUN_STORE_IMPORT_CAP)
}

/// Where reap copies a run's store before it packs: `reap-import` beside the
/// store, so `spawn/<run-id>/reap-import` for a run. The run's own write grant
/// is `spawn/<run-id>/objects` and the rest of `spawn/` is denied to it, so
/// nothing the run left running can write here.
pub fn run_store_copy(store: &Path) -> PathBuf {
    store.with_file_name("reap-import")
}

/// Moves a run's objects into the parent's store once git has named each one
/// from its bytes, and answers how many names the store offered.
///
/// **Git re-hashes, so the store cannot lie.** `pack-objects` reads every
/// object the store's files name, loose or packed, and `index-pack --strict`
/// in `parent` computes each object's name from what it read. A pack whose
/// index maps a sha to other bytes arrives under the sha of those bytes, which
/// is an object nobody asked for.
///
/// **Git reads a copy, never the run's store.** Re-hashing checks that an
/// object's bytes match its name, not where the bytes came from, and
/// `pack-objects` outside the seatbelt reads whatever its object directory
/// leads it to: an `info/alternates` naming another repository the user can
/// read, measured by the review of build ticket 103, or a link. Build ticket
/// 106 removed those from the store in place, and a process the run left
/// running could write `info/alternates` again between that and the pack.
/// So [`copy_run_store`] copies the store's objects into [`run_store_copy`],
/// which the run cannot write, and git packs from there. The copy holds no
/// `info/`, and each file in it was opened without following a link and
/// checked as a regular file with one link through the descriptor that was
/// then read, so no path the run changes afterwards reaches git. Build ticket
/// 112.
///
/// A store over `cap` bytes imports nothing, and `index-pack` refuses a
/// stream over `cap` too. Either way reap still commits the run's work: it
/// hashes the checkout again. [`import_run_objects`] passes
/// [`RUN_STORE_IMPORT_CAP`].
///
/// A store that is not a real directory offers nothing. Reap commits from the
/// checkout either way: [`unstage_missing_objects`] takes back any index
/// entry whose object did not arrive.
pub fn import_run_objects_within(parent: &Path, store: &Path, cap: u64) -> Result<usize> {
    import_run_objects_racing(parent, store, cap, || {})
}

/// [`import_run_objects_within`], calling `between` once the copy is made and
/// before git reads it. A test writes into the run's store there, the moment
/// a process the run left running would.
#[doc(hidden)]
pub fn import_run_objects_racing(
    parent: &Path,
    store: &Path,
    cap: u64,
    between: impl FnOnce(),
) -> Result<usize> {
    if !std::fs::symlink_metadata(store).is_ok_and(|meta| meta.is_dir()) {
        return Ok(0);
    }
    let copy = run_store_copy(store);
    let imported = copy_run_store(store, &copy, cap).and_then(|()| {
        between();
        pack_into_parent(parent, &copy, cap)
    });
    let removed = match std::fs::remove_dir_all(&copy) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    };
    let imported = imported?;
    removed.with_context(|| format!("removing {}", copy.display()))?;
    Ok(imported)
}

/// `pack-objects` over `objects` alone, piped into `index-pack --strict` in
/// `parent`.
fn pack_into_parent(parent: &Path, objects: &Path, cap: u64) -> Result<usize> {
    use std::process::Stdio;

    let names = store_object_names(parent, objects)?;
    if names.is_empty() {
        return Ok(0);
    }
    let mut pack = parent
        .command()
        .args(["pack-objects", "--stdout", "-q"])
        .env("GIT_OBJECT_DIRECTORY", objects)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("starting git pack-objects over the copy of the run's store")?;
    let packed = pack.stdout.take().expect("stdout is piped");
    let max_input = format!("--max-input-size={cap}");
    let index = parent
        .command()
        .args(["index-pack", "--stdin", "--strict", &max_input])
        .stdin(Stdio::from(packed))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("starting git index-pack in the parent")?;
    {
        use std::io::Write;
        let mut input = pack.stdin.take().expect("stdin is piped");
        for name in &names {
            writeln!(input, "{name}")?;
        }
    }
    let packed = pack.wait_with_output()?;
    let indexed = index.wait_with_output()?;
    if !packed.status.success() || !indexed.status.success() {
        bail!(
            "git would not verify the run's objects copied to {}: {}{}",
            objects.display(),
            String::from_utf8_lossy(&packed.stderr).trim(),
            String::from_utf8_lossy(&indexed.stderr).trim()
        );
    }
    Ok(names.len())
}

/// Copies the objects of a run's store into `copy`, a fresh directory: the
/// loose objects of each fan-out directory and the `.pack` and `.idx` files
/// of `pack/`. Nothing else, so no `info/`.
///
/// **Every step goes through a descriptor.** Each directory and file is opened
/// relative to the directory above it with `O_NOFOLLOW`, so a link anywhere
/// fails to open and is skipped. A file is copied only when `fstat` on the
/// descriptor answers a regular file with one link, and the bytes come from
/// that same descriptor, so swapping the path afterwards changes nothing that
/// is read. A second hard link would be a file of another repository, and git
/// writes every loose object and pack with one.
///
/// More than `cap` bytes read fails the copy, counted as the bytes arrive, so
/// a file that grows while it is read is caught too.
fn copy_run_store(store: &Path, copy: &Path, cap: u64) -> Result<()> {
    use std::io::{Read, Write};
    use std::os::fd::AsFd;

    match std::fs::remove_dir_all(copy) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
            return Err(err).with_context(|| format!("removing a stale {}", copy.display()));
        }
        _ => {}
    }
    std::fs::create_dir(copy).with_context(|| format!("creating {}", copy.display()))?;

    let is_hex = |name: &str| !name.is_empty() && name.bytes().all(|b| b.is_ascii_hexdigit());
    let root = nofollow::open_dir(None, store.as_os_str())
        .with_context(|| format!("opening {}", store.display()))?;
    let mut size: u64 = 0;
    for dir in nofollow::entries(root.as_fd())? {
        let name = dir.to_string_lossy().into_owned();
        let pack = name == "pack";
        if !pack && (name.len() != 2 || !is_hex(&name)) {
            continue;
        }
        // A link, or anything else that is not a directory, fails here.
        let Ok(inner) = nofollow::open_dir(Some(root.as_fd()), &dir) else {
            continue;
        };
        let target = copy.join(&name);
        std::fs::create_dir(&target).with_context(|| format!("creating {}", target.display()))?;
        for file in nofollow::entries(inner.as_fd())? {
            let file_name = file.to_string_lossy().into_owned();
            let wanted = if pack {
                file_name.ends_with(".pack") || file_name.ends_with(".idx")
            } else {
                is_hex(&file_name)
            };
            if !wanted {
                continue;
            }
            let Ok(source) = nofollow::open_regular(inner.as_fd(), &file) else {
                continue;
            };
            let mut source = std::fs::File::from(source);
            let mut out = std::fs::File::create_new(target.join(&file_name))
                .with_context(|| format!("creating {}", target.join(&file_name).display()))?;
            let room = cap.saturating_sub(size);
            let copied = std::io::copy(&mut (&mut source).take(room.saturating_add(1)), &mut out)
                .with_context(|| {
                format!("copying {name}/{file_name} out of the run's store")
            })?;
            size += copied;
            if size > cap {
                bail!(
                    "the run's store holds more than the {cap} bytes reap imports; its work is \
                     hashed again from the checkout instead"
                );
            }
            out.flush()?;
        }
    }
    Ok(())
}

/// The descriptor calls [`copy_run_store`] walks the store with.
mod nofollow {
    use std::ffi::{CString, OsStr, OsString};
    use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    use anyhow::{Context, Result};

    fn open(at: Option<BorrowedFd<'_>>, name: &OsStr, flags: libc::c_int) -> Result<OwnedFd> {
        let name = CString::new(name.as_bytes()).context("a name with a NUL byte")?;
        let flags = flags | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        // SAFETY: `name` is a valid C string for the call, and `at` is a live
        // descriptor borrowed for it.
        let fd = unsafe {
            match at {
                Some(dir) => libc::openat(dir.as_raw_fd(), name.as_ptr(), flags),
                None => libc::open(name.as_ptr(), flags),
            }
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: `open` just returned this descriptor and nothing else owns it.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    /// A directory, refused when the last component is a link.
    pub fn open_dir(at: Option<BorrowedFd<'_>>, name: &OsStr) -> Result<OwnedFd> {
        open(at, name, libc::O_RDONLY | libc::O_DIRECTORY)
    }

    /// A regular file with one link, refused when it is a link or anything else.
    /// `O_NONBLOCK` so a fifo cannot hang the open.
    pub fn open_regular(at: BorrowedFd<'_>, name: &OsStr) -> Result<OwnedFd> {
        let fd = open(Some(at), name, libc::O_RDONLY | libc::O_NONBLOCK)?;
        // SAFETY: `stat` is plain data, and `fd` is live for the call.
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: `fd` is live for the call and `stat` is writable `fstat` output.
        if unsafe { libc::fstat(fd.as_raw_fd(), &mut stat) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        anyhow::ensure!(
            stat.st_mode & libc::S_IFMT == libc::S_IFREG && stat.st_nlink == 1,
            "not a regular file with one link"
        );
        Ok(fd)
    }

    /// The names in the directory `dir` is open on, without `.` and `..`.
    pub fn entries(dir: BorrowedFd<'_>) -> Result<Vec<OsString>> {
        let own = dir.try_clone_to_owned()?;
        // SAFETY: `fdopendir` takes ownership of the duplicate on success, and
        // `closedir` below releases it. On failure the duplicate is still ours.
        let stream = unsafe { libc::fdopendir(own.as_raw_fd()) };
        if stream.is_null() {
            return Err(std::io::Error::last_os_error().into());
        }
        std::mem::forget(own);
        let mut names = Vec::new();
        loop {
            // SAFETY: `stream` is open until `closedir`, and each entry is read
            // before the next `readdir` call.
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break;
            }
            // SAFETY: `entry` came from `readdir` on this iteration and `d_name` is
            // a NUL-terminated name within it.
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            let name = name.to_bytes();
            if name != b"." && name != b".." {
                names.push(OsString::from_vec(name.to_vec()));
            }
        }
        // SAFETY: `stream` came from `fdopendir` and is closed once.
        unsafe { libc::closedir(stream) };
        Ok(names)
    }
}

/// Every object name the store's own files offer: loose objects in its
/// fan-out directories and the entries of each pack index. The names are
/// claims, and [`import_run_objects_within`] lets git check them.
///
/// **This listing skips links, and that alone never kept git from following
/// one.** `pack-objects` looks each name up by itself, through `info/` and
/// any link in its object directory, the review of build ticket 103, item 2.
/// So the directory this reads is [`copy_run_store`]'s copy, which holds
/// neither.
fn store_object_names(parent: &Path, store: &Path) -> Result<Vec<String>> {
    let is_hex = |name: &str| !name.is_empty() && name.bytes().all(|b| b.is_ascii_hexdigit());
    let real_dir = |path: &Path| std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir());
    let real_file = |path: &Path| std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file());
    let mut names = Vec::new();
    for entry in std::fs::read_dir(store)? {
        let entry = entry?;
        let fan = entry.file_name().to_string_lossy().into_owned();
        if fan.len() != 2 || !is_hex(&fan) || !real_dir(&entry.path()) {
            continue;
        }
        for object in std::fs::read_dir(entry.path())? {
            let object = object?;
            let rest = object.file_name().to_string_lossy().into_owned();
            if is_hex(&rest) && real_file(&object.path()) {
                names.push(format!("{fan}{rest}"));
            }
        }
    }
    let packs = store.join("pack");
    if real_dir(&packs) {
        for entry in std::fs::read_dir(&packs)? {
            let path = entry?.path();
            if path.extension().is_none_or(|ext| ext != "idx") || !real_file(&path) {
                continue;
            }
            let out = parent
                .command()
                .arg("show-index")
                .stdin(std::fs::File::open(&path)?)
                .output()?;
            if !out.status.success() {
                continue;
            }
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(name) = line.split_whitespace().nth(1).filter(|name| is_hex(name)) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

/// Takes out of the checkout's index every entry whose object the parent's
/// store does not hold, and answers their paths.
///
/// Called by reap after [`import_run_objects`]. An entry the run staged
/// against an object that failed git's check would otherwise stop the commit,
/// or, if the stat data still matched, keep a name whose bytes were never
/// verified. Removed from the index, the path is hashed again from the
/// checkout by the `git add -A` that follows. A submodule entry names a
/// commit in another repository and is left alone.
pub fn unstage_missing_objects<G: GitLocation + ?Sized>(checkout: &G) -> Result<Vec<String>> {
    use std::io::Write;
    use std::process::Stdio;

    let listing = git_bytes(checkout, &["ls-files", "--stage", "-z"])?;
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for record in listing.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let Some(tab) = record.iter().position(|b| *b == b'\t') else {
            continue;
        };
        let head = String::from_utf8_lossy(&record[..tab]).into_owned();
        let mut fields = head.split(' ');
        let (Some(mode), Some(sha)) = (fields.next(), fields.next()) else {
            continue;
        };
        if mode != "160000" {
            entries.push((sha.to_string(), record[tab + 1..].to_vec()));
        }
    }
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let mut check = checkout
        .command()
        .args(["cat-file", "--batch-check=%(objectname) %(objecttype)"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // Fed from a thread: git answers line by line, and a large index would
    // fill the answer pipe while this side was still writing questions.
    let questions: String = entries.iter().map(|(sha, _)| format!("{sha}\n")).collect();
    let mut input = check.stdin.take().expect("stdin is piped");
    let feeder = std::thread::spawn(move || input.write_all(questions.as_bytes()));
    let out = check.wait_with_output()?;
    feeder
        .join()
        .map_err(|_| anyhow::anyhow!("the thread feeding git cat-file panicked"))??;
    if !out.status.success() {
        bail!(
            "git cat-file --batch-check failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let answers: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    let missing: Vec<&Vec<u8>> = entries
        .iter()
        .zip(&answers)
        .filter(|(_, answer)| answer.ends_with(" missing"))
        .map(|((_, path), _)| path)
        .collect();
    if missing.is_empty() {
        return Ok(Vec::new());
    }
    let mut remove = checkout
        .command()
        .args(["update-index", "--force-remove", "-z", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut input = remove.stdin.take().expect("stdin is piped");
        for path in &missing {
            input.write_all(path)?;
            input.write_all(&[0])?;
        }
    }
    let out = remove.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "git update-index --force-remove failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(missing
        .iter()
        .map(|path| String::from_utf8_lossy(path).into_owned())
        .collect())
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
pub fn roll_back(repo: &Path, dest: &Path, branch: &str) -> Result<()> {
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
    use super::{CheckoutGit, GitLocation, inside_checkout};

    #[test]
    fn no_git_hydra_starts_inherits_a_runs_object_directory() {
        // The watcher's environment is the child's, and the child's carries
        // both variables. Every runner has to take them back out.
        let checkout = CheckoutGit::new(
            std::path::Path::new("/tmp/wt/r1"),
            std::path::Path::new("/tmp/parent/.git"),
        )
        .unwrap();
        let plain = std::path::Path::new("/tmp/parent");
        for command in [
            checkout.command(),
            checkout.configured(),
            plain.command(),
            plain.configured(),
        ] {
            for name in ["GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES"] {
                assert!(
                    command
                        .get_envs()
                        .any(|(key, value)| key == name && value.is_none()),
                    "{name} is inherited by {command:?}"
                );
            }
        }
    }

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
