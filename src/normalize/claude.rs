//! The `claude` adapter: the first column of contract 7.2's table.
//!
//! One pass over the file builds the events that come straight off a frame, and
//! a second derives the permission decisions, which need the whole run before
//! any of them can be answered. Nothing here writes.
//!
//! **Three sources say what happened to a tool call and only one of them
//! decides.** 7.7 gives the derivation the last word: the run's own policy,
//! re-evaluated against the call. The `hook_response` frames and the tool result
//! corroborate, and where they contradict the re-evaluation the disagreement is
//! reported rather than resolved.
//!
//! The one place this file reads the provider's words to decide anything is the
//! `hydra/` prefix on a refusal reason, and that is not parsing prose: ticket 01
//! made the prefix a stable machine-readable code so a reader could tell Hydra's
//! own gate from the user's `PreToolUse` guards firing on the same event. 7.7
//! forbids reading the reason *text* for attribution, which is a different thing
//! and is what the Cursor probe in 6.6 was about.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;

use super::{
    Counts, Decider, Decision, Event, Finding, Message, PermissionDecision, RecordedGate,
    ReplaySpawn, ResultStatus, Role, Run, RunEnd, RunStart, Thinking, ToolCall, ToolResult, Usage,
    UsageSource, policy_moved, splice, verdict_decision,
};
use crate::paths::Layout;
use crate::policy::replay::{Replay, reason_code};
use crate::policy::table::claude_capability;
use crate::policy::{Capability, PolicySet};
use crate::provider::Provider;

/// What a `PreToolUse` hook answered.
///
/// `claude` names the tool in `hook_name` as `PreToolUse:<Tool>`, and a run
/// fires several hooks per call: the 2026-09-05 measurement caught four
/// responses for one `Bash` call, of which one was Hydra's. The frame carries
/// no `tool_use_id`, so a response is joined to its call by the `hook_id` it
/// shares with a `hook_started` frame, or by tool name and order on a record
/// old enough to carry no `hook_started` frames at all.
struct HookAnswer {
    seq: u64,
    hook_id: String,
    tool: String,
    decision: Decision,
    reason: String,
    /// Whether the reason carries the `hydra/` prefix, which is what tells
    /// Hydra's own gate from the user's guards on the same event.
    from_hydra: bool,
    taken: bool,
}

/// A `PreToolUse` hook's own start, kept to place its later `hook_response` in
/// the call it belongs to.
///
/// The response can land anywhere once Claude Code pipelines calls, but the
/// start cannot: 6.6 measured it firing the moment the call is emitted, before
/// the next call of the same tool. `hook_id` is what joins it to its response.
struct HookStart {
    seq: u64,
    tool: String,
    hook_id: String,
}

/// The stretch of a run in which a hook answer can belong to one tool call.
///
/// It opens at the call's own frame and closes at the next call that came later,
/// which is the bound the join was missing: without it, the first of two `Bash`
/// calls takes a refusal the guard wrote about the second, and both decisions
/// come out wrong. Two calls in a single assistant frame share a `seq` and
/// therefore share a window, and the `taken` flag is what keeps them from
/// claiming the same answer.
///
/// This is the fallback join now, used only on a record with no `hook_started`
/// frames to join a response through instead (ticket 22).
#[derive(Clone, Copy)]
struct Window {
    after: u64,
    before: u64,
}

impl Window {
    fn holds(&self, seq: u64) -> bool {
        seq > self.after && seq < self.before
    }
}

/// The `result.usage` keys section 9 maps. An object with none of them says
/// nothing, whatever else it holds.
const COUNT_FIELDS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
];

/// A refusal the record shows, as against one the policy implies.
struct Refusal {
    by: Decider,
    reason: String,
}

/// Everything one pass over the file collects.
#[derive(Default)]
struct Walk {
    run_id: String,
    parent_run_id: Option<String>,
    provider: Option<String>,
    policy: Option<String>,
    policy_sha: Option<String>,
    /// `hydra.skills.store_sha` off line 0. Contract 7.7's re-evaluation needs
    /// the `skill_snapshot` scope, and this is the field it is recomputed from.
    store_sha: Option<String>,
    cwd: Option<PathBuf>,
    worktree: Option<PathBuf>,
    /// Contract 5.1's `scopes` and `floor_lifted`, the other two inputs to the
    /// compile 7.7 re-runs. Absent on every record written before ticket 42,
    /// which is what `RecordedGate` answers for.
    gate: RecordedGate,
    events: Vec<Event>,
    findings: Vec<Finding>,
    hooks: Vec<HookAnswer>,
    /// `hook_started` frames, `PreToolUse` only, in file order. Empty on a
    /// record written before Claude Code emitted them, which is what tells
    /// `decide` to fall back to the seq window.
    starts: Vec<HookStart>,
    /// `system`/`permission_denied`, keyed by `tool_use_id`. The provider's own
    /// refusal, which appears only on a run launched without the permissive flag.
    provider_denied: HashMap<String, String>,
    result: Option<Value>,
    /// The names the `system`/`init` frame offered, for 7.1's table check.
    offered: Vec<String>,
    unknown_tools: Vec<String>,
    usage_emitted: bool,
    /// Lines the provider wrote that were not JSON, kept as strings.
    noise: Vec<String>,
}

pub fn normalize(text: &str, policies: &PolicySet, layout: &Layout) -> Result<Run> {
    let mut walk = Walk::default();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        // A torn line costs its own frames and nothing else. `record/mod.rs`
        // writes and syncs per line, so a run Hydra was killed during can leave
        // half a line at the tail, and contract 6 calls that an abandoned run
        // rather than a corrupt file. Failing the whole read would hand a corpus
        // walk nothing at all for a file that is intact up to its last line.
        match serde_json::from_str::<Value>(line) {
            Ok(envelope) => walk.line(&envelope),
            Err(err) => walk.findings.push(Finding::Limit(format!(
                "line {} did not parse, so whatever it carried is unread: {err}",
                index + 1
            ))),
        }
    }
    walk.close();
    walk.finish(policies, layout)
}

impl Walk {
    fn line(&mut self, envelope: &Value) {
        let seq = envelope["seq"].as_u64().unwrap_or_default();
        if self.run_id.is_empty() {
            self.run_id = envelope["run_id"].as_str().unwrap_or_default().to_string();
        }
        if self.provider.is_none() {
            self.provider = envelope["provider"].as_str().map(str::to_string);
        }
        if self.parent_run_id.is_none() {
            self.parent_run_id = envelope["parent_run_id"].as_str().map(str::to_string);
        }

        if let Some(hydra) = envelope.get("hydra") {
            self.hydra_line(seq, hydra);
        } else if let Some(raw) = envelope.get("raw") {
            // A line the provider wrote that is not JSON is kept as a string
            // (contract 3) and normalizes to nothing. It is not a finding: the
            // one `claude` produces per print-mode run is a show-cursor escape.
            if let Some(frame) = raw.as_object() {
                self.frame(seq, raw, frame.get("type").and_then(Value::as_str));
            } else if let Some(text) = raw.as_str() {
                self.noise.push(text.to_string());
            }
        }
    }

    fn hydra_line(&mut self, seq: u64, body: &Value) {
        match body["type"].as_str() {
            Some("run_start") => {
                self.policy = body["policy"].as_str().map(str::to_string);
                self.policy_sha = body["policy_sha"].as_str().map(str::to_string);
                self.store_sha = body["skills"]["store_sha"].as_str().map(str::to_string);
                self.cwd = body["cwd"].as_str().map(PathBuf::from);
                self.worktree = body["worktree"]["path"].as_str().map(PathBuf::from);
                self.gate = RecordedGate::of(body);
                self.events.push(Event::RunStart(RunStart {
                    seq,
                    body: body.clone(),
                }));
            }
            Some("run_end") => {
                // 7.8's never-a-bare-null, for a run whose provider wrote no
                // `result` frame at all. It goes in before the closing line so
                // the event order still follows the file.
                self.emit_missing_usage();
                self.events.push(Event::RunEnd(RunEnd {
                    seq,
                    body: body.clone(),
                }));
            }
            _ => {}
        }
    }

    fn frame(&mut self, seq: u64, raw: &Value, kind: Option<&str>) {
        match kind {
            Some("assistant") => self.blocks(seq, raw, Role::Assistant),
            Some("user") => self.blocks(seq, raw, Role::User),
            Some("system") => match raw["subtype"].as_str() {
                Some("init") => {
                    if let Some(tools) = raw["tools"].as_array() {
                        self.offered
                            .extend(tools.iter().filter_map(Value::as_str).map(str::to_string));
                    }
                }
                Some("hook_started") => self.hook_started(seq, raw),
                Some("hook_response") => self.hook_response(seq, raw),
                Some("permission_denied") => {
                    if let Some(id) = raw["tool_use_id"].as_str() {
                        let message = raw["message"].as_str().unwrap_or_default().to_string();
                        self.provider_denied.insert(id.to_string(), message);
                    }
                }
                _ => {}
            },
            Some("result") => {
                self.result = Some(raw.clone());
                self.events.push(Event::Usage(usage_of(seq, raw)));
                self.usage_emitted = true;
            }
            _ => {}
        }
    }

    /// The content blocks of one `assistant` or `user` frame, in order.
    fn blocks(&mut self, seq: u64, raw: &Value, role: Role) {
        let Some(blocks) = raw["message"]["content"].as_array() else {
            return;
        };
        for (block, body) in blocks.iter().enumerate() {
            match body["type"].as_str() {
                Some("text") => self.events.push(Event::Message(Message {
                    seq,
                    role,
                    text: body["text"].as_str().unwrap_or_default().to_string(),
                    block,
                })),
                Some("thinking") => self.events.push(Event::Thinking(Thinking {
                    seq,
                    text: body["thinking"].as_str().unwrap_or_default().to_string(),
                    signature: body["signature"].as_str().map(str::to_string),
                    block,
                })),
                Some("tool_use") => {
                    let name = body["name"].as_str().unwrap_or_default().to_string();
                    let capability = claude_capability(&name);
                    if capability.is_none() {
                        self.note_unknown(&name);
                    }
                    self.events.push(Event::ToolCall(ToolCall {
                        seq,
                        call_id: body["id"].as_str().unwrap_or_default().to_string(),
                        name,
                        arguments: body["input"].clone(),
                        capability,
                    }));
                }
                Some("tool_result") => self.events.push(Event::ToolResult(ToolResult {
                    seq,
                    call_id: body["tool_use_id"].as_str().unwrap_or_default().to_string(),
                    status: match body.get("is_error").and_then(Value::as_bool) {
                        Some(true) => ResultStatus::Error,
                        Some(false) => ResultStatus::Ok,
                        // Measured on 2.1.261: a successful `Write` came back
                        // with no `is_error` field at all. 7.6 forbids reading
                        // that as success.
                        None => ResultStatus::Unclassified,
                    },
                    output: result_output(body.get("content")),
                    // `claude` reports no truncation on the stream and nothing
                    // here cuts an output, so the marker is always false.
                    truncated: false,
                })),
                _ => {}
            }
        }
    }

    /// A `PreToolUse` hook starting, before it has answered anything.
    ///
    /// Kept only to place the matching `hook_response` in a call's window by
    /// where it actually began, since the response itself can arrive after the
    /// next call once Claude Code pipelines them.
    fn hook_started(&mut self, seq: u64, raw: &Value) {
        if raw["hook_event"].as_str() != Some("PreToolUse") {
            return;
        }
        let Some(hook_id) = raw["hook_id"].as_str() else {
            return;
        };
        let tool = raw["hook_name"]
            .as_str()
            .and_then(|name| name.split_once(':'))
            .map(|(_, tool)| tool.to_string())
            .unwrap_or_default();
        self.starts.push(HookStart {
            seq,
            tool,
            hook_id: hook_id.to_string(),
        });
    }

    fn hook_response(&mut self, seq: u64, raw: &Value) {
        // The user's own hooks fire on the same event, so this filters on the
        // event name rather than assuming every hook frame is Hydra's.
        if raw["hook_event"].as_str() != Some("PreToolUse") {
            return;
        }
        let hook_id = raw["hook_id"].as_str().unwrap_or_default().to_string();
        let tool = raw["hook_name"]
            .as_str()
            .and_then(|name| name.split_once(':'))
            .map(|(_, tool)| tool.to_string())
            .unwrap_or_default();
        // A hook that answered nothing writes an empty `output`, which is what
        // the three guards beside Hydra's did on every measured call.
        let Some(output) = raw["output"].as_str() else {
            return;
        };
        let Ok(answer) = serde_json::from_str::<Value>(output) else {
            return;
        };
        let specific = &answer["hookSpecificOutput"];
        let decision = match specific["permissionDecision"].as_str() {
            Some("allow") => Decision::Allow,
            Some("deny") => Decision::Deny,
            _ => return,
        };
        let reason = specific["permissionDecisionReason"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        self.hooks.push(HookAnswer {
            seq,
            hook_id,
            tool,
            from_hydra: reason_code(&reason).is_some(),
            decision,
            reason,
            taken: false,
        });
    }

    fn note_unknown(&mut self, name: &str) {
        if !self.unknown_tools.iter().any(|known| known == name) {
            self.unknown_tools.push(name.to_string());
        }
    }

    fn emit_missing_usage(&mut self) {
        if self.usage_emitted {
            return;
        }
        self.events.push(Event::Usage(Usage {
            seq: None,
            source: UsageSource::Unavailable,
            estimator: None,
            counts: None,
        }));
        self.usage_emitted = true;
    }

    /// A run Hydra was killed during has no `run_end`, so the usage event that
    /// rides in front of it needs a second home.
    fn close(&mut self) {
        self.emit_missing_usage();
    }

    fn finish(mut self, policies: &PolicySet, layout: &Layout) -> Result<Run> {
        super::note_missing_run_start(&self.events, &mut self.findings);
        super::note_policy_sha(
            self.policy.as_deref(),
            self.policy_sha.as_deref(),
            policies,
            &mut self.findings,
        );
        self.check_table();
        let decisions = self.decide(policies, layout);
        self.check_subagents(policies);

        let events = splice(self.events, decisions);
        Ok(Run {
            run_id: self.run_id,
            parent_run_id: self.parent_run_id,
            provider: self.provider,
            policy: self.policy,
            events,
            findings: self.findings,
            result: self.result,
            noise: self.noise,
        })
    }

    /// 7.1's run-time table check, against the names the `init` frame offered.
    ///
    /// The frame lists every tool at no cost, so a self-update that renames one
    /// is visible in the first record written after it rather than at the first
    /// denial.
    fn check_table(&mut self) {
        let offered = std::mem::take(&mut self.offered);
        for name in offered {
            if claude_capability(&name).is_none() {
                self.note_unknown(&name);
            }
        }
        let unknown = std::mem::take(&mut self.unknown_tools);
        self.findings.extend(
            unknown
                .into_iter()
                .map(|name| Finding::UnknownTool { name }),
        );
    }

    /// 8.5's `subagent_stats`, read as the second instance of the gate-leak check.
    fn check_subagents(&mut self, policies: &PolicySet) {
        let Some(name) = self.policy.clone() else {
            return;
        };
        let spawned = self
            .result
            .as_ref()
            .and_then(|result| result["subagent_stats"]["spawned"].as_u64())
            .unwrap_or_default();
        if spawned == 0 {
            return;
        }
        let Ok(policy) = policies.get(&name) else {
            return;
        };
        let granted = policy
            .allow
            .iter()
            .any(|allow| allow.capability == Capability::SubagentSpawn);
        if !granted {
            self.findings.push(Finding::SubagentLeak { spawned });
        }
    }

    /// One decision per call, by re-evaluating the run's own policy (7.7).
    ///
    /// Absent, never empty, on a run with no policy: there is nothing to
    /// re-evaluate and `external` would presume a policy that could have allowed.
    fn decide(&mut self, policies: &PolicySet, layout: &Layout) -> Vec<Option<PermissionDecision>> {
        // One slot per call, in call order, because `call_id` is the provider's
        // to supply and a block without one would collide with every other
        // block without one.
        let mut decisions: Vec<Option<PermissionDecision>> = Vec::new();
        let Some(name) = self.policy.clone() else {
            return decisions;
        };
        let calls: Vec<(u64, String, String, Value)> = self
            .events
            .iter()
            .filter_map(|event| match event {
                Event::ToolCall(call) => Some((
                    call.seq,
                    call.call_id.clone(),
                    call.name.clone(),
                    call.arguments.clone(),
                )),
                _ => None,
            })
            .collect();
        if calls.is_empty() {
            return decisions;
        }

        // Every scope the gate was compiled with, recomputed from the record.
        // The three adapters build it the same way and differ in the provider.
        let spawn = ReplaySpawn::of(
            layout,
            &self.run_id,
            &name,
            self.cwd.as_deref(),
            self.worktree.as_deref(),
            self.store_sha.as_deref(),
            &self.gate,
        );
        // Once per record, before any decision, so a reader sees what the
        // derivation stood in for ahead of what it derived.
        self.findings.extend(spawn.scope_limit());
        let replay = match Replay::compile(policies, &spawn.spawn(Provider::Claude)) {
            Ok(replay) => replay,
            Err(err) => {
                self.findings.push(Finding::Limit(format!(
                    "policy {name} would not compile, so no decision was derived: {err:#}"
                )));
                return decisions;
            }
        };

        let results = self.results_by_call();
        let seqs: Vec<u64> = calls.iter().map(|call| call.0).collect();
        // Empty on a record with no `hook_started` frames, which is what tells
        // the loop below to keep using the seq window instead.
        let groups = Self::group_starts(&calls, &self.starts);
        for (index, (seq, call_id, tool, arguments)) in calls.into_iter().enumerate() {
            let window = Window {
                after: seq,
                before: seqs[index + 1..]
                    .iter()
                    .copied()
                    .find(|later| *later > seq)
                    .unwrap_or(u64::MAX),
            };
            let group = (!self.starts.is_empty()).then(|| groups[index].as_slice());
            if call_id.is_empty() {
                // Nothing to join a result or a hook answer to. The policy still
                // answers for the call, so the decision is derived and this says
                // what it could not be checked against.
                self.findings.push(Finding::Limit(format!(
                    "a {tool} call at seq {seq} carries no tool_use_id, so no result \
                     and no hook answer can be joined to it"
                )));
            }
            let verdict = match replay.decide(&tool, &arguments, spawn.cwd()) {
                Ok(verdict) => verdict,
                Err(err) => {
                    self.findings.push(Finding::Limit(format!(
                        "the gate would not answer for {call_id} ({tool}): {err:#}"
                    )));
                    decisions.push(None);
                    continue;
                }
            };
            let gate_said = self.take_gate_answer(window, group, &tool);
            let refusal = self.refusal(&call_id, window, group, &tool, results.get(&call_id));

            // 6.6: a hook that fails to start fails open on this provider, and
            // `--include-hook-events` is on for every recorded run, so a call
            // with no gate answer and no refusal in Hydra's own voice is a call
            // the gate never spoke about.
            let hydra_spoke = gate_said.is_some()
                || refusal
                    .as_ref()
                    .is_some_and(|refusal| refusal.by == Decider::HydraPolicy);
            if !hydra_spoke {
                self.findings.push(Finding::Ungated {
                    call_id: call_id.clone(),
                    tool: tool.clone(),
                });
            }

            if let Some((decision, reason)) = &gate_said
                && *decision != verdict_decision(verdict.allowed)
            {
                // The only place in the record where two independent sources
                // answer the same question, so a difference is reported rather
                // than resolved (7.7). Unless the policy directory moved between
                // the run and the read, in which case the two sources answered
                // about two policies and the difference says nothing.
                if policy_moved(self.policy_sha.as_deref(), policies) {
                    self.findings.push(Finding::Limit(format!(
                        "the gate answered {:?} for {call_id} ({tool}) and the derivation \
                         says {:?}, under a policy_sha that is no longer today's, so the \
                         two answered about two policies",
                        decision,
                        verdict_decision(verdict.allowed),
                    )));
                } else if let Some(limit) = spawn.stood_in(&format!(
                    "the gate answered {:?} for {call_id} ({tool}) and the derivation says {:?}",
                    decision,
                    verdict_decision(verdict.allowed),
                )) {
                    self.findings.push(limit);
                } else {
                    self.findings.push(Finding::Disagreement {
                        call_id: call_id.clone(),
                        derived: verdict_decision(verdict.allowed),
                        derived_code: verdict.code().map(str::to_string),
                        gate_said: *decision,
                        gate_code: reason_code(reason).map(str::to_string),
                    });
                }
            }

            let derived = if !verdict.allowed {
                if refusal.is_none() {
                    // The policy denies this call. Whether that is a leak depends
                    // on whether the record says what happened to the call at
                    // all: a run Hydra was killed during can stop between a
                    // `tool_use` block and its result, and "the file ends here"
                    // is not evidence that the call went through.
                    let fate_known = results.contains_key(&call_id)
                        || gate_said.is_some()
                        || self.denied_in_result(&call_id);
                    if !fate_known {
                        self.findings.push(Finding::Limit(format!(
                            "{call_id} ({tool}) is the last thing the record says, so \
                             whether the gate refused a call the policy denies is unknown"
                        )));
                    } else if let Some(limit) = spawn.stood_in(&format!(
                        "the policy denies {call_id} ({tool}) and nothing in the record refused it"
                    )) {
                        self.findings.push(limit);
                    } else {
                        self.findings.push(Finding::GateLeak {
                            call_id: call_id.clone(),
                            tool: tool.clone(),
                            reason: verdict.reason.clone(),
                        });
                    }
                }
                PermissionDecision {
                    seq,
                    call_id: call_id.clone(),
                    decision: Decision::Deny,
                    decider: Decider::HydraPolicy,
                    reason: verdict.reason,
                }
            } else {
                match refusal {
                    // Refused, and the run's own policy would have allowed it.
                    Some(refusal) if refusal.by == Decider::External => PermissionDecision {
                        seq,
                        call_id: call_id.clone(),
                        decision: Decision::Deny,
                        decider: Decider::External,
                        reason: refusal.reason,
                    },
                    // Either nothing refused, or Hydra's own gate did and said
                    // so in its own voice, which is the skew the disagreement
                    // above already reports. 7.7 gives the policy the last word.
                    _ => PermissionDecision {
                        seq,
                        call_id: call_id.clone(),
                        decision: Decision::Allow,
                        decider: Decider::HydraPolicy,
                        reason: verdict.reason,
                    },
                }
            };
            decisions.push(Some(derived));
        }
        decisions
    }

    /// The results, owned rather than borrowed, because deriving a decision
    /// appends findings to the same struct the events live in.
    fn results_by_call(&self) -> HashMap<String, (ResultStatus, String)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Event::ToolResult(result) if !result.call_id.is_empty() => Some((
                    result.call_id.clone(),
                    (result.status, result.output.clone()),
                )),
                _ => None,
            })
            .collect()
    }

    /// Every `hook_started` frame assigned to the call it belongs to.
    ///
    /// The n-th `PreToolUse:<Tool>` start group belongs to the n-th `<Tool>`
    /// call, a call's group being every start between its own frame and the
    /// next call of the same tool. `calls` is already in seq order, which is
    /// what lets this walk each tool's calls and starts together in one pass.
    fn group_starts(
        calls: &[(u64, String, String, Value)],
        starts: &[HookStart],
    ) -> Vec<Vec<String>> {
        let mut groups = vec![Vec::new(); calls.len()];
        let mut by_tool: HashMap<&str, Vec<usize>> = HashMap::new();
        for (index, call) in calls.iter().enumerate() {
            by_tool.entry(call.2.as_str()).or_default().push(index);
        }
        for indices in by_tool.values() {
            for (position, &index) in indices.iter().enumerate() {
                let after = calls[index].0;
                let before = indices
                    .get(position + 1)
                    .map(|&next| calls[next].0)
                    .unwrap_or(u64::MAX);
                let tool = calls[index].2.as_str();
                groups[index] = starts
                    .iter()
                    .filter(|start| start.tool == tool && start.seq > after && start.seq < before)
                    .map(|start| start.hook_id.clone())
                    .collect();
            }
        }
        groups
    }

    /// Hydra's own gate answer for this call, if the record kept one.
    ///
    /// Joined by `hook_id` when `group` names one: every `hook_id` a
    /// `hook_started` frame in this call's own window carries. The response
    /// itself is not where a pipelined call is placed, because run 01a07efe-97f4
    /// on 2026-09-07 showed `claude-sonnet-5` issuing a second `Read` before the
    /// first one's result or hook answer arrived, so the first response landed
    /// inside what an order-only join would have called the second call's
    /// window. `group` is `None` on a record with no `hook_started` frames at
    /// all, and there ordering is still the join key: two calls to one tool in
    /// a single assistant frame are answered in the order the hooks ran, and if
    /// that ever stops holding the disagreement check is what reports it.
    fn take_gate_answer(
        &mut self,
        window: Window,
        group: Option<&[String]>,
        tool: &str,
    ) -> Option<(Decision, String)> {
        let answer = self.hooks.iter_mut().find(|hook| {
            if hook.taken || !hook.from_hydra || hook.tool != tool {
                return false;
            }
            match group {
                Some(ids) => ids.contains(&hook.hook_id),
                None => window.holds(hook.seq),
            }
        })?;
        answer.taken = true;
        Some((answer.decision, answer.reason.clone()))
    }

    /// A refusal from a hook that is not Hydra's, joined the same way.
    ///
    /// This looks for a deny rather than for the next answer of any kind. A hook
    /// outside Hydra that allowed decides nothing, and consuming its slot would
    /// push a real deny onto the following call.
    fn take_external_deny(
        &mut self,
        window: Window,
        group: Option<&[String]>,
        tool: &str,
    ) -> Option<String> {
        let answer = self.hooks.iter_mut().find(|hook| {
            if hook.taken || hook.from_hydra || hook.decision != Decision::Deny || hook.tool != tool
            {
                return false;
            }
            match group {
                Some(ids) => ids.contains(&hook.hook_id),
                None => window.holds(hook.seq),
            }
        })?;
        answer.taken = true;
        Some(answer.reason.clone())
    }

    /// What the record shows happened to the call, as against what the policy
    /// says should have.
    fn refusal(
        &mut self,
        call_id: &str,
        window: Window,
        group: Option<&[String]>,
        tool: &str,
        result: Option<&(ResultStatus, String)>,
    ) -> Option<Refusal> {
        // Strongest first. The provider's own refusal names the call, and it
        // appears only on a run launched without the permissive flag.
        if let Some(message) = self.provider_denied.get(call_id) {
            return Some(Refusal {
                by: Decider::External,
                reason: message.clone(),
            });
        }
        // A hook outside Hydra that denied. Hydra's own answer is taken
        // separately, because it is the corroboration rather than a second
        // decider.
        if let Some(reason) = self.take_external_deny(window, group, tool) {
            return Some(Refusal {
                by: Decider::External,
                reason,
            });
        }
        // The gate's reason arrives verbatim as the tool result with
        // `is_error: true`, which is the one signal a killed run still has.
        if let Some((ResultStatus::Error, output)) = result
            && let Some(code) = reason_code(output)
            && code != "allow"
        {
            return Some(Refusal {
                by: Decider::HydraPolicy,
                reason: output.clone(),
            });
        }
        // 8.5's list. It names the call and carries no reason, so the tool
        // result is the best text available and the attribution is the weakest
        // of the four.
        if self.denied_in_result(call_id) {
            return Some(Refusal {
                by: Decider::External,
                reason: result
                    .map(|(_, output)| output.clone())
                    .unwrap_or_else(|| "refused, per result.permission_denials".into()),
            });
        }
        None
    }

    fn denied_in_result(&self, call_id: &str) -> bool {
        self.result
            .as_ref()
            .and_then(|result| result["permission_denials"].as_array())
            .is_some_and(|denials| {
                denials
                    .iter()
                    .any(|denial| denial["tool_use_id"].as_str() == Some(call_id))
            })
    }
}

/// A tool result's body, which is a string on most calls and a block list on the
/// ones that return structured content.
fn result_output(content: Option<&Value>) -> String {
    match content {
        None => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .map(|block| match block["text"].as_str() {
                Some(text) => text.to_string(),
                // Not text, so keep the block rather than drop it.
                None => block.to_string(),
            })
            .collect::<Vec<_>>()
            // The provider laid these out as separate blocks, and running them
            // together would put the seam inside a word for every reader
            // downstream, the outcome derivation of contract 8 included.
            .join("\n"),
        Some(other) => other.to_string(),
    }
}

/// Section 9's mapping, off `result.usage`.
///
/// A `result` frame with no `usage` object is `unavailable` rather than zeroes,
/// which is the whole point of 7.8: a null and a provider reporting zero are the
/// same value and mean opposite things.
fn usage_of(seq: u64, result: &Value) -> Usage {
    // An object with none of section 9's counts in it is the same as no object:
    // reading it as zeroes is 7.8's null-against-zero confusion arriving through
    // the other door, since a reader could not tell "the provider said zero" from
    // "the provider said nothing this maps".
    let counted = result
        .get("usage")
        .filter(|usage| usage.is_object())
        .filter(|usage| {
            COUNT_FIELDS
                .iter()
                .any(|field| usage.get(field).and_then(Value::as_u64).is_some())
        });
    let Some(usage) = counted else {
        return Usage {
            seq: None,
            source: UsageSource::Unavailable,
            estimator: None,
            counts: None,
        };
    };
    Usage {
        seq: Some(seq),
        source: UsageSource::Provider,
        estimator: None,
        counts: Some(Counts {
            input_tokens: usage["input_tokens"].as_u64().unwrap_or_default(),
            output_tokens: usage["output_tokens"].as_u64().unwrap_or_default(),
            cache_read_tokens: usage["cache_read_input_tokens"]
                .as_u64()
                .unwrap_or_default(),
            cache_write_tokens: usage["cache_creation_input_tokens"].as_u64(),
            thinking_tokens: usage["output_tokens_details"]["thinking_tokens"].as_u64(),
        }),
    }
}
