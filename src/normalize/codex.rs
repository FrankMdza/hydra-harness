//! The `codex` adapter: the fourth column contract 7.2 will carry once ticket 48
//! lands, read off map ticket 58's measurement of 0.154.0 rather than a
//! recorded run on this machine.
//!
//! One pass over the file builds the events that come straight off a frame, and
//! a second derives the permission decisions, which need the whole run before
//! any of them can be answered. Nothing here writes.
//!
//! **The stream is JSONL of `item.*` and `turn.*` events**, not the three
//! providers' frame shapes. `item.completed` carries tool work; `turn.completed`
//! closes a turn and names usage.
//!
//! **MEASURE 48.5** still owns the exact field names on a logged-in run. The
//! shapes below follow ticket 58's prose and the unit tests' fixtures until that
//! measurement lands.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Value, json};

use super::{
    Counts, Decider, Decision, Event, Finding, Message, PermissionDecision, RecordedGate,
    ReplaySpawn, ResultStatus, Role, Run, RunEnd, RunStart, Thinking, ToolCall, ToolResult, Usage,
    UsageSource, hydra_voice, policy_moved, splice, verdict_decision,
};
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::policy::replay::Replay;
use crate::policy::table::codex_capability;
use crate::provider::Provider;

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
    store_sha: Option<String>,
    cwd: Option<PathBuf>,
    worktree: Option<PathBuf>,
    gate: RecordedGate,
    events: Vec<Event>,
    findings: Vec<Finding>,
    result: Option<Value>,
    usage_emitted: bool,
    noise: Vec<String>,
    counts: Option<Counts>,
    /// Tool calls keyed by the id the stream names, for pairing results.
    calls: HashMap<String, (u64, String, Value)>,
    unknown_tools: Vec<String>,
}

pub fn normalize(text: &str, policies: &PolicySet, layout: &Layout) -> Result<Run> {
    let mut walk = Walk::default();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
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
                self.emit_usage();
                self.events.push(Event::RunEnd(RunEnd {
                    seq,
                    body: body.clone(),
                }));
            }
            _ => {}
        }
    }

    fn frame(&mut self, seq: u64, raw: &Value, event: Option<&str>) {
        match event {
            Some("item.completed") => self.item_completed(seq, &raw["item"]),
            Some("turn.completed") => {
                self.result = Some(raw.clone());
                self.add_counts(&raw["usage"]);
                self.emit_usage();
            }
            _ => {}
        }
    }

    fn item_completed(&mut self, seq: u64, item: &Value) {
        match item["type"].as_str() {
            Some("command_execution") => self.command_execution(seq, item),
            Some("mcp_tool_call") => self.mcp_tool_call(seq, item),
            Some("apply_patch") | Some("file_change") => self.apply_patch(seq, item),
            Some("agent_message") => {
                if let Some(text) = item["text"].as_str().filter(|text| !text.is_empty()) {
                    self.events.push(Event::Message(Message {
                        seq,
                        role: Role::Assistant,
                        text: text.to_string(),
                        block: seq as usize,
                    }));
                }
            }
            Some("reasoning") => {
                if let Some(text) = item["text"].as_str().filter(|text| !text.is_empty()) {
                    self.events.push(Event::Thinking(Thinking {
                        seq,
                        text: text.to_string(),
                        signature: None,
                        block: seq as usize,
                    }));
                }
            }
            Some(other) => self.findings.push(Finding::Limit(format!(
                "item.completed carried type {other}, which no event type in contract 7.2 covers \
                 on this provider yet, so nothing was derived from it"
            ))),
            None => {}
        }
    }

    fn command_execution(&mut self, seq: u64, item: &Value) {
        let call_id = item_id(item, seq);
        let command = item["command"].clone();
        let name = "Bash".to_string();
        let capability = codex_capability(&name);
        if capability.is_none() {
            self.note_unknown(&name);
        }
        self.calls.insert(
            call_id.clone(),
            (seq, name.clone(), json!({ "command": command })),
        );
        self.events.push(Event::ToolCall(ToolCall {
            seq,
            call_id,
            name,
            arguments: json!({ "command": command }),
            capability,
        }));
        let output = item["output"].as_str().unwrap_or_default();
        let exit_code = item["exit_code"].as_i64().unwrap_or_default();
        let (status, message) = if exit_code == 0 {
            (ResultStatus::Ok, output.to_string())
        } else {
            (
                classify(output),
                if output.is_empty() {
                    format!("exit code {exit_code}")
                } else {
                    output.to_string()
                },
            )
        };
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id: item_id(item, seq),
            status,
            output: message,
            truncated: false,
        }));
    }

    fn mcp_tool_call(&mut self, seq: u64, item: &Value) {
        let server = item["server"].as_str().unwrap_or_default();
        let tool = item["tool"].as_str().unwrap_or_default();
        let name = format!("mcp__{server}__{tool}");
        let call_id = item_id(item, seq);
        let arguments = item["arguments"].clone();
        let capability = codex_capability(&name);
        if capability.is_none() {
            self.note_unknown(&name);
        }
        self.events.push(Event::ToolCall(ToolCall {
            seq,
            call_id: call_id.clone(),
            name,
            arguments,
            capability,
        }));
        let output = item["result"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| item["result"].to_string());
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            status: ResultStatus::Ok,
            output,
            truncated: false,
        }));
    }

    fn apply_patch(&mut self, seq: u64, item: &Value) {
        let call_id = item_id(item, seq);
        let command = item["command"].clone();
        let name = "apply_patch".to_string();
        let capability = codex_capability(&name);
        self.events.push(Event::ToolCall(ToolCall {
            seq,
            call_id: call_id.clone(),
            name,
            arguments: json!({ "command": command }),
            capability,
        }));
        let output = item["summary"]
            .as_str()
            .or_else(|| item["output"].as_str())
            .unwrap_or_default()
            .to_string();
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            status: ResultStatus::Ok,
            output,
            truncated: false,
        }));
    }

    fn add_counts(&mut self, usage: &Value) {
        let Some(usage) = usage.as_object() else {
            return;
        };
        let read = |field: &str| usage.get(field).and_then(Value::as_u64);
        if read("input_tokens").is_none() && read("output_tokens").is_none() {
            return;
        }
        let running = self.counts.get_or_insert_with(Counts::default);
        running.input_tokens += read("input_tokens").unwrap_or_default();
        running.output_tokens += read("output_tokens").unwrap_or_default();
        running.cache_read_tokens += read("cache_read_tokens").unwrap_or_default();
        if let Some(thinking) = read("reasoning_tokens").or_else(|| read("thinking_tokens")) {
            running.thinking_tokens = Some(running.thinking_tokens.unwrap_or_default() + thinking);
        }
    }

    fn emit_usage(&mut self) {
        if self.usage_emitted {
            return;
        }
        let usage = match self.counts {
            Some(counts) => Usage {
                seq: None,
                source: UsageSource::Provider,
                estimator: None,
                counts: Some(counts),
            },
            None => Usage {
                seq: None,
                source: UsageSource::Unavailable,
                estimator: None,
                counts: None,
            },
        };
        self.events.push(Event::Usage(usage));
        self.usage_emitted = true;
    }

    fn note_unknown(&mut self, name: &str) {
        if !self.unknown_tools.iter().any(|known| known == name) {
            self.unknown_tools.push(name.to_string());
        }
    }

    fn close(&mut self) {
        self.emit_usage();
    }

    fn finish(mut self, policies: &PolicySet, layout: &Layout) -> Result<Run> {
        super::note_missing_run_start(&self.events, &mut self.findings);
        super::note_capped_lines(&self.events, &mut self.findings);
        super::note_policy_sha(
            self.policy.as_deref(),
            self.policy_sha.as_deref(),
            policies,
            &mut self.findings,
        );
        let unknown = std::mem::take(&mut self.unknown_tools);
        self.findings.extend(
            unknown
                .into_iter()
                .map(|name| Finding::UnknownTool { name }),
        );
        let decisions = self.decide(policies, layout);
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

    fn decide(&mut self, policies: &PolicySet, layout: &Layout) -> Vec<Option<PermissionDecision>> {
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

        let spawn = ReplaySpawn::of(
            layout,
            super::Recorded {
                run_id: &self.run_id,
                policy: &name,
                cwd: self.cwd.as_deref(),
                worktree: self.worktree.as_deref(),
                store_sha: self.store_sha.as_deref(),
                gate: &self.gate,
            },
            policies,
            Provider::Codex,
        );
        self.findings.extend(spawn.scope_limit());
        let replay = match Replay::compile(policies, &spawn.spawn(Provider::Codex)) {
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
            let verdict = match replay.decide(&tool, &arguments, spawn.cwd()) {
                Ok(verdict) => verdict,
                Err(err) => {
                    self.findings.push(Finding::Limit(format!(
                        "the gate would not answer for step {call_id} ({tool}): {err:#}"
                    )));
                    decisions.push(None);
                    continue;
                }
            };
            let result = results.get(&call_id);
            let refusal = result.and_then(|(status, output)| refusal_in(*status, output));

            let derived = if !verdict.allowed {
                match &refusal {
                    None => match result {
                        Some((ResultStatus::Ok, _)) => {
                            match spawn.cannot_back(&format!(
                                "the policy denies step {call_id} ({tool}) and nothing in the \
                                 record refused it"
                            )) {
                                Some(limit) => self.findings.push(limit),
                                None => self.findings.push(Finding::GateLeak {
                                    call_id: call_id.clone(),
                                    tool: tool.clone(),
                                    reason: verdict.reason.clone(),
                                }),
                            }
                        }
                        Some(_) => self.findings.push(Finding::Limit(format!(
                            "step {call_id} ({tool}) failed with an error this provider spells \
                             the same way it spells a refusal, so whether the gate refused a call \
                             the policy denies is unknown"
                        ))),
                        None => self.findings.push(Finding::Limit(format!(
                            "step {call_id} ({tool}) is the last thing the record says, so \
                             whether the gate refused a call the policy denies is unknown"
                        ))),
                    },
                    Some(refusal) if refusal.by == Decider::External => {
                        if let Some(code) = hydra_voice(&refusal.reason) {
                            if policy_moved(self.policy_sha.as_deref(), policies) {
                                self.findings.push(Finding::Limit(format!(
                                    "the gate allowed step {call_id} ({tool}) and the derivation \
                                     denies it, under a policy_sha that is no longer today's, so \
                                     the two answered about two policies"
                                )));
                            } else {
                                match spawn.cannot_back(&format!(
                                    "the gate allowed step {call_id} ({tool}) and the derivation \
                                     denies it"
                                )) {
                                    Some(limit) => self.findings.push(limit),
                                    None => self.findings.push(Finding::Disagreement {
                                        call_id: call_id.clone(),
                                        derived: Decision::Deny,
                                        derived_code: verdict.code().map(str::to_string),
                                        gate_said: Decision::Allow,
                                        gate_code: Some(code.to_string()),
                                    }),
                                }
                            }
                        } else {
                            self.findings.push(Finding::Ungated {
                                call_id: call_id.clone(),
                                tool: tool.clone(),
                            });
                        }
                    }
                    Some(_) => {}
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
                    Some(refusal) if refusal.by == Decider::External => PermissionDecision {
                        seq,
                        call_id: call_id.clone(),
                        decision: Decision::Deny,
                        decider: Decider::External,
                        reason: refusal.reason,
                    },
                    Some(refusal) => {
                        if policy_moved(self.policy_sha.as_deref(), policies) {
                            self.findings.push(Finding::Limit(format!(
                                "the gate denied step {call_id} ({tool}) and the derivation \
                                 allows it, under a policy_sha that is no longer today's, so the \
                                 two answered about two policies"
                            )));
                        } else if let Some(limit) = spawn.cannot_back(&format!(
                            "the gate denied step {call_id} ({tool}) and the derivation allows it"
                        )) {
                            self.findings.push(limit);
                        } else {
                            self.findings.push(Finding::Disagreement {
                                call_id: call_id.clone(),
                                derived: verdict_decision(true),
                                derived_code: verdict.code().map(str::to_string),
                                gate_said: Decision::Deny,
                                gate_code: hydra_voice(&refusal.reason).map(str::to_string),
                            });
                        }
                        PermissionDecision {
                            seq,
                            call_id: call_id.clone(),
                            decision: Decision::Allow,
                            decider: Decider::HydraPolicy,
                            reason: verdict.reason,
                        }
                    }
                    None => PermissionDecision {
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
}

fn item_id(item: &Value, seq: u64) -> String {
    item["id"]
        .as_str()
        .or_else(|| item["tool_use_id"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| seq.to_string())
}

fn classify(message: &str) -> ResultStatus {
    if hydra_voice(message).is_some() {
        return ResultStatus::Error;
    }
    ResultStatus::Unclassified
}

fn refusal_in(status: ResultStatus, output: &str) -> Option<Refusal> {
    if status != ResultStatus::Error {
        return None;
    }
    match hydra_voice(output) {
        Some("allow") => Some(Refusal {
            by: Decider::External,
            reason: output.to_string(),
        }),
        Some(_) => Some(Refusal {
            by: Decider::HydraPolicy,
            reason: output.to_string(),
        }),
        None => Some(Refusal {
            by: Decider::External,
            reason: output.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::Event;

    fn fixture_record(body: &str) -> String {
        format!(
            r#"{{"seq":0,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","hydra":{{"type":"run_start","policy":"standard","policy_sha":"0000000000000000000000000000000000000000000000000000000000000000","cwd":"/tmp/wt","worktree":{{"path":"/tmp/wt"}}}}}}
{body}
{{"seq":99,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","hydra":{{"type":"run_end","exit_code":0}}}}"#
        )
    }

    #[test]
    fn a_command_execution_becomes_a_bash_call_and_result() {
        let text = fixture_record(
            r#"{"seq":1,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","raw":{"type":"item.completed","item":{"type":"command_execution","id":"c1","command":"echo hi","output":"hi\n","exit_code":0}}}"#,
        );
        let layout = crate::paths::Layout::at("/tmp/hydra-codex-norm");
        let policies = crate::policy::PolicySet::discover().unwrap();
        let run = normalize(&text, &policies, &layout).unwrap();
        let calls: Vec<_> = run
            .events
            .iter()
            .filter_map(|event| match event {
                Event::ToolCall(call) => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "Bash");
        assert_eq!(calls[0].call_id, "c1");
        assert!(
            run.tool_results()
                .next()
                .is_some_and(|r| r.status == ResultStatus::Ok)
        );
    }

    #[test]
    fn an_agent_message_and_reasoning_become_message_and_thinking() {
        let text = fixture_record(
            r#"{"seq":1,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","raw":{"type":"item.completed","item":{"type":"reasoning","text":"thinking"}}}
{"seq":2,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","raw":{"type":"item.completed","item":{"type":"agent_message","text":"hello"}}}"#,
        );
        let layout = crate::paths::Layout::at("/tmp/hydra-codex-norm-2");
        let policies = crate::policy::PolicySet::discover().unwrap();
        let run = normalize(&text, &policies, &layout).unwrap();
        assert!(run.messages().any(|message| message.text == "hello"));
        assert!(
            run.events
                .iter()
                .any(|event| matches!(event, Event::Thinking(_)))
        );
    }

    #[test]
    fn turn_completed_usage_is_summed_into_one_usage_event() {
        let text = fixture_record(
            r#"{"seq":1,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","raw":{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":4,"reasoning_tokens":2}}}"#,
        );
        let layout = crate::paths::Layout::at("/tmp/hydra-codex-norm-3");
        let policies = crate::policy::PolicySet::discover().unwrap();
        let run = normalize(&text, &policies, &layout).unwrap();
        let usage = run
            .events
            .iter()
            .find_map(|event| match event {
                Event::Usage(usage) => Some(usage),
                _ => None,
            })
            .unwrap();
        assert_eq!(usage.counts.as_ref().unwrap().input_tokens, 10);
        assert_eq!(usage.counts.as_ref().unwrap().output_tokens, 4);
        assert_eq!(usage.counts.as_ref().unwrap().thinking_tokens, Some(2));
    }

    #[test]
    fn a_hook_refusal_in_output_is_classified_as_an_error() {
        let blocked = "hydra/capability: policy read-only does not allow shell (Bash).";
        assert_eq!(classify(blocked), ResultStatus::Error);
        assert!(
            refusal_in(ResultStatus::Error, blocked).is_some_and(|r| r.by == Decider::HydraPolicy)
        );
    }
}
