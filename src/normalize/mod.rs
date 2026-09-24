//! The ten normalized event types, contract section 7.
//!
//! **Nothing here writes anything.** Contract 11 makes normalization a read-time
//! library function, and the reason is worth keeping in front of whoever edits
//! this file: an adapter that misreads a frame today is fixed next month and
//! last month's records normalize correctly on the next read, because nothing
//! was migrated. None of the types below derives `Serialize`, so a caller that
//! wants to put one in a file has to say so out loud first.
//!
//! Four providers. [`claude`], [`cursor`], [`agy`] and [`codex`] are one per
//! column of contract 7.2's table, and [`read_with`] picks one by the envelope's
//! `provider`, which every line carries (contract 4).
//!
//! One of the ten has no producer here and is on the list anyway, because a
//! type the reader cannot name is a type nobody knows is missing.
//! [`MessageDelta`] needs `--include-partial-messages`, which contract 7.4 keeps
//! off in v1. [`Correction`] comes from an interactive harvest (7.9), which
//! `claude::` reads since build ticket 27.

pub mod agy;
pub mod claude;
pub mod codex;
pub mod cursor;
pub mod paths;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::paths::Layout;
use crate::policy::{Capability, PolicySet, Spawn, keys};
use crate::provider::Provider;

/// One normalized event. Ten variants, contract 7.2.
#[derive(Debug, Clone)]
pub enum Event {
    RunStart(RunStart),
    RunEnd(RunEnd),
    Message(Message),
    MessageDelta(MessageDelta),
    Thinking(Thinking),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    PermissionDecision(PermissionDecision),
    Usage(Usage),
    Correction(Correction),
}

impl Event {
    /// The type name the contract uses. A reader grouping events by type should
    /// use this rather than a match on the variant, so the two never drift.
    pub fn type_name(&self) -> &'static str {
        match self {
            Event::RunStart(_) => "run_start",
            Event::RunEnd(_) => "run_end",
            Event::Message(_) => "message",
            Event::MessageDelta(_) => "message_delta",
            Event::Thinking(_) => "thinking",
            Event::ToolCall(_) => "tool_call",
            Event::ToolResult(_) => "tool_result",
            Event::PermissionDecision(_) => "permission_decision",
            Event::Usage(_) => "usage",
            Event::Correction(_) => "correction",
        }
    }
}

/// Contract 5, verbatim.
///
/// The body is the object Hydra wrote rather than a struct with the same fields
/// in a second spelling. `record::lines::RunStart` is the write side and copying
/// it here would be a second place to keep in sync for no reader's benefit.
#[derive(Debug, Clone)]
pub struct RunStart {
    pub seq: u64,
    pub body: Value,
}

/// Contract 6. Absent on a run Hydra was killed during, which is an abandoned
/// run rather than a corrupt file.
#[derive(Debug, Clone)]
pub struct RunEnd {
    pub seq: u64,
    pub body: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Assistant,
    User,
}

/// Contract 7.3. Text that arrived whole on `claude`.
#[derive(Debug, Clone)]
pub struct Message {
    pub seq: u64,
    pub role: Role,
    pub text: String,
    /// Which content block of the frame this came from, so the derivation is
    /// reproducible from `raw` rather than trusted. On `agy` this is the
    /// `step_index` the concatenation ran over.
    pub block: usize,
}

/// Contract 7.4. Dark in v1 on all four providers, and nothing constructs it.
#[derive(Debug, Clone)]
pub struct MessageDelta {
    pub seq: u64,
    pub role: Role,
    pub text: String,
}

/// Contract 7.5. Text here, the token count on [`Usage`], so a provider that
/// reports one never implies the other.
#[derive(Debug, Clone)]
pub struct Thinking {
    pub seq: u64,
    pub text: String,
    /// `claude` signs every thinking block. `cursor-agent` signs none.
    pub signature: Option<String>,
    pub block: usize,
}

/// Contract 7.6.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub seq: u64,
    /// `tool_use_id` on `claude`, which is the one provider of three that gives
    /// a real join key on both sides.
    pub call_id: String,
    pub name: String,
    pub arguments: Value,
    /// What Hydra's own table maps the name to, `None` when the table does not
    /// know it. 7.1 denies an unknown name, so `None` is a finding rather than a
    /// shrug, and [`Finding::UnknownTool`] carries it.
    pub capability: Option<Capability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultStatus {
    Ok,
    Error,
    /// The adapter cannot tell. 7.6 makes this a feature: `agy` gives a
    /// permission denial and a file-not-found the same error type, and an
    /// adapter that guesses produces a curator that rewrites a skill because a
    /// file was missing. Nothing upgrades it to `Ok` later.
    Unclassified,
}

/// Contract 7.6, joined to its call on `tool_use_id`.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub seq: u64,
    pub call_id: String,
    pub status: ResultStatus,
    pub output: String,
    /// Whether what the record holds was cut. `claude` reports no truncation on
    /// the stream and this normalizer truncates nothing, so it is always false
    /// here. The field exists because 7.6 requires the adapter to produce it,
    /// and because the alternative it warns about, dropping the marker, teaches
    /// a model that a cut-off output was the whole output.
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
        }
    }
}

/// Which policy accounts for the decision, not which process fired (7.7).
///
/// `provider` is the exception: the tool never reached a policy, because the
/// provider ran it without asking the hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decider {
    /// The run's own policy answers for it.
    HydraPolicy,
    /// The call was refused and the run's policy would have allowed it, so
    /// something outside Hydra denied. On this machine that means the user's own
    /// `PreToolUse` guards.
    External,
    /// The call went to a tool the provider runs without asking the hook
    /// (contract 7.7, map ticket 51). No policy answered and none could.
    Provider,
}

impl Decider {
    pub fn as_str(self) -> &'static str {
        match self {
            Decider::HydraPolicy => "hydra_policy",
            Decider::External => "external",
            Decider::Provider => "provider",
        }
    }
}

/// Contract 7.7, one per [`ToolCall`] on a recorded run that had a policy.
///
/// **Absent, never empty, on a run with no policy.** A pre-gate record and an
/// interactive one both carry `policy: null`, there is nothing to re-evaluate,
/// and 7.7 refuses to attribute the refusal to the user's hooks instead:
/// `external` presumes a policy exists to have allowed.
#[derive(Debug, Clone)]
pub struct PermissionDecision {
    /// The `tool_call` this answers, so the event is reproducible against the
    /// frame it was derived from.
    pub seq: u64,
    pub call_id: String,
    pub decision: Decision,
    pub decider: Decider,
    pub reason: String,
}

/// Contract 7.8 and 9.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSource {
    Provider,
    /// In the enum and unproduced in v1. 7.8 keeps it because `usage` is a
    /// per-run decision and a future producer may earn the value.
    Estimated,
    Unavailable,
}

impl UsageSource {
    pub fn as_str(self) -> &'static str {
        match self {
            UsageSource::Provider => "provider",
            UsageSource::Estimated => "estimated",
            UsageSource::Unavailable => "unavailable",
        }
    }
}

/// The parts, never a bare null (7.8).
///
/// A null cannot be told apart from a provider reporting zero, so a run whose
/// provider reported nothing gets a [`Usage`] event with `source:
/// UsageSource::Unavailable` and no counts, rather than counts of zero.
#[derive(Debug, Clone)]
pub struct Usage {
    /// The frame the counts came from. `None` when nothing reported any, which
    /// is the `unavailable` case: there is no frame to point at.
    pub seq: Option<u64>,
    pub source: UsageSource,
    /// Which tokenizer ran. Null unless `source` is `estimated`, and nothing in
    /// v1 estimates.
    pub estimator: Option<String>,
    pub counts: Option<Counts>,
}

/// Section 9's field mapping, normalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// **Net of cache on all four providers** (9.1). Reading it as the prompt
    /// size is the first of section 9's two traps.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    /// Absent on `agy`, which reports no cache write.
    pub cache_write_tokens: Option<u64>,
    /// Absent on `cursor-agent`, which reports no reasoning count.
    pub thinking_tokens: Option<u64>,
}

impl Counts {
    /// 9.1's reconstruction: `input_tokens + cache_read_tokens +
    /// cache_write_tokens`, over whichever of the three a provider reports.
    ///
    /// This is the number that means "the prompt the model was sent". One
    /// observed `claude` frame settles that it is needed: a real prompt reported
    /// `input_tokens: 2` beside `cache_creation_input_tokens: 10558`.
    pub fn raw_input(&self) -> u64 {
        self.input_tokens + self.cache_read_tokens + self.cache_write_tokens.unwrap_or(0)
    }

    /// Everything the run moved through the model, prompt and completion.
    ///
    /// The 9 table says `total_tokens` is derived on `claude` and does not give
    /// the formula, and there are two candidates. `agy` reports its own
    /// `total_tokens` as `input + output`, excluding cache read, so matching
    /// that spelling would make a warm-cache run look like a cheap one, which is
    /// exactly what 9.1 warns against for `cursor-agent`'s `inputTokens`. This
    /// sums the reconstruction instead. A reader wanting `agy`'s narrower number
    /// has `input_tokens` and `output_tokens` beside it.
    pub fn total_tokens(&self) -> u64 {
        self.raw_input() + self.output_tokens
    }
}

/// Contract 7.9. One producer, a human correcting a tool call in an interactive
/// `claude` run, recovered at harvest. Nothing constructs it here.
#[derive(Debug, Clone)]
pub struct Correction {
    /// The `seq` of the `tool_call` or `tool_result` this corrects.
    pub corrects: u64,
    pub rejected: Value,
    pub chosen: Value,
    /// The human's words, when the walk in 7.9 reaches a plain user turn. Step 4
    /// of that rule is the weak one: a human who rejects a call and then changes
    /// the subject produces a record indistinguishable from a correction.
    pub words: Option<String>,
}

/// Something the derivation found that a reader must not average away.
///
/// These are not events. An event is a normalized view of what happened; a
/// finding is the normalizer saying two sources disagree, or that it could not
/// do the job the contract asks of it.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    /// 7.7's gate-leak check. The policy denies this call and nothing refused
    /// it, so the gate leaked.
    GateLeak {
        call_id: String,
        tool: String,
        reason: String,
    },
    /// The same check through 8.5's `subagent_stats`: in-process subagents ran
    /// under a policy that grants no `subagent_spawn`. A count is not a
    /// trajectory, which is why this is a finding and not a reopened decision.
    SubagentLeak { spawned: u64 },
    /// The derived decision and `claude`'s own hook frames answer the same
    /// question differently. 7.7: a finding, not noise. It is the only place in
    /// the record where two independent sources speak.
    Disagreement {
        call_id: String,
        derived: Decision,
        derived_code: Option<String>,
        gate_said: Decision,
        gate_code: Option<String>,
    },
    /// The run had a policy and the stream carries no gate answer for this call.
    ///
    /// On `claude` a hook that fails to start fails open (harness spec 6.6), so
    /// a call that reached the model with nothing in front of it looks exactly
    /// like this. The innocent reading is that `--include-hook-events` was off,
    /// which is why it is a finding rather than a leak.
    Ungated { call_id: String, tool: String },
    /// 7.1's run-time table check. The provider offered a name Hydra's
    /// capability table does not know, so the gate denied it all run and the
    /// table is stale.
    UnknownTool { name: String },
    /// `policy_sha` on the record is not the sha of the policy directory this
    /// re-evaluation used, so every decision below answers for a policy the run
    /// never saw. Nothing recovers an old policy directory from a sha.
    StalePolicy { recorded: String, current: String },
    /// Something the derivation could not do, said out loud rather than papered
    /// over with a default.
    Limit(String),
}

/// One run, normalized.
#[derive(Debug, Clone)]
pub struct Run {
    pub run_id: String,
    /// The run that spawned this one, contract 10's edge. **Written by the parent
    /// at spawn and never reconciled against the child** (contract 4), so it is
    /// the envelope's field rather than anything the run said about itself.
    /// Null on a run a human started, and set on one launched with `--parent`.
    ///
    /// Rung one reads it twice: to find a retry loop, and to leave out the runs a
    /// curator pass spawned, which is how it avoids learning from itself.
    pub parent_run_id: Option<String>,
    /// `null` on a harness run, which contract 8.6 excludes from the corpus.
    pub provider: Option<String>,
    /// `run_start.policy`. `None` is what makes `permission_decision` absent.
    pub policy: Option<String>,
    pub events: Vec<Event>,
    pub findings: Vec<Finding>,
    /// The provider's terminator frame, verbatim, or `None` on a run that never
    /// reached one.
    ///
    /// **This is a raw frame and not a normalized field**, which is why it is a
    /// `Value` and why nothing here reshapes it. Contract 8.5 names three
    /// `claude` fields that stay in `raw` and two of them live on this frame,
    /// `permission_denials` and `subagent_stats`. Contract 8's outcome
    /// derivation reads three more off it, and the nastiest shape in the
    /// contract is one of them: a rate-limited run reports `subtype: success`
    /// with `is_error: true`. Keeping the frame is what stops the next reader
    /// re-opening the file to find it.
    ///
    /// The third of 8.5's fields, `rate_limit_event`, is its own frame type and
    /// has no reader until rung two's early abort, so nothing keeps it.
    pub result: Option<Value>,
    /// Every line the provider wrote that was not JSON, verbatim (contract 3).
    ///
    /// On `claude` that is one show-cursor escape per run and nothing reads it.
    /// On `cursor-agent` 2026.09.02-c22c1a3 it is a coloured tip before the
    /// first frame, the same escape after the last, and the failure path: the
    /// provider's `catch` writes no frame, prints one sentence to stderr and
    /// exits 1 (research 25, read off the emitter on 2026-09-02 and reproduced
    /// by research 33 on 2026-09-04), and over a pty that sentence lands here.
    /// Contract 8's outcome derivation reads it for the two refusals that never
    /// reach a frame on that provider, a usage limit and a lapsed login, so that
    /// rung one does not learn a skill lesson from an exhausted quota.
    pub noise: Vec<String>,
}

impl Run {
    pub fn tool_calls(&self) -> impl Iterator<Item = &ToolCall> {
        self.events.iter().filter_map(|event| match event {
            Event::ToolCall(call) => Some(call),
            _ => None,
        })
    }

    pub fn tool_results(&self) -> impl Iterator<Item = &ToolResult> {
        self.events.iter().filter_map(|event| match event {
            Event::ToolResult(result) => Some(result),
            _ => None,
        })
    }

    pub fn messages(&self) -> impl Iterator<Item = &Message> {
        self.events.iter().filter_map(|event| match event {
            Event::Message(message) => Some(message),
            _ => None,
        })
    }

    pub fn permission_decisions(&self) -> impl Iterator<Item = &PermissionDecision> {
        self.events.iter().filter_map(|event| match event {
            Event::PermissionDecision(decision) => Some(decision),
            _ => None,
        })
    }

    pub fn usage(&self) -> Option<&Usage> {
        self.events.iter().find_map(|event| match event {
            Event::Usage(usage) => Some(usage),
            _ => None,
        })
    }

    pub fn run_start(&self) -> Option<&RunStart> {
        self.events.iter().find_map(|event| match event {
            Event::RunStart(start) => Some(start),
            _ => None,
        })
    }

    pub fn run_end(&self) -> Option<&RunEnd> {
        self.events.iter().find_map(|event| match event {
            Event::RunEnd(end) => Some(end),
            _ => None,
        })
    }
}

/// Reads a run file and yields its events.
///
/// The policy set is the caller's so that a corpus walk loads and hashes
/// `policies/` once. It goes unused on a record with no policy.
pub fn read_file(path: &Path, policies: &PolicySet) -> Result<Run> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let layout = Layout::discover()?;
    read_with(&text, policies, &layout)
}

/// The same, from text, against the real `~/hydra`.
pub fn read(text: &str, policies: &PolicySet) -> Result<Run> {
    let layout = Layout::discover()?;
    read_with(text, policies, &layout)
}

/// The same, against a layout the caller names.
///
/// The layout is an argument because re-evaluating a policy needs the scopes the
/// run had, and two of the three are Hydra's own directories rather than
/// anything on the record: `skill_store` and `corpus`. A test that points them
/// at a temporary tree is asking a real question about a synthetic run.
///
/// **The adapter is picked by the envelope's `provider`**, read off the first
/// line that parses. A harness run carries `null` and holds no frames, so any
/// adapter reads it the same way; a provider this build has no adapter for gets
/// `claude`'s, which normalizes none of its frames, plus a finding that says so
/// rather than a run that quietly has no events.
pub fn read_with(text: &str, policies: &PolicySet, layout: &Layout) -> Result<Run> {
    let provider = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|envelope| envelope["provider"].as_str().map(str::to_string));
    match provider.as_deref() {
        Some("cursor-agent") => cursor::normalize(text, policies, layout),
        Some("agy") => agy::normalize(text, policies, layout),
        Some("codex") => codex::normalize(text, policies, layout),
        Some("claude") | None => claude::normalize(text, policies, layout),
        Some(other) => {
            let mut run = claude::normalize(text, policies, layout)?;
            run.findings.push(Finding::Limit(format!(
                "no adapter reads {other} frames in this build, so nothing between the two \
                 Hydra lines was normalized"
            )));
            Ok(run)
        }
    }
}

/// **What line 0 says about the gate the run was compiled with.** Contract 5.1's
/// `scopes` and `floor_lifted`, read the way every other line 0 field is read: by
/// name, out of a `Value`, with absence meaning something. A record written
/// before build ticket 15 carries neither, so both are optional here and the
/// reader says out loud what it stood in for.
#[derive(Default, Debug, Clone)]
pub(crate) struct RecordedGate {
    /// `None` on a record that predates the field.
    ///
    /// **A key missing from a present object is an answer for two of the seven
    /// and a gap for the rest.** `skill_checkout` and `parent_git_dir` are absent
    /// when the run had no such path, which contract 5.1 makes the meaning of an
    /// absent key. The other four are on every record a gate was compiled for, so
    /// a present object missing one is a record from somewhere else, and the
    /// reader rebuilds it and says so rather than reading the gap as "no
    /// worktree".
    scopes: Option<std::collections::BTreeMap<String, PathBuf>>,
    floor_lifted: bool,
    /// Contract 5.1's `gate_sha`, build ticket 49. `None` on every record
    /// written before the field existed.
    gate_sha: Option<String>,
}

impl RecordedGate {
    pub(crate) fn of(body: &Value) -> Self {
        let scopes = body.get("scopes").and_then(Value::as_object).map(|object| {
            object
                .iter()
                .filter_map(|(name, path)| Some((name.clone(), PathBuf::from(path.as_str()?))))
                .collect()
        });
        Self {
            scopes,
            // Harness spec 16's flag, and false is what a record without it
            // means: the floor was in force, which is the shape ticket 03 left.
            floor_lifted: body["floor_lifted"].as_bool().unwrap_or(false),
            gate_sha: body["gate_sha"].as_str().map(str::to_string),
        }
    }

    fn path(&self, name: &str) -> Option<&Path> {
        self.scopes.as_ref()?.get(name).map(PathBuf::as_path)
    }

    fn carries_scopes(&self) -> bool {
        self.scopes.is_some()
    }

    /// The sha the record says its gate had, if it says one.
    ///
    /// The gate's code is not the policy set. `policy_sha` moves when a policy
    /// changes and stands still when the deny list, the floor's parser or the
    /// shell check changes, so a record without `gate_sha` cannot tell today's
    /// gate from the one that ran. Build ticket 49 and
    /// `docs/reviews/2026-09-14-the-gate-under-attack.md`.
    fn named_gate(&self) -> Option<&str> {
        self.gate_sha.as_deref()
    }
}

/// The six fields a replay rebuilds a spawn from, taken off one record.
///
/// They travel together because they come from one place and are passed
/// unchanged by all three adapters, and because nine positional parameters is a
/// shape a caller gets wrong silently.
pub(crate) struct Recorded<'a> {
    pub run_id: &'a str,
    pub policy: &'a str,
    /// The record's own `cwd`, absent on a run whose opening line is gone.
    pub cwd: Option<&'a Path>,
    pub worktree: Option<&'a Path>,
    /// Contract 5.3's `skills.store_sha`, which the `skill_snapshot` scope
    /// recomputes from and from nothing else.
    pub store_sha: Option<&'a str>,
    pub gate: &'a RecordedGate,
}

/// What a record says about the gate that answered for it, build ticket 49.
///
/// The sha is of the compiled script's bytes, which carry the policy, the deny
/// list, the scopes this spawn resolved and the Python the gate runs. So it
/// moves when any of those move, and `policy_sha` beside it moves only when a
/// policy does. Two marks and no third: either this build compiles the same
/// bytes the run was launched with or it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateMark {
    /// No `gate_sha` on `run_start`. Every record written before build ticket
    /// 49, and every run that compiled no gate.
    Unnamed,
    /// The record names the gate this build compiles for the same spawn.
    Todays,
    /// The record names a gate whose bytes are not the ones compiled here.
    Foreign,
}

impl GateMark {
    /// Compiles the gate this build would write for `spawn` and compares.
    ///
    /// A policy that will not compile is `Foreign` rather than `Todays`: the
    /// adapter reports the compile failure on its own line, and until it does
    /// nothing here may claim a leak. The sha is taken over the same bytes
    /// `write_gate` puts on disk, which is what `src/run.rs` hashes back.
    fn of(recorded: Option<&str>, set: &crate::policy::PolicySet, spawn: &Spawn<'_>) -> Self {
        let Some(recorded) = recorded else {
            return Self::Unnamed;
        };
        match crate::policy::gate_sha(set, spawn) {
            Ok(today) if today == recorded => Self::Todays,
            _ => Self::Foreign,
        }
    }

    /// Why this mark leaves a finding unbacked, as one clause, or `None`.
    fn why_not(self) -> Option<&'static str> {
        match self {
            Self::Todays => None,
            Self::Unnamed => Some("the record does not say which gate answered for it"),
            Self::Foreign => Some("the gate it names is not the one compiled here"),
        }
    }
}

/// The launch a record describes, rebuilt for contract 7.7's re-evaluation.
///
/// **A stored path wins and a missing one is rebuilt.** Contract 7.7: a reader
/// never prefers its own layout over a path on the record, because the stored
/// path is what the gate saw and the layout is what this build would have done.
/// Where the record says nothing, every scope below is recomputed by the same
/// rule the launch used, and [`ReplaySpawn::scope_limit`] is the reader saying
/// which ones.
///
/// **The three adapters differ in the provider and in nothing else here.** A
/// second spelling of any one of these fields in one adapter would make that
/// adapter's `GateLeak` findings mean something different from the other two's,
/// and nothing would say which.
///
/// It owns its paths because [`Spawn`] borrows them.
pub(crate) struct ReplaySpawn<'a> {
    layout: &'a Layout,
    policy: String,
    cwd: PathBuf,
    worktree: PathBuf,
    skill_store: PathBuf,
    corpus: PathBuf,
    skill_snapshot: PathBuf,
    skill_checkout: Option<PathBuf>,
    parent_git_dir: Option<PathBuf>,
    gate_script: PathBuf,
    floor_lifted: bool,
    /// What the record says about the gate that answered for it, build ticket 49.
    gate: GateMark,
    /// The scope names this rebuilt rather than read, in the order the compile
    /// takes them.
    rebuilt: Vec<&'static str>,
    /// The parent repo's `.git` was rebuilt and nothing answered, so the replay
    /// runs one denied path short of the gate's.
    parent_git_dir_lost: bool,
}

impl<'a> ReplaySpawn<'a> {
    /// `cwd` and `worktree` are the record's own fields, both absent on a run
    /// whose opening line is gone.
    pub(crate) fn of(
        layout: &'a Layout,
        recorded: Recorded<'_>,
        policies: &crate::policy::PolicySet,
        provider: Provider,
    ) -> Self {
        let Recorded {
            run_id,
            policy,
            cwd,
            worktree,
            store_sha,
            gate,
        } = recorded;
        let here = cwd
            .or(worktree)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| layout.worktree(run_id));
        let mut rebuilt: Vec<&'static str> = Vec::new();
        // A stored path, or the rebuild with the key named. The four below are on
        // every record a gate was compiled for, so a missing one is a gap rather
        // than an answer.
        let mut take = |key: &'static str, derive: &mut dyn FnMut() -> PathBuf| match gate.path(key)
        {
            Some(path) => path.to_path_buf(),
            None => {
                rebuilt.push(key);
                derive()
            }
        };

        // The same path the launch used for a run with no worktree, and not the
        // record's own `cwd`. 7.5 carves the run's worktree out of the denials,
        // so a fallback to the cwd would let a replay read the whole directory
        // the human stood in as a path the run owned.
        let worktree_scope = take(keys::WORKTREE, &mut || {
            worktree
                .map(Path::to_path_buf)
                .unwrap_or_else(|| layout.spawn(run_id).join("no-worktree"))
        });
        let skill_store = take(keys::SKILL_STORE, &mut || layout.skills());
        let corpus = take(keys::CORPUS, &mut || layout.runs());
        let gate_script = take(keys::GATE_SCRIPT, &mut || layout.gate_script(run_id));

        // These two are absent from a `scopes` object when the run had no path
        // for them, so on a record that carries the object an absent key is the
        // answer and nothing is rebuilt.
        let mut parent_git_dir_lost = false;
        let (skill_checkout, parent_git_dir) = if gate.carries_scopes() {
            (
                gate.path(keys::SKILL_CHECKOUT).map(Path::to_path_buf),
                gate.path(keys::PARENT_GIT_DIR).map(Path::to_path_buf),
            )
        } else {
            // A curate run's writes are scoped to its own checkout of the store,
            // and the rule is a function of the run id so that this side can
            // recompute what the gate was compiled with. Without it every
            // curator's record reads back as a gate that leaked on its own
            // proposal. Every other policy was given no checkout, so nothing is
            // rebuilt and the limit below does not claim a path the run never
            // had.
            let checkout = (policy == "curate").then(|| layout.skill_checkout(run_id));
            if checkout.is_some() {
                rebuilt.push(keys::SKILL_CHECKOUT);
            }
            // Never rediscover Git metadata through a run's writable checkout.
            // Old records can recover the parent from Hydra's saved launch data;
            // without either source, report the missing scope.
            let found = worktree.and_then(|path| {
                crate::worktree::CheckoutGit::for_run(run_id, path, None)
                    .ok()
                    .map(|context| context.common_dir().to_path_buf())
            });
            match &found {
                Some(_) => rebuilt.push(keys::PARENT_GIT_DIR),
                None => parent_git_dir_lost = true,
            }
            (checkout, found)
        };

        let mut replay = Self {
            layout,
            policy: policy.to_string(),
            cwd: here,
            worktree: worktree_scope,
            skill_store,
            corpus,
            // The launch derived this scope from the sha it pinned, contract 5.3
            // put that sha on line 0, and both sides call one function on it. A
            // run with no `skills` object was linked nothing, and the scope is
            // then a path nothing creates. It is not on `scopes` for that
            // reason: the record already answers it once.
            skill_snapshot: layout.skill_snapshot_scope(run_id, store_sha),
            skill_checkout,
            parent_git_dir,
            gate_script,
            floor_lifted: gate.floor_lifted,
            // Filled below, once there is a spawn to compile a gate from.
            gate: GateMark::Unnamed,
            rebuilt,
            parent_git_dir_lost,
        };
        // The gate the record names, against the gate this build compiles for
        // the same spawn. Once per record: `compile` is string building over a
        // dozen resolved paths, and the corpus walk reads two hundred records.
        let mark = GateMark::of(gate.named_gate(), policies, &replay.spawn(provider));
        replay.gate = mark;
        replay
    }

    /// The directory the run worked in, which is what a path in a tool call
    /// resolves against.
    pub(crate) fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// What the derivation stood in for, once per record.
    ///
    /// **A limit and never a disagreement.** A disagreement reaches the
    /// curator's brief weighted as a clean-looking failure, and rebuilding a
    /// path the record never carried earns nothing of the sort. Map ticket 42
    /// measured the cost of getting that arm wrong: one real record would have
    /// arrived at the top of a pass's brief as a run whose gate leaked twenty
    /// times, having leaked none.
    pub(crate) fn scope_limit(&self) -> Option<Finding> {
        if self.rebuilt.is_empty() && !self.parent_git_dir_lost {
            return None;
        }
        let mut said = format!(
            "the record carries no scopes for {}, so the paths this run's decisions \
             resolve against were rebuilt from this build's layout rather than read \
             off line 0",
            self.rebuilt.join(", ")
        );
        if self.parent_git_dir_lost {
            said.push_str(
                ", and nothing answered for parent_git_dir, so the replay runs one \
                 denied path short of the gate's",
            );
        }
        Some(Finding::Limit(said))
    }

    /// The limit that stands in for a finding this record cannot back, or
    /// `None` when it can.
    ///
    /// **The same shape [`policy_moved`] gives `StalePolicy`, and the same
    /// argument.** A `GateLeak` and a `Disagreement` both claim the gate and the
    /// derivation answered the same question and differed. Two things make that
    /// claim unbackable, and this is the one door every caller goes through so
    /// that the three adapters cannot drift apart on which of them they ask.
    ///
    /// The gate question is the larger doubt: a record that does not say which
    /// gate answered, or that names one whose bytes are not the ones compiled
    /// here, was decided by code this build no longer has.
    /// A change to the deny list, the floor's parser or the shell check turned
    /// every older record into a leak three times on 2026-09-12 and once on
    /// 2026-09-14, and `policy_sha` stood still through all four. Build ticket 49.
    ///
    /// The scopes question is map ticket 42's decision 5: rebuild one of the
    /// paths the question resolves against and the derivation answered about a
    /// gate compiled from this build's layout. That cost was chosen out loud, so
    /// a record with no `scopes` cannot report a leak, and
    /// `src/rungs/corpus.rs` keeps limits out of the weighted
    /// `signals.disagreements` a brief ranks on.
    pub(crate) fn cannot_back(&self, what: &str) -> Option<Finding> {
        // Every reason, not the first one. Both can be true of one record, and
        // a reader fixing the record wants to know it carries neither mark
        // rather than discovering the second after supplying the first.
        let mut why: Vec<String> = Vec::new();
        if let Some(said) = self.gate.why_not() {
            why.push(said.to_string());
        }
        if !self.rebuilt.is_empty() {
            why.push(format!(
                "the scopes it was decided against were rebuilt here ({}) rather than \
                 read off the record",
                self.rebuilt.join(", ")
            ));
        }
        if why.is_empty() {
            return None;
        }
        Some(Finding::Limit(format!(
            "{what}, and {}, so the two answered about two gates",
            why.join(", and ")
        )))
    }

    pub(crate) fn spawn(&self, provider: Provider) -> Spawn<'_> {
        Spawn {
            provider,
            policy: &self.policy,
            layout: self.layout,
            worktree: &self.worktree,
            skill_store: &self.skill_store,
            corpus: &self.corpus,
            skill_snapshot: &self.skill_snapshot,
            skill_checkout: self.skill_checkout.as_deref(),
            parent_git_dir: self.parent_git_dir.as_deref(),
            gate_script: &self.gate_script,
            floor_lifted: self.floor_lifted,
        }
    }
}

/// The bar this provider's record let the reader apply, said once per record.
///
/// Contract 7.7's table: the three readers ask the same questions of records
/// that answer them differently, and rung one compares the counts across
/// providers. `claude`'s init frame lists every tool it offered and its result
/// frames say what became of every call, so its counts are a ceiling.
/// `cursor-agent` carries no tool list, so a renamed tool is invisible until
/// something calls it. `agy` reports a leak only on the one result status that
/// is evidence. Both of those are floors, and a reader holding a floor beside a
/// ceiling needs to be told which it has.
///
/// Nothing is emitted on `claude`, because it is the bar the other two are
/// measured against and a line saying "this count is complete" on 79 of 213
/// records buys nothing. Build ticket 55.
pub(crate) fn note_finding_bar(provider: Option<Provider>, findings: &mut Vec<Finding>) {
    let said = match provider {
        Some(Provider::CursorAgent) => {
            "this provider's init frame carries no tool list, so the tool-table check saw only \
             the names this run called, and a call the record cannot say the fate of raises no \
             leak: both counts on this record are floors rather than totals"
        }
        Some(Provider::Agy) => {
            "this reader raises a leak only where the tool reported success, which is the one \
             status on this provider that is evidence, so the leak count on this record is a \
             floor rather than a total"
        }
        _ => return,
    };
    findings.push(Finding::Limit(said.to_string()));
}

/// A file whose opening line is gone reads exactly like a pre-gate run, and the
/// two mean opposite things.
///
/// Contract 2 puts `run_start` on the first line of every run file, so a file
/// without one was truncated or is not a run file. Saying nothing would hand a
/// reader a run with a null policy and no permission decisions, which is a real
/// and different thing (5.6).
pub(crate) fn note_missing_run_start(events: &[Event], findings: &mut Vec<Finding>) {
    if !events
        .iter()
        .any(|event| matches!(event, Event::RunStart(_)))
    {
        findings.push(Finding::Limit(
            "the file carries no run_start, so the policy, the cwd and the scopes a \
             decision would be re-evaluated against are all unknown"
                .into(),
        ));
    }
}

/// A run whose `run_end.error` names lines the tee split at [`crate::pty::LINE_CAP`].
///
/// The frames themselves are ordinary provider lines: a piece that is not JSON
/// lands in `noise`, which is what any other non-JSON line already does. The
/// finding is the count, read off the error sentence the detached half wrote.
pub(crate) fn note_capped_lines(events: &[Event], findings: &mut Vec<Finding>) {
    let Some(end) = events.iter().rev().find_map(|event| match event {
        Event::RunEnd(end) => Some(end),
        _ => None,
    }) else {
        return;
    };
    let Some(error) = end.body["error"].as_str() else {
        return;
    };
    let Some(count) = crate::pty::capped_count(error) else {
        return;
    };
    findings.push(Finding::Limit(format!(
        "{count} frames were split by the tee"
    )));
}

/// The policy directory moved since the run, so every decision derived below
/// answers for a policy the run never saw.
pub(crate) fn note_policy_sha(
    policy: Option<&str>,
    policy_sha: Option<&str>,
    policies: &PolicySet,
    findings: &mut Vec<Finding>,
) {
    let Some(name) = policy else {
        return;
    };
    let Some(recorded) = policy_sha else {
        // 5.1 writes the two together, so this is a record from somewhere else.
        // The decisions are still derived, and nothing in them would otherwise
        // say they were never checked against the policy the run actually had.
        findings.push(Finding::Limit(format!(
            "the record names policy {name} and carries no policy_sha, so every decision \
             below answers for today's policies/ rather than for the directory the run \
             was launched from"
        )));
        return;
    };
    if recorded != policies.sha() {
        findings.push(Finding::StalePolicy {
            recorded: recorded.to_string(),
            current: policies.sha().to_string(),
        });
    }
}

/// Whether the record was written under a different policy directory than the
/// one being replayed.
///
/// **A disagreement means something only when both sources answered the same
/// question** (contract 7.7's "two independent sources"). The gate's answer was
/// written under the sha on the record and the derivation answers under today's,
/// so once those differ the two are not disagreeing, they are answering about
/// two policies. Build ticket 13 is where that stopped being hypothetical: it
/// gave `standard` the `skill_invoke` it lacked, and the one recorded run whose
/// `Skill` call the old gate correctly refused started reporting a contradiction
/// it never had.
///
/// A record carrying no `policy_sha` is not stale, it is unattributable, and
/// [`note_policy_sha`] already says so as a limit of its own.
pub(crate) fn policy_moved(recorded: Option<&str>, policies: &PolicySet) -> bool {
    recorded.is_some_and(|sha| sha != policies.sha())
}

/// The codes the gate writes, so that a `hydra/` inside somebody else's prose,
/// such as a guard quoting `~/hydra/runs`, is not mistaken for Hydra's voice.
pub(crate) const GATE_CODES: &[&str] = &[
    "allow",
    "capability",
    "out-of-scope",
    "protected-path",
    "floor",
    "no-path",
    "unknown-tool",
    "gate-tampered",
    "no-tool-name",
    "bad-payload",
    "wrong-dialect",
    "wrong-envelope",
    "gate-error",
    "seatbelt-delegation",
];

/// The gate's own code inside a refusal text, wherever it sits.
///
/// **Two adapters read this and neither can key on anything else.** Cursor
/// concatenates every hook's message into one reason, so Hydra's answer can
/// arrive behind `Command execution was blocked by a hook: ` or after another
/// hook's paragraph. `agy` puts the reason verbatim into the tool error behind
/// `tool call denied by pre-tool hook: `. The prefix is a machine-readable code
/// ticket 01 made stable, which is what contract 7.7 allows a reader to key on;
/// the prose after it is what 7.7 forbids.
pub fn hydra_voice(text: &str) -> Option<&'static str> {
    let mut rest = text;
    while let Some(at) = rest.find("hydra/") {
        let after = &rest[at + "hydra/".len()..];
        if let Some(colon) = after.find(':') {
            let code = &after[..colon];
            if let Some(known) = GATE_CODES.iter().find(|known| **known == code) {
                return Some(known);
            }
        }
        rest = after;
    }
    None
}

pub(crate) fn verdict_decision(allowed: bool) -> Decision {
    if allowed {
        Decision::Allow
    } else {
        Decision::Deny
    }
}

/// Each decision goes in behind the call it answers, so the event list still
/// reads in the order the run happened.
pub(crate) fn splice(events: Vec<Event>, decisions: Vec<Option<PermissionDecision>>) -> Vec<Event> {
    if decisions.is_empty() {
        return events;
    }
    let mut spliced = Vec::with_capacity(events.len() + decisions.len());
    // Position rather than `call_id`, because the adapters build one slot per
    // call in this order and an id is the provider's to supply.
    let mut derived = decisions.into_iter();
    for event in events {
        let is_call = matches!(event, Event::ToolCall(_));
        spliced.push(event);
        if is_call && let Some(Some(decision)) = derived.next() {
            spliced.push(Event::PermissionDecision(decision));
        }
    }
    spliced
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `hydra/<code>:` a text carries.
    fn codes_in(text: &str) -> std::collections::BTreeSet<String> {
        let mut found = std::collections::BTreeSet::new();
        let mut rest = text;
        while let Some(at) = rest.find("hydra/") {
            let after = &rest[at + "hydra/".len()..];
            let code: String = after
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '-')
                .collect();
            if !code.is_empty() && after[code.len()..].starts_with(':') {
                found.insert(code);
            }
            rest = after;
        }
        found
    }

    #[test]
    fn the_known_codes_are_exactly_the_ones_the_gate_and_its_wrappers_write() {
        // `GATE_CODES` is a hand copy, and a code added to the script without a
        // row here would make a real refusal read as somebody else's. All three
        // wrappers are read, because each speaks its own provider's dialect and
        // any of them could grow a code of its own. The delegate face is a fourth
        // source: it is not the gate script, but its refusals carry `hydra/` codes
        // the reader must recognise.
        let mut written = codes_in(include_str!("../policy/gate.py"));
        for dialect in [
            crate::policy::Dialect::Claude,
            crate::policy::Dialect::Cursor,
            crate::policy::Dialect::Agy,
        ] {
            written.extend(codes_in(&crate::policy::hook_command(
                std::path::Path::new("/tmp/gate"),
                "sha",
                dialect,
            )));
        }
        written.extend(codes_in(crate::face::SEATBELT_DELEGATION_REFUSAL));
        let known: std::collections::BTreeSet<String> =
            GATE_CODES.iter().map(|code| code.to_string()).collect();
        assert_eq!(written, known);
    }
}
