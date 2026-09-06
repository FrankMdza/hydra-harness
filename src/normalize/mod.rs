//! The ten normalized event types, contract section 7.
//!
//! **Nothing here writes anything.** Contract 11 makes normalization a read-time
//! library function, and the reason is worth keeping in front of whoever edits
//! this file: an adapter that misreads a frame today is fixed next month and
//! last month's records normalize correctly on the next read, because nothing
//! was migrated. None of the types below derives `Serialize`, so a caller that
//! wants to put one in a file has to say so out loud first.
//!
//! Two providers. [`claude`] and [`cursor`] are the adapters this build routes,
//! one per column of contract 7.2's table, and [`read_with`] picks one by the
//! envelope's `provider`, which every line carries (contract 4). `agy` is
//! ticket 10.
//!
//! Two of the ten have no producer here and both are on the list anyway, because
//! a type the reader cannot name is a type nobody knows is missing.
//! [`MessageDelta`] needs `--include-partial-messages`, which contract 7.4 keeps
//! off in v1. [`Correction`] comes from an interactive harvest (7.9) and this
//! build launches no interactive runs.

pub mod claude;
pub mod cursor;

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::paths::Layout;
use crate::policy::{Capability, PolicySet};

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

/// Contract 7.4. Dark in v1 on all three providers, and nothing constructs it.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decider {
    /// The run's own policy answers for it.
    HydraPolicy,
    /// The call was refused and the run's policy would have allowed it, so
    /// something outside Hydra denied. On this machine that means the user's own
    /// `PreToolUse` guards.
    External,
}

impl Decider {
    pub fn as_str(self) -> &'static str {
        match self {
            Decider::HydraPolicy => "hydra_policy",
            Decider::External => "external",
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
    /// **Net of cache on all three providers** (9.1). Reading it as the prompt
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
    /// Always null in this build, because delegation is not built.
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
    /// On `cursor-agent` it is the failure path: the provider's `catch` writes
    /// no frame, prints one sentence to stderr and exits 1, and over a pty that
    /// sentence lands here. Contract 8's outcome derivation reads it for the
    /// two refusals that never reach a frame on that provider, a usage limit
    /// and a lapsed login, so that rung one does not learn a skill lesson from
    /// an exhausted quota.
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

/// A file whose opening line is gone reads exactly like a pre-gate run, and the
/// two mean opposite things.
///
/// Contract 2 puts `run_start` on the first line of every run file, so a file
/// without one was truncated or is not a run file. Saying nothing would hand a
/// reader a run with a null policy and no permission decisions, which is a real
/// and different thing (5.6).
pub(crate) fn note_missing_run_start(events: &[Event], findings: &mut Vec<Finding>) {
    if !events.iter().any(|event| matches!(event, Event::RunStart(_))) {
        findings.push(Finding::Limit(
            "the file carries no run_start, so the policy, the cwd and the scopes a \
             decision would be re-evaluated against are all unknown"
                .into(),
        ));
    }
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
