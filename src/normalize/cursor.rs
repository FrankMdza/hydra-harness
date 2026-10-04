//! The `cursor-agent` adapter: the second column of contract 7.2's table,
//! measured on 2026.09.02-c22c1a3 on 2026-09-06.
//!
//! One pass over the file builds the events that come straight off a frame, and
//! a second derives the permission decisions, which need the whole run before
//! any of them can be answered. Nothing here writes.
//!
//! **Two sources say what happened to a tool call on this provider, not three.**
//! `claude` puts its hook answers on the stream as frames of their own; Cursor
//! has no such frame. What it has is the tool result, which carries every hook's
//! message concatenated into one reason string when a hook blocked the call.
//! 7.7 says that string cannot carry attribution, and it does not: the decider
//! is derived by re-evaluating the run's own policy, as on `claude`. What the
//! string can carry is Hydra's own voice, because the gate writes a `hydra/<code>:`
//! prefix on every answer and no other hook does, and that is the one thing
//! this file reads out of it: whether Hydra's gate spoke, and which code it
//! spoke.
//!
//! **A stream key and a hook name are two spellings of one tool.** The stream
//! says `editToolCall` with `path` and `streamContent`; the hook that gated it
//! saw `Write` with `file_path` and `content`. Contract 7.7 re-evaluates the
//! policy against the call, and the gate reads the hook's spelling, so
//! [`as_hook_call`] rebuilds it from the stream's `args` the way the bundle's
//! own `createToolInput` does. The rebuilt input is what `tool_call.arguments`
//! carries; the stream's own object survives in `raw`.

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
use crate::policy::table::{cursor_capability, cursor_is_unhooked};
use crate::provider::Provider;

/// The `result.usage` keys section 9 maps. An object with none of them says
/// nothing, whatever else it holds.
const COUNT_FIELDS: [&str; 4] = [
    "inputTokens",
    "outputTokens",
    "cacheReadTokens",
    "cacheWriteTokens",
];

/// The sentence Cursor appends to every reason a hook blocked with, measured
/// on three tickets' probes and again on 2026-09-06. It is the provider's own
/// fixed marker rather than a hook's prose, and it is what tells a hook block
/// from a tool that failed on its own.
const BLOCKED_BY_HOOK: &str = "Agent note: Do not suggest workarounds to the blocked tool.";

/// A refusal the record shows, as against one the policy implies.
struct Refusal {
    by: Decider,
    reason: String,
}

/// What one `tool_call` frame's oneof key says: the name the hook saw and the
/// input the hook saw, rebuilt from the stream's `args`.
///
/// **Six of these were measured on a real run and the rest are read off the
/// bundle.** The measured six are the first six rows. A key this table does not
/// know keeps its own spelling as the name and its `args` as the arguments,
/// and carries no capability, which 7.1 turns into a finding rather than a
/// shrug.
fn as_hook_call(kind: &str, args: &Value) -> Option<(String, Value)> {
    let path = || args["path"].clone();
    let pathed = |field: &str| json!({ "file_path": args[field] });
    Some(match kind {
        "readToolCall" => ("Read".into(), pathed("path")),
        "shellToolCall" => (
            "Shell".into(),
            json!({
                "command": args["command"],
                "cwd": args["workingDirectory"],
                "timeout": args["timeout"],
            }),
        ),
        // The write tool. Its stream key is `edit`, its hook name is `Write`,
        // and the content rides as `streamContent` on the stream where the
        // bundle's descriptor calls it `fileText`.
        "editToolCall" => (
            "Write".into(),
            json!({
                "file_path": path(),
                "content": args.get("streamContent").or_else(|| args.get("fileText")).cloned().unwrap_or(Value::Null),
            }),
        ),
        "deleteToolCall" => ("Delete".into(), pathed("path")),
        "grepToolCall" => (
            "Grep".into(),
            json!({ "pattern": args["pattern"], "file_path": args["path"] }),
        ),
        // The listing tool reaches the hook as `Grep` with an `output_mode`,
        // measured: `globPattern: "*"` arrived as `glob: "**/*"`, and the
        // directory is what the gate scopes on.
        "globToolCall" => (
            "Grep".into(),
            json!({
                "pattern": "",
                "file_path": args["targetDirectory"],
                "glob": args["globPattern"],
                "output_mode": "files_with_matches",
            }),
        ),
        // Read off the bundle's `createToolInput` table, unmeasured on a run.
        "lsToolCall" => ("List".into(), pathed("path")),
        // Map ticket 51: the provider runs these without asking the hook, so
        // the rebuilt input is the stream's own args. `paths` is a list;
        // `file_path` was the field the table used to pretend to scope.
        "readLintsToolCall" => ("ReadLints".into(), args.clone()),
        "getMcpToolsToolCall" => ("getMcpToolsToolCall".into(), args.clone()),
        "awaitToolCall" => ("awaitToolCall".into(), args.clone()),
        "webSearchToolCall" => ("WebSearch".into(), args.clone()),
        "webFetchToolCall" => ("Fetch".into(), json!({ "url": args["url"] })),
        "taskToolCall" => ("task".into(), args.clone()),
        "writeShellStdinToolCall" => ("WriteShellStdin".into(), args.clone()),
        "readMcpResourceToolCall" => ("FetchMcpResource".into(), args.clone()),
        "listMcpResourcesToolCall" => ("ListMcpResources".into(), args.clone()),
        "updateTodosToolCall" => ("updateTodos".into(), args.clone()),
        "createPlanToolCall" => ("createPlan".into(), args.clone()),
        "generateImageToolCall" => ("generateImage".into(), args.clone()),
        "askQuestionToolCall" => ("askQuestion".into(), args.clone()),
        // `MCP:<tool>` is the hook's spelling, read off the bundle; which field
        // of the stream's args holds the tool name is unmeasured, because no
        // server is configured on this machine.
        "mcpToolCall" => {
            let name = args["toolName"]
                .as_str()
                .or_else(|| args["name"].as_str())
                .unwrap_or_default();
            (format!("MCP:{name}"), args.clone())
        }
        _ => return None,
    })
}

/// A thinking block in progress: `thinking/delta` frames between one
/// `thinking/completed` and the next.
struct Thought {
    seq: u64,
    text: String,
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
    thought: Option<Thought>,
    thinking_blocks: usize,
    /// The calls whose `started` frame was seen, so a `completed` frame for a
    /// call the record never saw start still yields a call.
    started: HashSet<String>,
    /// Which oneof case each result came back as, by call id, so a `rejected`
    /// shell command is told from an `error` without re-reading the output.
    cases: HashMap<String, &'static str>,
    unknown_tools: Vec<String>,
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
                // The failure path lands here: the provider's `catch` prints one
                // sentence to stderr and writes no frame, and over a pty the
                // sentence is a line of the record (contract 3).
                self.noise.push(text.to_string());
            }
        }
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
                self.flush_thought();
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
        if kind != Some("thinking") {
            // A thought ends at its `completed` frame, and when the provider
            // moves on without one, at whatever came next.
            self.flush_thought();
        }
        match kind {
            Some("assistant") => self.blocks(seq, raw, Role::Assistant),
            Some("user") => self.blocks(seq, raw, Role::User),
            Some("thinking") => match raw["subtype"].as_str() {
                Some("delta") => {
                    let text = raw["text"].as_str().unwrap_or_default();
                    match &mut self.thought {
                        Some(thought) => thought.text.push_str(text),
                        None => {
                            self.thought = Some(Thought {
                                seq,
                                text: text.to_string(),
                            })
                        }
                    }
                }
                Some("completed") => self.flush_thought(),
                _ => {}
            },
            Some("tool_call") => self.tool_call(seq, raw),
            Some("result") => {
                self.result = Some(raw.clone());
                self.events.push(Event::Usage(usage_of(seq, raw)));
                self.usage_emitted = true;
            }
            // `system/init` carries the display name and the session id, which
            // the envelope already took, and no tool list: 7.1's table check
            // has nothing to read on this provider.
            _ => {}
        }
    }

    /// Contract 7.5: `thinking/delta` frames, no flag needed. One event per
    /// block rather than per delta, because a block is one thought and eight
    /// fragments of it are not eight thoughts. `seq` names the first delta and
    /// `block` counts blocks in the run, so the text is reproducible from `raw`
    /// by concatenating from that frame to the next `thinking/completed`.
    fn flush_thought(&mut self) {
        if let Some(thought) = self.thought.take() {
            self.events.push(Event::Thinking(Thinking {
                seq: thought.seq,
                text: thought.text,
                // Cursor signs nothing. Whether the text is the model's own
                // trace or a provider summary is not answerable from the CLI.
                signature: None,
                block: self.thinking_blocks,
            }));
            self.thinking_blocks += 1;
        }
    }

    /// The content blocks of one `assistant` or `user` frame, in order. Text
    /// only: this provider's messages carry no other block type.
    fn blocks(&mut self, seq: u64, raw: &Value, role: Role) {
        let Some(blocks) = raw["message"]["content"].as_array() else {
            return;
        };
        for (block, body) in blocks.iter().enumerate() {
            if body["type"].as_str() == Some("text") {
                self.events.push(Event::Message(Message {
                    seq,
                    role,
                    text: body["text"].as_str().unwrap_or_default().to_string(),
                    block,
                }));
            }
        }
    }

    /// One `tool_call` frame, `started` or `completed`.
    ///
    /// The call id is the frame's own `call_id`, with the object's `toolCallId`
    /// behind it; the two agreed on every frame measured. Contract 7.6 says the
    /// adapter mints the pairing on this provider, and it does: both events
    /// come from frames sharing one id, and the `completed` frame carries the
    /// result inside the same object that carried the args.
    fn tool_call(&mut self, seq: u64, raw: &Value) {
        let object = &raw["tool_call"];
        let call_id = raw["call_id"]
            .as_str()
            .or_else(|| object["toolCallId"].as_str())
            .unwrap_or_default()
            .to_string();
        let Some(kind) = object
            .as_object()
            .and_then(|fields| fields.keys().find(|key| key.ends_with("ToolCall")))
            .cloned()
        else {
            self.findings.push(Finding::Limit(format!(
                "the tool_call frame at seq {seq} names no tool, so nothing was derived from it"
            )));
            return;
        };
        let inner = &object[&kind];
        let args = inner.get("args").cloned().unwrap_or(Value::Null);
        let completed = raw["subtype"].as_str() == Some("completed");

        // A call is taken from its `started` frame, or from a `completed` frame
        // whose start the record never saw. A frame with no id at all cannot be
        // matched to anything, so it yields a call on `started`, a result on
        // `completed`, and a finding in `decide` saying what it lost.
        let seen = !call_id.is_empty() && !self.started.insert(call_id.clone());
        if !seen && (!completed || !call_id.is_empty()) {
            let (name, arguments) = match as_hook_call(&kind, &args) {
                Some(mapped) => mapped,
                None => (kind.clone(), args.clone()),
            };
            let capability = cursor_capability(&name);
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
        }

        if !completed {
            return;
        }
        let (status, case, output) = classify(inner.get("result"));
        if let Some(case) = case {
            self.cases.insert(call_id.clone(), case);
        }
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            status,
            output,
            // Cursor reports truncation inside its own result bodies and this
            // normalizer cuts nothing, so the marker is always false.
            truncated: false,
        }));
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
        // 7.8's never-a-bare-null, and on this provider the common case: the
        // failure path writes no `result` frame, so a run that ends with no
        // counts is `unavailable`, never `estimated`.
        self.events.push(Event::Usage(Usage {
            seq: None,
            source: UsageSource::Unavailable,
            estimator: None,
            counts: None,
        }));
        self.usage_emitted = true;
    }

    fn close(&mut self) {
        self.flush_thought();
        self.emit_missing_usage();
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
        // Contract 7.7's table, build ticket 55: this reader's counts are floors
        // and `claude`'s are not, so the record says which it holds.
        super::note_finding_bar(Some(Provider::CursorAgent), &mut self.findings);
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

    /// One decision per call, by re-evaluating the run's own policy (7.7).
    ///
    /// Absent, never empty, on a run with no policy: there is nothing to
    /// re-evaluate and `external` would presume a policy that could have allowed.
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

        // Every scope the gate was compiled with, recomputed from the record.
        // The three adapters build it the same way and differ in the provider.
        let spawn = ReplaySpawn::of(
            layout,
            self.start.recorded(&self.run_id, &name),
            policies,
            Provider::CursorAgent,
        );
        // Once per record, before any decision, so a reader sees what the
        // derivation stood in for ahead of what it derived.
        self.findings.extend(spawn.scope_limit());
        let replay = match Replay::compile(policies, &spawn.spawn(Provider::CursorAgent)) {
            Ok(replay) => replay,
            Err(err) => {
                self.findings.push(Finding::Limit(format!(
                    "policy {name} would not compile, so no decision was derived: {err:#}"
                )));
                return decisions;
            }
        };

        let policy = policies.get(&name).expect("policy name matched run_start");
        let results = super::results_by_call(&self.events);
        let replay_inputs: Vec<(&str, &Value, &Path)> = calls
            .iter()
            .filter(|(_, _, tool, _)| !cursor_is_unhooked(tool))
            .map(|(_, _, tool, arguments)| (tool.as_str(), arguments, spawn.cwd()))
            .collect();
        let mut replay_verdicts = replay.decide_all(&replay_inputs).into_iter();
        for (seq, call_id, tool, _arguments) in calls {
            if call_id.is_empty() {
                // Nothing to join a result to. The policy still answers for the
                // call, and this says what it could not be checked against.
                self.findings.push(Finding::Limit(format!(
                    "a {tool} call at seq {seq} carries no call id, so no result can be \
                     joined to it"
                )));
            }
            let result = results.get(&call_id);
            let refusal = result.and_then(|(status, output)| {
                refusal_in(*status, output, self.cases.get(&call_id).copied())
            });

            if cursor_is_unhooked(&tool) {
                // The provider ran this without asking the hook. The table
                // says so; the replay judges the row's real capability with no
                // path to check. If the hook ever does fire, the gate's own
                // answer is in the result and the two sources disagree.
                let capability = cursor_capability(&tool).expect("unhooked row");
                let allowed = policy.allows_capability(capability);
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
                        "hydra/unhooked: {tool} is run by cursor-agent without asking the hook"
                    ),
                }));
                continue;
            }

            let verdict = match replay_verdicts.next().expect("one replay answer per call") {
                Ok(verdict) => verdict,
                Err(err) => {
                    self.findings.push(Finding::Limit(format!(
                        "the gate would not answer for {call_id} ({tool}): {err:#}"
                    )));
                    decisions.push(None);
                    continue;
                }
            };

            let derived = if !verdict.allowed {
                match &refusal {
                    None => {
                        // The policy denies this call and the record shows no
                        // refusal. A result is the fate; no result is a record
                        // that stops here, which is not evidence either way.
                        if result.is_none() {
                            self.findings.push(Finding::Limit(format!(
                                "{call_id} ({tool}) is the last thing the record says, so \
                                 whether the gate refused a call the policy denies is unknown"
                            )));
                        } else if verdict.code() == Some("no-path")
                            && result.is_some_and(|(status, _)| *status == ResultStatus::Error)
                        {
                            // The replay saw no path, so it could not judge, and
                            // the provider answered an error of its own. Measured
                            // 2026-09-08 on run 01a0801d-7811: an `editToolCall`
                            // frame with no `args` at all and "Invalid arguments:
                            // old_string: Required" as the result. The tool did
                            // not run and the record cannot say what path it
                            // would have taken, which is a limit and not a leak.
                            self.findings.push(Finding::Limit(format!(
                                "{call_id} ({tool}) named no path the replay could judge and \
                                 the provider answered an error, so whether it ran is unknown"
                            )));
                        } else if let Some(limit) = spawn.cannot_back(&format!(
                            "the policy denies {call_id} ({tool}) and nothing in the record \
                             refused it"
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
                    // Something else refused. Cursor runs every hook and
                    // concatenates every message (research 30), so Hydra's
                    // answer is in the text whenever its hook ran: an `allow`
                    // there is the gate allowing at run time what the policy
                    // denies now, which is two sources disagreeing (7.7), and
                    // no voice at all is a gate that never ran. The config
                    // fails closed here, so the innocent reading of the second
                    // is narrower than on `claude`: a hook config the loader
                    // refused, which it does silently for a symlinked one.
                    Some(refusal) if refusal.by == Decider::External => {
                        match hydra_voice(&refusal.reason) {
                            // A moved policy directory means the gate and the
                            // derivation answered about two policies rather than
                            // one, so the difference is a limit and not a
                            // disagreement. See `policy_moved`.
                            Some(_) if policy_moved(self.start.policy_sha.as_deref(), policies) => {
                                self.findings.push(Finding::Limit(format!(
                                    "the gate allowed {call_id} ({tool}) and the \
                                     derivation denies it, under a policy_sha that is no \
                                     longer today's, so the two answered about two policies"
                                )));
                            }
                            Some(code) => {
                                match spawn.cannot_back(&format!(
                                    "the gate allowed {call_id} ({tool}) and the derivation \
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
                            None => self.findings.push(Finding::Ungated {
                                call_id: call_id.clone(),
                                tool: tool.clone(),
                            }),
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
                    // Refused, and the run's own policy would have allowed it.
                    Some(refusal) if refusal.by == Decider::External => PermissionDecision {
                        seq,
                        call_id: call_id.clone(),
                        decision: Decision::Deny,
                        decider: Decider::External,
                        reason: refusal.reason,
                    },
                    // Hydra's gate refused, in its own voice, what the policy
                    // now allows. Two sources disagree (7.7), and on this
                    // provider the result text is the only second source.
                    Some(refusal) => {
                        if policy_moved(self.start.policy_sha.as_deref(), policies) {
                            self.findings.push(Finding::Limit(format!(
                                "the gate denied {call_id} ({tool}) and the derivation \
                                 allows it, under a policy_sha that is no longer today's, \
                                 so the two answered about two policies"
                            )));
                        } else if let Some(limit) = spawn.cannot_back(&format!(
                            "the gate denied {call_id} ({tool}) and the derivation allows it"
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
                    // Nothing refused. No `Ungated` finding here, unlike
                    // `claude`: the stream carries no gate answer for an
                    // allowed call on this provider, so its absence says
                    // nothing, and `failClosed` means a hook that failed to
                    // run would have blocked rather than let the call through.
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
}

/// The three cases of a Cursor tool result, and what each means.
///
/// A protobuf oneof, confirmed in the bundle's descriptors by research 14 and
/// measured on 2026-09-06: `success`, `error` and `rejected`. A refusal is a
/// first-class case on a shell command and an `error` on a read or a write, so
/// both non-success cases are `Error` here and the case name is kept beside
/// the status for the refusal reading. Anything else is `Unclassified`, which
/// 7.6 makes a feature. The output is the result object's JSON, because the
/// bodies differ per tool and a reader wanting one field has `raw`.
fn classify(result: Option<&Value>) -> (ResultStatus, Option<&'static str>, String) {
    let Some(result) = result else {
        return (ResultStatus::Unclassified, None, String::new());
    };
    for (case, status) in [
        ("success", ResultStatus::Ok),
        ("error", ResultStatus::Error),
        ("rejected", ResultStatus::Error),
    ] {
        if let Some(body) = result.get(case) {
            return (status, Some(case), body.to_string());
        }
    }
    (ResultStatus::Unclassified, None, result.to_string())
}

/// What the record shows happened to a call, as against what the policy says
/// should have.
///
/// Three readings, in the order they are trusted. A gate code that is not
/// `allow` is Hydra's own refusal. `hydra/allow` in a refusal text means Hydra
/// allowed and another hook denied, because an allowing hook's message is
/// concatenated into the reason too (research 30). No code at all beside
/// Cursor's own marker, or a `rejected` case, is a refusal by something that is
/// not Hydra: the user's guards, or the shell runner's allowlist.
fn refusal_in(status: ResultStatus, output: &str, case: Option<&'static str>) -> Option<Refusal> {
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
        None if output.contains(BLOCKED_BY_HOOK) || case == Some("rejected") => Some(Refusal {
            by: Decider::External,
            reason: output.to_string(),
        }),
        None => None,
    }
}

/// Section 9's mapping, off `result.usage`.
///
/// **Never `inputTokens` as the prompt size.** The emitter computes
/// `max(inputTokens - cacheReadTokens - cacheWriteTokens, 0)`, so a warm cache
/// reports 0 input on a turn that consumed tens of thousands; the parts are
/// stored and `Counts::raw_input` puts them back together. No thinking count:
/// the print-mode writer drops `reasoningTokens` (research 25).
fn usage_of(seq: u64, result: &Value) -> Usage {
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
            input_tokens: usage["inputTokens"].as_u64().unwrap_or_default(),
            output_tokens: usage["outputTokens"].as_u64().unwrap_or_default(),
            cache_read_tokens: usage["cacheReadTokens"].as_u64().unwrap_or_default(),
            cache_write_tokens: usage["cacheWriteTokens"].as_u64(),
            thinking_tokens: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydras_voice_is_found_behind_cursors_own_prefix_and_nowhere_it_is_not() {
        // The shell shape, measured 2026-09-06 on the read probe and by
        // research 30 on the shell one.
        assert_eq!(
            hydra_voice(
                "File read was blocked by a hook: hydra/out-of-scope: policy read-only \
                 allows file_read inside worktree and this call is outside it.\n\nTo view or \
                 modify configured hooks, go to Cursor Settings > Hooks.\n\nAgent note: Do \
                 not suggest workarounds to the blocked tool."
            ),
            Some("out-of-scope")
        );
        // An allowing hook's message rides in the concatenation too.
        assert_eq!(
            hydra_voice(
                "Command execution was blocked by a hook: hydra/allow: policy standard \
                 allows shell.\n\n---\n\nHook blocked with message: 'git t30probe' is not \
                 a known subcommand.\n\nAgent note: Do not suggest workarounds to the \
                 blocked tool."
            ),
            Some("allow")
        );
        // A guard quoting a protected path is not the gate speaking.
        assert_eq!(
            hydra_voice("Hook blocked with message: 'cat ~/hydra/runs/x.jsonl': no."),
            None
        );
        assert_eq!(hydra_voice("File not found"), None);
    }

    #[test]
    fn the_stream_spelling_is_rebuilt_into_the_hooks_spelling() {
        // Measured pairs, 2026-09-06: the hook saw `Write` with `file_path` and
        // `content` for the `editToolCall` whose args were `path` and
        // `streamContent`.
        let (name, input) = as_hook_call(
            "editToolCall",
            &json!({ "path": "/ws/b.txt", "streamContent": "hello-b\n" }),
        )
        .unwrap();
        assert_eq!(name, "Write");
        assert_eq!(input["file_path"], "/ws/b.txt");
        assert_eq!(input["content"], "hello-b\n");

        let (name, input) = as_hook_call(
            "shellToolCall",
            &json!({ "command": "echo T09PROBE", "workingDirectory": "", "timeout": 30000 }),
        )
        .unwrap();
        assert_eq!(name, "Shell");
        assert_eq!(input["command"], "echo T09PROBE");
        assert_eq!(input["cwd"], "");

        let (name, input) = as_hook_call(
            "globToolCall",
            &json!({ "targetDirectory": "/ws", "globPattern": "*" }),
        )
        .unwrap();
        assert_eq!(name, "Grep", "the listing tool is gated as Grep");
        assert_eq!(input["file_path"], "/ws");

        let (name, input) = as_hook_call(
            "readLintsToolCall",
            &json!({ "paths": ["/ws/src/main.rs", "/etc/hosts"] }),
        )
        .unwrap();
        assert_eq!(name, "ReadLints");
        assert_eq!(input["paths"], json!(["/ws/src/main.rs", "/etc/hosts"]));

        let (name, input) = as_hook_call("awaitToolCall", &json!({ "taskId": "bg-1" })).unwrap();
        assert_eq!(name, "awaitToolCall");
        assert_eq!(input["taskId"], "bg-1");

        let (name, input) =
            as_hook_call("getMcpToolsToolCall", &json!({ "server": "hydra" })).unwrap();
        assert_eq!(name, "getMcpToolsToolCall");
        assert_eq!(input["server"], "hydra");

        let (name, input) = as_hook_call(
            "webSearchToolCall",
            &json!({ "searchTerm": "rust async", "toolCallId": "tool_1" }),
        )
        .unwrap();
        assert_eq!(name, "WebSearch");
        assert_eq!(input["searchTerm"], "rust async");

        assert!(as_hook_call("semSearchToolCall", &json!({})).is_none());
    }

    #[test]
    fn a_result_is_one_of_three_cases_and_a_fourth_is_unclassified() {
        let (status, case, _) = classify(Some(&json!({ "success": { "stdout": "x" } })));
        assert_eq!((status, case), (ResultStatus::Ok, Some("success")));
        let (status, case, output) = classify(Some(
            &json!({ "rejected": { "command": "exit 3", "reason": "" } }),
        ));
        assert_eq!((status, case), (ResultStatus::Error, Some("rejected")));
        assert!(output.contains("exit 3"));
        let (status, case, _) = classify(Some(&json!({ "somethingNew": {} })));
        assert_eq!((status, case), (ResultStatus::Unclassified, None));
        assert_eq!(classify(None).0, ResultStatus::Unclassified);
    }

    #[test]
    fn a_rejection_without_hydras_voice_is_somebody_elses() {
        let ours = refusal_in(
            ResultStatus::Error,
            "{\"errorMessage\":\"File read was blocked by a hook: hydra/capability: policy \
             judge does not allow file_read (Read).\"}",
            Some("error"),
        )
        .unwrap();
        assert_eq!(ours.by, Decider::HydraPolicy);

        let theirs = refusal_in(
            ResultStatus::Error,
            "{\"command\":\"exit 3\",\"reason\":\"\"}",
            Some("rejected"),
        )
        .unwrap();
        assert_eq!(theirs.by, Decider::External);

        assert!(
            refusal_in(
                ResultStatus::Error,
                "{\"errorMessage\":\"File not found\"}",
                Some("error")
            )
            .is_none(),
            "a tool that failed on its own is not a refusal"
        );
        assert!(refusal_in(ResultStatus::Ok, "anything", Some("success")).is_none());
    }
}
