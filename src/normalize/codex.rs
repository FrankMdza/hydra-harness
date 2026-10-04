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
//! **Field names measured 2026-09-21 on 0.154.0, run 01a0c223-4432.** The
//! session id is `thread_id` on a `thread.started` frame; `turn.completed`
//! carries `usage` with `input_tokens`, `cached_input_tokens`,
//! `cache_write_input_tokens`, `output_tokens` and `reasoning_output_tokens`;
//! the stream never names the model; and the `--dangerously-bypass-hook-trust`
//! warning arrives as an `item.completed` of type `error`, twice per run,
//! which is noise and not a finding. The tool item shapes are measured on run
//! 01a0c223-9ec2, the four-step gate probe.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use serde_json::{Value, json};

use super::{
    Counts, Decider, Decision, Event, Finding, Message, PermissionDecision, RecordedStart,
    ReplaySpawn, ResultStatus, Role, Run, RunEnd, RunStart, Thinking, ToolCall, ToolResult, Usage,
    UsageSource, hydra_voice, policy_moved, splice, verdict_decision,
};
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::policy::replay::Replay;
use crate::policy::table::{codex_capability, codex_is_unhooked};
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
    start: RecordedStart,
    events: Vec<Event>,
    findings: Vec<Finding>,
    result: Option<Value>,
    usage_emitted: bool,
    noise: Vec<String>,
    counts: Option<Counts>,
    /// Tool calls keyed by the id the stream names, for pairing results.
    calls: HashMap<String, (u64, String, Value)>,
    unknown_tools: Vec<String>,
    unread_frames: Vec<String>,
    /// Calls whose command ran and exited nonzero: an `error` result that is
    /// not a refusal. Build ticket 259.
    ran_and_failed: HashSet<String>,
}

pub fn normalize(text: &str, policies: &PolicySet, layout: &Layout) -> Result<Run> {
    walk(text).finish(policies, layout)
}

/// The usage the record carries so far, off the same walk and without the
/// gate replay [`normalize`] finishes with. Build ticket 172.
pub fn usage_so_far(text: &str) -> Option<Usage> {
    events_so_far(text)
        .into_iter()
        .find_map(|event| match event {
            Event::Usage(usage) => Some(usage),
            _ => None,
        })
}

/// Every event the walk derives, in file order, without the gate replay, so
/// no `permission_decision` among them. Build ticket 178.
pub fn events_so_far(text: &str) -> Vec<Event> {
    walk(text).events
}

fn walk(text: &str) -> Walk {
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
    walk
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
                self.stderr_line(seq, text);
            }
        }
    }

    /// A denied call leaves no `item.*` frame at all, measured 2026-09-21 on
    /// 0.154.0: the only trace is one stderr line,
    /// `... ERROR codex_core::tools::router: error=Command blocked by PreToolUse
    /// hook: <reason>. Command: <command>`. The reader turns that line into the
    /// call, an error result carrying the reason, and later a decision, so a
    /// refused run reads `gated` the way it does on the other three providers.
    /// Everything else on stderr is noise.
    fn stderr_line(&mut self, seq: u64, text: &str) {
        const MARK: &str = "Command blocked by PreToolUse hook: ";
        const SPLIT: &str = ". Command: ";
        let Some(start) = text.find(MARK) else {
            self.noise.push(text.to_string());
            return;
        };
        let rest = &text[start + MARK.len()..];
        let (reason, command) = match rest.rfind(SPLIT) {
            Some(at) => (&rest[..at], &rest[at + SPLIT.len()..]),
            None => (rest, ""),
        };
        let name = if command.starts_with("*** Begin Patch") {
            "apply_patch".to_string()
        } else {
            "Bash".to_string()
        };
        let call_id = format!("blocked-{seq}");
        let capability = codex_capability(&name);
        self.events.push(Event::ToolCall(ToolCall {
            seq,
            call_id: call_id.clone(),
            name,
            arguments: json!({ "command": command }),
            capability,
        }));
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            status: ResultStatus::Error,
            output: reason.to_string(),
            truncated: false,
        }));
    }

    fn hydra_line(&mut self, seq: u64, body: &Value) {
        match body["type"].as_str() {
            Some("run_start") => {
                self.start = RecordedStart::of(body);
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
            // Measured on record 01a0cfdb-e5c3, 0.154.0, 2026-09-23: a turn the
            // provider could not finish ends on `turn.failed` carrying
            // `error.message`, and the same sentence arrives just before it as
            // a top-level `error` frame. The frame is this run's terminator, so
            // it is kept as the result the way `turn.completed` is, and the
            // outcome derivation reads its message.
            Some("turn.failed") => {
                self.result = Some(raw.clone());
                self.add_counts(&raw["usage"]);
                self.emit_usage();
            }
            Some("error") => {
                if let Some(message) = raw["message"].as_str() {
                    self.noise.push(message.to_string());
                }
            }
            // Read and dropped on purpose. `item.started` is the half of a
            // tool item whose `item.completed` carries everything, the session
            // id on `thread.started` has no field in contract 7.2 yet, and
            // `turn.started` says nothing a later frame does not.
            Some("thread.started" | "turn.started" | "item.started" | "item.updated") => {}
            // Build ticket 255: an interactive run's body is its session file.
            // `session_meta`, `turn_context` and `world_state` name settings,
            // and the session id on the first has no field in 7.2 yet.
            Some(
                kind @ ("response_item" | "event_msg" | "token_usage_record" | "session_meta"
                | "turn_context" | "world_state"),
            ) => self.session_line(seq, raw, kind),
            Some(other) => self.note_unread_frame(other),
            None => self.note_unread_frame("(none)"),
        }
    }

    /// One finding per frame type this reader drops, so a shape the provider
    /// adds later is named in the record's reading rather than lost in silence.
    fn note_unread_frame(&mut self, kind: &str) {
        if self.unread_frames.iter().any(|known| known == kind) {
            return;
        }
        self.unread_frames.push(kind.to_string());
        self.findings.push(Finding::Limit(format!(
            "a frame of type {kind} is one this reader does not know, so nothing was derived \
             from it"
        )));
    }

    fn item_completed(&mut self, seq: u64, item: &Value) {
        match item["type"].as_str() {
            Some("command_execution") => self.command_execution(seq, item),
            Some("mcp_tool_call") => self.mcp_tool_call(seq, item),
            Some("web_search") => self.web_search(seq, item),
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
            // The provider's own notices, the hook-trust warning among them.
            // Measured as noise: nothing in contract 7.2 is derived from one.
            Some("error") => {
                if let Some(message) = item["message"].as_str() {
                    self.noise.push(message.to_string());
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
        let command = match item["command"].as_str() {
            Some(text) => Value::String(unwrap_shell(text)),
            None => item["command"].clone(),
        };
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
        // `aggregated_output` is the measured name; `output` was ticket 58's guess.
        let output = item["aggregated_output"]
            .as_str()
            .or_else(|| item["output"].as_str())
            .unwrap_or_default();
        // Build ticket 259: a nonzero exit is the command failing, so `error`.
        // It is not a refusal, because a refused call leaves no item at all
        // (see `stderr_line`), unless the output carries Hydra's own voice.
        let (status, message) = match item["exit_code"].as_i64() {
            Some(0) => (ResultStatus::Ok, output.to_string()),
            Some(exit_code) => {
                if hydra_voice(output).is_none() {
                    self.ran_and_failed.insert(item_id(item, seq));
                }
                (
                    ResultStatus::Error,
                    if output.is_empty() {
                        format!("exit code {exit_code}")
                    } else {
                        output.to_string()
                    },
                )
            }
            None => (classify(output), output.to_string()),
        };
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id: item_id(item, seq),
            status,
            output: message,
            truncated: false,
        }));
    }

    /// Hosted web search never reaches the hook on 0.154.0. The item shape is
    /// not measured on a Hydra run yet; `query` is the field name map ticket 58
    /// read off the hooks reference.
    fn web_search(&mut self, seq: u64, item: &Value) {
        let call_id = item_id(item, seq);
        let name = "WebSearch".to_string();
        let capability = codex_capability(&name);
        let arguments = match item.get("query") {
            Some(query) => json!({ "query": query }),
            None => json!({}),
        };
        self.events.push(Event::ToolCall(ToolCall {
            seq,
            call_id: call_id.clone(),
            name,
            arguments,
            capability,
        }));
        let output = item["results"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| item["results"].to_string());
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            status: ResultStatus::Ok,
            output,
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

    /// Measured 2026-09-21: an applied patch is an `item.completed` of type
    /// `file_change` carrying `changes: [{path, kind}]` and a `status`, never
    /// the patch text. The hook saw the text under `tool_input.command`; the
    /// stream keeps the paths. Both go on the call so a replay has what the
    /// gate had and a reader has what the run changed. The gate's replay reads
    /// `changes[].path` the way it reads patch headers from `command`.
    fn apply_patch(&mut self, seq: u64, item: &Value) {
        let call_id = item_id(item, seq);
        let name = "apply_patch".to_string();
        let capability = codex_capability(&name);
        let mut arguments = json!({ "changes": item["changes"].clone() });
        if let Some(command) = item.get("command") {
            arguments["command"] = command.clone();
        }
        self.events.push(Event::ToolCall(ToolCall {
            seq,
            call_id: call_id.clone(),
            name,
            arguments,
            capability,
        }));
        let output = item["summary"]
            .as_str()
            .or_else(|| item["output"].as_str())
            .map(str::to_string)
            .unwrap_or_else(|| item["changes"].to_string());
        let status = if item["status"].as_str() == Some("failed") {
            ResultStatus::Unclassified
        } else {
            ResultStatus::Ok
        };
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            status,
            output,
            truncated: false,
        }));
    }

    fn add_counts(&mut self, usage: &Value) {
        let Some(counts) = counts_of(usage) else {
            return;
        };
        let running = self.counts.get_or_insert_with(Counts::default);
        running.input_tokens += counts.input_tokens;
        running.output_tokens += counts.output_tokens;
        running.cache_read_tokens += counts.cache_read_tokens;
        if let Some(written) = counts.cache_write_tokens {
            running.cache_write_tokens =
                Some(running.cache_write_tokens.unwrap_or_default() + written);
        }
        if let Some(thinking) = counts.thinking_tokens {
            running.thinking_tokens = Some(running.thinking_tokens.unwrap_or_default() + thinking);
        }
    }

    /// A session file's usage is a running total rather than one turn's, so
    /// the latest one stands for the session instead of adding to it.
    fn set_counts(&mut self, total: &Value) {
        if let Some(counts) = counts_of(total) {
            self.counts = Some(counts);
        }
    }

    /// One line of an interactive session file, harvested whole, build
    /// ticket 255. `provider::codex` holds the measured shapes.
    fn session_line(&mut self, seq: u64, raw: &Value, kind: &str) {
        let payload = &raw["payload"];
        match kind {
            "response_item" => self.response_item(seq, payload),
            "event_msg" => match payload["type"].as_str() {
                Some("token_count") => self.set_counts(&payload["info"]["total_token_usage"]),
                // Build ticket 259: a turn the provider would not finish ends
                // on `task_complete` with `error`, which is the session file's
                // `turn.failed`. Kept as the result only while the latest turn
                // ended on one, so a session that went on past an error does
                // not read as stopped by it.
                Some("task_complete") => {
                    self.result = (!payload["error"].is_null()).then(|| raw.clone());
                }
                _ => {}
            },
            "token_usage_record" => self.set_counts(&payload["thread_token_usage"]),
            _ => {}
        }
    }

    fn response_item(&mut self, seq: u64, item: &Value) {
        match item["type"].as_str() {
            Some("message") => {
                // `developer` is the TUI's own instructions, not a turn.
                let role = match item["role"].as_str() {
                    Some("user") => Role::User,
                    Some("assistant") => Role::Assistant,
                    _ => return,
                };
                let text = joined_text(&item["content"]);
                if !text.is_empty() {
                    self.events.push(Event::Message(Message {
                        seq,
                        role,
                        text,
                        block: seq as usize,
                    }));
                }
            }
            Some("reasoning") => {
                let text = joined_text(&item["summary"]);
                if !text.is_empty() {
                    self.events.push(Event::Thinking(Thinking {
                        seq,
                        text,
                        signature: None,
                        block: seq as usize,
                    }));
                }
            }
            Some("custom_tool_call" | "function_call") => {
                let call_id = item["call_id"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| seq.to_string());
                let name = item["name"].as_str().unwrap_or_default().to_string();
                let arguments = match (item.get("input"), item["arguments"].as_str()) {
                    (Some(input), _) => json!({ "input": input }),
                    (None, Some(text)) => {
                        serde_json::from_str(text).unwrap_or_else(|_| json!({ "arguments": text }))
                    }
                    (None, None) => item["arguments"].clone(),
                };
                let capability = codex_capability(&name);
                if capability.is_none() {
                    self.note_unknown(&name);
                }
                self.events.push(Event::ToolCall(ToolCall {
                    seq,
                    call_id,
                    name,
                    arguments,
                    capability,
                }));
            }
            Some("custom_tool_call_output" | "function_call_output") => {
                let output = match &item["output"] {
                    Value::String(text) => text.clone(),
                    list @ Value::Array(_) => joined_text(list),
                    other => other.to_string(),
                };
                self.events.push(Event::ToolResult(ToolResult {
                    seq,
                    call_id: item["call_id"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| seq.to_string()),
                    // The output is the script's text, with no exit status
                    // this reader can trust. Build ticket 259 looked: the
                    // `CommandExecution` items beside it carry `exit_code`,
                    // but no id ties one to a call, a script runs none or
                    // several, and its own `Script completed` or `Script
                    // failed` disagrees with their exits on 82 of 1,440 calls.
                    status: classify(&output),
                    output,
                    truncated: false,
                }));
            }
            Some(other) => self.note_unread_frame(&format!("response_item {other}")),
            None => {}
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
            self.start.policy.as_deref(),
            self.start.policy_sha.as_deref(),
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
            policy: self.start.policy,
            events,
            findings: self.findings,
            result: self.result,
            noise: self.noise,
        })
    }

    fn decide(&mut self, policies: &PolicySet, layout: &Layout) -> Vec<Option<PermissionDecision>> {
        let mut decisions: Vec<Option<PermissionDecision>> = Vec::new();
        let Some(name) = self.start.policy.clone() else {
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
            self.start.recorded(&self.run_id, &name),
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

        let policy = policies.get(&name).expect("policy name matched run_start");
        let results = self.results_by_call();
        let replay_inputs: Vec<(&str, &Value, &Path)> = calls
            .iter()
            .filter(|(_, _, tool, _)| !codex_is_unhooked(tool))
            .map(|(_, _, tool, arguments)| (tool.as_str(), arguments, spawn.cwd()))
            .collect();
        let mut replay_verdicts = replay.decide_all(&replay_inputs).into_iter();
        for (seq, call_id, tool, _arguments) in calls {
            if codex_is_unhooked(&tool) {
                let capability = codex_capability(&tool).expect("unhooked row");
                let allowed = policy.allows_capability(capability);
                let result = results.get(&call_id);
                let refusal = self.refusal_of(&call_id, result);
                if let Some(refusal) = &refusal
                    && let Some(code) = hydra_voice(&refusal.reason)
                    && code != "allow"
                {
                    self.findings.push(Finding::Disagreement {
                        call_id: call_id.clone(),
                        derived: if allowed {
                            Decision::Allow
                        } else {
                            Decision::Deny
                        },
                        derived_code: Some(if allowed {
                            "unhooked".into()
                        } else {
                            "capability".into()
                        }),
                        gate_said: Decision::Deny,
                        gate_code: Some(code.to_string()),
                    });
                }
                // The decision is what happened: the provider ran the call. A
                // policy that denies the capability is a finding beside it, not
                // a `Deny`, which would say the call was refused when it ran.
                if !allowed {
                    self.findings.push(Finding::UnhookedDenied {
                        call_id: call_id.clone(),
                        tool: tool.clone(),
                        reason: policy.capability_deny_reason(capability, &tool),
                    });
                }
                decisions.push(Some(PermissionDecision {
                    seq,
                    call_id: call_id.clone(),
                    decision: Decision::Allow,
                    decider: Decider::Provider,
                    reason: format!(
                        "hydra/unhooked: {tool} is run by codex without asking the hook"
                    ),
                }));
                continue;
            }
            let verdict = match replay_verdicts.next().expect("one replay answer per call") {
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
            let refusal = self.refusal_of(&call_id, result);

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
                            if policy_moved(self.start.policy_sha.as_deref(), policies) {
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
                        if policy_moved(self.start.policy_sha.as_deref(), policies) {
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

    /// The refusal a call's result shows, if any. A command that ran and
    /// exited nonzero is a failure, never a refusal.
    fn refusal_of(
        &self,
        call_id: &str,
        result: Option<&(ResultStatus, String)>,
    ) -> Option<Refusal> {
        if self.ran_and_failed.contains(call_id) {
            return None;
        }
        result.and_then(|(status, output)| refusal_in(*status, output))
    }

    /// Not [`super::results_by_call`], which drops a result with an empty id.
    /// An item whose `id` is `""` gets that id from [`item_id`], and its call
    /// joins its result here. Build 279 kept that join rather than decide it.
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

/// The command the hook judged, out of the one the stream records.
///
/// Measured 2026-09-21 on 0.154.0, run 01a0c231-6583: the hook payload carries
/// `tool_input.command` as the model wrote it, `cat ~/hydra/runs/nothing.jsonl`,
/// and the `command_execution` item carries what the provider ran,
/// `/bin/zsh -lc 'cat ~/hydra/runs/nothing.jsonl'`, or `/bin/zsh -lc ls` when
/// one word needs no quoting. A replay has to see the former: a protected path
/// inside a quoted token is the shape build ticket 56 documented the token
/// pass does not see, and on run 01a0c223-9ec2 the wrapped form derived allow
/// for a call the live gate refuses. Anything not in that wrapper is returned
/// as it came.
fn unwrap_shell(command: &str) -> String {
    const WRAPPERS: &[&str] = &["/bin/zsh -lc ", "/bin/bash -lc ", "/bin/sh -lc "];
    let Some(rest) = WRAPPERS
        .iter()
        .find_map(|wrapper| command.strip_prefix(wrapper))
    else {
        return command.to_string();
    };
    let Some(inner) = rest
        .strip_prefix('\'')
        .and_then(|inner| inner.strip_suffix('\''))
    else {
        return rest.to_string();
    };
    // POSIX single quoting: the only escape is `'\''`.
    inner.replace("'\\''", "'")
}

fn item_id(item: &Value, seq: u64) -> String {
    item["id"]
        .as_str()
        .or_else(|| item["tool_use_id"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| seq.to_string())
}

/// The usage object both `codex` shapes name, mapped to section 9's counts.
/// `None` when it names neither input nor output, which is `info: null`.
fn counts_of(usage: &Value) -> Option<Counts> {
    let usage = usage.as_object()?;
    let read = |field: &str| usage.get(field).and_then(Value::as_u64);
    if read("input_tokens").is_none() && read("output_tokens").is_none() {
        return None;
    }
    // Measured names first, the ticket-58 guesses as fallbacks.
    let cache_read_tokens = read("cached_input_tokens")
        .or_else(|| read("cache_read_tokens"))
        .unwrap_or_default();
    // Build ticket 259: `total_tokens` is `input_tokens` plus `output_tokens`,
    // so the cache sits inside `input_tokens`, and contract 9.1 counts input
    // net of it on every provider.
    Some(Counts {
        input_tokens: read("input_tokens")
            .unwrap_or_default()
            .saturating_sub(cache_read_tokens),
        output_tokens: read("output_tokens").unwrap_or_default(),
        cache_read_tokens,
        cache_write_tokens: read("cache_write_input_tokens"),
        thinking_tokens: read("reasoning_output_tokens")
            .or_else(|| read("reasoning_tokens"))
            .or_else(|| read("thinking_tokens")),
    })
}

/// The `text` of every part in a session file's content or summary list.
fn joined_text(parts: &Value) -> String {
    parts
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| part["text"].as_str())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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
            r#"{"seq":1,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","raw":{"type":"turn.completed","usage":{"input_tokens":10,"cached_input_tokens":6,"cache_write_input_tokens":1,"output_tokens":4,"reasoning_output_tokens":2}}}"#,
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
        // Net of the six cached, build ticket 259.
        assert_eq!(usage.counts.as_ref().unwrap().input_tokens, 4);
        assert_eq!(usage.counts.as_ref().unwrap().output_tokens, 4);
        assert_eq!(usage.counts.as_ref().unwrap().thinking_tokens, Some(2));
        assert_eq!(usage.counts.as_ref().unwrap().cache_read_tokens, 6);
        assert_eq!(usage.counts.as_ref().unwrap().cache_write_tokens, Some(1));
    }

    #[test]
    fn a_blocked_call_is_read_off_the_stderr_line_it_leaves() {
        let text = fixture_record(
            r#"{"seq":1,"run_id":"01a0bcdb-e6ed-7180-8a96-3674b960cbc8","provider":"codex","raw":"2026-09-21T04:15:28.638480Z ERROR codex_core::tools::router: error=Command blocked by PreToolUse hook: hydra/protected-path: the command names /users/user/hydra/runs, denied under every policy by harness spec 7.5. Command: cat ~/hydra/runs/nothing.jsonl"}"#,
        );
        let layout = crate::paths::Layout::at("/tmp/hydra-codex-norm-4");
        let policies = crate::policy::PolicySet::discover().unwrap();
        let run = normalize(&text, &policies, &layout).unwrap();
        let call = run
            .events
            .iter()
            .find_map(|event| match event {
                Event::ToolCall(call) => Some(call),
                _ => None,
            })
            .unwrap();
        assert_eq!(call.name, "Bash");
        assert_eq!(call.arguments["command"], "cat ~/hydra/runs/nothing.jsonl");
        let result = run.tool_results().next().unwrap();
        assert_eq!(result.status, ResultStatus::Error);
        assert!(
            result.output.starts_with("hydra/protected-path"),
            "{}",
            result.output
        );
        assert!(run.noise.is_empty(), "{:?}", run.noise);
    }

    #[test]
    fn the_shell_wrapper_is_taken_off_so_the_replay_sees_what_the_hook_saw() {
        assert_eq!(
            unwrap_shell("/bin/zsh -lc 'cat ~/hydra/runs/nothing.jsonl'"),
            "cat ~/hydra/runs/nothing.jsonl"
        );
        assert_eq!(unwrap_shell("/bin/zsh -lc ls"), "ls");
        assert_eq!(
            unwrap_shell("/bin/zsh -lc 'echo '\\''quoted'\\'' x'"),
            "echo 'quoted' x"
        );
        assert_eq!(unwrap_shell("cat a.txt"), "cat a.txt");
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
