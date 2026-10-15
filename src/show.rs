//! `hydra show <run-id> [--events]`, harness spec 17's full record.
//!
//! Harness spec 9 keeps a delegation result to contract 17's nine fields on
//! purpose, so a child's trajectory stays out of its parent's context. This
//! command is where a parent that wants more asks for it, explicitly: the
//! whole run, normalized, with the same projection `hydra status` prints
//! beside it.
//!
//! Without `--events` this prints only the projection, the object contract 17
//! already gives `hydra run` and `hydra status <id>` under `result`. With it,
//! this adds `run_id`, the events in file order and the findings the
//! normalizer raised. Nothing here writes anything, and nothing here decides
//! anything a policy has not already decided.
//!
//! **Every field the run's own model or tools wrote comes out marked**, map 105
//! D3: `result_text`, and the `text` of a message, a delta or a thinking block
//! and the `output` of a tool result under `--events`, each inside the boundary
//! the delegate face draws (`marker.rs`), always. This command's reader is
//! usually an orchestrator no gate binds. `error` is left plain: in this build
//! only Hydra writes it, and the outcome derivation and the readers match on its
//! prefixes. The machine readers strip the boundary in `hydra_pyjson::marker`.

use std::io::Write;

use anyhow::Result;
use serde_json::{Value, json};

use crate::marker::mark;
use crate::normalize::{Counts, Event, Finding};
use crate::paths::Layout;
use crate::policy::{Capability, PolicySet};

/// `out` is where the JSON goes, stdout for the command and a buffer for the
/// suite.
pub fn show(layout: &Layout, run_id: &str, events: bool, out: &mut dyn Write) -> Result<()> {
    let run_id = &crate::resolver::resolve(layout, run_id)?;
    // One token per call, drawn now: every word below was on disk before it existed.
    let token = crate::marker::token();
    let mut projection = crate::status::read_projection(layout, run_id)?;
    if let Some(text) = projection["result_text"].as_str() {
        projection["result_text"] = Value::String(mark(text, &token));
    }
    if !events {
        writeln!(out, "{}", serde_json::to_string_pretty(&projection)?)?;
        return Ok(());
    }

    let policies = PolicySet::discover()?;
    let run = crate::normalize::read_file(&layout.run_file(run_id), &policies)?;
    let report = json!({
        "run_id": run_id,
        "projection": projection,
        "events": run.events.iter().map(|e| event_json(e, &token)).collect::<Vec<Value>>(),
        "findings": run.findings.iter().map(finding_json).collect::<Vec<Value>>(),
    });
    writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
    Ok(())
}

/// One event, as `seq`, `type` off [`Event::type_name`], and that variant's
/// own fields by name, with the run's own words marked under `token`.
///
/// `thinking` carries `signed` rather than the signature itself. The bytes are
/// `claude`'s to keep, and a reader here only needs to know one arrived.
/// `tool_result.output` is untruncated inside its boundary, since contract 7.6 makes
/// the truncation marker the reader's to see rather than something this cuts
/// again. `run_start` and `run_end` carry the body the record holds, the same
/// object a launch wrote.
fn event_json(event: &Event, token: &str) -> Value {
    let type_name = event.type_name();
    match event {
        Event::RunStart(e) => json!({"seq": e.seq, "type": type_name, "body": e.body}),
        Event::RunEnd(e) => json!({"seq": e.seq, "type": type_name, "body": e.body}),
        Event::GateRefusal(e) => {
            let mut body = e.body.clone();
            if let Some(command) = body["command"].as_str() {
                body["command"] = Value::String(mark(command, token));
            }
            json!({"seq": e.seq, "type": type_name, "body": body})
        }
        Event::Message(e) => json!({
            "seq": e.seq,
            "type": type_name,
            "role": e.role.as_str(),
            "text": mark(&e.text, token),
            "block": e.block,
        }),
        Event::MessageDelta(e) => json!({
            "seq": e.seq,
            "type": type_name,
            "role": e.role.as_str(),
            "text": mark(&e.text, token),
        }),
        Event::Thinking(e) => json!({
            "seq": e.seq,
            "type": type_name,
            "text": mark(&e.text, token),
            "signed": e.signature.is_some(),
            "block": e.block,
        }),
        Event::ToolCall(e) => json!({
            "seq": e.seq,
            "type": type_name,
            "call_id": e.call_id,
            "name": e.name,
            "arguments": e.arguments,
            "capability": e.capability.map(Capability::as_str),
        }),
        Event::ToolResult(e) => json!({
            "seq": e.seq,
            "type": type_name,
            "call_id": e.call_id,
            "status": e.status.as_str(),
            "output": mark(&e.output, token),
            "truncated": e.truncated,
        }),
        Event::PermissionDecision(e) => json!({
            "seq": e.seq,
            "type": type_name,
            "call_id": e.call_id,
            "decision": e.decision.as_str(),
            "decider": e.decider.as_str(),
            "reason": e.reason,
        }),
        Event::Usage(e) => json!({
            "seq": e.seq,
            "type": type_name,
            "source": e.source.as_str(),
            "estimator": e.estimator,
            "counts": e.counts.map(counts_json),
        }),
        // No producer in this build, contract 7.9. Included so a reader
        // grouping by `Event::type_name` never meets a name it cannot find
        // here.
        Event::Correction(e) => json!({
            "type": type_name,
            "corrects": e.corrects,
            "rejected": e.rejected,
            "chosen": e.chosen,
            "words": e.words,
        }),
    }
}

fn counts_json(counts: Counts) -> Value {
    json!({
        "input_tokens": counts.input_tokens,
        "output_tokens": counts.output_tokens,
        "cache_read_tokens": counts.cache_read_tokens,
        "cache_write_tokens": counts.cache_write_tokens,
        "thinking_tokens": counts.thinking_tokens,
    })
}

/// One finding, its own type name and its fields by name, the same shape
/// [`event_json`] gives an event.
fn finding_json(finding: &Finding) -> Value {
    match finding {
        Finding::GateLeak {
            call_id,
            tool,
            reason,
        } => json!({"type": "gate_leak", "call_id": call_id, "tool": tool, "reason": reason}),
        Finding::SubagentLeak { spawned } => json!({"type": "subagent_leak", "spawned": spawned}),
        Finding::Disagreement {
            call_id,
            derived,
            derived_code,
            gate_said,
            gate_code,
        } => json!({
            "type": "disagreement",
            "call_id": call_id,
            "derived": derived.as_str(),
            "derived_code": derived_code,
            "gate_said": gate_said.as_str(),
            "gate_code": gate_code,
        }),
        Finding::Ungated { call_id, tool } => {
            json!({"type": "ungated", "call_id": call_id, "tool": tool})
        }
        Finding::UnknownTool { name } => json!({"type": "unknown_tool", "name": name}),
        Finding::StalePolicy { recorded, current } => {
            json!({"type": "stale_policy", "recorded": recorded, "current": current})
        }
        Finding::Limit(text) => json!({"type": "limit", "text": text}),
        Finding::UnhookedDenied {
            call_id,
            tool,
            reason,
        } => json!({
            "type": "unhooked_denied",
            "call_id": call_id,
            "tool": tool,
            "reason": reason,
        }),
    }
}
