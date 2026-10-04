use std::collections::HashMap;

use serde_json::Value;

use super::{
    Correction, Counts, Decider, Decision, Event, Message, PermissionDecision, ResultStatus, Role,
    Run, RunEnd, RunStart, Usage, UsageSource, message_blocks, splice,
};
use anyhow::Result;

use crate::policy::PolicySet;

/// The `result.usage` keys this walk also accumulates, one assistant
/// record at a time rather than one terminator frame (contract 7.2.1: "
/// `usage` rides `message.usage` on the assistant records rather than a
/// `result` frame").
const COUNT_FIELDS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
];

/// How many hops the `parentUuid` chain walk takes before it gives up.
///
/// The chain is a session's own linear history, so the walk answers in a
/// handful of steps on every measured transcript; this is a backstop
/// against a malformed file rather than a bound anyone expects to hit.
const MAX_CHAIN_HOPS: usize = 500;

pub(super) fn normalize(text: &str, policies: &PolicySet) -> Result<Run> {
    let mut walk = Walk::default();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(envelope) => walk.line(&envelope),
            Err(err) => walk.findings.push(super::Finding::Limit(format!(
                "line {} did not parse, so whatever it carried is unread: {err}",
                index + 1
            ))),
        }
    }
    walk.finish(policies)
}

/// One tool call's name and input, read off a `tool_use` block, which is
/// the shape both halves of a correction pair share.
struct ToolUse {
    name: String,
    input: Value,
}

#[derive(Default)]
struct Walk {
    run_id: String,
    parent_run_id: Option<String>,
    provider: Option<String>,
    policy: Option<String>,
    policy_sha: Option<String>,
    events: Vec<Event>,
    findings: Vec<super::Finding>,
    noise: Vec<String>,
    /// Every transcript record, `seq` beside it, in file order. Kept
    /// whole rather than folded into `events` as it arrives, because
    /// deriving a correction needs to look both ways along the chain:
    /// backward to the call a denial rejected, forward to the retry a
    /// human asked for.
    records: Vec<(u64, Value)>,
    usage: Counts,
    usage_emitted: bool,
    usage_seen: bool,
    last_usage_seq: Option<u64>,
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
            match hydra["type"].as_str() {
                Some("run_start") => {
                    self.policy = hydra["policy"].as_str().map(str::to_string);
                    self.policy_sha = hydra["policy_sha"].as_str().map(str::to_string);
                    self.events.push(Event::RunStart(RunStart {
                        seq,
                        body: hydra.clone(),
                    }));
                }
                Some("run_end") => {
                    self.emit_usage();
                    self.events.push(Event::RunEnd(RunEnd {
                        seq,
                        body: hydra.clone(),
                    }));
                }
                _ => {}
            }
            return;
        }
        let Some(raw) = envelope.get("raw") else {
            return;
        };
        let Some(record) = raw.as_object() else {
            // Contract 3: a line the provider wrote that is not JSON is
            // kept as a string and normalizes to nothing.
            if let Some(text) = raw.as_str() {
                self.noise.push(text.to_string());
            }
            return;
        };
        self.record(seq, record, raw);
    }

    /// One transcript record. `mode`, `permission-mode`, `attachment`,
    /// `file-history-snapshot`, `ai-title`, `last-prompt`, `bridge-session`
    /// and `atis-latch` stay verbatim in `raw` (contract 7.2.1's "eight
    /// transcript record types no stream has") and normalize to nothing
    /// here, the same as `summary` and the transcript's own `system`
    /// entries.
    fn record(&mut self, seq: u64, record: &serde_json::Map<String, Value>, raw: &Value) {
        match record.get("type").and_then(Value::as_str) {
            Some("assistant") => self.message(seq, raw, Role::Assistant),
            Some("user") => self.message(seq, raw, Role::User),
            _ => {}
        }
        self.records.push((seq, raw.clone()));
    }

    /// A `user` or `assistant` record's `message.content`, which is a bare
    /// string on a human's own typed turn and a block array everywhere
    /// else (7.9's step 4 traversal relies on exactly this split).
    fn message(&mut self, seq: u64, raw: &Value, role: Role) {
        match &raw["message"]["content"] {
            Value::String(text) => self.events.push(Event::Message(Message {
                seq,
                role,
                text: text.clone(),
                block: 0,
            })),
            Value::Array(blocks) => {
                let (events, unknown) = message_blocks(seq, raw, blocks, role);
                self.events.extend(events);
                // No run-time tool table to check a name against here
                // (7.1's check reads `system/init`'s tool list, which no
                // transcript record carries), so an unrecognised name is
                // still worth a finding rather than a silent capability
                // of `None`.
                for name in unknown {
                    self.findings.push(super::Finding::UnknownTool { name });
                }
            }
            _ => {}
        }
        if role == Role::Assistant {
            self.accumulate_usage(seq, &raw["message"]["usage"]);
        }
    }

    /// One assistant turn's `message.usage`, folded into a running total.
    ///
    /// **Summed rather than kept per turn.** Contract 7.8 scopes `usage`
    /// to a run, and an interactive session is many API calls, one per
    /// turn; the number that answers "what did this run cost" is every
    /// turn's input, output and cache movement added together, the same
    /// total a single `result.usage` would have named on a recorded run.
    fn accumulate_usage(&mut self, seq: u64, usage: &Value) {
        if !usage.is_object() {
            return;
        }
        let has_any = COUNT_FIELDS
            .iter()
            .any(|field| usage.get(*field).and_then(Value::as_u64).is_some());
        if !has_any {
            return;
        }
        self.usage.input_tokens += usage["input_tokens"].as_u64().unwrap_or_default();
        self.usage.output_tokens += usage["output_tokens"].as_u64().unwrap_or_default();
        self.usage.cache_read_tokens += usage["cache_read_input_tokens"]
            .as_u64()
            .unwrap_or_default();
        self.usage.cache_write_tokens = Some(
            self.usage.cache_write_tokens.unwrap_or_default()
                + usage["cache_creation_input_tokens"]
                    .as_u64()
                    .unwrap_or_default(),
        );
        if let Some(thinking) = usage["output_tokens_details"]["thinking_tokens"].as_u64() {
            self.usage.thinking_tokens =
                Some(self.usage.thinking_tokens.unwrap_or_default() + thinking);
        }
        self.usage_seen = true;
        self.last_usage_seq = Some(seq);
    }

    fn emit_usage(&mut self) {
        if self.usage_emitted {
            return;
        }
        self.usage_emitted = true;
        self.events.push(Event::Usage(if self.usage_seen {
            Usage {
                seq: self.last_usage_seq,
                source: UsageSource::Provider,
                estimator: None,
                counts: Some(self.usage),
            }
        } else {
            Usage {
                seq: None,
                source: UsageSource::Unavailable,
                estimator: None,
                counts: None,
            }
        }));
    }

    fn finish(mut self, policies: &PolicySet) -> Result<Run> {
        super::super::note_missing_run_start(&self.events, &mut self.findings);
        super::super::note_capped_lines(&self.events, &mut self.findings);
        super::super::note_policy_sha(
            self.policy.as_deref(),
            self.policy_sha.as_deref(),
            policies,
            &mut self.findings,
        );
        self.emit_usage();
        let corrections = self.derive_corrections();
        self.splice_corrections(corrections);
        let decisions = self.derive_policy_decisions();
        if self.policy.is_some() {
            self.findings.push(super::Finding::Limit(
                "allows were not recorded on this launch mode".to_string(),
            ));
        }
        let events = splice(self.events, decisions);
        Ok(Run {
            run_id: self.run_id,
            parent_run_id: self.parent_run_id,
            provider: self.provider,
            policy: self.policy,
            events,
            findings: self.findings,
            // No `result` frame exists in a transcript at all (7.2.1);
            // `usage` above is what stands in for the fields 8.5 keeps on
            // it.
            result: None,
            noise: self.noise,
        })
    }

    /// Map ticket 50: every error `tool_result` carrying Hydra's voice
    /// becomes one `permission_decision` of `deny` under `hydra_policy`,
    /// joined to its call by `tool_use_id`.
    ///
    /// **`hydra_voice` and not `starts_with("hydra/")`, since build ticket
    /// 55.** The stream path has validated the code against [`GATE_CODES`]
    /// since ticket 01 and this one did not, so the two halves of the same
    /// reader disagreed about what counts as Hydra speaking. The loose test
    /// reads `hydra/runs/x.jsonl: permission denied`, which is a guard
    /// quoting a path, as a gate that denied the call.
    fn derive_policy_decisions(&self) -> Vec<Option<PermissionDecision>> {
        if self.policy.is_none() {
            return Vec::new();
        }
        let results = super::super::results_by_call(&self.events);
        let mut decisions = Vec::new();
        for event in &self.events {
            if let Event::ToolCall(call) = event {
                let decision = results.get(&call.call_id).and_then(|(status, output)| {
                    (*status == ResultStatus::Error
                        && crate::normalize::hydra_voice(output).is_some())
                    .then(|| PermissionDecision {
                        seq: call.seq,
                        call_id: call.call_id.clone(),
                        decision: Decision::Deny,
                        decider: Decider::HydraPolicy,
                        reason: output.clone(),
                    })
                });
                decisions.push(decision);
            }
        }
        decisions
    }

    /// 7.9's rule, over every denial the transcript carries.
    fn derive_corrections(&self) -> Vec<(u64, Correction)> {
        let mut by_uuid: HashMap<&str, &Value> = HashMap::new();
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        for (_, record) in &self.records {
            let Some(uuid) = record["uuid"].as_str() else {
                continue;
            };
            by_uuid.insert(uuid, record);
            if let Some(parent) = record["parentUuid"].as_str() {
                children.entry(parent).or_default().push(uuid);
            }
        }

        let mut out = Vec::new();
        for (seq, record) in &self.records {
            if record["type"].as_str() != Some("user") {
                continue;
            }
            // Step 2, ticket 35's corrected conjunction: both conditions,
            // never one field alone. `user-rejected` alone also means the
            // ask path answered under `sdk-cli`, which is code rather
            // than a human.
            if record["toolDenialKind"].as_str() != Some("user-rejected")
                || record["entrypoint"].as_str() != Some("cli")
            {
                continue;
            }
            let Some(source_uuid) = record["sourceToolAssistantUUID"].as_str() else {
                continue;
            };
            let Some(tool_use_id) = denied_tool_use_id(record) else {
                continue;
            };
            let Some(source_record) = by_uuid.get(source_uuid).copied() else {
                continue;
            };
            let Some(rejected) = tool_use_by_id(source_record, tool_use_id) else {
                continue;
            };
            let Some(uuid) = record["uuid"].as_str() else {
                continue;
            };

            // Steps 4 and 5: walk the chain once for the human's words and
            // once more, from wherever that left off, for the retry.
            let (words, chosen) = walk_for_reply(uuid, &by_uuid, &children);
            let Some(chosen) = chosen else {
                continue;
            };
            // Step 6: a pair is emitted only when the retry names the
            // same tool. A text reply means the agent argued instead of
            // retrying, which step 5 already turned into `None`.
            if chosen.name != rejected.name {
                continue;
            }

            out.push((
                *seq,
                Correction {
                    corrects: *seq,
                    rejected: rejected.input,
                    chosen: chosen.input,
                    words,
                },
            ));
        }
        out
    }

    /// Inserts each correction right after the `tool_result` event of the
    /// denial it corrects, so `hydra show --events` still reads in file
    /// order. Reverse order, so an earlier insertion never shifts the
    /// index a later one already found.
    fn splice_corrections(&mut self, mut corrections: Vec<(u64, Correction)>) {
        corrections.sort_by_key(|(seq, _)| std::cmp::Reverse(*seq));
        for (seq, correction) in corrections {
            let at = self
                .events
                .iter()
                .position(|event| matches!(event, Event::ToolResult(result) if result.seq == seq));
            match at {
                Some(index) => self.events.insert(index + 1, Event::Correction(correction)),
                None => self.events.push(Event::Correction(correction)),
            }
        }
    }
}

/// The `tool_use_id` a denial's own `tool_result` block names, which is
/// the join key back to the call it rejected.
fn denied_tool_use_id(record: &Value) -> Option<&str> {
    record["message"]["content"]
        .as_array()?
        .iter()
        .find(|block| block["type"] == "tool_result")
        .and_then(|block| block["tool_use_id"].as_str())
}

/// The `tool_use` block on `record` whose `id` matches, if any.
fn tool_use_by_id(record: &Value, id: &str) -> Option<ToolUse> {
    record["message"]["content"]
        .as_array()?
        .iter()
        .find(|block| block["type"] == "tool_use" && block["id"].as_str() == Some(id))
        .map(tool_use_of)
}

/// The first `tool_use` block on `record`, if it holds one at all.
fn first_tool_use(record: &Value) -> Option<ToolUse> {
    record["message"]["content"]
        .as_array()?
        .iter()
        .find(|block| block["type"] == "tool_use")
        .map(tool_use_of)
}

fn tool_use_of(block: &Value) -> ToolUse {
    ToolUse {
        name: block["name"].as_str().unwrap_or_default().to_string(),
        input: block["input"].clone(),
    }
}

/// Ticket 35's steps 4 and 5: walk `parentUuid`'s children from the
/// denial forward, first for a plain-string human turn, then for the
/// first assistant record holding a `tool_use`.
///
/// **The interrupt marker and a `tool_result` echo both fail the
/// plain-string test on purpose.** `[Request interrupted by user for
/// tool use]` and a `tool_result` block both arrive as a block array, not
/// a bare string, which is the split step 4 measured rather than a
/// special case for either one.
///
/// **The walk stops at the first assistant record, whatever it holds.**
/// One that carries no `tool_use` is a text reply: the model argued
/// instead of retrying, which is a rejection without a correction, and
/// nothing forces the walk to keep looking for one further on.
fn walk_for_reply<'a>(
    start: &'a str,
    by_uuid: &HashMap<&'a str, &'a Value>,
    children: &HashMap<&'a str, Vec<&'a str>>,
) -> (Option<String>, Option<ToolUse>) {
    let mut words = None;
    let mut current = start;
    for _ in 0..MAX_CHAIN_HOPS {
        let Some(next) = children.get(current).and_then(|kids| kids.first()).copied() else {
            break;
        };
        let Some(record) = by_uuid.get(next).copied() else {
            break;
        };
        current = next;
        match record["type"].as_str() {
            Some("user") if words.is_none() => {
                if let Value::String(text) = &record["message"]["content"] {
                    words = Some(text.clone());
                }
            }
            Some("assistant") => {
                return (words, first_tool_use(record));
            }
            _ => {}
        }
    }
    (words, None)
}
