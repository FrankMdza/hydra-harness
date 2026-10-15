//! The `agy` adapter: the third column of contract 7.2's table, measured on
//! 1.1.26 on 2026-09-06 and read back off a recorded 1.1.27 run the same evening.
//!
//! One pass over the file builds the events that come straight off a frame, and
//! a second derives the permission decisions, which need the whole run before
//! any of them can be answered. Nothing here writes.
//!
//! **One frame type carries almost everything.** `step_update` is the message,
//! the tool call, the tool result and the usage, told apart by `step_type` and
//! `state`. `init` carries the model, the session and the tool list; `result`
//! closes the run. So the shapes below are three fields of one object rather
//! than four frame types.
//!
//! **`text_delta` is a buffered flush wearing a streaming name** (contract 7.3).
//! The frames are concatenated over `step_index` to make one `message`, which is
//! reproducible from `raw` rather than trusted, and `Message::block` carries the
//! step index the concatenation ran over. Measured on 1.1.26: an
//! `agent_response` step that only called a tool carries no `text_delta` at all,
//! so a step with nothing to concatenate produces no message rather than an
//! empty one.
//!
//! **And the split is not at a line boundary.** The recorded 1.1.27 run split one
//! answer mid-word, inside a file URL, between `...204771091` and
//! `1a76/Cargo.toml...`. Research 20's single sample split at a newline, which
//! left open the reading that each frame is a whole line; it is not. Without the
//! concatenation that run reads as two messages, one of them a truncated path.
//!
//! **The pairing is `step_index`, because there is no id** (contract 7.6). An
//! `ACTIVE` tool step is the call and the `DONE` or `ERROR` step that shares its
//! index is the result. The index is the `call_id`, which is honest about what
//! it is: a position in one run, unique inside it and meaningless outside.
//!
//! **The stream's `tool_info.parameters` is a subset of what the hook saw**, and
//! it matters for one field. Measured on the same run: the hook payload for
//! `write_to_file` carried `TargetFile`, `CodeContent`, `Overwrite`,
//! `Description` and `ArtifactMetadata`, and the stream frame carried
//! `TargetFile` alone; `run_command`'s hook payload carried `CommandLine` and
//! `Cwd`, the frame carried `CommandLine`. Every path field the gate reads
//! survives, which is what makes contract 7.7's re-evaluation answerable, and
//! the `Cwd` that does not survive is the run's own checkout on every launch
//! Hydra makes, which is what [`super::agy`]'s caller passes as the cwd anyway.
//!
//! **`usage` is summed and no frame names it** (contract 9.2). A run's usage is
//! the sum of its own `step_update` counts, and `result.usage` is recorded and
//! not used for it, because a resumed run reports its predecessor's tokens on
//! that frame. So the single `Usage` event carries `seq: None` beside real
//! counts, which is a combination no other adapter produces: there is no one
//! frame to point at and pointing at the last step would name a frame that
//! reported a fraction of the total. On the fresh run measured, the sum and
//! `result.usage` agreed exactly on all five fields, which is the check
//! `tests/normalize.rs` keeps.
//!
//! **`thinking` is never text on this provider's stream** (contract 7.5). The
//! count arrives as `usage.thinking_tokens` and there is no reasoning text on
//! any frame, so the stream constructs no [`Thinking`] event and the count rides
//! on [`Usage`] where the other two providers' counts do.
//!
//! **A harvested interactive run is a transcript, not a stream** (build ticket
//! 267). Its lines carry `type` and `step_index` and no `event`, and a step's
//! tool output arrives on a line Hydra wrapped as `step_output`
//! ([`crate::provider::agy::step_output_line`]). The transcript does carry
//! `thinking` text, and no usage at all. A call has no id, so its `call_id` is
//! `<step>.<n>`, and an output answers the oldest unanswered call at or before
//! its step: order, as on the stream, and a guess where one step made several
//! calls, which a finding says.

use std::collections::{HashSet, VecDeque};
use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use super::{
    Counts, Decider, Decision, Event, Finding, Message, PermissionDecision, RecordedStart,
    ReplaySpawn, ResultStatus, Role, Run, RunEnd, RunStart, Thinking, ToolCall, ToolResult, Usage,
    UsageSource, hydra_voice, policy_moved, splice, verdict_decision,
};
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::policy::replay::Replay;
use crate::policy::table::agy_capability;
use crate::provider::Provider;

/// The sentence this provider puts in front of a hook's reason, measured by
/// ticket 26 on 1.1.24 and unchanged in the 1.1.26 strings. It is the provider's
/// own fixed marker rather than a hook's prose, and it is what tells a hook
/// block from a tool that failed on its own.
const DENIED_BY_HOOK: &str = "denied by pre-tool hook";

/// The generated id this provider names a hook by in a tool error,
/// `jsonhook__<name>_<event>_<group>_<handler>`. Contract 7.7's corroboration
/// table calls this provider's attribution strong, and this is the part of it a
/// reader can key on.
const HOOK_ID_PREFIX: &str = "jsonhook__";

/// A refusal the record shows, as against one the policy implies.
struct Refusal {
    by: Decider,
    reason: String,
}

/// One step's text, concatenated over the frames that share its index.
struct Response {
    seq: u64,
    step: usize,
    role: Role,
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
    /// The step being concatenated, contract 7.3.
    response: Option<Response>,
    /// The tool steps whose `ACTIVE` frame was seen, so a terminal frame for a
    /// step the record never saw start still yields a call.
    started: HashSet<usize>,
    /// The counts summed off every step that reported any, which is what a run's
    /// usage is on this provider (9.2).
    counts: Option<Counts>,
    /// The names the `init` frame offered, for 7.1's table check.
    offered: Vec<String>,
    unknown_tools: Vec<String>,
    /// Contract 9.2: a resumed run's `result` usage covers earlier turns.
    resumed: bool,
    /// A harvested transcript's calls with no output yet, oldest first, with
    /// the step that made each. Build ticket 267.
    unanswered: VecDeque<(String, u64)>,
    /// Transcript steps already read, since a step's line may be written again
    /// as its status moves.
    transcript_steps: HashSet<u64>,
    repeated_steps: usize,
    /// Transcript steps that made more than one call.
    multi_call_steps: usize,
    /// Step outputs no call was waiting for.
    orphan_outputs: usize,
    /// Transcript line types this reader does not know, each named once.
    unread_types: Vec<String>,
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
                self.frame(seq, raw, frame.get("event").and_then(Value::as_str));
            } else if let Some(text) = raw.as_str() {
                // Contract 3: a line the provider wrote that was not a frame.
                // On this provider that is where its stderr lands over the pty,
                // and `jetski: no output produced ...` is the one sentence
                // measured there.
                self.noise.push(text.to_string());
            }
        }
    }

    fn hydra_line(&mut self, seq: u64, body: &Value) {
        match body["type"].as_str() {
            Some("gate_refusal") => self.events.push(Event::GateRefusal(super::GateRefusal {
                seq,
                body: body.clone(),
            })),
            Some("run_start") => {
                self.start = RecordedStart::of(body);
                self.resumed = body["command"].as_str() == Some("resume");
                self.events.push(Event::RunStart(RunStart {
                    seq,
                    body: body.clone(),
                }));
            }
            Some("run_end") => {
                self.flush_response();
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
            Some("init") => {
                if let Some(tools) = raw["init"]["tools"].as_array() {
                    self.offered
                        .extend(tools.iter().filter_map(Value::as_str).map(str::to_string));
                }
            }
            Some("step_update") => self.step(seq, &raw["step_update"]),
            Some("result") => {
                self.flush_response();
                self.result = Some(raw.clone());
                // **`result.usage` is not read here**, and 9.2 is the whole
                // reason: on a resumed run it carries the predecessor's tokens
                // too, and Hydra mints a fresh run id per resume, so taking it
                // would count the first run's tokens twice in any corpus that
                // sums them. The frame is kept whole in `raw` and in
                // [`Run::result`], because a discrepancy between it and the sum
                // is how a resumed run announces itself.
                self.emit_usage();
            }
            // Build ticket 267: a harvested interactive run's lines carry no
            // `event`.
            None if raw.get("step_output").is_some() => self.step_output(seq, &raw["step_output"]),
            None if raw.get("step_index").is_some() && raw["type"].is_string() => {
                self.transcript_line(seq, raw)
            }
            // `command_result` is one of the free read-only slash commands. It
            // cannot appear on a recorded run, since 6.7's argv carries a task
            // rather than a slash command, and it is named here so a reader who
            // finds one in a record knows it is not a frame the launch produced.
            _ => {}
        }
    }

    /// One line of a harvested transcript, build ticket 267.
    /// `provider::agy` holds the measured shapes.
    fn transcript_line(&mut self, seq: u64, raw: &Value) {
        self.flush_response();
        let step = raw["step_index"].as_u64().unwrap_or_default();
        let kind = raw["type"].as_str().unwrap_or_default();
        let role = match kind {
            "USER_INPUT" => Role::User,
            "PLANNER_RESPONSE" => Role::Assistant,
            // A `GENERIC` step is the provider's own bookkeeping around a
            // call, and its output arrives on the step's `step_output` line;
            // the other two are the TUI's own instructions and its saves.
            "GENERIC" | "SYSTEM_MESSAGE" | "CHECKPOINT" => return,
            other => {
                if !self.unread_types.iter().any(|seen| seen == other) {
                    self.findings.push(Finding::Limit(format!(
                        "a transcript line of type {other} is one this reader does not know, \
                         so nothing was derived from it"
                    )));
                    self.unread_types.push(other.to_string());
                }
                return;
            }
        };
        if !self.transcript_steps.insert(step) {
            self.repeated_steps += 1;
            return;
        }
        let block = step as usize;
        if let Some(text) = raw["thinking"].as_str().filter(|text| !text.is_empty()) {
            self.events.push(Event::Thinking(Thinking {
                seq,
                text: text.to_string(),
                signature: None,
                block,
            }));
        }
        if let Some(text) = raw["content"].as_str().filter(|text| !text.is_empty()) {
            self.events.push(Event::Message(Message {
                seq,
                role,
                text: text.to_string(),
                block,
            }));
        }
        let calls = raw["tool_calls"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        if role != Role::Assistant || calls.is_empty() {
            return;
        }
        if calls.len() > 1 {
            self.multi_call_steps += 1;
        }
        for (n, call) in calls.iter().enumerate() {
            let name = call["name"].as_str().unwrap_or_default().to_string();
            let capability = agy_capability(&name);
            if capability.is_none() {
                self.note_unknown(&name);
            }
            let call_id = format!("{step}.{n}");
            self.unanswered.push_back((call_id.clone(), step));
            self.events.push(Event::ToolCall(ToolCall {
                seq,
                call_id,
                name,
                arguments: call["args"].clone(),
                capability,
            }));
        }
    }

    /// One step's output, which the harvest wrapped, build ticket 267: the
    /// answer to the oldest call still waiting that its step had made by then.
    fn step_output(&mut self, seq: u64, body: &Value) {
        self.flush_response();
        let step = body["step_index"].as_u64().unwrap_or_default();
        let output = body["output"].as_str().unwrap_or_default().to_string();
        let call_id = match self.unanswered.front() {
            Some((_, made_at)) if *made_at <= step => self
                .unanswered
                .pop_front()
                .map(|(id, _)| id)
                .unwrap_or_default(),
            _ => {
                self.orphan_outputs += 1;
                format!("{step}.output")
            }
        };
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            // The output file says nothing about success, so only a hook's own
            // marker makes this an error.
            status: classify(&output),
            output,
            truncated: false,
        }));
    }

    /// One `step_update`, told apart by `step_type` and `state`.
    fn step(&mut self, seq: u64, step: &Value) {
        let index = step["step_index"].as_u64().unwrap_or_default() as usize;
        let state = step["state"].as_str().unwrap_or_default();
        // Every step that reports counts contributes to the run's usage,
        // whatever its type. Measured: the counts ride the `agent_response`
        // steps and not the tool ones, which is a fact about what the model was
        // billed for rather than a rule this adapter should encode.
        self.add_counts(&step["usage"]);

        match step["step_type"].as_str() {
            Some("agent_response") => self.text(seq, index, step, Role::Assistant),
            Some("user_input") => self.text(seq, index, step, Role::User),
            Some("tool") => self.tool(seq, index, state, step),
            // A step type this build has not seen. The frame is in `raw` and the
            // finding says the run holds a shape nothing derived an event from,
            // which 7.1's stance on an unknown tool name applies here too.
            Some(other) => self.findings.push(Finding::Limit(format!(
                "the step at index {index} is a {other} step, which no event type in \
                 contract 7.2 covers on this provider, so nothing was derived from it"
            ))),
            None => {}
        }
    }

    /// Contract 7.3's concatenation, over `step_index`.
    ///
    /// A step's frames are contiguous on this provider, so a step index that
    /// differs from the one being built closes the previous one. The `seq` names
    /// the first frame of the step, which is where a reader starts concatenating
    /// to reproduce the text.
    fn text(&mut self, seq: u64, index: usize, step: &Value, role: Role) {
        if self
            .response
            .as_ref()
            .is_some_and(|open| open.step != index)
        {
            self.flush_response();
        }
        let delta = step["text_delta"].as_str().unwrap_or_default();
        if delta.is_empty() && self.response.is_none() {
            // Measured: a step that only called a tool carries no `text_delta`
            // on either frame. An empty message would claim the model said
            // nothing rather than that it said nothing here.
            return;
        }
        match &mut self.response {
            Some(open) => open.text.push_str(delta),
            None => {
                self.response = Some(Response {
                    seq,
                    step: index,
                    role,
                    text: delta.to_string(),
                })
            }
        }
        if role == Role::User {
            // The echoed prompt arrives as one step and closes immediately, so
            // it is not held open waiting for a delta that never comes.
            self.flush_response();
        }
    }

    fn flush_response(&mut self) {
        if let Some(open) = self.response.take() {
            if open.text.is_empty() {
                return;
            }
            self.events.push(Event::Message(Message {
                seq: open.seq,
                // **The role is the step's own and not a constant.** A
                // `user_input` step carrying text is the human's prompt echoed
                // back, and recording it as the model's would teach a corpus
                // reader that the model said what it was asked. The step
                // measured carried no text, so nothing on this machine has
                // exercised the other branch.
                role: open.role,
                text: open.text,
                block: open.step,
            }));
        }
    }

    /// One tool step: the call on `ACTIVE`, the result on `DONE` or `ERROR`.
    fn tool(&mut self, seq: u64, index: usize, state: &str, step: &Value) {
        self.flush_response();
        let info = &step["tool_info"];
        let name = step["tool_name"]
            .as_str()
            .or_else(|| info["name"].as_str())
            .unwrap_or_default()
            .to_string();
        let call_id = index.to_string();

        if self.started.insert(index) {
            let capability = agy_capability(&name);
            if capability.is_none() {
                self.note_unknown(&name);
            }
            self.events.push(Event::ToolCall(ToolCall {
                seq,
                call_id: call_id.clone(),
                name,
                arguments: info["parameters"].clone(),
                capability,
            }));
        }

        let (status, output) = match state {
            "ACTIVE" => return,
            "DONE" => (
                ResultStatus::Ok,
                info["output"].as_str().unwrap_or_default().to_string(),
            ),
            "ERROR" => {
                let message = info["error"]["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                (classify(&message), message)
            }
            // `INTERRUPTED` is in the binary's strings and unmeasured on a run.
            // Nothing about it says whether the tool ran, which is exactly what
            // 7.6's `unclassified` is for.
            _ => (
                ResultStatus::Unclassified,
                info["error"]["message"]
                    .as_str()
                    .or_else(|| info["output"].as_str())
                    .unwrap_or_default()
                    .to_string(),
            ),
        };
        self.events.push(Event::ToolResult(ToolResult {
            seq,
            call_id,
            status,
            output,
            // This provider reports no truncation marker and this normalizer
            // cuts nothing, so the marker is always false for a result. The
            // `view_file` output measured was a summary rather than the file,
            // which is the provider summarizing rather than truncating. Its
            // arguments are another matter: see `AGY_PARAMETER_CAP`.
            truncated: false,
        }));
    }

    fn add_counts(&mut self, usage: &Value) {
        let Some(usage) = usage.as_object() else {
            return;
        };
        // A step's usage object carries all five fields or none of them.
        // `cache_write_tokens` is absent on this provider (section 9's table),
        // and absent stays absent rather than becoming a zero, because a zero
        // claims the provider reported no cache write on a run where it reported
        // nothing at all.
        let read = |field: &str| usage.get(field).and_then(Value::as_u64);
        if read("input_tokens").is_none() && read("output_tokens").is_none() {
            return;
        }
        let running = self.counts.get_or_insert_with(Counts::default);
        running.input_tokens += read("input_tokens").unwrap_or_default();
        running.output_tokens += read("output_tokens").unwrap_or_default();
        running.cache_read_tokens += read("cache_read_tokens").unwrap_or_default();
        if let Some(thinking) = read("thinking_tokens") {
            running.thinking_tokens = Some(running.thinking_tokens.unwrap_or_default() + thinking);
        }
    }

    /// The one `usage` event, from the sum rather than from a frame (9.2).
    fn emit_usage(&mut self) {
        if self.usage_emitted {
            return;
        }
        let usage = match self.counts {
            Some(counts) => Usage {
                // No frame reported this. Pointing at the last step would name a
                // frame carrying a fraction of it, and 7.8 asks where the counts
                // came from rather than which line to reread.
                seq: None,
                source: UsageSource::Provider,
                estimator: None,
                counts: Some(counts),
            },
            // 7.8's never-a-bare-null: a run that reported nothing is
            // `unavailable`, never counts of zero.
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
        self.flush_response();
        self.emit_usage();
        self.note_transcript_join();
    }

    /// What a harvested transcript's join by order could not settle, build
    /// ticket 267.
    fn note_transcript_join(&mut self) {
        if self.multi_call_steps > 0 {
            self.findings.push(Finding::Limit(format!(
                "{} transcript steps made more than one tool call, and the transcript carries \
                 no call id, so which output answers which of those calls is a guess by order",
                self.multi_call_steps
            )));
        }
        if !self.unanswered.is_empty() {
            self.findings.push(Finding::Limit(format!(
                "{} tool calls in the transcript have no step output, so nothing says what they \
                 returned",
                self.unanswered.len()
            )));
        }
        if self.orphan_outputs > 0 {
            self.findings.push(Finding::Limit(format!(
                "{} step outputs came with no call waiting for them, and carry the call id \
                 <step>.output",
                self.orphan_outputs
            )));
        }
        if self.repeated_steps > 0 {
            self.findings.push(Finding::Limit(format!(
                "{} transcript lines repeat a step already read, and the first line of each \
                 step is the one read",
                self.repeated_steps
            )));
        }
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
        super::note_finding_bar(Some(Provider::Agy), &mut self.findings);
        self.check_table();
        if self.resumed {
            self.findings.push(Finding::Limit(
                "this run's command is resume, so the result frame's usage covers earlier turns \
                 in the conversation and is not this run's own token count"
                    .to_string(),
            ));
        }
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

    /// 7.1's run-time table check, against the names the `init` frame offered.
    ///
    /// This provider lists every tool at process start like `claude` does, so a
    /// self-update that renames one is visible in the first record written after
    /// it rather than at the first denial. Cursor is the one that cannot answer
    /// this.
    fn check_table(&mut self) {
        let offered = std::mem::take(&mut self.offered);
        for name in offered {
            if agy_capability(&name).is_none() {
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

    /// One decision per call, by re-evaluating the run's own policy (7.7).
    ///
    /// Absent, never empty, on a run with no policy: there is nothing to
    /// re-evaluate and `external` would presume a policy that could have allowed.
    fn decide(&mut self, policies: &PolicySet, layout: &Layout) -> Vec<Option<PermissionDecision>> {
        let mut decisions: Vec<Option<PermissionDecision>> = Vec::new();
        if self.start.gate.skip_other_host(&mut self.findings) {
            return decisions;
        }
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
            Provider::Agy,
        );
        // Once per record, before any decision, so a reader sees what the
        // derivation stood in for ahead of what it derived.
        self.findings.extend(spawn.scope_limit());
        let replay = match Replay::compile(policies, &spawn.spawn(Provider::Agy)) {
            Ok(replay) => replay,
            Err(err) => {
                self.findings.push(Finding::Limit(format!(
                    "policy {name} would not compile, so no decision was derived: {err:#}"
                )));
                return decisions;
            }
        };

        let results = super::results_by_call(&self.events);
        let replay_inputs: Vec<(&str, &Value, &Path)> = calls
            .iter()
            .map(|(_, _, tool, arguments)| (tool.as_str(), arguments, spawn.cwd()))
            .collect();
        let mut replay_verdicts = replay.decide_all(&replay_inputs).into_iter();
        for (seq, call_id, tool, arguments) in calls {
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
            let refusal = result.and_then(|(status, output)| refusal_in(*status, output));

            // Build ticket 45. The stream cuts every string parameter at
            // `AGY_PARAMETER_CAP`, so a deny on a capped argument is a verdict
            // about a command the hook never saw: the hook read the whole
            // string, this replay read the cut. Measured 2026-09-12 on run
            // 01a0902c-8d66, where two `run_command` calls the floor "could not
            // parse" had run with output, because the cut fell inside a quote.
            // Nothing is derived for such a call, and the record says why.
            if !verdict.allowed
                && refusal.is_none()
                && matches!(result, Some((ResultStatus::Ok, _)))
                && argument_at_cap(&arguments)
            {
                self.findings.push(Finding::Limit(format!(
                    "step {call_id} ({tool}) carries an argument cut at this provider's \
                     {AGY_PARAMETER_CAP}-character cap, so the replay judged a command the \
                     hook never saw and cannot say whether the gate would have refused it"
                )));
                // Contract 7.7 owes one decision per call. The one this record
                // can back is the hook's: it read the whole argument, nothing
                // refused, the tool ran. The reason says the replay did not
                // re-evaluate it, so a reader knows this line was not derived
                // from the policy the way its neighbours were.
                decisions.push(Some(PermissionDecision {
                    seq,
                    call_id: call_id.clone(),
                    decision: Decision::Allow,
                    decider: Decider::HydraPolicy,
                    reason: format!(
                        "hydra/capped-argument: the hook allowed a command this record holds \
                         the first {AGY_PARAMETER_CAP} characters of; the replay could not \
                         re-evaluate it"
                    ),
                }));
                continue;
            }

            let derived = if !verdict.allowed {
                match &refusal {
                    None => {
                        // The policy denies this call and the record shows no
                        // refusal. A result is the fate; no result is a record
                        // that stops here, which is not evidence either way.
                        //
                        // **An `unclassified` result is not evidence either.**
                        // 7.6's whole point on this provider is that a denial
                        // and a failure wear one error type, so a leak is only
                        // claimed when the tool reported success.
                        match result {
                            Some((ResultStatus::Ok, _)) => {
                                match spawn.cannot_back(&format!(
                                    "the policy denies step {call_id} ({tool}) and nothing in \
                                     the record refused it"
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
                                "step {call_id} ({tool}) failed with an error this provider \
                                 spells the same way it spells a refusal, so whether the \
                                 gate refused a call the policy denies is unknown"
                            ))),
                            None => self.findings.push(Finding::Limit(format!(
                                "step {call_id} ({tool}) is the last thing the record says, \
                                 so whether the gate refused a call the policy denies is \
                                 unknown"
                            ))),
                        }
                    }
                    // Something else refused. On this provider the first deny
                    // short-circuits, so Hydra's hook may never have run and the
                    // reason belongs to whichever hook did (7.7). Hydra's own
                    // voice saying `allow` is the one shape that means two
                    // sources disagree.
                    Some(refusal) if refusal.by == Decider::External => {
                        match hydra_voice(&refusal.reason) {
                            // A moved policy directory means the gate and the
                            // derivation answered about two policies rather
                            // than one, so the difference is a limit and not a
                            // disagreement. See `policy_moved`.
                            Some(_) if policy_moved(self.start.policy_sha.as_deref(), policies) => {
                                self.findings.push(Finding::Limit(format!(
                                    "the gate allowed step {call_id} ({tool}) and the \
                                     derivation denies it, under a policy_sha that is no \
                                     longer today's, so the two answered about two policies"
                                )));
                            }
                            Some(code) => {
                                match spawn.cannot_back(&format!(
                                    "the gate allowed step {call_id} ({tool}) and the \
                                     derivation denies it"
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
                            // A hook refused and it was not Hydra's. The gate
                            // fails closed on this provider and there is no
                            // flag to change that, so the innocent reading is
                            // narrow: another hook denied first and
                            // short-circuited Hydra's.
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
                    // now allows. Two sources disagree (7.7), and the tool
                    // error is the second source: it carries Hydra's reason
                    // verbatim.
                    Some(refusal) => {
                        if policy_moved(self.start.policy_sha.as_deref(), policies) {
                            self.findings.push(Finding::Limit(format!(
                                "the gate denied step {call_id} ({tool}) and the derivation \
                                 allows it, under a policy_sha that is no longer today's, \
                                 so the two answered about two policies"
                            )));
                        } else if let Some(limit) = spawn.cannot_back(&format!(
                            "the gate denied step {call_id} ({tool}) and the derivation \
                             allows it"
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
                    // Nothing refused. No `Ungated` finding here, the same as on
                    // Cursor: the stream carries no gate answer for an allowed
                    // call, so its absence says nothing, and a hook that failed
                    // to run would have denied every tool in the run rather than
                    // letting this one through.
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

/// What an `ERROR` tool step's message says about whether the tool ran.
///
/// **This is where contract 7.6's `unclassified` earns its place.** Measured in
/// one run on 1.1.26: `write_to_file` failed with `declaring permissions: cortex
/// tool write_to_file: ... is not a valid artifact path`, and `run_command`
/// failed with `permission check failed for command ...: user denied permission
/// to run command`. Same `state: ERROR`, same `error.type: TOOL_ERROR`, both
/// mentioning permissions, and one is a refusal while the other is the provider
/// rejecting an argument. A substring rule over the word "permission" gets both
/// wrong, so the only strings keyed on here are the two the provider writes for
/// a hook block and nothing else: an error that is not one of those is
/// `unclassified`, which keeps the gap visible instead of averaged away.
fn classify(message: &str) -> ResultStatus {
    if hydra_voice(message).is_some()
        || message.contains(DENIED_BY_HOOK)
        || message.contains(HOOK_ID_PREFIX)
    {
        return ResultStatus::Error;
    }
    ResultStatus::Unclassified
}

/// What the record shows happened to a call, as against what the policy says
/// should have.
///
/// Three readings, in the order they are trusted. A gate code that is not
/// `allow` is Hydra's own refusal. `hydra/allow` inside a refusal means Hydra
/// allowed and something else denied. The provider's own two markers with no
/// gate code in them are a hook block that was not Hydra's: a hook that
/// short-circuited ahead of it, or a hook whose script never started, which on
/// this provider denies every tool in the run.
///
/// An `unclassified` result is deliberately not a refusal. It is the shape a
/// file-not-found and a provider-side argument rejection both wear, and calling
/// it a refusal would teach rung one that the gate blocked work the gate never
/// saw.
/// The longest string parameter `agy` streams in a `step_update` frame.
///
/// Measured 2026-09-12 over every `agy` record on this machine: no parameter is
/// longer, sixty sit exactly here, and 59 of the 71 over 300 characters end with
/// an odd number of double quotes. The hook receives the whole argument; the
/// record keeps this much of it.
pub const AGY_PARAMETER_CAP: usize = 513;

/// Whether any string in `arguments` reached the cap, at the top level or one
/// level down, which is where every parameter this provider streams lives.
fn argument_at_cap(arguments: &Value) -> bool {
    fn at_cap(value: &Value) -> bool {
        value
            .as_str()
            .is_some_and(|text| text.chars().count() >= AGY_PARAMETER_CAP)
    }
    match arguments {
        Value::Object(fields) => fields.values().any(|value| {
            at_cap(value)
                || value
                    .as_object()
                    .is_some_and(|inner| inner.values().any(at_cap))
        }),
        other => at_cap(other),
    }
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

    /// Build ticket 267: a repeated step is read once, an output with no call
    /// waiting keeps its step, a call with no output is named, and an unknown
    /// line type is named once.
    #[test]
    fn a_transcript_names_what_its_join_by_order_could_not_settle() {
        let line = |seq: u64, raw: Value| {
            serde_json::json!({"seq": seq, "run_id": "r", "provider": "agy", "raw": raw})
                .to_string()
        };
        let call = serde_json::json!([{"name": "run_command", "args": {"CommandLine": "ls"}}]);
        let text = [
            line(
                1,
                serde_json::json!({"step_output": {"step_index": 0, "output": "early"}}),
            ),
            line(
                2,
                serde_json::json!({"type": "PLANNER_RESPONSE", "step_index": 1,
                                       "status": "RUNNING", "tool_calls": call}),
            ),
            line(
                3,
                serde_json::json!({"type": "PLANNER_RESPONSE", "step_index": 1,
                                       "status": "DONE", "tool_calls": call}),
            ),
            line(4, serde_json::json!({"type": "NEW_KIND", "step_index": 2})),
            line(5, serde_json::json!({"type": "NEW_KIND", "step_index": 3})),
        ]
        .join("\n");
        let walk = walk(&text);
        let calls: Vec<&ToolCall> = walk
            .events
            .iter()
            .filter_map(|event| match event {
                Event::ToolCall(call) => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 1, "the step written twice is read once");
        assert_eq!(calls[0].call_id, "1.0");
        let results: Vec<&str> = walk
            .events
            .iter()
            .filter_map(|event| match event {
                Event::ToolResult(result) => Some(result.call_id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(results, ["0.output"]);
        let limits: Vec<&str> = walk
            .findings
            .iter()
            .filter_map(|finding| match finding {
                Finding::Limit(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(limits.len(), 4, "{limits:?}");
        assert!(limits[0].contains("type NEW_KIND"));
        assert!(limits[1].starts_with("1 tool calls in the transcript have no step output"));
        assert!(limits[2].starts_with("1 step outputs came with no call waiting"));
        assert!(limits[3].starts_with("1 transcript lines repeat a step"));
    }

    #[test]
    fn a_permission_denial_and_a_missing_file_are_told_apart_by_the_markers_only() {
        // Both measured on 1.1.26 in one run. Neither is a substring rule over
        // the word "permission", which both of them contain.
        let artifact = "declaring permissions: cortex tool write_to_file: convert tool call \
                        for permissions: model output error: invalid tool call error \
                        (invalid_args) /tmp/wt/t10-out.txt is not a valid artifact path";
        let headless = "permission check failed for command \"printf OK > t10-out.txt\": \
                        user denied permission to run command";
        assert_eq!(classify(artifact), ResultStatus::Unclassified);
        assert_eq!(classify(headless), ResultStatus::Unclassified);

        // A hook block, which the provider marks in its own words, and Hydra's
        // reason travels inside it verbatim.
        let blocked = "tool call denied by pre-tool hook: hydra/capability: policy read-only \
                       does not allow shell (run_command).";
        assert_eq!(classify(blocked), ResultStatus::Error);
        assert_eq!(hydra_voice(blocked), Some("capability"));

        // A broken hook script, which denies every tool and names itself by the
        // generated id (ticket 26 measured this the hard way).
        let broken = "JSON hook \"jsonhook__hydra-gate_PreToolUse_0_0\" failed: command \
                      failed: exit status 127";
        assert_eq!(classify(broken), ResultStatus::Error);
        assert!(refusal_in(ResultStatus::Error, broken).is_some_and(|r| r.by == Decider::External));

        // An unclassified result is not a refusal, whatever it says.
        assert!(refusal_in(ResultStatus::Unclassified, headless).is_none());
        assert!(refusal_in(ResultStatus::Ok, "fine").is_none());
    }

    #[test]
    fn hydras_own_allow_inside_a_refusal_means_something_else_denied() {
        let text = "tool call denied by pre-tool hook: hydra/allow: policy standard allows \
                    shell. some other hook: no.";
        let refusal = refusal_in(ResultStatus::Error, text).unwrap();
        assert_eq!(refusal.by, Decider::External);
        let mine = "tool call denied by pre-tool hook: hydra/out-of-scope: policy read-only \
                    allows file_read inside worktree and this call is outside it.";
        assert_eq!(
            refusal_in(ResultStatus::Error, mine).unwrap().by,
            Decider::HydraPolicy
        );
    }
}
