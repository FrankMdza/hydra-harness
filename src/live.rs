//! Liveness, derived from a run file and nothing else.
//!
//! Contract 5.2: a run is alive when a process matching both the recorded pid and
//! the recorded start time still exists. No sidecar, no lockfile, no second source
//! of truth. This module is the only place that reconciles the two, and
//! [`crate::procinfo`] is the only place that asks the kernel.
//!
//! **Three states, and the third is a fact rather than a corruption.** A file with
//! a `run_end` has ended. A file without one whose pid still matches is live. A
//! file without one whose pid is gone is ticket 14's abandoned class: Hydra died
//! before it could write the closing line, and contract 6 says that is derivable
//! only because the pid went into the file before the child started.
//!
//! **Reading the tail is enough to answer "has it ended".** `run_end` is the last
//! line of a run file, so the last 64 KiB always holds it whole whatever the frames
//! before it look like. A run file whose closing line is bigger than that does not
//! exist: the line is six envelope fields and a body of six.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::paths::Layout;

/// How much of a run file's tail is read to look for the closing line.
const TAIL_BYTES: u64 = 64 * 1024;

/// How many children may be in flight at once, harness spec 9's cap.
///
/// **The number is this build's, not the design's.** Section 9 asks for "a cap on
/// live children" and names none, so eight is a choice: it is more than the
/// parallel tool calls an orchestrator issues in one turn and few enough that
/// eight `claude` processes on one subscription is a bill you notice before the
/// machine does. Nothing inherits it and no policy sets it.
pub const LIVE_RUN_CAP: usize = 8;

/// How deep a delegation tree may go, harness spec 6.1 item 2 and section 9.
///
/// **"Configurable" means a constant today.** Section 9 says the cap is
/// configurable and names no mechanism, and there is no configuration file for a
/// number like this one: the five protected configs are policies, the preamble,
/// the MCP document and two that do not exist. A flag would be worse, because a
/// child that can raise its own cap is not a cap. So it moves when this line
/// moves, and `IMPLEMENTATION.md` says so.
///
/// A human is depth 0, its child is 1, and that child's child is 2. Section 7.6
/// closes the blind spot: an in-process subagent gets no run id and counts against
/// nothing, so every policy but `permissive` denies one.
pub const MAX_DEPTH: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// A `run_end` is on disk. The run is over whatever its outcome.
    Ended,
    /// No `run_end`, and a process matching the pid and the start time exists.
    Live,
    /// No `run_end`, and no such process. Contract 6's abandoned class.
    Abandoned,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Ended => "ended",
            State::Live => "live",
            State::Abandoned => "abandoned",
        }
    }
}

/// What one run file says about itself without being normalized.
///
/// Line 0 and the tail, nothing between. Reading a whole record to answer "is this
/// alive" would cost a python process per recorded tool call, which is the price
/// [`crate::normalize`] pays on purpose and this must not.
#[derive(Debug, Clone)]
pub struct RunView {
    pub run_id: String,
    pub run_file: PathBuf,
    pub state: State,
    pub pid: Option<u32>,
    pub process_start_time: Option<DateTime<Utc>>,
    /// The envelope timestamp of line 0, which is when the record was opened.
    pub started_at: Option<DateTime<Utc>>,
    pub worktree: Option<crate::record::WorktreeRef>,
    pub provider: Option<String>,
    pub policy: Option<String>,
    /// The store sha this run pinned, contract 5.3. It is the refcount on
    /// `~/hydra/skills-snapshots/<sha>/`: a snapshot is referenced while a live
    /// run's record names it, and nothing else counts.
    pub skills_sha: Option<String>,
}

impl RunView {
    pub fn is_live(&self) -> bool {
        self.state == State::Live
    }

    /// Whether anything in the record proves a process ever existed for this run.
    ///
    /// **A run file that exists with nothing in it is a launch in progress.**
    /// [`crate::watch`] creates the file before it spawns the child and writes
    /// line 0 after, so for as long as [`crate::run::LAUNCH_BUDGET`] allows there
    /// is a record with no pid on it, and [`read_view`] can only call that
    /// [`State::Abandoned`]. A torn line 0 looks identical.
    ///
    /// **Nothing that removes things may act on that state.** The record says
    /// nothing yet, so age is the only evidence there is, and the callers that
    /// delete a checkout or move a record pair this with
    /// [`crate::status::just_created`]. A refused spawn passes, because it has a
    /// closing line: contract 5.2's null pid is a run that never had a process,
    /// which is a different claim from a record that has not spoken.
    pub fn launched(&self) -> bool {
        self.pid.is_some() || self.state == State::Ended
    }
}

/// Reads one run file's opening envelope and its closing line.
///
/// **A torn line 0 does not fail the read.** The normalizer already tolerates one
/// with a `Finding::Limit`, and a status command that refuses to report a damaged
/// record is a status command that hides the run most worth looking at. The run id
/// falls back to the filename, which is where it came from.
pub fn read_view(path: &Path) -> Result<RunView> {
    let run_id_from_name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();

    let start = first_line(path)?.and_then(|line| serde_json::from_str::<Value>(&line).ok());
    let ended = has_run_end(path)?;

    let (run_id, started_at, provider, pid, process_start_time, policy, worktree) = match &start {
        Some(envelope) => {
            let body = &envelope["hydra"];
            let pid = body["pid"].as_u64().and_then(|p| u32::try_from(p).ok());
            let process_start_time = body["process_start_time"].as_str().and_then(parse_time);
            let worktree = match (
                body["worktree"]["path"].as_str(),
                body["worktree"]["branch"].as_str(),
            ) {
                (Some(path), Some(branch)) => Some(crate::record::WorktreeRef {
                    path: path.to_string(),
                    branch: branch.to_string(),
                    base_tree: body["worktree"]["base_tree"].as_str().map(str::to_string),
                }),
                _ => None,
            };
            (
                envelope["run_id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| run_id_from_name.clone()),
                envelope["ts"].as_str().and_then(parse_time),
                envelope["provider"].as_str().map(str::to_string),
                pid,
                process_start_time,
                body["policy"].as_str().map(str::to_string),
                worktree,
            )
        }
        None => (run_id_from_name, None, None, None, None, None, None),
    };

    // Read beside the tuple rather than inside it. It is the one field here that
    // no other field is derived from, and a nine-way tuple is worse than a second
    // look at the same value.
    let skills_sha = start
        .as_ref()
        .and_then(|envelope| envelope["hydra"]["skills"]["store_sha"].as_str())
        .map(str::to_string);

    // Contract 5.2's test, both halves. A null pid is a run that never had a
    // process, which is a refused spawn rather than something to go looking for.
    let state = match (ended, pid, process_start_time) {
        (true, _, _) => State::Ended,
        (false, Some(pid), Some(start)) if crate::procinfo::is_alive(pid, start)? => State::Live,
        (false, _, _) => State::Abandoned,
    };

    Ok(RunView {
        run_id,
        run_file: path.to_path_buf(),
        state,
        pid,
        process_start_time,
        started_at,
        worktree,
        provider,
        policy,
        skills_sha,
    })
}

/// Every `.jsonl` under `~/hydra/runs/`, oldest first by run id.
///
/// A run id is a UUIDv7, so sorting the filenames sorts by launch time.
///
/// **A file that will not read is skipped with a note, not propagated.** This runs
/// before every launch, so one unreadable entry under `~/hydra/runs/` would
/// otherwise stop `hydra run` and `hydra status` alike until a human found it by
/// hand. [`read_view`] already tolerates a torn record; this is the same stance one
/// level up, for the file that cannot be opened at all.
pub fn scan(layout: &Layout) -> Result<Vec<RunView>> {
    let dir = layout.runs();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", dir.display())),
    };

    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
        .collect();
    paths.sort();

    let mut views = Vec::with_capacity(paths.len());
    for path in &paths {
        match read_view(path) {
            Ok(view) => views.push(view),
            Err(err) => eprintln!("hydra: skipping {}: {err:#}", path.display()),
        }
    }
    Ok(views)
}

/// Whether the run file already carries its closing line.
///
/// The cheap half of [`read_view`], for the tail loop that asks once a tick.
pub fn has_run_end(path: &Path) -> Result<bool> {
    Ok(closing_line(path)?.is_some())
}

/// The body of line 0, or `None` on a file that has none to read.
///
/// **The twin of [`closing_line`], and it is here for the same reason.** A
/// delegating launch reads two fields off its parent's record, `depth` and
/// `policy`, and normalizing the whole file to get them would cost a python
/// process per recorded tool call. [`read_view`] already reads this line and keeps
/// what liveness needs; this hands back the body so a caller can read a field
/// liveness has no opinion about.
pub fn opening_line(path: &Path) -> Result<Option<Value>> {
    let Some(first) = first_line(path)? else {
        return Ok(None);
    };
    let Ok(mut envelope) = serde_json::from_str::<Value>(&first) else {
        return Ok(None);
    };
    if envelope["hydra"]["type"].as_str() != Some("run_start") {
        return Ok(None);
    }
    Ok(Some(envelope["hydra"].take()))
}

/// The body of the closing line, or `None` on a run that has not ended.
///
/// **This is not a substitute for reading the record.** It answers the two
/// questions a tail loop has, "is it over" and "what did the process exit with",
/// and nothing derived. Contract 6 keeps the outcome out of the file precisely so
/// that a cheap read like this one cannot become a second source for it.
pub fn closing_line(path: &Path) -> Result<Option<Value>> {
    let Some(last) = last_line(path)? else {
        return Ok(None);
    };
    let Ok(mut envelope) = serde_json::from_str::<Value>(&last) else {
        return Ok(None);
    };
    if envelope["hydra"]["type"].as_str() != Some("run_end") {
        return Ok(None);
    }
    Ok(Some(envelope["hydra"].take()))
}

/// The cap of harness spec 9, as a check that refuses rather than queues.
pub fn guard(cap: usize, views: &[RunView]) -> Result<(), TooManyLive> {
    let live: Vec<InFlight> = views
        .iter()
        .filter(|view| view.is_live())
        .map(|view| InFlight {
            run_id: view.run_id.clone(),
            pid: view.pid,
        })
        .collect();
    if live.len() < cap {
        return Ok(());
    }
    Err(TooManyLive { cap, live })
}

/// One live child, as the refusal names it.
#[derive(Debug, Clone)]
pub struct InFlight {
    pub run_id: String,
    pub pid: Option<u32>,
}

/// The cap refused this spawn.
///
/// **A type rather than a message.** Harness spec 9 makes concurrency a refusal
/// and not a queue, so a caller that hits it has to decide something, and deciding
/// needs the run ids: the answer is either to wait on one of them or to reap it.
/// A formatted string would make that a parsing job.
///
/// **No run record is written.** Contract 8.3 makes a refused spawn a record when
/// the refusal is one of section 6's four codes, and the cap is not one of them.
/// Minting a fifth is a contract change rather than an implementation, so the
/// refusal stays on stderr and the run id is never minted.
#[derive(Debug)]
pub struct TooManyLive {
    pub cap: usize,
    pub live: Vec<InFlight>,
}

impl std::fmt::Display for TooManyLive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} run(s) are already in flight and the cap is {}, so this spawn is refused \
             rather than queued.",
            self.live.len(),
            self.cap
        )?;
        for run in &self.live {
            match run.pid {
                Some(pid) => write!(f, "\n  {} (pid {pid})", run.run_id)?,
                None => write!(f, "\n  {}", run.run_id)?,
            }
        }
        // Not "reap one": the count is of live runs, so a run that has ended has
        // already given its slot back and reaping it frees nothing. Waiting is
        // the whole of what a caller can do, and these are the two ways to wait.
        f.write_str(
            "\nWatch one with `hydra logs <run-id> --follow`, or `hydra status` for all of them.",
        )
    }
}

impl std::error::Error for TooManyLive {}

fn parse_time(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn first_line(path: &Path) -> Result<Option<String>> {
    use std::io::BufRead;

    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("opening {}", path.display())),
    };
    let mut line = String::new();
    let read = std::io::BufReader::new(file)
        .read_line(&mut line)
        .with_context(|| format!("reading line 0 of {}", path.display()))?;
    if read == 0 {
        return Ok(None);
    }
    Ok(Some(line))
}

/// The last complete line, read from the tail rather than from the whole file.
///
/// A run file holds every frame a provider wrote and one of them can be hundreds
/// of kilobytes, so reading it whole to find the closing line would make a status
/// pass cost the size of the corpus. `run_end` is always last and always small, so
/// a fixed tail either holds it whole or the file has not ended.
fn last_line(path: &Path) -> Result<Option<String>> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("opening {}", path.display())),
    };
    let len = file
        .metadata()
        .with_context(|| format!("sizing {}", path.display()))?
        .len();
    if len == 0 {
        return Ok(None);
    }
    let from = len.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(from))
        .with_context(|| format!("seeking in {}", path.display()))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)
        .with_context(|| format!("reading the tail of {}", path.display()))?;

    let text = String::from_utf8_lossy(&tail);
    Ok(text
        .lines()
        .rfind(|line| !line.trim().is_empty())
        .map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hydra-live-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    fn record(pid: u32, start: Option<DateTime<Utc>>, closed: bool) -> String {
        let start = start
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
            .map(Value::from)
            .unwrap_or(Value::Null);
        let mut text = serde_json::json!({
            "run_id": "01a07357-58a5-79e2-a25a-3df578a750b4",
            "parent_run_id": null,
            "seq": 0,
            "ts": "2026-09-05T20:51:39.370Z",
            "provider": "claude",
            "model": null,
            "secret_flag": null,
            "hydra": {
                "type": "run_start",
                "policy": "standard",
                "pid": pid,
                "process_start_time": start,
                "worktree": {"path": "/tmp/w", "branch": "hydra/x"},
            },
        })
        .to_string();
        text.push('\n');
        if closed {
            text.push_str(
                &serde_json::json!({
                    "run_id": "01a07357-58a5-79e2-a25a-3df578a750b4",
                    "seq": 1,
                    "ts": "2026-09-05T20:52:39.370Z",
                    "hydra": {"type": "run_end", "exit_code": 0},
                })
                .to_string(),
            );
            text.push('\n');
        }
        text
    }

    #[test]
    fn our_own_pid_with_our_own_start_time_reads_as_live() {
        let pid = std::process::id();
        let start = crate::procinfo::start_time(pid).unwrap().unwrap();
        let path = scratch("live.jsonl");
        std::fs::write(&path, record(pid, Some(start), false)).unwrap();

        let view = read_view(&path).unwrap();
        assert_eq!(view.state, State::Live);
        assert_eq!(view.pid, Some(pid));
        assert_eq!(view.policy.as_deref(), Some("standard"));
        assert_eq!(view.worktree.unwrap().branch, "hydra/x");
    }

    #[test]
    fn the_start_time_is_half_the_test_and_not_a_decoration() {
        // macOS recycles pids. A record whose pid is now somebody else's has to
        // read as abandoned rather than as live, or `hydra status` reports a run
        // that died last week as working.
        let pid = std::process::id();
        let start = crate::procinfo::start_time(pid).unwrap().unwrap();
        let path = scratch("recycled.jsonl");
        std::fs::write(
            &path,
            record(pid, Some(start - chrono::Duration::seconds(1)), false),
        )
        .unwrap();

        assert_eq!(read_view(&path).unwrap().state, State::Abandoned);
    }

    #[test]
    fn a_closing_line_ends_a_run_whatever_the_pid_says() {
        let pid = std::process::id();
        let start = crate::procinfo::start_time(pid).unwrap().unwrap();
        let path = scratch("ended.jsonl");
        std::fs::write(&path, record(pid, Some(start), true)).unwrap();

        assert_eq!(read_view(&path).unwrap().state, State::Ended);
        assert!(has_run_end(&path).unwrap());
    }

    #[test]
    fn a_dead_pid_with_no_closing_line_is_abandoned() {
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let pid = child.id();
        let start = crate::procinfo::start_time(pid).unwrap().unwrap();
        child.kill().unwrap();
        child.wait().unwrap();

        let path = scratch("abandoned.jsonl");
        std::fs::write(&path, record(pid, Some(start), false)).unwrap();
        assert_eq!(read_view(&path).unwrap().state, State::Abandoned);
    }

    #[test]
    fn a_torn_line_zero_still_reports_a_run() {
        let path = scratch("01a07357-58a5-79e2-a25a-3df578a750b4.jsonl");
        std::fs::write(&path, "{\"run_id\": \"01a0735\n").unwrap();

        let view = read_view(&path).unwrap();
        assert_eq!(view.run_id, "01a07357-58a5-79e2-a25a-3df578a750b4");
        assert_eq!(view.state, State::Abandoned);
        assert_eq!(view.pid, None);
    }

    #[test]
    fn a_closing_line_behind_a_frame_bigger_than_the_tail_is_still_found() {
        let path = scratch("huge.jsonl");
        let mut text = record(std::process::id(), None, false);
        let mut fat = serde_json::json!({"run_id": "x", "seq": 1, "raw": {}}).to_string();
        fat = fat.replace("{}", &format!("{{\"t\":\"{}\"}}", "x".repeat(200_000)));
        text.push_str(&fat);
        text.push('\n');
        text.push_str(&serde_json::json!({"hydra": {"type": "run_end"}}).to_string());
        text.push('\n');
        std::fs::write(&path, text).unwrap();

        assert!(has_run_end(&path).unwrap());
    }

    #[test]
    fn the_cap_refuses_and_names_what_is_in_flight() {
        let views: Vec<RunView> = ["a", "b"]
            .iter()
            .map(|id| RunView {
                run_id: (*id).to_string(),
                run_file: PathBuf::from("/tmp/x.jsonl"),
                state: State::Live,
                pid: Some(42),
                process_start_time: None,
                started_at: None,
                worktree: None,
                provider: None,
                policy: None,
                skills_sha: None,
            })
            .collect();

        assert!(guard(3, &views).is_ok());
        let refused = guard(2, &views).unwrap_err();
        assert_eq!(refused.live.len(), 2);
        let said = refused.to_string();
        assert!(said.contains('a') && said.contains('b'), "{said}");
        assert!(said.contains("pid 42"), "{said}");
        // Every command it names has to be one this binary answers to.
        assert!(!said.contains("hydra reap"), "{said}");
    }

    #[test]
    fn an_ended_run_does_not_count_against_the_cap() {
        let views = vec![RunView {
            run_id: "a".into(),
            run_file: PathBuf::from("/tmp/x.jsonl"),
            state: State::Ended,
            pid: Some(42),
            process_start_time: None,
            started_at: None,
            worktree: None,
            provider: None,
            policy: None,
            skills_sha: None,
        }];
        assert!(guard(1, &views).is_ok());
    }
}
