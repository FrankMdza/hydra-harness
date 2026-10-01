//! The goal log: one append-only JSONL file per goal under the puppets home.
//!
//! Only newline-terminated lines count. The reader stops at the first line without one and
//! skips a bad final line, as `read_log` does; a bad line before the last is an error there,
//! and it is one here. The writer under the lock skips any bad line, as `_locked_update` does,
//! and appends nothing while the file ends without a newline.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use crate::json::{self, Dict, ParseError, Value};
use crate::py;

/// Where the helper reads and writes, from the same environment `puppets.py` reads.
#[derive(Clone, Debug)]
pub struct Home {
    /// `HYDRA_PUPPETS_HOME`, else `~/hydra/puppets`: goal logs and per-goal plan copies.
    pub puppets: PathBuf,
    /// `HYDRA_GRADER_HOME`, else `$HYDRA_ROOT/grader`: `scores.jsonl` for `queue` and `board`.
    pub grader: PathBuf,
    /// `HYDRA_PUPPETS_VENDORS`, else the `vendors.json` beside `puppets.py`.
    pub vendors: PathBuf,
}

/// `integrations/puppets/`, where `puppets.py` and its data files live until the cutover.
pub fn python_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/puppets");
    dir.canonicalize().unwrap_or(dir)
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
        let grader = match named("HYDRA_GRADER_HOME") {
            Some(path) => path,
            None => match named("HYDRA_ROOT") {
                Some(root) => root.join("grader"),
                None => user_home()?.join("hydra").join("grader"),
            },
        };
        let vendors =
            named("HYDRA_PUPPETS_VENDORS").unwrap_or_else(|| python_dir().join("vendors.json"));
        Ok(Home {
            puppets,
            grader,
            vendors,
        })
    }

    /// A home rooted in one directory, for tests: `puppets/`, `grader/` and `vendors.json`.
    pub fn at(dir: &Path) -> Home {
        Home {
            puppets: dir.join("puppets"),
            grader: dir.join("grader"),
            vendors: dir.join("vendors.json"),
        }
    }

    pub fn goal_log_path(&self, goal_id: &str) -> PathBuf {
        self.puppets.join(format!("{goal_id}.jsonl"))
    }
}

/// Split as `bytes.splitlines(keepends=True)` does: on `\r\n`, `\r` and `\n`.
fn split_lines(raw: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < raw.len() {
        match raw[i] {
            b'\n' => {
                lines.push(&raw[start..=i]);
                start = i + 1;
            }
            b'\r' => {
                let end = if raw.get(i + 1) == Some(&b'\n') {
                    i + 1
                } else {
                    i
                };
                lines.push(&raw[start..=end]);
                i = end;
                start = end + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < raw.len() {
        lines.push(&raw[start..]);
    }
    lines
}

fn decode(line: &[u8]) -> Result<&str> {
    std::str::from_utf8(line).map_err(|err| anyhow!("UnicodeDecodeError: {err}"))
}

/// `json.loads(bytes)`: a UTF-8 BOM is dropped, bad UTF-8 raises, bad JSON is `Decode`.
fn loads_line(line: &[u8]) -> Result<Result<Value, ParseError>> {
    let body = line.strip_prefix(b"\xef\xbb\xbf").unwrap_or(line);
    Ok(json::loads(decode(body)?))
}

/// The goal log's events: newline-terminated lines up to the first that is not.
pub fn read_log(home: &Home, goal_id: &str) -> Result<Vec<Value>> {
    let path = home.goal_log_path(goal_id);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let lines = split_lines(&raw);
    let mut events = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.ends_with(b"\n") {
            break;
        }
        match loads_line(line)? {
            Ok(event) => events.push(event),
            Err(ParseError::Decode) if index == lines.len() - 1 => break,
            Err(ParseError::Decode) => {
                bail!(
                    "json.decoder.JSONDecodeError: goal log {goal_id} line {} is not JSON",
                    index + 1
                )
            }
            Err(ParseError::Unsupported) => {
                bail!(
                    "goal log {goal_id} line {} holds a lone surrogate",
                    index + 1
                )
            }
        }
    }
    Ok(events)
}

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
            "goal log {goal} ends without a newline; wrote nothing; puppets.py repair {goal}"
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

/// An exclusive `flock` on an open file, released when dropped.
struct Flock<'a>(&'a File);

impl<'a> Flock<'a> {
    fn exclusive(file: &'a File) -> Result<Flock<'a>> {
        // SAFETY: `flock` reads only the descriptor, which `file` keeps open for the guard's life.
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
        if rc != 0 {
            return Err(std::io::Error::last_os_error()).context("flock");
        }
        Ok(Flock(file))
    }
}

impl Drop for Flock<'_> {
    fn drop(&mut self) {
        // SAFETY: as above; unlocking a descriptor this guard locked.
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

/// Hold the goal log's lock, read it, and append what `decide` returns.
///
/// `decide` sees the newline-terminated lines read under the lock, blank and bad ones
/// skipped. When the file ends without a newline this returns `UnterminatedLog` after
/// `decide` runs and writes nothing. `None` from `decide` writes nothing. The appended
/// record is `{"seq": <lines read + 1>, "ts": <now>, ...body}`, compact, on one line.
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
    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    let _lock = Flock::exclusive(&file)?;
    let mut raw = Vec::new();
    (&file).seek(SeekFrom::Start(0))?;
    (&file).read_to_end(&mut raw)?;
    let mut events = Vec::new();
    let mut unterminated = false;
    for line in split_lines(&raw) {
        if !line.ends_with(b"\n") {
            unterminated = true;
            break;
        }
        let stripped = py::strip(decode(line)?);
        if stripped.is_empty() {
            continue;
        }
        match json::loads(stripped) {
            Ok(event) => events.push(event),
            Err(ParseError::Decode) => continue,
            Err(ParseError::Unsupported) => bail!("goal log {goal_id} holds a lone surrogate"),
        }
    }
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
    record.set("ts", Value::Str(utc_now()));
    for (key, value) in body.iter() {
        record.set(key, value.clone());
    }
    let line = json::dumps(&Value::Dict(record.clone()), None) + "\n";
    (&file).write_all(line.as_bytes())?;
    (&file).flush()?;
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

/// `repair`: print a torn tail, or with `yes` move it beside the log and truncate the log
/// to its last newline, under the log's lock. Returns the exit code.
pub fn repair(home: &Home, goal_id: &str, yes: bool, out: &mut dyn Write) -> Result<i32> {
    let path = home.goal_log_path(goal_id);
    if !path.is_file() {
        writeln!(out, "goal log {goal_id} has no torn tail")?;
        return Ok(0);
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    let _lock = Flock::exclusive(&file)?;
    let mut raw = Vec::new();
    (&file).read_to_end(&mut raw)?;
    let Some(tail) = torn_tail(&raw) else {
        writeln!(out, "goal log {goal_id} has no torn tail")?;
        return Ok(0);
    };
    if !yes {
        let preview: String = String::from_utf8_lossy(tail).chars().take(300).collect();
        writeln!(out, "unterminated tail: {preview}")?;
        writeln!(
            out,
            "would move the tail into {goal_id}.torn-<UTC timestamp>.txt beside the log, \
             then truncate the log to its last newline"
        )?;
        return Ok(0);
    }
    let dest = path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{goal_id}.torn-{}.txt", utc_now()));
    let mut torn = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&dest)
        .with_context(|| format!("FileExistsError: {}", dest.display()))?;
    torn.write_all(tail)?;
    torn.flush()?;
    torn.sync_all()?;
    let keep = (raw.len() - tail.len()) as u64;
    file.set_len(keep)?;
    file.sync_all()?;
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
        body.set("event", Value::Str(event.into()));
        body
    }

    #[test]
    fn splitlines_matches_python_bytes() {
        let lines = split_lines(b"a\r\nb\rc\n\nd");
        assert_eq!(lines, vec![&b"a\r\n"[..], b"b\r", b"c\n", b"\n", b"d"]);
    }

    #[test]
    fn the_reader_ignores_an_unterminated_tail_and_a_bad_last_line() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, b"{\"event\":\"task\""].concat()).unwrap();
        assert_eq!(read_log(&home, "g1").unwrap().len(), 1);
        std::fs::write(&path, [GOAL, b"not json\n"].concat()).unwrap();
        assert_eq!(read_log(&home, "g1").unwrap().len(), 1);
        std::fs::write(&path, [GOAL, b"not json\n", GOAL].concat()).unwrap();
        assert!(
            read_log(&home, "g1").is_err(),
            "a bad interior line is an error, as in read_log"
        );
        std::fs::write(&path, [GOAL, b"{\"event\":\"x\"}\r"].concat()).unwrap();
        assert_eq!(read_log(&home, "g1").unwrap().len(), 1);
        assert!(read_log(&home, "missing").unwrap().is_empty());
    }

    #[test]
    fn a_locked_update_appends_the_next_seq_and_skips_bad_lines() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, b"not json\n", b"\n"].concat()).unwrap();
        let record = locked_update(&home, "g1", |events| {
            assert_eq!(events.len(), 1);
            Ok(Some(body("note")))
        })
        .unwrap()
        .unwrap();
        assert!(matches!(record.get("seq"), Some(Value::Int(2))));
        let raw = std::fs::read_to_string(&path).unwrap();
        let last = raw.lines().last().unwrap();
        assert!(last.starts_with("{\"seq\":2,\"ts\":\""), "{last}");
        assert!(last.ends_with(",\"event\":\"note\"}"), "{last}");
        assert!(raw.ends_with('\n'));
        assert!(locked_update(&home, "g1", |_| Ok(None)).unwrap().is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    }

    #[test]
    fn a_locked_update_writes_nothing_after_an_unterminated_tail() {
        let (_dir, home) = scratch();
        let path = home.goal_log_path("g1");
        for tail in [&b"{\"event\":\"assignment\","[..], b"{\"event\":\"x\"}\r"] {
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
                "goal log g1 ends without a newline; wrote nothing; puppets.py repair g1"
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
