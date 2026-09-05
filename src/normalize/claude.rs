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

use anyhow::{Context, Result};
use serde_json::Value;

use super::{
    Counts, Decider, Decision, Event, Finding, Message, PermissionDecision, ResultStatus, Role,
    Run, RunEnd, RunStart, Thinking, ToolCall, ToolResult, Usage, UsageSource,
};
use crate::paths::Layout;
use crate::policy::replay::{Replay, reason_code};
use crate::policy::table::claude_capability;
use crate::policy::{Capability, PolicySet, Spawn};

/// What a `PreToolUse` hook answered.
///
/// Joined to a call by tool name and order, because the frame carries no
/// `tool_use_id`. `claude` names the tool in `hook_name` as `PreToolUse:<Tool>`,
/// and a run fires several hooks per call: the 2026-09-05 measurement caught
/// four responses for one `Bash` call, of which one was Hydra's.
struct HookAnswer {
    seq: u64,
    tool: String,
    decision: Decision,
    reason: String,
    /// Whether the reason carries the `hydra/` prefix, which is what tells
    /// Hydra's own gate from the user's guards on the same event.
    from_hydra: bool,
    taken: bool,
}

/// A refusal the record shows, as against one the policy implies.
struct Refusal {
    by: Decider,
    reason: String,
}

/// Everything one pass over the file collects.
#[derive(Default)]
struct Walk {
    run_id: String,
    provider: Option<String>,
    policy: Option<String>,
    policy_sha: Option<String>,
    cwd: Option<PathBuf>,
    worktree: Option<PathBuf>,
    events: Vec<Event>,
    findings: Vec<Finding>,
    hooks: Vec<HookAnswer>,
    /// `system`/`permission_denied`, keyed by `tool_use_id`. The provider's own
    /// refusal, which appears only on a run launched without the permissive flag.
    provider_denied: HashMap<String, String>,
    result: Option<Value>,
    /// The names the `system`/`init` frame offered, for 7.1's table check.
    offered: Vec<String>,
    unknown_tools: Vec<String>,
    usage_emitted: bool,
}

pub fn normalize(text: &str, policies: &PolicySet, layout: &Layout) -> Result<Run> {
    let mut walk = Walk::default();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let envelope: Value = serde_json::from_str(line)
            .with_context(|| format!("line {} is not a JSON object", index + 1))?;
        walk.line(&envelope);
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

        if let Some(hydra) = envelope.get("hydra") {
            self.hydra_line(seq, hydra);
        } else if let Some(raw) = envelope.get("raw") {
            // A line the provider wrote that is not JSON is kept as a string
            // (contract 3) and normalizes to nothing. It is not a finding: the
            // one `claude` produces per print-mode run is a show-cursor escape.
            if let Some(frame) = raw.as_object() {
                self.frame(seq, raw, frame.get("type").and_then(Value::as_str));
            }
        }
    }

    fn hydra_line(&mut self, seq: u64, body: &Value) {
        match body["type"].as_str() {
            Some("run_start") => {
                self.policy = body["policy"].as_str().map(str::to_string);
                self.policy_sha = body["policy_sha"].as_str().map(str::to_string);
                self.cwd = body["cwd"].as_str().map(PathBuf::from);
                self.worktree = body["worktree"]["path"].as_str().map(PathBuf::from);
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

    fn hook_response(&mut self, seq: u64, raw: &Value) {
        // The user's own hooks fire on the same event, so this filters on the
        // event name rather than assuming every hook frame is Hydra's.
        if raw["hook_event"].as_str() != Some("PreToolUse") {
            return;
        }
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
        self.check_run_start();
        self.check_policy_sha(policies);
        self.check_table();
        let decisions = self.decide(policies, layout);
        self.check_subagents(policies);

        let events = splice(self.events, decisions);
        Ok(Run {
            run_id: self.run_id,
            provider: self.provider,
            policy: self.policy,
            events,
            findings: self.findings,
            result: self.result,
        })
    }

    /// A file whose opening line is gone reads exactly like a pre-gate run, and
    /// the two mean opposite things.
    ///
    /// Contract 2 puts `run_start` on the first line of every run file, so a file
    /// without one was truncated or is not a run file. Saying nothing would hand
    /// a reader a run with a null policy and no permission decisions, which is a
    /// real and different thing (5.6).
    fn check_run_start(&mut self) {
        if !self
            .events
            .iter()
            .any(|event| matches!(event, Event::RunStart(_)))
        {
            self.findings.push(Finding::Limit(
                "the file carries no run_start, so the policy, the cwd and the scopes a \
                 decision would be re-evaluated against are all unknown"
                    .into(),
            ));
        }
    }

    /// The policy directory moved since the run, so every decision below answers
    /// for a policy the run never saw.
    fn check_policy_sha(&mut self, policies: &PolicySet) {
        let (Some(_), Some(recorded)) = (&self.policy, &self.policy_sha) else {
            return;
        };
        if recorded != policies.sha() {
            self.findings.push(Finding::StalePolicy {
                recorded: recorded.clone(),
                current: policies.sha().to_string(),
            });
        }
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
    fn decide(
        &mut self,
        policies: &PolicySet,
        layout: &Layout,
    ) -> HashMap<String, PermissionDecision> {
        let mut decisions = HashMap::new();
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

        let cwd = self
            .cwd
            .clone()
            .or_else(|| self.worktree.clone())
            .unwrap_or_else(|| layout.worktree(&self.run_id));
        let worktree = self.worktree.clone().unwrap_or_else(|| cwd.clone());
        let gate_script = layout.gate_script(&self.run_id);
        let skills = layout.skills();
        let runs = layout.runs();
        let spawn = Spawn {
            policy: &name,
            worktree: &worktree,
            skill_store: &skills,
            runs: &runs,
            gate_script: &gate_script,
            // No field on the record carries it, so a run launched with
            // `--allow-floor-escape` re-evaluates to a deny it did not get.
            // `IMPLEMENTATION.md` has carried that gap since ticket 02.
            floor_lifted: false,
        };
        let replay = match Replay::compile(policies, &spawn) {
            Ok(replay) => replay,
            Err(err) => {
                self.findings.push(Finding::Limit(format!(
                    "policy {name} would not compile, so no decision was derived: {err:#}"
                )));
                return decisions;
            }
        };

        let results = self.results_by_call();
        for (seq, call_id, tool, arguments) in calls {
            let verdict = match replay.decide(&tool, &arguments, &cwd) {
                Ok(verdict) => verdict,
                Err(err) => {
                    self.findings.push(Finding::Limit(format!(
                        "the gate would not answer for {call_id} ({tool}): {err:#}"
                    )));
                    continue;
                }
            };
            let gate_said = self.take_gate_answer(seq, &tool);
            let refusal = self.refusal(&call_id, seq, &tool, results.get(&call_id));

            if let Some((decision, reason)) = &gate_said
                && *decision != verdict_decision(verdict.allowed)
            {
                // The only place in the record where two independent sources
                // answer the same question, so a difference is reported rather
                // than resolved (7.7).
                self.findings.push(Finding::Disagreement {
                    call_id: call_id.clone(),
                    derived: verdict_decision(verdict.allowed),
                    derived_code: verdict.code().map(str::to_string),
                    gate_said: *decision,
                    gate_code: reason_code(reason).map(str::to_string),
                });
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
                    if fate_known {
                        self.findings.push(Finding::GateLeak {
                            call_id: call_id.clone(),
                            tool: tool.clone(),
                            reason: verdict.reason.clone(),
                        });
                    } else {
                        self.findings.push(Finding::Limit(format!(
                            "{call_id} ({tool}) is the last thing the record says, so \
                             whether the gate refused a call the policy denies is unknown"
                        )));
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
            decisions.insert(call_id, derived);
        }
        decisions
    }

    /// The results, owned rather than borrowed, because deriving a decision
    /// appends findings to the same struct the events live in.
    fn results_by_call(&self) -> HashMap<String, (ResultStatus, String)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Event::ToolResult(result) => Some((
                    result.call_id.clone(),
                    (result.status, result.output.clone()),
                )),
                _ => None,
            })
            .collect()
    }

    /// Hydra's own gate answer for this call, if the record kept one.
    ///
    /// Ordering is the join key, because the frame carries no `tool_use_id`. Two
    /// calls to one tool in a single assistant frame are answered in the order
    /// the hooks ran, and if that ever stops holding the disagreement check is
    /// what reports it.
    fn take_gate_answer(&mut self, after: u64, tool: &str) -> Option<(Decision, String)> {
        let answer = self
            .hooks
            .iter_mut()
            .find(|hook| !hook.taken && hook.from_hydra && hook.tool == tool && hook.seq > after)?;
        answer.taken = true;
        Some((answer.decision, answer.reason.clone()))
    }

    /// A refusal from a hook that is not Hydra's, joined the same way.
    ///
    /// This looks for a deny rather than for the next answer of any kind. A hook
    /// outside Hydra that allowed decides nothing, and consuming its slot would
    /// push a real deny onto the following call.
    fn take_external_deny(&mut self, after: u64, tool: &str) -> Option<String> {
        let answer = self.hooks.iter_mut().find(|hook| {
            !hook.taken
                && !hook.from_hydra
                && hook.decision == Decision::Deny
                && hook.tool == tool
                && hook.seq > after
        })?;
        answer.taken = true;
        Some(answer.reason.clone())
    }

    /// What the record shows happened to the call, as against what the policy
    /// says should have.
    fn refusal(
        &mut self,
        call_id: &str,
        seq: u64,
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
        if let Some(reason) = self.take_external_deny(seq, tool) {
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

fn verdict_decision(allowed: bool) -> Decision {
    if allowed {
        Decision::Allow
    } else {
        Decision::Deny
    }
}

/// Each decision goes in behind the call it answers, so the event list still
/// reads in the order the run happened.
fn splice(events: Vec<Event>, mut decisions: HashMap<String, PermissionDecision>) -> Vec<Event> {
    if decisions.is_empty() {
        return events;
    }
    let mut spliced = Vec::with_capacity(events.len() + decisions.len());
    for event in events {
        let decision = match &event {
            Event::ToolCall(call) => decisions.remove(&call.call_id),
            _ => None,
        };
        spliced.push(event);
        if let Some(decision) = decision {
            spliced.push(Event::PermissionDecision(decision));
        }
    }
    spliced
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
            .join(""),
        Some(other) => other.to_string(),
    }
}

/// Section 9's mapping, off `result.usage`.
///
/// A `result` frame with no `usage` object is `unavailable` rather than zeroes,
/// which is the whole point of 7.8: a null and a provider reporting zero are the
/// same value and mean opposite things.
fn usage_of(seq: u64, result: &Value) -> Usage {
    let Some(usage) = result.get("usage").filter(|usage| usage.is_object()) else {
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
