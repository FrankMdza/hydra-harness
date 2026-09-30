//! What the cockpit reads, off the input thread, once a tick.
//!
//! **Everything here reads and nothing writes.** It does not call
//! `hydra status`, whose scan harvests a dead interactive run and so writes a
//! record; it reads each record's liveness the way that command does, through
//! [`crate::live::read_view`], and leaves an abandoned run abandoned.
//!
//! **An ended record is normalized once.** Normalizing replays the gate, which
//! is a Python process per recorded tool call, and a corpus of a few hundred
//! runs cannot pay that every two seconds. A record with a closing line does
//! not grow, so its outcome, tokens and result text are kept by run id and
//! file length, and a record whose length changed is read again.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use anyhow::{Context, Result};

use super::goal::{self, Goal};
use crate::approve::Pending;
use crate::live::State;
use crate::paths::Layout;
use crate::policy::PolicySet;

/// How many record lines the detail shows.
pub const TAIL_LINES: usize = 20;
/// Where a long record line is cut in the detail.
const TAIL_LINE_CHARS: usize = 400;
/// How far back from the end the tail looks for its lines.
const TAIL_WINDOW: u64 = 256 * 1024;

/// The four lanes of the kanban, left to right in the order drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Live,
    Waiting,
    EndedOk,
    EndedNotOk,
}

impl Lane {
    pub const ALL: [Lane; 4] = [Lane::Live, Lane::Waiting, Lane::EndedOk, Lane::EndedNotOk];

    pub fn title(self) -> &'static str {
        match self {
            Lane::Live => "live",
            Lane::Waiting => "waiting approval",
            Lane::EndedOk => "ended ok",
            Lane::EndedNotOk => "ended not ok",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Lane::Live => 0,
            Lane::Waiting => 1,
            Lane::EndedOk => 2,
            Lane::EndedNotOk => 3,
        }
    }
}

/// Which lane a run goes in, from contract 5.2's liveness and contract 8's
/// outcome.
///
/// **Only `completed` is ok.** An abandoned run, one whose record would not
/// read back, and every other outcome of the nine are not, and the card says
/// which. `Waiting` is never a run's lane: it holds seat requests.
pub fn lane_of(state: State, outcome: Option<&str>) -> Lane {
    match (state, outcome) {
        (State::Live, _) => Lane::Live,
        (State::Ended, Some("completed")) => Lane::EndedOk,
        _ => Lane::EndedNotOk,
    }
}

/// One run, as a card and as the head of its detail.
#[derive(Clone, Debug)]
pub struct Card {
    pub run_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub policy: Option<String>,
    /// The sha of the policy set the run launched under, off line 0. A live
    /// run keeps the gate compiled from that set whatever `policies/` says now.
    pub policy_sha: Option<String>,
    pub state: State,
    pub outcome: Option<String>,
    pub total_tokens: Option<u64>,
    pub result_text: Option<String>,
    /// Why a field is missing, when it is not simply a run that has not ended.
    pub note: Option<String>,
}

impl Card {
    pub fn lane(&self) -> Lane {
        lane_of(self.state, self.outcome.as_deref())
    }
}

/// Everything one tick read.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub goal: Option<Goal>,
    /// Whether the goal was named with `--goal` rather than picked as the log
    /// that changed last.
    pub goal_named: bool,
    /// Why there is no table, when there is none.
    pub goal_note: Option<String>,
    /// Newest first.
    pub cards: Vec<Card>,
    /// What `hydra approve` with no id lists.
    pub requests: Vec<Pending>,
    /// The last record lines of the run the detail has open.
    pub tail: Option<(String, Vec<String>)>,
    pub errors: Vec<String>,
    /// False only on the empty snapshot drawn before the first read lands.
    pub read: bool,
}

/// What an ended record said once normalized, kept by run id.
#[derive(Clone)]
struct Ended {
    len: u64,
    outcome: Option<String>,
    total_tokens: Option<u64>,
    result_text: Option<String>,
    note: Option<String>,
}

/// The reader thread's state: where to look and what it has normalized.
pub struct Reader {
    layout: Layout,
    policies: std::result::Result<PolicySet, String>,
    puppets_home: PathBuf,
    goal: Option<String>,
    ended: HashMap<String, Ended>,
}

impl Reader {
    pub fn new(
        layout: Layout,
        policies: Result<PolicySet>,
        puppets_home: PathBuf,
        goal: Option<String>,
    ) -> Self {
        Self {
            layout,
            policies: policies.map_err(|err| format!("{err:#}")),
            puppets_home,
            goal,
            ended: HashMap::new(),
        }
    }

    /// Reads once, with the tail of `open` when the detail has a run open.
    pub fn read(&mut self, open: Option<&str>) -> Snapshot {
        let mut snapshot = Snapshot {
            goal_named: self.goal.is_some(),
            read: true,
            ..Snapshot::default()
        };
        self.read_goal(&mut snapshot);
        snapshot.cards = self.read_cards(&mut snapshot.errors);
        match crate::approve::pending(&self.layout) {
            Ok(requests) => snapshot.requests = requests,
            Err(err) => snapshot.errors.push(format!("{err:#}")),
        }
        if let Some(run_id) = open {
            match tail(&self.layout.run_file(run_id), TAIL_LINES) {
                Ok(lines) => snapshot.tail = Some((run_id.to_string(), lines)),
                Err(err) => snapshot.errors.push(format!("{err:#}")),
            }
        }
        snapshot
    }

    /// The thread's loop: read, hand the snapshot over, wait for `interval`
    /// or for the input side to name a run to open. Ends when either side of
    /// the channel is gone.
    pub fn serve(
        mut self,
        interval: Duration,
        asks: Receiver<Option<String>>,
        out: Sender<Snapshot>,
    ) {
        let mut open: Option<String> = None;
        loop {
            if out.send(self.read(open.as_deref())).is_err() {
                return;
            }
            match asks.recv_timeout(interval) {
                Ok(asked) => open = asked,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            // A burst of asks is one read, of the last one.
            while let Ok(asked) = asks.try_recv() {
                open = asked;
            }
        }
    }

    fn read_goal(&self, snapshot: &mut Snapshot) {
        let path = match &self.goal {
            Some(id) => self.puppets_home.join(format!("{id}.jsonl")),
            None => match goal::latest(&self.puppets_home) {
                Ok(Some(path)) => path,
                Ok(None) => {
                    snapshot.goal_note =
                        Some(format!("no goal log under {}", self.puppets_home.display()));
                    return;
                }
                Err(err) => {
                    snapshot.goal_note = Some(format!("{err:#}"));
                    return;
                }
            },
        };
        if !path.is_file() {
            snapshot.goal_note = Some(format!("no goal log at {}", path.display()));
            return;
        }
        match goal::read(&path) {
            Ok(goal) => snapshot.goal = Some(goal),
            Err(err) => snapshot.goal_note = Some(format!("{err:#}")),
        }
    }

    /// Every record under `runs/`, newest first.
    ///
    /// The listing is [`crate::live::scan`]'s, done here so a file that will
    /// not read lands in `errors` rather than on stderr, which is the screen.
    pub fn read_cards(&mut self, errors: &mut Vec<String>) -> Vec<Card> {
        let dir = self.layout.runs();
        let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
                .collect(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => {
                errors.push(format!("reading {}: {err}", dir.display()));
                Vec::new()
            }
        };
        paths.sort();
        paths.reverse();

        let mut cards = Vec::with_capacity(paths.len());
        for path in &paths {
            let view = match crate::live::read_view(path) {
                Ok(view) => view,
                Err(err) => {
                    errors.push(format!("skipping {}: {err:#}", path.display()));
                    continue;
                }
            };
            let (model, policy_sha) = opening_of(path);
            let mut card = Card {
                run_id: view.run_id.clone(),
                provider: view.provider.clone(),
                model,
                policy: view.policy.clone(),
                policy_sha,
                state: view.state,
                outcome: None,
                total_tokens: None,
                result_text: None,
                note: None,
            };
            match view.state {
                State::Ended => {
                    let ended = self.ended(&view.run_id, path);
                    card.outcome = ended.outcome;
                    card.total_tokens = ended.total_tokens;
                    card.result_text = ended.result_text;
                    card.note = ended.note;
                }
                State::Abandoned => {
                    card.note = Some("abandoned: no closing line and no process".to_string());
                }
                State::Live => {}
            }
            cards.push(card);
        }
        let present: std::collections::HashSet<&str> =
            cards.iter().map(|card| card.run_id.as_str()).collect();
        self.ended
            .retain(|run_id, _| present.contains(run_id.as_str()));
        cards
    }

    fn ended(&mut self, run_id: &str, path: &Path) -> Ended {
        let len = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        if let Some(kept) = self.ended.get(run_id)
            && kept.len == len
        {
            return kept.clone();
        }
        let read = self.normalize(path, len);
        self.ended.insert(run_id.to_string(), read.clone());
        read
    }

    fn normalize(&self, path: &Path, len: u64) -> Ended {
        let unread = |note: String| Ended {
            len,
            outcome: None,
            total_tokens: None,
            result_text: None,
            note: Some(note),
        };
        let policies = match &self.policies {
            Ok(policies) => policies,
            Err(err) => return unread(format!("no policies to read it with: {err}")),
        };
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => return unread(format!("reading {}: {err}", path.display())),
        };
        let run = match crate::normalize::read_with(&text, policies, &self.layout) {
            Ok(run) => run,
            Err(err) => return unread(format!("would not read back: {err:#}")),
        };
        let derived = crate::outcome::derive(&run);
        Ended {
            len,
            outcome: derived.map(|d| d.outcome.as_str().to_string()),
            total_tokens: crate::outcome::Usage::of(&run)
                .counts
                .map(|counts| counts.total_tokens),
            result_text: crate::outcome::final_assistant_text(&run),
            note: None,
        }
    }
}

/// The model on line 0's envelope and the `policy_sha` in its body.
/// [`crate::live::read_view`] reads neither.
fn opening_of(path: &Path) -> (Option<String>, Option<String>) {
    use std::io::BufRead;
    let envelope = std::fs::File::open(path).ok().and_then(|file| {
        let mut first = String::new();
        std::io::BufReader::new(file).read_line(&mut first).ok()?;
        serde_json::from_str::<serde_json::Value>(&first).ok()
    });
    let Some(envelope) = envelope else {
        return (None, None);
    };
    let field = |value: &serde_json::Value| value.as_str().map(str::to_string);
    (
        field(&envelope["model"]),
        field(&envelope["hydra"]["policy_sha"]),
    )
}

/// The last `n` lines of a record, each cut to a length a pane can hold.
pub fn tail(path: &Path, n: usize) -> Result<Vec<String>> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(TAIL_WINDOW);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut lines: Vec<&str> = text.lines().collect();
    // A window that starts mid-file starts mid-line.
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    let from = lines.len().saturating_sub(n);
    Ok(lines[from..]
        .iter()
        .map(|line| {
            let mut cut: String = line.chars().take(TAIL_LINE_CHARS).collect();
            if cut.len() < line.len() {
                cut.push_str(" [cut]");
            }
            cut
        })
        .collect())
}
