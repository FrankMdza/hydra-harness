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

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use super::call_head::call_inputs;
use super::goal::{self, Goal};
use super::window;

pub use super::call_head::REVEAL_SECONDS;
pub use super::call_head::shell_head;
use crate::approve::Pending;
use crate::live::State;
use crate::normalize::Counts;
use crate::paths::Layout;
use crate::policy::PolicySet;

/// How many record lines the detail shows.
pub const TAIL_LINES: usize = 20;
/// How many tool calls the detail shows, the last ones.
pub const DETAIL_CALLS: usize = 15;
/// Where a long record line is cut in the detail.
/// How far back from the end the tail looks for its lines.
const TAIL_WINDOW: u64 = 256 * 1024;
/// How far back the header's tokens per minute look.
pub const RATE_WINDOW_MINUTES: i64 = 5;
/// How often the reader asks `hydra-puppets pending`, and how long it waits.
/// Build ticket 182.
pub const PENDING_EVERY: chrono::TimeDelta = chrono::TimeDelta::seconds(10);
pub const PENDING_BUDGET: Duration = Duration::from_secs(2);

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
    /// The run that launched this one, off line 0. A session's seats name the
    /// session here. Build ticket 177.
    pub parent_run_id: Option<String>,
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
    /// What `hydra-puppets pending <goal> --json` last listed for the goal the
    /// table shows. Build ticket 182.
    pub steps: Vec<Step>,
    /// When the reader last asked `pending` for them, which may be before
    /// this tick.
    pub steps_read_at: Option<DateTime<Utc>>,
    /// The last record lines of the run the detail has open.
    pub tail: Option<(String, Vec<String>)>,
    /// The last [`DETAIL_CALLS`] tool calls of the run the detail has open,
    /// oldest first. Build ticket 178.
    pub calls: Option<(String, Vec<CallLine>)>,
    pub errors: Vec<String>,
    /// False only on the empty snapshot drawn before the first read lands.
    pub read: bool,
    /// What each provider's live runs have spent, one entry per provider with
    /// a live run, by name.
    pub spend: Vec<Spend>,
}

/// One approval a goal waits on, as `hydra-puppets pending --json` lists it.
/// Build ticket 182.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub goal: String,
    /// `plan`, `table`, `launch`, `diff` or `degraded`.
    pub kind: String,
    /// The task id, or the missing provider for `degraded`.
    pub reference: Option<String>,
    pub revision: u64,
    /// The exact command that approves it, `--expect-revision` included. The
    /// cockpit runs it as given and never builds one.
    pub argv: Vec<String>,
}

impl Step {
    /// What the lane shows and the detail is keyed by.
    pub fn label(&self) -> String {
        match &self.reference {
            Some(reference) => format!("{} {} {reference}", self.goal, self.kind),
            None => format!("{} {}", self.goal, self.kind),
        }
    }
}

/// Reads `pending --json`'s output. An item whose argv does not end in
/// `--expect-revision` and its own revision is refused rather than shown, so
/// no step runs unbound to the revision the human saw.
pub fn parse_steps(goal: &str, bytes: &[u8]) -> Result<Vec<Step>> {
    let items: Vec<serde_json::Value> =
        serde_json::from_slice(bytes).context("reading what pending printed")?;
    items
        .iter()
        .map(|item| {
            let kind = item["kind"].as_str().context("a step without a kind")?;
            let revision = item["revision"]
                .as_u64()
                .with_context(|| format!("the {kind} step has no revision"))?;
            let argv = item["argv"]
                .as_array()
                .and_then(|argv| {
                    argv.iter()
                        .map(|arg| arg.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                })
                .with_context(|| format!("the {kind} step has no argv"))?;
            let bound = argv.len() > 2
                && argv[argv.len() - 2] == "--expect-revision"
                && argv[argv.len() - 1] == revision.to_string();
            if !bound {
                anyhow::bail!("the {kind} step's argv is not bound to revision {revision}");
            }
            Ok(Step {
                goal: goal.to_string(),
                kind: kind.to_string(),
                reference: item["ref"].as_str().map(str::to_string),
                revision,
                argv,
            })
        })
        .collect()
}

/// What the last `pending` said, and when it was asked.
struct PendingRead {
    goal: String,
    at: DateTime<Utc>,
    said: std::result::Result<Vec<Step>, String>,
}

/// One tool call, as the detail lists it. Build ticket 178.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallLine {
    pub name: String,
    /// The head of what the call touched, on one line.
    pub input: String,
    /// The whole input from the record, for the reveal key.
    pub full_input: String,
    /// What the call's result says of it: `allowed`, `refused <code>`,
    /// `error` or `unclassified`. `None` while it has no result.
    pub decision: Option<String>,
}

/// One provider's live runs, for the header. Build ticket 172.
///
/// **Tokens, never dollars.** The human pays subscriptions, so the panel of
/// 2026-09-30 asked for the rate and the share of the rate-limit window.
#[derive(Clone, Debug, PartialEq)]
pub struct Spend {
    pub provider: String,
    /// Live runs of this provider.
    pub live: usize,
    /// How many of them carry counts yet.
    pub counted: usize,
    /// Output tokens so far, summed over the counted runs.
    pub output_tokens: u64,
    /// Output tokens per minute over the last five minutes, summed over the
    /// counted runs. `None` until some run has two points in that time to
    /// measure between, its start counting as one at zero.
    pub tokens_per_minute: Option<f64>,
    /// The counted runs' share of the provider's rate-limit window, by
    /// [`super::window`]. `None` when any of them has no budget, and always
    /// when one's provider is `claude` once aliased.
    pub window_share: Option<f64>,
}

/// What a live record has counted so far, kept by run id.
///
/// Kept only while the file grows under the same line 0: a record that
/// shrinks or opens differently is another record, and starts over.
struct Live {
    len: u64,
    line_zero: Option<String>,
    counts: Option<Counts>,
    started: Option<DateTime<Utc>>,
    /// Output tokens at each read in the last five minutes, oldest first.
    samples: VecDeque<(DateTime<Utc>, u64)>,
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

/// The open run's calls as last read, kept while the file keeps its length
/// and its line 0. One slot: opening another run drops it.
struct OpenCalls {
    run_id: String,
    len: u64,
    line_zero: Option<String>,
    calls: Vec<CallLine>,
}

/// The reader thread's state: where to look and what it has normalized.
pub struct Reader {
    layout: Layout,
    policies: std::result::Result<PolicySet, String>,
    puppets_home: PathBuf,
    goal: Option<String>,
    ended: HashMap<String, Ended>,
    live: HashMap<String, Live>,
    calls: Option<OpenCalls>,
    /// Where `vendors.json` and `aliases.json` are, or `None` for no budgets.
    vendors: Option<PathBuf>,
    /// `hydra-puppets`, asked what the shown goal waits on, or `None` to ask
    /// nothing. Build ticket 182.
    puppets: Option<PathBuf>,
    pending: Option<PendingRead>,
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
            live: HashMap::new(),
            calls: None,
            vendors: None,
            puppets: None,
            pending: None,
        }
    }

    /// Reads once, with the tail of `open` when the detail has a run open.
    pub fn read(&mut self, open: Option<&str>) -> Snapshot {
        self.read_at(open, Utc::now())
    }

    /// Where `vendors.json` and `aliases.json` are. Without it every window
    /// share is `-`.
    pub fn with_vendors(mut self, dir: PathBuf) -> Self {
        self.vendors = Some(dir);
        self
    }

    /// The helper whose `pending` fills the waiting lane with steps.
    pub fn with_puppets(mut self, script: PathBuf) -> Self {
        self.puppets = Some(script);
        self
    }

    /// The same, with the clock the rates are measured on named.
    pub fn read_at(&mut self, open: Option<&str>, now: DateTime<Utc>) -> Snapshot {
        let mut snapshot = Snapshot {
            goal_named: self.goal.is_some(),
            read: true,
            ..Snapshot::default()
        };
        self.read_goal(&mut snapshot);
        self.read_steps(&mut snapshot, now);
        snapshot.cards = self.read_cards(&mut snapshot.errors);
        snapshot.spend = self.spend(&snapshot.cards, now, &mut snapshot.errors);
        match crate::approve::pending(&self.layout) {
            Ok(requests) => snapshot.requests = requests,
            Err(err) => snapshot.errors.push(format!("{err:#}")),
        }
        if let Some(run_id) = open {
            match tail(&self.layout.run_file(run_id), TAIL_LINES) {
                Ok(lines) => snapshot.tail = Some((run_id.to_string(), lines)),
                Err(err) => snapshot.errors.push(format!("{err:#}")),
            }
            snapshot.calls = Some((run_id.to_string(), self.open_calls(run_id)));
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

    /// The steps the shown goal waits on, from `pending <goal> --json`, asked
    /// at most once per [`PENDING_EVERY`] and again when the goal changes.
    /// It reads the log and writes nothing, so a helper that does not answer
    /// in [`PENDING_BUDGET`] is killed.
    fn read_steps(&mut self, snapshot: &mut Snapshot, now: DateTime<Utc>) {
        let (Some(script), Some(goal)) = (&self.puppets, &snapshot.goal) else {
            return;
        };
        if goal.damaged_at.is_some() {
            return;
        };
        let fresh = self
            .pending
            .as_ref()
            .is_some_and(|read| read.goal == goal.id && now - read.at < PENDING_EVERY);
        if !fresh {
            let said = self
                .ask_pending(script, &goal.id)
                .map_err(|err| format!("{err:#}"));
            self.pending = Some(PendingRead {
                goal: goal.id.clone(),
                at: now,
                said,
            });
        }
        let Some(read) = &self.pending else {
            return;
        };
        snapshot.steps_read_at = Some(read.at);
        match &read.said {
            Ok(steps) => snapshot.steps = steps.clone(),
            Err(err) => snapshot.errors.push(err.clone()),
        }
    }

    fn ask_pending(&self, script: &Path, goal: &str) -> Result<Vec<Step>> {
        super::check_goal_id(goal)?;
        let mut command = std::process::Command::new(script);
        command
            .args(["pending", goal, "--json"])
            .env("HYDRA_ROOT", self.layout.root())
            .env("HYDRA_PUPPETS_HOME", &self.puppets_home);
        match super::bounded(command, PENDING_BUDGET)
            .with_context(|| format!("hydra-puppets pending {goal}"))?
        {
            (super::Bounded::Exited(status, _), out) if status.success() => parse_steps(goal, &out),
            (super::Bounded::Exited(status, line), _) => {
                anyhow::bail!("hydra-puppets pending {goal} exited with {status}: {line}")
            }
            (super::Bounded::Late(mut child), _) => {
                // The helper leads its own group, so this reaches whatever it started.
                if let Ok(pgid) = libc::pid_t::try_from(child.id()) {
                    // SAFETY: `killpg` takes two integers and touches no memory.
                    unsafe { libc::killpg(pgid, libc::SIGKILL) };
                }
                let _ = child.wait();
                anyhow::bail!(
                    "hydra-puppets pending {goal} did not answer in {}s",
                    PENDING_BUDGET.as_secs()
                )
            }
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
            let line_zero = line_zero(path);
            let (model, policy_sha, started) = opening_of(line_zero.as_deref());
            let mut card = Card {
                run_id: view.run_id.clone(),
                provider: view.provider.clone(),
                model,
                policy: view.policy.clone(),
                policy_sha,
                parent_run_id: view.parent_run_id.clone(),
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
                State::Live => self.count_live(&view.run_id, path, line_zero, started),
            }
            cards.push(card);
        }
        let present: std::collections::HashSet<&str> =
            cards.iter().map(|card| card.run_id.as_str()).collect();
        self.ended
            .retain(|run_id, _| present.contains(run_id.as_str()));
        let live: std::collections::HashSet<&str> = cards
            .iter()
            .filter(|card| card.state == State::Live)
            .map(|card| card.run_id.as_str())
            .collect();
        self.live.retain(|run_id, _| live.contains(run_id.as_str()));
        cards
    }

    /// Reads a live record's usage so far, again only when it grew.
    fn count_live(
        &mut self,
        run_id: &str,
        path: &Path,
        line_zero: Option<String>,
        started: Option<DateTime<Utc>>,
    ) {
        let len = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        let kept = self
            .live
            .get(run_id)
            .is_some_and(|live| live.len <= len && live.line_zero == line_zero);
        if !kept {
            self.live.insert(
                run_id.to_string(),
                Live {
                    len: 0,
                    line_zero,
                    counts: None,
                    started,
                    samples: VecDeque::new(),
                },
            );
        }
        let live = self.live.get_mut(run_id).expect("inserted above");
        if live.len == len && len > 0 {
            return;
        }
        live.len = len;
        live.counts = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| crate::normalize::usage_so_far(&text))
            .and_then(|usage| usage.counts);
    }

    /// The open run's last calls, read again only when its file changed
    /// length or line 0, as [`Self::count_live`] does for usage.
    ///
    /// **Off the walk, never the replay.** A decision is what the call's
    /// result says, which is why this needs no policy set: see [`calls_of`].
    fn open_calls(&mut self, run_id: &str) -> Vec<CallLine> {
        let path = self.layout.run_file(run_id);
        let len = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        let line_zero = line_zero(&path);
        if let Some(kept) = &self.calls
            && kept.run_id == run_id
            && kept.len == len
            && kept.line_zero == line_zero
        {
            return kept.calls.clone();
        }
        let calls = std::fs::read_to_string(&path)
            .map(|text| calls_of(&text))
            .unwrap_or_default();
        self.calls = Some(OpenCalls {
            run_id: run_id.to_string(),
            len,
            line_zero,
            calls: calls.clone(),
        });
        calls
    }

    /// The header's entries, one per provider with a live card.
    fn spend(
        &mut self,
        cards: &[Card],
        now: DateTime<Utc>,
        errors: &mut Vec<String>,
    ) -> Vec<Spend> {
        let live: Vec<&Card> = cards.iter().filter(|c| c.state == State::Live).collect();
        if live.is_empty() {
            return Vec::new();
        }
        let budgets = self.vendors.as_deref().and_then(|dir| {
            window::Budgets::load(dir)
                .map_err(|err| errors.push(format!("no window budgets: {err:#}")))
                .ok()
        });
        let from = now - chrono::Duration::minutes(RATE_WINDOW_MINUTES);
        let mut by_provider: BTreeMap<String, Spend> = BTreeMap::new();
        // Whether every counted run of the provider had a share.
        let mut shared: HashMap<String, bool> = HashMap::new();
        for card in live {
            let provider = card.provider.clone().unwrap_or_else(|| "-".to_string());
            let spend = by_provider.entry(provider.clone()).or_insert(Spend {
                provider: provider.clone(),
                live: 0,
                counted: 0,
                output_tokens: 0,
                tokens_per_minute: None,
                window_share: None,
            });
            spend.live += 1;
            let Some(run) = self.live.get_mut(&card.run_id) else {
                continue;
            };
            let Some(counts) = run.counts else {
                continue;
            };
            spend.counted += 1;
            spend.output_tokens += counts.output_tokens;

            run.samples.push_back((now, counts.output_tokens));
            while run.samples.front().is_some_and(|(at, _)| *at < from) {
                run.samples.pop_front();
            }
            // A run that started inside the window had zero at its start.
            let base = match run.started {
                Some(started) if started >= from && started < now => Some((started, 0)),
                _ => run.samples.front().copied(),
            };
            if let Some((at, tokens)) = base.filter(|(at, _)| *at < now) {
                let minutes = (now - at).num_milliseconds() as f64 / 60_000.0;
                let rate = counts.output_tokens.saturating_sub(tokens) as f64 / minutes;
                spend.tokens_per_minute = Some(spend.tokens_per_minute.unwrap_or(0.0) + rate);
            }

            let share = budgets
                .as_ref()
                .and_then(|b| b.share(&provider, card.model.as_deref(), &counts));
            let all = shared.entry(provider).or_insert(true);
            *all &= share.is_some();
            spend.window_share = match (*all, share) {
                (true, Some(share)) => Some(spend.window_share.unwrap_or(0.0) + share),
                _ => None,
            };
        }
        by_provider.into_values().collect()
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

/// A record's last [`DETAIL_CALLS`] tool calls, each with what its result says.
///
/// The decision is read off the result, not derived: a result carrying one of
/// the gate's codes other than `allow`, found by
/// [`crate::normalize::hydra_voice`], is `refused <code>`. Any other result is
/// `allowed`, `error` or `unclassified` by its status, so a refusal by the
/// user's own guards shows as `error`. A call with no result yet has none.
///
/// Ids repeat (`codex` reuses them), so a result answers the latest earlier
/// call of its id still open, in file order, and a result with none is
/// dropped. A call on a line the secret sweep flagged shows no input.
pub fn calls_of(text: &str) -> Vec<CallLine> {
    use crate::normalize::{Event, ResultStatus};
    let flagged = flagged_seqs(text);
    let mut calls: Vec<CallLine> = Vec::new();
    let mut open: HashMap<String, Vec<usize>> = HashMap::new();
    for event in crate::normalize::events_so_far(text) {
        match event {
            Event::ToolCall(call) => {
                if !call.call_id.is_empty() {
                    open.entry(call.call_id).or_default().push(calls.len());
                }
                let (input, full_input) = if flagged.contains(&call.seq) {
                    (
                        "input hidden: secret_flag".to_string(),
                        "input hidden: secret_flag".to_string(),
                    )
                } else {
                    call_inputs(&call.name, &call.arguments)
                };
                calls.push(CallLine {
                    name: call.name,
                    input,
                    full_input,
                    decision: None,
                });
            }
            Event::ToolResult(result) => {
                let Some(index) = open.get_mut(&result.call_id).and_then(Vec::pop) else {
                    continue;
                };
                let decision = match crate::normalize::hydra_voice(&result.output) {
                    Some(code) if code != "allow" => format!("refused {code}"),
                    _ => match result.status {
                        ResultStatus::Ok => "allowed",
                        ResultStatus::Error => "error",
                        ResultStatus::Unclassified => "unclassified",
                    }
                    .to_string(),
                };
                calls[index].decision = Some(decision);
            }
            _ => {}
        }
    }
    let from = calls.len().saturating_sub(DETAIL_CALLS);
    calls.split_off(from)
}

/// The `seq` of every line whose envelope carries a `secret_flag`.
fn flagged_seqs(text: &str) -> HashSet<u64> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|envelope| !envelope["secret_flag"].is_null())
        .filter_map(|envelope| envelope["seq"].as_u64())
        .collect()
}

/// A record's first line, or `None` when there is none to read.
pub(super) fn line_zero(path: &Path) -> Option<String> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).ok()?;
    let mut first = String::new();
    std::io::BufReader::new(file).read_line(&mut first).ok()?;
    Some(first)
}

/// The model on line 0's envelope, the `policy_sha` in its body and the
/// envelope's `ts`. [`crate::live::read_view`] reads none of them.
fn opening_of(line_zero: Option<&str>) -> (Option<String>, Option<String>, Option<DateTime<Utc>>) {
    let envelope = line_zero.and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok());
    let Some(envelope) = envelope else {
        return (None, None, None);
    };
    let field = |value: &serde_json::Value| value.as_str().map(str::to_string);
    (
        field(&envelope["model"]),
        field(&envelope["hydra"]["policy_sha"]),
        envelope["ts"]
            .as_str()
            .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(|ts| ts.with_timezone(&Utc)),
    )
}

/// One provider's entry in the header.
///
/// A provider whose live runs carry no counts yet says so rather than showing
/// zero: only `agy` counts as it goes, see [`crate::normalize::usage_so_far`].
pub fn spend_text(spend: &Spend) -> String {
    if spend.counted == 0 {
        return format!("{} no count until a run ends", spend.provider);
    }
    let rate = spend
        .tokens_per_minute
        .map_or_else(|| "-".to_string(), |rate| format!("{rate:.0}"));
    let share = spend
        .window_share
        .map_or_else(|| "-".to_string(), |share| format!("{:.1}%", share * 100.0));
    let mut text = format!("{} {rate} tok/min, window {share}", spend.provider);
    if spend.counted < spend.live {
        text.push_str(&format!(
            " ({} of {} runs counted)",
            spend.counted, spend.live
        ));
    }
    text
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
            if line.contains("\"secret_flag\":[") {
                return "[line hidden: secret_flag]".to_string();
            }
            line.to_string()
        })
        .collect())
}
