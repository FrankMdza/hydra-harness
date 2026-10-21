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
use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use super::{
    Correction, Counts, Decider, Decision, Event, Finding, Message, PermissionDecision,
    RecordedStart, ReplaySpawn, ResultStatus, Role, Run, RunEnd, RunStart, Thinking, ToolCall,
    ToolResult, Usage, UsageSource, policy_moved, splice, verdict_decision,
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
    start: RecordedStart,
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
    // Contract 5.6: line 0 is where a reader learns this run compiled no gate,
    // and it is the first line of every file, so one parse of it decides the
    // adapter rather than a scan of the whole text the way picking a provider
    // does in `read_with`.
    if is_interactive(text) {
        return harvest::normalize(text, policies);
    }
    walk(text).finish(policies, layout)
}

/// The usage the record carries so far, off the same walk and without the
/// gate replay [`normalize`] finishes with. Build ticket 172.
///
/// `None` on an interactive record: its frames are harvested once the session
/// ends, so while it runs there is nothing to count.
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
///
/// Empty on an interactive record, for the reason [`usage_so_far`] is `None`.
pub fn events_so_far(text: &str) -> Vec<Event> {
    if is_interactive(text) {
        return Vec::new();
    }
    walk(text).events
}

fn walk(text: &str) -> Walk {
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
    walk
}

/// Whether line 0 says this run compiled no gate.
///
/// **A launch mode is a fact about the file that decides which vocabulary the
/// rest of it is read in, and it lives on the first line.** Contract 3: a
/// harvested line wraps a transcript record rather than a stream frame, and
/// [`harvest::normalize`] is the reader for that vocabulary. Only line 0 is
/// parsed, because contract 5's `run_start` is always first, and a run that has
/// not gotten that far yet has no frames either way.
fn is_interactive(text: &str) -> bool {
    let Some(first) = text.lines().next() else {
        return false;
    };
    let Ok(envelope) = serde_json::from_str::<Value>(first) else {
        return false;
    };
    envelope["hydra"]["launch_mode"].as_str() == Some("interactive")
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
            Some("gate_refusal") => self.events.push(Event::GateRefusal(super::GateRefusal {
                seq,
                body: body.clone(),
            })),
            Some("run_start") => {
                self.start = RecordedStart::of(body);
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
        let (events, unknown) = message_blocks(seq, raw, blocks, role);
        self.events.extend(events);
        for name in unknown {
            self.note_unknown(&name);
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
        super::note_capped_lines(&self.events, &mut self.findings);
        super::note_policy_sha(
            self.start.policy.as_deref(),
            self.start.policy_sha.as_deref(),
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
            policy: self.start.policy,
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
        if self
            .start
            .gate
            .host
            .as_ref()
            .is_some_and(|host| host.is_other())
        {
            return;
        }
        let Some(name) = self.start.policy.clone() else {
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
            Provider::Claude,
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

        let results = super::results_by_call(&self.events);
        let shell_moved = Self::shell_moved_before(&calls, &results);
        let replay_inputs: Vec<(&str, &Value, &Path)> = calls
            .iter()
            .map(|(_, _, tool, arguments)| (tool.as_str(), arguments, spawn.cwd()))
            .collect();
        let mut replay_verdicts = replay.decide_all(&replay_inputs).into_iter();
        let seqs: Vec<u64> = calls.iter().map(|call| call.0).collect();
        // Empty on a record with no `hook_started` frames, which is what tells
        // the loop below to keep using the seq window instead.
        let groups = Self::group_starts(&calls, &self.starts);
        for (index, (seq, call_id, tool, _arguments)) in calls.into_iter().enumerate() {
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
            let gate_said = self.take_gate_answer(window, group, &tool);
            // The gate answered from wherever the shell stood, and the replay
            // asks from the run's own directory. Only a call the gate spoke
            // for: a silent gate is `Ungated` whatever the directory was.
            let unvouched_cwd = (shell_moved[index] && gate_said.is_some()).then_some(SHELL_MOVED);
            let refusal = self.refusal(&call_id, window, group, &tool, results.get(&call_id));

            // 6.6: a hook that fails to start fails open on this provider, and
            // `--include-hook-events` is on for every recorded run, so a call
            // with no gate answer and no refusal in Hydra's own voice is a call
            // the gate never spoke about.
            let hydra_spoke = gate_said.is_some()
                || refusal
                    .as_ref()
                    .is_some_and(|refusal| refusal.by == Decider::HydraPolicy);
            // A call Claude Code refused on its own never reached a hook. Four
            // spellings measured, all on haiku: `InputValidationError`,
            // 2026-09-07 on run 01a07f37-8a67, where it passed `offset` as an
            // array five times; `<tool_use_error>Blocked: ...`, 2026-09-08 on run
            // 01a0812a-ad31, the binary's own guard refusing `sleep 70 && tail
            // ...`; `No such tool available: bash`, the same day on run
            // 01a0822f-9e37, where it invented a lowercase `bash`; and
            // `<tool_use_error>File has not been read yet. Read it first before
            // writing to it.`, 2026-09-12 on run 01a096ee-8791, a `Write` into
            // the skill store that 2.1.268 refused before any hook, with 43
            // `hook_started` frames in the record and none for that call. And a
            // fifth, `<tool_use_error>Error calling tool (Write): ENOTDIR ...`,
            // 2026-09-21 on run 01a0c5b2-8048, sonnet on 2.1.268: a `Write` into
            // a linked worktree's `.git`, which is a file, failed the binary's
            // own `stat` before any hook, with four `PreToolUse:Write` starts in
            // the record for the run's other four writes and none for this one.
            // A sixth, `<tool_use_error>File does not exist.`, 2026-09-23 on run
            // 01a0cc56-c15b, sonnet on 2.1.268: an `Edit` to a mistyped
            // `worktrades/` path, refused on its own `stat` with no hook frame.
            // And a seventh, the same day on run 01a0cf89-8b3d, opus: `Not run:
            // the response that made this tool call was stopped by a safety
            // classifier.`, a `Write` withheld by the model provider before
            // Claude Code ran any tool or hook. An eighth, `<tool_use_error>No
            // task found with ID: ...`, 2026-10-04 on run 01a1049c-35c1, opus on
            // 2.1.289: a `TaskStop` for a task that had already ended, with two
            // `PreToolUse:TaskStop` starts in the record for the two calls
            // before it and none for this one. Build ticket 286.
            // The gate was never asked, so neither its silence nor the table's
            // ignorance of an invented name is a finding.
            let refused_on_input = results.get(&call_id).is_some_and(|(status, output)| {
                *status == ResultStatus::Error
                    && (output.contains("InputValidationError")
                        || output.contains("No such tool available")
                        || output.starts_with("<tool_use_error>Blocked:")
                        || output.starts_with("<tool_use_error>File has not been read yet")
                        || output.starts_with("<tool_use_error>File does not exist.")
                        || output.starts_with("<tool_use_error>No task found with ID")
                        || output.starts_with("<tool_use_error>Error calling tool (")
                        || output.starts_with(
                            "Not run: the response that made this tool call was stopped",
                        ))
            });
            let rejected_before_hook = !hydra_spoke
                && results
                    .get(&call_id)
                    .is_some_and(|(status, _)| *status == ResultStatus::Error);
            if !hydra_spoke {
                if rejected_before_hook {
                    let (_, output) = &results[&call_id];
                    self.findings.push(Finding::RejectedBeforeHook {
                        call_id: call_id.clone(),
                        tool: tool.clone(),
                        error: output.lines().next().unwrap_or_default().to_string(),
                    });
                } else {
                    self.findings.push(Finding::Ungated {
                        call_id: call_id.clone(),
                        tool: tool.clone(),
                    });
                }
            }

            if let Some((decision, reason)) = &gate_said
                && *decision != verdict_decision(verdict.allowed)
            {
                // The only place in the record where two independent sources
                // answer the same question, so a difference is reported rather
                // than resolved (7.7). Unless the policy directory moved between
                // the run and the read, in which case the two sources answered
                // about two policies and the difference says nothing.
                if policy_moved(self.start.policy_sha.as_deref(), policies) {
                    self.findings.push(Finding::Limit(format!(
                        "the gate answered {:?} for {call_id} ({tool}) and the derivation \
                         says {:?}, under a policy_sha that is no longer today's, so the \
                         two answered about two policies",
                        decision,
                        verdict_decision(verdict.allowed),
                    )));
                } else if let Some(limit) = spawn.cannot_back_call(
                    &format!(
                        "the gate answered {:?} for {call_id} ({tool}) and the derivation says {:?}",
                        decision,
                        verdict_decision(verdict.allowed),
                    ),
                    unvouched_cwd,
                ) {
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
                    } else if refused_on_input || rejected_before_hook {
                        // The provider refused the call on its own, so the gate
                        // never spoke and its verdict is about a call that never
                        // ran. An invented tool name lands here, denied
                        // `unknown-tool` by a table that is right to not know it.
                        self.findings.push(Finding::Limit(format!(
                            "{call_id} ({tool}) was refused by the provider itself, so the \
                             policy's answer is about a call that never ran"
                        )));
                    } else if verdict.code() == Some("no-path")
                        && results
                            .get(&call_id)
                            .is_some_and(|(status, _)| *status == ResultStatus::Error)
                    {
                        // The replay saw no path, so it could not judge, and the
                        // provider answered an error of its own: measured
                        // 2026-09-08 on run 01a0812a-ad31, an `Edit` whose
                        // `file_path` the model sent as nothing and `claude`
                        // refused with `InputValidationError` before any hook.
                        // The tool did not run and the record cannot say what
                        // path it would have taken: a limit, not a leak.
                        self.findings.push(Finding::Limit(format!(
                            "{call_id} ({tool}) named no path the replay could judge and \
                             the provider answered an error, so whether it ran is unknown"
                        )));
                    } else if let Some(limit) = spawn.cannot_back_call(
                        &format!(
                            "the policy denies {call_id} ({tool}) and nothing in the record \
                             refused it"
                        ),
                        unvouched_cwd,
                    ) {
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

    /// Whether Claude Code's shell may stand somewhere other than the run's own
    /// directory when each call was asked, one answer per call.
    ///
    /// **The hook's `cwd` is the shell's, not the run's.** Claude Code keeps the
    /// directory a `Bash` call left its shell in, hands it to the next call's
    /// `PreToolUse` hook as `cwd`, and resets it only when it lands outside the
    /// project, saying `Shell cwd was reset to <dir>` at the end of that
    /// call's result. The record carries no other trace of it, and the replay
    /// asks every call from `run_start.cwd`. Build ticket 286 measured what
    /// that costs: four calls on 2026-10-04, each a relative `..` climbing out
    /// of a subdirectory an earlier call had `cd`'d into, which the live gate
    /// allowed and the replay resolved one level too high, into
    /// `~/hydra/worktrees` or `~/hydra`, and filed as leaks.
    ///
    /// Coarse in one direction on purpose. Any `cd`, `pushd` or `popd` that
    /// begins a segment counts, including one inside a subshell that never
    /// moved anything, because reading which ones moved the shell is running
    /// the command. The cost is a `Limit` where a `GateLeak` or a
    /// `Disagreement` stood, and only on a call the gate itself answered.
    fn shell_moved_before(
        calls: &[(u64, String, String, Value)],
        results: &HashMap<String, (ResultStatus, String)>,
    ) -> Vec<bool> {
        let mut moved = false;
        calls
            .iter()
            .map(|(_, call_id, tool, arguments)| {
                let before = moved;
                if tool == "Bash" {
                    let reset = results
                        .get(call_id)
                        .is_some_and(|(_, output)| output.contains("Shell cwd was reset to "));
                    if reset {
                        moved = false;
                    } else if arguments["command"].as_str().is_some_and(changes_directory) {
                        moved = true;
                    }
                }
                before
            })
            .collect()
    }

    /// Every `hook_started` frame assigned to the call it belongs to.
    ///
    /// The n-th `PreToolUse:<Tool>` start group belongs to the n-th `<Tool>`
    /// call, a call's group being every start between its own frame and the
    /// next call of the same tool. `calls` is already in seq order, which is
    /// what lets this walk each tool's calls and starts together in one pass.
    ///
    /// **When a tool has exactly one start per call, the n-th start is the n-th
    /// call's, whichever side of the call's frame it landed on.** Ticket 350 added
    /// this rule.
    /// Claude Code 2.1.301 wrote a call's start one seq before the call, so
    /// the window rule gave that start to the call before, and the last call
    /// was left with none. The window rule and the borrow below apply only
    /// when the counts differ.
    ///
    /// **A call whose window holds no start borrows from a later window of the
    /// same tool that holds more than one** (build ticket 46). Run
    /// 01a095e6-626f on 2.1.268 emitted two `WebFetch` calls back to back and
    /// both starts after them, so the first call's window held nothing and the
    /// second's held both. Starts are emitted in call order even when responses
    /// are not, so the earliest surplus start is the borrower's. A later window
    /// holding exactly one keeps it: run 01a07f37-8a67 has `Read` calls the
    /// binary refused on their input, for which no hook ever started, and
    /// those must not take the one start the next real `Read` owns.
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
            let tool = calls[indices[0]].2.as_str();
            let own: Vec<&HookStart> = starts.iter().filter(|start| start.tool == tool).collect();
            // Ticket 350: with one start per call, the n-th start is the n-th
            // call's, wherever it landed. The window rule below put the last
            // call's start in the call before it on runs 01a14290-f30d and
            // 01a14295-9d3f, so the last call had no start and read `Ungated`.
            if own.len() == indices.len() {
                for (&index, start) in indices.iter().zip(&own) {
                    groups[index] = vec![start.hook_id.clone()];
                }
                continue;
            }
            for (position, &index) in indices.iter().enumerate() {
                let after = calls[index].0;
                let before = indices
                    .get(position + 1)
                    .map(|&next| calls[next].0)
                    .unwrap_or(u64::MAX);
                groups[index] = starts
                    .iter()
                    .filter(|start| start.tool == tool && start.seq > after && start.seq < before)
                    .map(|start| start.hook_id.clone())
                    .collect();
            }
            for (position, &index) in indices.iter().enumerate() {
                if !groups[index].is_empty() {
                    continue;
                }
                let donor = indices[position + 1..]
                    .iter()
                    .copied()
                    .find(|&later| groups[later].len() > 1);
                if let Some(donor) = donor {
                    let borrowed = groups[donor].remove(0);
                    groups[index].push(borrowed);
                }
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
        if let Some((ResultStatus::Error, output)) = result {
            let output = pretooluse_hook_error_reason(tool, output);
            if let Some(code) = reason_code(output)
                && code != "allow"
            {
                return Some(Refusal {
                    by: Decider::HydraPolicy,
                    reason: output.to_string(),
                });
            }
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

/// Strips the `PreToolUse:<Tool> hook error: ` prefix Claude Code 2.1.280 added
/// to error `tool_result` text, only when `<Tool>` is the call's own name.
/// Build ticket 116.
fn pretooluse_hook_error_reason<'a>(tool: &str, text: &'a str) -> &'a str {
    text.strip_prefix(&format!("PreToolUse:{tool} hook error: "))
        .unwrap_or(text)
}

/// The clause a finding loses to [`Walk::shell_moved_before`].
const SHELL_MOVED: &str = "an earlier Bash call ran cd, and Claude Code hands the hook the \
     directory its shell was left in, so the gate resolved this call from a directory the \
     record does not carry and the replay from the run's own";

/// Whether a `Bash` command changes its shell's directory, read coarsely: a
/// segment, split at the shell's separators, whose first word is `cd`,
/// `pushd` or `popd`.
fn changes_directory(command: &str) -> bool {
    command
        .split([';', '&', '|', '\n', '(', '{'])
        .any(|segment| {
            matches!(
                segment.split_whitespace().next(),
                Some("cd" | "pushd" | "popd")
            )
        })
}

/// The content blocks of one `assistant` or `user` message, in order.
///
/// **A stream frame and a harvested transcript record wrap the same Anthropic
/// message shape here**, which is what lets the stream's [`Walk::blocks`] and
/// [`harvest::Walk::message`] read one function rather than two copies of the
/// same four-way match drifting apart. Unknown tool names come back rather
/// than being noted directly, because the two callers keep their own
/// deduplicated list.
fn message_blocks(
    seq: u64,
    frame: &Value,
    blocks: &[Value],
    role: Role,
) -> (Vec<Event>, Vec<String>) {
    let mut events = Vec::new();
    let mut unknown = Vec::new();
    for (block, body) in blocks.iter().enumerate() {
        match body["type"].as_str() {
            Some("text") => events.push(Event::Message(Message {
                seq,
                role,
                text: body["text"].as_str().unwrap_or_default().to_string(),
                block,
            })),
            Some("thinking") => events.push(Event::Thinking(Thinking {
                seq,
                text: body["thinking"].as_str().unwrap_or_default().to_string(),
                signature: body["signature"].as_str().map(str::to_string),
                block,
            })),
            Some("tool_use") => {
                let name = body["name"].as_str().unwrap_or_default().to_string();
                let capability = claude_capability(&name);
                if capability.is_none() {
                    unknown.push(name.clone());
                }
                events.push(Event::ToolCall(ToolCall {
                    seq,
                    call_id: body["id"].as_str().unwrap_or_default().to_string(),
                    name,
                    arguments: body["input"].clone(),
                    capability,
                }));
            }
            Some("tool_result") => events.push(Event::ToolResult(ToolResult {
                seq,
                call_id: body["tool_use_id"].as_str().unwrap_or_default().to_string(),
                status: result_status(body, frame),
                output: result_output(body.get("content")),
                // `claude` reports no truncation on the stream and nothing
                // here cuts an output, so the marker is always false.
                truncated: false,
            })),
            _ => {}
        }
    }
    (events, unknown)
}

/// What a `tool_result` block's `status` is, from `is_error` and then from the
/// frame's `tool_use_result`.
///
/// **`is_error` present decides.** True is `Error`, false is `Ok`. Absent, the
/// sibling field on the user frame is the second source: an object is `Ok`, a
/// string starting `Error:` is `Error`, and anything else, including absence,
/// stays `Unclassified`. Contract 7.6 forbids reading a missing `is_error` as
/// success; the object is presence, not absence.
///
/// Measured on 2.1.261: a successful `Write` came back with no `is_error` field
/// at all. On 2.1.263, 2026-09-08, runs 01a07f37-9cab and 01a07f18-910f, the
/// same absence still happens on `Read`, `Edit` and `Write`, and the user frame
/// carries `tool_use_result` beside `message`:
///
/// - `Bash`: `is_error: false` on every success, `tool_use_result` an object
///   with `stdout`, `stderr`, `interrupted`, `isImage`, `noOutputExpected` and
///   sometimes `returnCodeInterpretation` or `persistedOutputPath`;
///   `is_error: true` on failure with `tool_use_result` a string starting
///   `Error: `.
/// - `Read`: `is_error` absent on success, `tool_use_result` an object
///   `{"type": "text", "file": {"filePath": ..., ...}}`; `is_error: true` on a
///   refusal with the string `Error: hydra/...`.
/// - `Edit`: `is_error` absent on success, `tool_use_result` an object with
///   `filePath`, `oldString`, `newString`, `originalFile`, `replaceAll`,
///   `structuredPatch`.
/// - `Write`: `is_error` absent on success, `tool_use_result` an object with
///   `type: "create"`, `filePath`, `content`, `originalFile`,
///   `structuredPatch`.
///
/// No success was seen with `tool_use_result` as a string and no error with it
/// as an object.
fn result_status(block: &Value, frame: &Value) -> ResultStatus {
    match block.get("is_error").and_then(Value::as_bool) {
        Some(true) => ResultStatus::Error,
        Some(false) => ResultStatus::Ok,
        None => match frame.get("tool_use_result") {
            Some(Value::Object(_)) => ResultStatus::Ok,
            Some(Value::String(text)) if text.starts_with("Error:") => ResultStatus::Error,
            _ => ResultStatus::Unclassified,
        },
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

/// The harvest reader, contract 7.2.1 and 7.9.
///
/// **The vocabulary is the vendor's own transcript, not the stream's.** A
/// `run_start`/`run_end` line is a Hydra line exactly as it is on a recorded
/// run, but every provider line in between wraps one record of
/// `~/.claude/projects/<slug>/<session>.jsonl` rather than a `stream-json`
/// frame, and that record carries fields no frame ever will: `uuid`,
/// `parentUuid`, and on a rejection, `toolDenialKind`, `entrypoint` and
/// `sourceToolAssistantUUID`. `message`, `tool_call` and `tool_result` still
/// come off `message.content`, because that half of the shape is the Anthropic
/// API's own and both vocabularies wrap it, which is what lets
/// [`super::message_blocks`] read both.
///
/// **No policy on an ungated run, so no `permission_decision`.** Contract 5.6:
/// an interactive run that compiled no gate has nothing to re-evaluate a call
/// against. Map ticket 50's gated shape is different: denials arrive as error
/// `tool_result` blocks whose text starts with `hydra/`, and those become
/// `permission_decision` rows of `deny` under `hydra_policy` with the reason
/// verbatim. Allows leave no row, and one `Finding::Limit` says that out loud.
mod harvest;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn block(is_error: Option<bool>) -> Value {
        match is_error {
            Some(flag) => json!({"type": "tool_result", "is_error": flag}),
            None => json!({"type": "tool_result"}),
        }
    }

    #[test]
    fn is_error_present_decides_and_absent_reads_tool_use_result() {
        let object = json!({"type": "user", "tool_use_result": {"type": "text"}});
        let error_string = json!({"type": "user", "tool_use_result": "Error: x"});
        let none = json!({"type": "user"});
        let string_on_ok = json!({
            "type": "user",
            "tool_use_result": "Error: this would be Error if is_error were absent",
        });

        assert_eq!(result_status(&block(None), &object), ResultStatus::Ok);
        assert_eq!(
            result_status(&block(None), &error_string),
            ResultStatus::Error
        );
        assert_eq!(
            result_status(&block(None), &none),
            ResultStatus::Unclassified
        );
        assert_eq!(
            result_status(&block(Some(false)), &string_on_ok),
            ResultStatus::Ok
        );
        assert_eq!(
            result_status(&block(Some(true)), &object),
            ResultStatus::Error
        );
    }
}
