//! The run graph, build ticket 271: what `integrations/herdr/bin/hydra-graph`
//! drew in the Herdr pane, as a page of the cockpit.
//!
//! One line per event of the open run, in record order, and each tool call's
//! result drawn under it whatever came between. It is built from the same
//! walk as the detail's calls, in [`super::snapshot::open_run_of`], so the
//! graph adds no reader of the record. The Python read `hydra show --events`
//! and printed the replay's `permission_decision` on the call; this has no
//! policy set, so a call carries what its result says instead, the decision
//! the detail shows.

use std::collections::{HashMap, HashSet};

use crate::normalize::{Event, ToolResult};

use super::call_head::{call_inputs, mask_secrets};

/// Where a result's first line is cut.
const RESULT_CUT: usize = 120;

/// What an input or output on a line the secret sweep flagged shows instead.
const HIDDEN: &str = "input hidden: secret_flag";

/// One row of the graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphLine {
    /// The record line the event came from.
    pub seq: u64,
    /// The contract's type name, `tool_call`, `message` and so on.
    pub kind: &'static str,
    pub text: String,
    /// A tool result, drawn under its call.
    pub under_call: bool,
    /// A tool call whose result carries one of the gate's refusals.
    pub refused: bool,
}

/// Every event in `events` as a graph line, each result under its call.
/// A usage with no frame has no `seq` and no line, as in the Python.
///
/// A result answers the latest earlier call of its id still open, as
/// [`super::snapshot::calls_of`] joins them, and one with no call is dropped.
/// `decision` is what a result says of its call.
pub fn graph_of(
    events: &[Event],
    flagged: &HashSet<u64>,
    decision: impl Fn(&ToolResult) -> String,
) -> Vec<GraphLine> {
    let mut answered: HashMap<usize, &ToolResult> = HashMap::new();
    let mut open: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, event) in events.iter().enumerate() {
        match event {
            Event::ToolCall(call) if !call.call_id.is_empty() => {
                open.entry(call.call_id.as_str()).or_default().push(index);
            }
            Event::ToolResult(result) => {
                if let Some(call) = open.get_mut(result.call_id.as_str()).and_then(Vec::pop) {
                    answered.insert(call, result);
                }
            }
            _ => {}
        }
    }
    let mut lines = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let (seq, text) = match event {
            Event::ToolResult(_) | Event::PermissionDecision(_) | Event::GateRefusal(_) => continue,
            Event::ToolCall(call) => {
                let head = if flagged.contains(&call.seq) {
                    HIDDEN.to_string()
                } else {
                    call_head(&call.name, &call_inputs(&call.name, &call.arguments).0)
                };
                let result = answered.get(&index);
                let said = result.map(|result| decision(result));
                let refused = said.as_deref().is_some_and(|d| d.starts_with("refused"));
                let text = match &said {
                    Some(said) => format!("{head}  {said}"),
                    None => head,
                };
                lines.push(GraphLine {
                    seq: call.seq,
                    kind: event.type_name(),
                    text: mask_secrets(&text),
                    under_call: false,
                    refused,
                });
                if let Some(result) = result {
                    lines.push(GraphLine {
                        seq: result.seq,
                        kind: "tool_result",
                        text: result_text(result, flagged),
                        under_call: true,
                        refused: false,
                    });
                }
                continue;
            }
            Event::Message(message) => (
                message.seq,
                format!(
                    "{}: {}",
                    message.role.as_str(),
                    first_line(&mask_secrets(&message.text))
                ),
            ),
            Event::Thinking(thinking) => (
                thinking.seq,
                format!(
                    "len={} {} {}",
                    thinking.text.chars().count(),
                    if thinking.text.is_empty() {
                        "empty"
                    } else {
                        "text"
                    },
                    if thinking.signature.is_some() {
                        "signed"
                    } else {
                        "unsigned"
                    },
                ),
            ),
            Event::Usage(usage) => (
                match usage.seq {
                    Some(seq) => seq,
                    None => continue,
                },
                format!(
                    "total={}",
                    usage.counts.map_or(0, |counts| counts.total_tokens())
                ),
            ),
            Event::RunStart(start) => (
                start.seq,
                format!(
                    "policy={} mode={} depth={}",
                    field(&start.body, "policy"),
                    field(&start.body, "launch_mode"),
                    field(&start.body, "depth"),
                ),
            ),
            Event::RunEnd(end) => {
                let refusal = field(&end.body, "refusal");
                (
                    end.seq,
                    format!(
                        "exit_code={} refusal={}",
                        field(&end.body, "exit_code"),
                        if refusal.is_empty() { "none" } else { &refusal },
                    ),
                )
            }
            Event::MessageDelta(delta) => (delta.seq, String::new()),
            Event::Correction(correction) => (correction.corrects, String::new()),
        };
        lines.push(GraphLine {
            seq,
            kind: event.type_name(),
            text: if flagged.contains(&seq) {
                HIDDEN.to_string()
            } else {
                mask_secrets(&text)
            },
            under_call: false,
            refused: false,
        });
    }
    lines
}

/// The detail's head for a call, with the tool's name in front when the head
/// is a shell command that does not carry it.
fn call_head(name: &str, head: &str) -> String {
    if head.starts_with(name) {
        head.to_string()
    } else {
        format!("{name} {head}")
    }
}

fn result_text(result: &ToolResult, flagged: &HashSet<u64>) -> String {
    let status = result.status.as_str();
    if flagged.contains(&result.seq) {
        return format!("{status}: {HIDDEN}");
    }
    let masked = mask_secrets(&result.output);
    let line = first_line(&masked);
    if line.is_empty() {
        return status.to_string();
    }
    let cut: String = line.chars().take(RESULT_CUT).collect();
    if cut.len() < line.len() {
        format!("{status}: {cut}...")
    } else {
        format!("{status}: {cut}")
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// A field of a line 0 or `run_end` body as text, empty when absent or null.
fn field(body: &serde_json::Value, name: &str) -> String {
    match &body[name] {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}
