//! The goal log: one append-only JSONL file per goal under the puppets home.
//!
//! Only newline-terminated lines count, and only `\n` ends a line. A newline-terminated line
//! that is blank or is not JSON makes the log damaged: readers and writers refuse with
//! `DamagedLog`. An unterminated tail is not damage; readers stop before it and writers raise
//! `UnterminatedLog`.
//!
//! `repair --yes` replaces the log by rename, so the file a path names can change while a
//! process waits for the lock. Everything that locks the log goes through `open_locked`,
//! which checks, once it holds the lock, that the path still names the file it locked.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use crate::json::{self, Dict, ParseError, Value};

/// Where the helper reads and writes, from the same environment the Python helper read.
#[derive(Clone, Debug)]
pub struct Home {
    /// `HYDRA_PUPPETS_HOME`, else `~/hydra/puppets`: goal logs and per-goal plan copies.
    pub puppets: PathBuf,
    /// `HYDRA_GRADER_HOME`, else `$HYDRA_ROOT/grader`: `scores.jsonl` for `queue` and `board`.
    pub grader: PathBuf,
    /// `HYDRA_PUPPETS_VENDORS` when it is set. [`Home::vendors_file`] is how it is read:
    /// with none named the file is `vendors.json` in [`data_dir`], which has to exist.
    pub vendors: Option<PathBuf>,
    /// `HYDRA_ROOT`, else `~/hydra`: `runs/` and `worktrees/`, which the launch commands read.
    pub root: PathBuf,
}

/// Where a checkout keeps the helper's data files: `vendors.json`, `priors.json`,
/// `aliases.json` and `prompts/`.
const DATA_DIR: &str = "integrations/puppets";

/// The directory the data files are read from, found when the helper runs and never
/// compiled in (build 196): `HYDRA_PUPPETS_DATA` when set, else `integrations/puppets`
/// under `HYDRA_HOME`, the variable `hydra` finds its own checkout by, else the first
/// directory above this binary that holds `integrations/puppets/prompts`, which is the
/// checkout for a binary under its `target/`. With none of those the path is the one
/// beside the binary, so the error that follows names where the files were looked for.
///
/// **A directory that does not exist is an error, not a directory with no files in it**
/// (build 221). A file missing from a data directory that exists keeps the reading the
/// recorded Python gave it, and a missing `vendors.json` there means every provider is up.
/// A helper copied out of its checkout, with neither variable set, has no data directory
/// at all, and reading that the same way would sit a provider the checkout marks down.
pub fn data_dir() -> Result<PathBuf> {
    existing(data_dir_from(
        named("HYDRA_PUPPETS_DATA"),
        named("HYDRA_HOME"),
        std::env::current_exe().ok().as_deref(),
    ))
}

/// `dir`, or the one line that says it is not there and how to name another.
fn existing(dir: PathBuf) -> Result<PathBuf> {
    if !dir.is_dir() {
        bail!(
            "no data directory at {}; set HYDRA_PUPPETS_DATA to the directory that holds \
             vendors.json, or HYDRA_HOME to the checkout",
            dir.display()
        );
    }
    Ok(dir)
}

/// The vendors file: the one named, else `vendors.json` in the data directory.
fn vendors_from(named: Option<&Path>, data: impl FnOnce() -> Result<PathBuf>) -> Result<PathBuf> {
    match named {
        Some(path) => Ok(path.to_path_buf()),
        None => Ok(data()?.join("vendors.json")),
    }
}

fn data_dir_from(
    data: Option<PathBuf>,
    hydra_home: Option<PathBuf>,
    exe: Option<&Path>,
) -> PathBuf {
    if let Some(data) = data {
        return data;
    }
    if let Some(home) = hydra_home {
        return home.join(DATA_DIR);
    }
    let exe = exe.map(|exe| exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf()));
    let above = exe
        .as_deref()
        .map(Path::ancestors)
        .into_iter()
        .flatten()
        .skip(1);
    for dir in above {
        let found = dir.join(DATA_DIR);
        if found.join("prompts").is_dir() {
            return found;
        }
    }
    match exe.as_deref().and_then(Path::parent) {
        Some(beside) => beside.join(DATA_DIR),
        None => PathBuf::from(DATA_DIR),
    }
}

/// The data files of the checkout the tests were built from.
#[cfg(test)]
pub(crate) fn checkout_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DATA_DIR)
}

fn named(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

impl Home {
    pub fn from_env() -> Result<Home> {
        let user_home = || -> Result<PathBuf> {
            std::env::home_dir()
                .ok_or_else(|| anyhow!("RuntimeError: Could not determine home directory."))
        };
        let puppets = match named("HYDRA_PUPPETS_HOME") {
            Some(path) => path,
            None => user_home()?.join("hydra").join("puppets"),
        };
        let root = match named("HYDRA_ROOT") {
            Some(path) => path,
            None => user_home()?.join("hydra"),
        };
        let grader = named("HYDRA_GRADER_HOME").unwrap_or_else(|| root.join("grader"));
        let vendors = named("HYDRA_PUPPETS_VENDORS");
        Ok(Home {
            puppets,
            grader,
            vendors,
            root,
        })
    }

    /// A home rooted in one directory, for tests: `puppets/`, `grader/`, `vendors.json` and
    /// `root/`.
    pub fn at(dir: &Path) -> Home {
        Home {
            puppets: dir.join("puppets"),
            grader: dir.join("grader"),
            vendors: Some(dir.join("vendors.json")),
            root: dir.join("root"),
        }
    }

    /// The file `provider_available` reads. An error when no file is named and the data
    /// directory does not exist.
    pub fn vendors_file(&self) -> Result<PathBuf> {
        vendors_from(self.vendors.as_deref(), data_dir)
    }

    pub fn goal_log_path(&self, goal_id: &str) -> PathBuf {
        self.puppets.join(format!("{goal_id}.jsonl"))
    }
}

/// The file's lines, each with its newline; the last has none when the file does not end on
/// one. Only `\n` ends a line. Python's `bytes.splitlines` also breaks on a lone `\r`, which
/// made `BAD\r` in the middle of a log read as an unterminated tail and hid every line after
/// it. A CRLF line still parses, since `\r` is JSON whitespace.
fn split_lines(raw: &[u8]) -> Vec<&[u8]> {
    raw.split_inclusive(|b| *b == b'\n').collect()
}

/// Blank, not decodable, not JSON, or holding an integer past Python's digit limit: what
/// `json.loads(line)` raises `ValueError` or one of its subclasses on. The encoding is the one
/// `json.detect_encoding` reads off the line's first bytes, so a line may be UTF-16 or UTF-32.
///
/// Nesting past `json::MAX_NESTING` is not one of those: Python raises `RecursionError`
/// there, which is not a `ValueError`, so the command dies and the line is not moved aside.
fn bad_line(line: &[u8]) -> Result<bool> {
    match json::loads_bytes(line) {
        Ok(_) => Ok(false),
        Err(ParseError::Recursion) => bail!("RecursionError: goal log line is nested too deep"),
        Err(_) => Ok(true),
    }
}

fn damaged_line_numbers(raw: &[u8]) -> Result<Vec<usize>> {
    let mut bad = Vec::new();
    for (index, line) in split_lines(raw).iter().enumerate() {
        if !line.ends_with(b"\n") {
            break;
        }
        if bad_line(line)? {
            bad.push(index + 1);
        }
    }
    Ok(bad)
}

fn newline_terminated_events(raw: &[u8]) -> Result<Vec<Value>> {
    let mut events = Vec::new();
    for line in split_lines(raw) {
        if !line.ends_with(b"\n") {
            break;
        }
        match json::loads_bytes(line) {
            Ok(event) => events.push(event),
            Err(err) => bail!("{}: goal log line is not JSON", err.class()),
        }
    }
    Ok(events)
}

/// The goal log's events: newline-terminated lines up to the first that is not.
pub fn read_log(home: &Home, goal_id: &str) -> Result<Vec<Value>> {
    let path = home.goal_log_path(goal_id);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let bad = damaged_line_numbers(&raw)?;
    if !bad.is_empty() {
        return Err(DamagedLog {
            goal_id: goal_id.to_string(),
            line_numbers: bad,
        }
        .into());
    }
    newline_terminated_events(&raw)
}

/// A newline-terminated line is blank or is not JSON.
#[derive(Debug)]
pub struct DamagedLog {
    pub goal_id: String,
    pub line_numbers: Vec<usize>,
}

impl std::fmt::Display for DamagedLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nums = self
            .line_numbers
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let goal = &self.goal_id;
        write!(
            f,
            "goal log {goal} is damaged at line {nums}; wrote nothing; hydra-puppets repair {goal}"
        )
    }
}

impl std::error::Error for DamagedLog {}

/// A writer found the log ending without a newline and appended nothing.
#[derive(Debug)]
pub struct UnterminatedLog {
    pub goal_id: String,
}

impl std::fmt::Display for UnterminatedLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let goal = &self.goal_id;
        write!(
            f,
            "goal log {goal} ends without a newline; wrote nothing; hydra-puppets repair {goal}"
        )
    }
}

impl std::error::Error for UnterminatedLog {}

/// `utc_now()`: UTC to the millisecond with a `Z`.
pub fn utc_now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// The goal log, open, with an exclusive `flock` held until this is dropped.
struct Locked(File);

impl Drop for Locked {
    fn drop(&mut self) {
        // SAFETY: `flock` reads only the descriptor, which `self.0` keeps open until after
        // this returns; unlocking a descriptor this guard locked.
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

/// Open the goal log and lock it, on the file the path names.
///
/// A `repair --yes` that ran while this waited for the lock has renamed a new file over the
/// path, and the descriptor opened before that is of a file no path names any more: reading
/// it shows the old damage and appending to it loses the write. So once the lock is held
/// this compares the descriptor's inode with the path's and opens again when they differ.
fn open_locked(path: &Path, options: &OpenOptions) -> std::io::Result<Locked> {
    loop {
        let file = options.open(path)?;
        // SAFETY: `flock` reads only the descriptor, which `file` keeps open.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let locked = Locked(file);
        let held = locked.0.metadata()?;
        match std::fs::metadata(path) {
            Ok(named) if named.dev() == held.dev() && named.ino() == held.ino() => {
                return Ok(locked);
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
}

/// Put `content` where the goal log is, in one step: write it to a temporary file beside the
/// log, fsync, and rename it over the log. The caller holds the log's lock. `mode` is the
/// old log's, which the new one keeps.
#[derive(Debug)]
pub struct UnsafeRepairTemporary;
impl std::fmt::Display for UnsafeRepairTemporary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UnsafeRepairTemporary: repair temporary path is not a regular file; wrote nothing"
        )
    }
}
impl std::error::Error for UnsafeRepairTemporary {}

fn check_temporary(tmp: &Path) -> Result<()> {
    match std::fs::symlink_metadata(tmp) {
        Ok(meta) if meta.file_type().is_file() => Ok(()),
        Ok(_) => Err(UnsafeRepairTemporary.into()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn replace_log(path: &Path, content: &[u8], mode: u32) -> Result<()> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".repair-tmp");
    let tmp = path.with_file_name(name);
    let create = || {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&tmp)
    };
    let mut out = match create() {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            check_temporary(&tmp)?;
            std::fs::remove_file(&tmp)?;
            create()?
        }
        Err(err) => return Err(err.into()),
    };
    let written = (|| -> std::io::Result<()> {
        out.set_permissions(std::fs::Permissions::from_mode(mode & 0o7777))?;
        out.write_all(content)?;
        out.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(err) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(err).with_context(|| format!("replacing {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

struct Partitioned {
    good: Vec<Vec<u8>>,
    bad: Vec<(usize, Vec<u8>)>,
    tail: Option<Vec<u8>>,
}

fn partition_log(raw: &[u8]) -> Result<Partitioned> {
    let mut good = Vec::new();
    let mut bad = Vec::new();
    let mut tail = None;
    for (index, line) in split_lines(raw).iter().enumerate() {
        if !line.ends_with(b"\n") {
            tail = Some(line.to_vec());
            break;
        }
        if bad_line(line)? {
            bad.push((index + 1, line.to_vec()));
        } else {
            good.push(line.to_vec());
        }
    }
    Ok(Partitioned { good, bad, tail })
}

/// Hold the goal log's lock, read it, and append what `decide` returns.
///
/// `decide` sees the newline-terminated good lines read under the lock. Damage is
/// refused before `decide`. When the file ends without a newline this returns
/// `UnterminatedLog` after `decide` runs and writes nothing. `None` from `decide` writes
/// nothing. The appended record is `{"seq": <lines read + 1>, "ts": <now>, ...body}`,
/// compact, on one line.
pub fn locked_update(
    home: &Home,
    goal_id: &str,
    decide: impl FnOnce(&[Value]) -> Result<Option<Dict>>,
) -> Result<Option<Dict>> {
    let path = home.goal_log_path(goal_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let locked = open_locked(
        &path,
        OpenOptions::new().read(true).append(true).create(true),
    )
    .with_context(|| format!("opening {}", path.display()))?;
    let mut file = &locked.0;
    let mut raw = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut raw)?;
    let bad = damaged_line_numbers(&raw)?;
    if !bad.is_empty() {
        return Err(DamagedLog {
            goal_id: goal_id.to_string(),
            line_numbers: bad,
        }
        .into());
    }
    let events = newline_terminated_events(&raw)?;
    let unterminated = torn_tail(&raw).is_some();
    let body = decide(&events)?;
    if unterminated {
        return Err(UnterminatedLog {
            goal_id: goal_id.to_string(),
        }
        .into());
    }
    let Some(body) = body else {
        return Ok(None);
    };
    let mut record = Dict::new();
    record.set("seq", Value::Int(events.len() as i128 + 1));
    record.set("ts", Value::text(&utc_now()));
    for (key, value) in body.iter() {
        record.set_key(key.clone(), value.clone());
    }
    let line = json::dumps(&Value::Dict(record.clone()), None) + "\n";
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(Some(record))
}

/// The bytes after the last newline when the file does not end on one.
pub fn torn_tail(raw: &[u8]) -> Option<&[u8]> {
    if raw.is_empty() || raw.ends_with(b"\n") {
        return None;
    }
    let start = raw.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
    Some(&raw[start..])
}

/// `repair`: print damage and a torn tail, or with `yes` move them aside and replace the log,
/// under the log's lock. Returns the exit code.
///
/// The new log is its good lines and, when one or more bad lines went, a `repair` event. It
/// is written whole to a temporary file beside the log and renamed over it, so a repair
/// killed at any point leaves the old log or the new one. Writing in place could be killed
/// between the good lines and the event, which left a clean log whose old approvals were
/// still live.
pub fn repair(home: &Home, goal_id: &str, yes: bool, out: &mut dyn Write) -> Result<i32> {
    let path = home.goal_log_path(goal_id);
    if !path.is_file() {
        writeln!(out, "goal log {goal_id} has no torn tail")?;
        return Ok(0);
    }
    let locked = open_locked(&path, OpenOptions::new().read(true))
        .with_context(|| format!("opening {}", path.display()))?;
    let mut file = &locked.0;
    let mut raw = Vec::new();
    file.read_to_end(&mut raw)?;
    let parts = partition_log(&raw)?;
    if parts.bad.is_empty() && parts.tail.is_none() {
        writeln!(out, "goal log {goal_id} has no torn tail")?;
        return Ok(0);
    }
    if !yes {
        for (line_no, line) in &parts.bad {
            let preview: String = String::from_utf8_lossy(line)
                .trim_end_matches(['\r', '\n'])
                .chars()
                .take(300)
                .collect();
            writeln!(out, "bad line {line_no}: {preview}")?;
        }
        if let Some(tail) = &parts.tail {
            let preview: String = String::from_utf8_lossy(tail).chars().take(300).collect();
            writeln!(out, "unterminated tail: {preview}")?;
        }
        writeln!(
            out,
            "would move the bad lines and any unterminated tail into \
             {goal_id}.torn-<UTC timestamp>.txt beside the log, then rewrite the log with \
             only its good lines"
        )?;
        return Ok(0);
    }
    check_temporary(&path.with_file_name(format!("{goal_id}.jsonl.repair-tmp")))?;
    let dest = path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{goal_id}.torn-{}.txt", utc_now()));
    let mut torn = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&dest)
        .with_context(|| format!("FileExistsError: {}", dest.display()))?;
    for (_line_no, line) in &parts.bad {
        torn.write_all(line)?;
    }
    if let Some(tail) = &parts.tail {
        torn.write_all(tail)?;
    }
    torn.flush()?;
    torn.sync_all()?;
    let mut new_raw: Vec<u8> = parts.good.concat();
    let removed = parts.bad.len();
    if removed > 0 {
        let mut record = Dict::new();
        record.set("seq", Value::Int(parts.good.len() as i128 + 1));
        record.set("ts", Value::text(&utc_now()));
        record.set("event", Value::text("repair"));
        record.set("removed", Value::Int(removed as i128));
        new_raw.extend_from_slice(json::dumps(&Value::Dict(record), None).as_bytes());
        new_raw.push(b'\n');
    }
    replace_log(&path, &new_raw, file.metadata()?.mode())?;
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    writeln!(out, "moved the tail into {name}")?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> (tempdir::Dir, Home) {
        let dir = tempdir::Dir::new();
        let home = Home::at(dir.path());
        std::fs::create_dir_all(&home.puppets).unwrap();
        (dir, home)
    }

    /// A temp directory removed on drop, without a dependency for it.
    mod tempdir {
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicUsize, Ordering};

        pub struct Dir(PathBuf);

        impl Dir {
            pub fn new() -> Dir {
                static N: AtomicUsize = AtomicUsize::new(0);
                let path = std::env::temp_dir().join(format!(
                    "hydra-puppets-log-{}-{}",
                    std::process::id(),
                    N.fetch_add(1, Ordering::SeqCst)
                ));
                let _ = std::fs::remove_dir_all(&path);
                std::fs::create_dir_all(&path).unwrap();
                Dir(path)
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    const GOAL: &[u8] = b"{\"seq\":1,\"ts\":\"t\",\"event\":\"goal\",\"text\":\"x\",\"repo\":\"/r\",\"base_sha\":\"a\"}\n";

    fn body(event: &str) -> Dict {
        let mut body = Dict::new();
        body.set("event", Value::text(event));
        body
    }

    #[test]
    fn repair_refuses_temporary_symlink_without_writing() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        let before = [GOAL, b"torn"].concat();
        std::fs::write(&path, &before).unwrap();
        let tmp = path.with_file_name("g1.jsonl.repair-tmp");
        std::os::unix::fs::symlink(&path, &tmp).unwrap();
        let err = repair(&home, "g1", true, &mut Vec::new()).unwrap_err();
        assert!(err.to_string().contains("UnsafeRepairTemporary"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(tmp.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_dir(&home.puppets).unwrap().count(), 2);
        std::fs::remove_file(&tmp).unwrap();
        std::fs::create_dir(&tmp).unwrap();
        assert!(repair(&home, "g1", true, &mut Vec::new()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_dir(&tmp).unwrap();
        std::fs::write(&tmp, b"stale partial repair").unwrap();
        repair(&home, "g1", true, &mut Vec::new()).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), GOAL);
        assert!(!tmp.exists());
    }

    #[test]
    fn only_a_newline_ends_a_line() {
        let lines = split_lines(b"a\r\nb\rc\n\nd");
        assert_eq!(lines, vec![&b"a\r\n"[..], b"b\rc\n", b"\n", b"d"]);
        assert!(split_lines(b"").is_empty());
        assert_eq!(split_lines(b"x\r"), vec![&b"x\r"[..]]);
    }

    const NOTE: &[u8] = b"{\"event\":\"note\"}\n";

    fn torn_files(home: &Home) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&home.puppets)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_string_lossy().contains(".torn-"))
            .collect();
        found.sort();
        found
    }

    fn event_names(home: &Home) -> Vec<String> {
        read_log(home, "g1")
            .unwrap()
            .iter()
            .map(|event| match event {
                Value::Dict(d) => match d.get("event") {
                    Some(Value::Str(name)) => name.lossy(),
                    _ => String::new(),
                },
                _ => String::new(),
            })
            .collect()
    }

    /// Build 199, item 13: `BAD\r` in the middle of the file is part of a bad line, not an
    /// unterminated tail that hides the lines after it.
    #[test]
    fn a_lone_carriage_return_inside_the_file_is_damage_and_not_a_tail() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        let raw = [GOAL, b"BAD\r", NOTE, NOTE].concat();
        std::fs::write(&path, &raw).unwrap();
        let err = read_log(&home, "g1").unwrap_err();
        assert_eq!(
            err.downcast_ref::<DamagedLog>().unwrap().line_numbers,
            vec![2]
        );
        let err = locked_update(&home, "g1", |_| Ok(Some(body("note")))).unwrap_err();
        assert!(err.downcast_ref::<DamagedLog>().is_some());
        assert_eq!(std::fs::read(&path).unwrap(), raw);
        let mut out = Vec::new();
        repair(&home, "g1", true, &mut out).unwrap();
        // The bad line runs to the next newline: it takes the first note and no more.
        let torn = torn_files(&home);
        assert_eq!(std::fs::read(&torn[0]).unwrap(), [b"BAD\r", NOTE].concat());
        assert_eq!(event_names(&home), ["goal", "note", "repair"]);
        // CRLF still parses, and a tail that ends in CR is still a tail.
        std::fs::write(&path, b"{\"event\":\"a\"}\r\n{\"event\":\"b\"}\r").unwrap();
        assert_eq!(event_names(&home), ["a"]);
    }

    /// Build 199, item 11: the new log arrives whole, by rename.
    #[test]
    fn repair_replaces_the_log_by_rename_and_keeps_its_mode() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, b"not json\n", NOTE].concat()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        let before = std::fs::metadata(&path).unwrap().ino();
        let mut out = Vec::new();
        assert_eq!(repair(&home, "g1", true, &mut out).unwrap(), 0);
        let after = std::fs::metadata(&path).unwrap();
        assert_ne!(before, after.ino());
        assert_eq!(after.mode() & 0o777, 0o640);
        assert_eq!(event_names(&home), ["goal", "note", "repair"]);
        let last = read_log(&home, "g1").unwrap().pop().unwrap();
        let line = json::dumps(&last, None);
        assert!(line.starts_with("{\"seq\":3,\"ts\":\""), "{line}");
        assert!(
            line.ends_with("\",\"event\":\"repair\",\"removed\":1}"),
            "{line}"
        );
        // The log, the torn file, and no temporary file left beside them.
        assert_eq!(std::fs::read_dir(&home.puppets).unwrap().count(), 2);
    }

    /// Hold the lock on the file the path names now, as a writer mid-update does, and hand
    /// back the handle that holds it.
    fn hold_lock(path: &Path) -> File {
        let file = File::open(path).unwrap();
        // SAFETY: `flock` reads only the descriptor, which `file` keeps open.
        assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) }, 0);
        file
    }

    /// What `repair --yes` does to the path while another process waits on the old file.
    fn replace_by_rename(path: &Path, content: &[u8]) {
        let tmp = path.with_extension("swap");
        std::fs::write(&tmp, content).unwrap();
        std::fs::rename(&tmp, path).unwrap();
    }

    /// Build 199, item 11: a writer that opened the log before a repair replaced it must
    /// not append to the file the rename left behind.
    #[test]
    fn a_writer_waiting_on_the_old_file_appends_to_the_one_the_path_names() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, b"not json\n"].concat()).unwrap();
        let holder = hold_lock(&path);
        std::thread::scope(|scope| {
            let writer = scope.spawn(|| locked_update(&home, "g1", |_| Ok(Some(body("note")))));
            // Long enough for the writer to open the old file and block on its lock.
            std::thread::sleep(std::time::Duration::from_millis(300));
            assert!(!writer.is_finished());
            replace_by_rename(&path, &[GOAL, NOTE].concat());
            drop(holder);
            let record = writer.join().unwrap().unwrap().unwrap();
            assert!(matches!(record.get("seq"), Some(Value::Int(3))));
        });
        assert_eq!(event_names(&home), ["goal", "note", "note"]);
    }

    #[test]
    fn a_repair_waiting_on_the_old_file_reads_the_one_the_path_names() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, b"not json\n"].concat()).unwrap();
        let holder = hold_lock(&path);
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| {
                let mut out = Vec::new();
                repair(&home, "g1", true, &mut out).map(|_| out)
            });
            std::thread::sleep(std::time::Duration::from_millis(300));
            assert!(!waiter.is_finished());
            replace_by_rename(&path, &[GOAL, NOTE].concat());
            drop(holder);
            let out = waiter.join().unwrap().unwrap();
            assert_eq!(
                String::from_utf8(out).unwrap(),
                "goal log g1 has no torn tail\n"
            );
        });
        assert!(torn_files(&home).is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), [GOAL, NOTE].concat());
    }

    #[test]
    fn the_reader_ignores_an_unterminated_tail_and_refuses_bad_lines() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, b"{\"event\":\"task\""].concat()).unwrap();
        assert_eq!(read_log(&home, "g1").unwrap().len(), 1);
        std::fs::write(&path, [GOAL, b"not json\n"].concat()).unwrap();
        let err = read_log(&home, "g1").unwrap_err();
        let damaged = err.downcast_ref::<DamagedLog>().unwrap();
        assert_eq!(damaged.line_numbers, vec![2]);
        std::fs::write(&path, [GOAL, b"not json\n", GOAL].concat()).unwrap();
        assert!(
            read_log(&home, "g1")
                .unwrap_err()
                .downcast_ref::<DamagedLog>()
                .is_some()
        );
        std::fs::write(&path, [GOAL, b"{\"event\":\"x\"}\r"].concat()).unwrap();
        assert_eq!(read_log(&home, "g1").unwrap().len(), 1);
        assert!(read_log(&home, "missing").unwrap().is_empty());
    }

    #[test]
    fn a_locked_update_refuses_a_damaged_log() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, b"not json\n", b"\n"].concat()).unwrap();
        let err = locked_update(&home, "g1", |_| Ok(Some(body("note")))).unwrap_err();
        assert!(err.downcast_ref::<DamagedLog>().is_some());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            [GOAL, b"not json\n", b"\n"].concat()
        );
    }

    /// Build 221: a helper copied out of its checkout has no data directory, and that is
    /// an error for whatever reads a data file, where a `vendors.json` missing from a data
    /// directory that exists still means every provider is up.
    #[test]
    fn a_data_directory_that_does_not_exist_is_an_error_and_a_missing_file_in_one_is_not() {
        let dir = std::env::temp_dir().join(format!("hydra-puppets-nodata-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("copied/bin")).unwrap();
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let moved = dir.canonicalize().unwrap().join("copied/bin/hydra-puppets");
        let absent = data_dir_from(None, None, Some(&moved));
        assert!(!absent.exists());

        let said = format!("{:#}", existing(absent.clone()).unwrap_err());
        assert!(!said.contains('\n'), "{said}");
        for part in [absent.to_str().unwrap(), "HYDRA_PUPPETS_DATA", "HYDRA_HOME"] {
            assert!(said.contains(part), "{said}");
        }
        let err = vendors_from(None, || existing(absent.clone())).unwrap_err();
        assert_eq!(format!("{err:#}"), said);
        // A file is not a directory either.
        std::fs::write(dir.join("file"), "").unwrap();
        assert!(existing(dir.join("file")).is_err());

        // A vendors file that is named needs no data directory.
        let named = dir.join("named.json");
        let got = vendors_from(Some(&named), || existing(absent.clone())).unwrap();
        assert_eq!(got, named);

        // The directory exists and holds no `vendors.json`: every provider is up, as recorded.
        let vendors = vendors_from(None, || existing(dir.join("data"))).unwrap();
        assert_eq!(vendors, dir.join("data/vendors.json"));
        assert!(crate::fold::provider_available(&vendors, "codex").unwrap());
        std::fs::write(&vendors, r#"[{"provider": "codex", "available": false}]"#).unwrap();
        assert!(!crate::fold::provider_available(&vendors, "codex").unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Build 196: nothing about the checkout the binary was built in is compiled in.
    #[test]
    fn the_data_files_are_found_when_the_helper_runs() {
        let dir = std::env::temp_dir().join(format!("hydra-puppets-data-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let checkout = dir.join("checkout");
        std::fs::create_dir_all(checkout.join("integrations/puppets/prompts")).unwrap();
        std::fs::create_dir_all(checkout.join("target/debug")).unwrap();
        std::fs::create_dir_all(dir.join("elsewhere/bin")).unwrap();
        let checkout = checkout.canonicalize().unwrap();
        let built = checkout.join("target/debug/hydra-puppets");
        let moved = dir
            .canonicalize()
            .unwrap()
            .join("elsewhere/bin/hydra-puppets");
        let (data, home) = (PathBuf::from("/d"), PathBuf::from("/h"));

        assert_eq!(
            data_dir_from(Some(data), Some(home.clone()), Some(&built)),
            PathBuf::from("/d")
        );
        assert_eq!(
            data_dir_from(None, Some(home), Some(&built)),
            PathBuf::from("/h/integrations/puppets")
        );
        assert_eq!(
            data_dir_from(None, None, Some(&built)),
            checkout.join("integrations/puppets")
        );
        // No checkout above the binary: the path beside it, which the error will name.
        assert_eq!(
            data_dir_from(None, None, Some(&moved)),
            moved.parent().unwrap().join("integrations/puppets")
        );
        assert_eq!(
            data_dir_from(None, None, None),
            PathBuf::from("integrations/puppets")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn damaged_log_message_matches_python() {
        let err = DamagedLog {
            goal_id: "g1".into(),
            line_numbers: vec![2, 4],
        };
        assert_eq!(
            err.to_string(),
            "goal log g1 is damaged at line 2, 4; wrote nothing; hydra-puppets repair g1"
        );
    }

    #[test]
    fn a_locked_update_writes_nothing_after_an_unterminated_tail() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        // The last tail is not UTF-8. `decide` still runs first and the error is still
        // `UnterminatedLog`, as in the Python, which reads the log as bytes since build 197.
        for tail in [
            &b"{\"event\":\"assignment\","[..],
            b"{\"event\":\"x\"}\r",
            b"\xff",
        ] {
            let before = [GOAL, tail].concat();
            std::fs::write(&path, &before).unwrap();
            let mut seen = 0;
            let err = locked_update(&home, "g1", |events| {
                seen = events.len();
                Ok(Some(body("note")))
            })
            .unwrap_err();
            assert_eq!(seen, 1);
            let err = err.downcast::<UnterminatedLog>().unwrap();
            assert_eq!(
                err.to_string(),
                "goal log g1 ends without a newline; wrote nothing; hydra-puppets repair g1"
            );
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn repair_moves_the_tail_and_truncates_to_the_last_newline() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        let tail = b"{\"event\":\"assignment\",\"task\":\"t1\"";
        std::fs::write(&path, [GOAL, tail].concat()).unwrap();
        let mut out = Vec::new();
        assert_eq!(repair(&home, "g1", false, &mut out).unwrap(), 0);
        assert_eq!(std::fs::read(&path).unwrap(), [GOAL, tail].concat());
        let mut out = Vec::new();
        repair(&home, "g1", true, &mut out).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), GOAL);
        let torn: Vec<_> = std::fs::read_dir(&home.puppets)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_string_lossy().contains(".torn-"))
            .collect();
        assert_eq!(torn.len(), 1);
        assert_eq!(std::fs::read(&torn[0]).unwrap(), tail);
        let mut out = Vec::new();
        repair(&home, "g1", true, &mut out).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "goal log g1 has no torn tail\n"
        );
    }
}
