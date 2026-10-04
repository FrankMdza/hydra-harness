//! `monitor.py`: a logistic monitor over a prefix of a run record (build ticket 215).
//!
//! `baseline.rs` predicts from what is known before a launch. This predicts from that plus
//! the first fraction of the record's own lines. Nothing here stops a run.
//!
//! The fractions are 0.1, 0.25 and 0.5. A prefix is the first
//! `max(1, floor(fraction * lines))` lines, so a record of one line is read whole and a
//! record of ten lines gives 1, 2 and 5 lines.
//!
//! A record's frames are whatever a provider wrote, so the walk does with a value of the
//! wrong shape what the Python does: it steps over it where the Python tests the type
//! first, and returns an error whose text starts with the class Python raises where the
//! Python lets the exception through. A tool's name is kept as the JSON value it was,
//! because the Python never asks for a string and `features` prints it back.
//!
//! The split, the floor, the fit, the measures and the report are `baseline.rs`'s, called
//! and not copied. The arguments and the three commands are in `monitor_cli.rs`.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::json::{self, Dict, ParseError, PyStr, Value};
use hydra_pyjson::math::{self, fsum_like_python};
use hydra_pyjson::models::Pair;
use hydra_pyjson::py;
use sha2::{Digest, Sha256};

use crate::baseline::{self, Label, Refusal};
use hydra_records::isotime::{self, DateTime};
use hydra_records::records;

/// The three prefixes a model can be trained at.
pub const FRACTIONS: [f64; 3] = [0.1, 0.25, 0.5];
/// Counts that vary in size. One-hot names and `seat_requested` stay as 0 or 1, the same
/// way the baseline leaves provider and policy unscaled and scales only `log_task_chars`.
const SCALE_NAMES: [&str; 7] = [
    "log_task_chars",
    "events_so_far",
    "tool_calls_so_far",
    "refusals_so_far",
    "distinct_tools",
    "output_tokens_so_far",
    "seconds_elapsed",
];
/// What a row's prefix adds to the baseline's features, in the order the Python's dict
/// holds them.
const PREFIX_NAMES: [&str; 7] = [
    "events_so_far",
    "tool_calls_so_far",
    "refusals_so_far",
    "distinct_tools",
    "output_tokens_so_far",
    "seconds_elapsed",
    "seat_requested",
];
/// `cursor-agent` names a call by the key on `tool_call`. The spellings are the ones
/// `src/normalize/cursor.rs` maps; an unlisted `*ToolCall` key keeps its stem.
const CURSOR_TOOLS: [(&str, &str); 9] = [
    ("readToolCall", "Read"),
    ("shellToolCall", "Shell"),
    ("editToolCall", "Write"),
    ("deleteToolCall", "Delete"),
    ("grepToolCall", "Grep"),
    ("globToolCall", "Grep"),
    ("lsToolCall", "List"),
    ("webSearchToolCall", "WebSearch"),
    ("webFetchToolCall", "Fetch"),
];
const CODEX_TOOLS: [(&str, &str); 4] = [
    ("command_execution", "Bash"),
    ("web_search", "WebSearch"),
    ("apply_patch", "ApplyPatch"),
    ("file_change", "ApplyPatch"),
];
const TOOL_CALL: &str = "ToolCall";

static NULL: Value = Value::Null;

/// `format(fraction, "g")`, which for the three fractions is their `repr`.
pub fn fraction_token(fraction: f64) -> String {
    json::float_repr(fraction)
}

/// The fraction `value` names: the one of the three within 1e-9 of it.
pub fn allowed(value: f64) -> Option<f64> {
    FRACTIONS
        .into_iter()
        .find(|allowed| (value - allowed).abs() <= 1e-9)
}

/// `float(text)`, `None` where it raises `ValueError`. `float` reads more than Rust's
/// parser does: whitespace around the number, single underscores between digits, and the
/// decimal digits of any script.
fn py_float(text: &str) -> Option<f64> {
    let mut ascii = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            ascii.push(c);
        } else if py::is_space(c) {
            ascii.push(' ');
        } else {
            ascii.push(py::decimal_digit(c)?);
        }
    }
    let body = py::strip_str(&ascii).as_bytes();
    let mut digits = String::with_capacity(body.len());
    for (i, byte) in body.iter().enumerate() {
        if *byte == b'_' {
            let digit = |at: Option<&u8>| at.is_some_and(u8::is_ascii_digit);
            if i == 0 || !digit(body.get(i - 1)) || !digit(body.get(i + 1)) {
                return None;
            }
        } else {
            digits.push(*byte as char);
        }
    }
    digits.parse().ok()
}

/// `parse_fraction`: the fraction `--at` names, `None` where `argparse` refuses it.
pub fn parse_fraction(text: &str) -> Option<f64> {
    py_float(text).and_then(allowed)
}

/// `prefix_len`: how many of `n` lines a fraction reads.
fn prefix_len(n: usize, fraction: f64) -> usize {
    if n == 0 {
        return 0;
    }
    let floor = (fraction * n as f64).floor() as usize;
    floor.max(1).min(n)
}

fn find(hay: &[u32], needle: &[u32]) -> Option<usize> {
    hay.windows(needle.len())
        .position(|window| window == needle)
}

/// `is_denial`: Hydra's own refusal voice. `hydra/allow` is the gate allowing a call.
fn is_denial(text: &[u32]) -> bool {
    let voice = PyStr::from("hydra/");
    let allow = PyStr::from("hydra/allow");
    if find(text, voice.code_points()).is_none() {
        return false;
    }
    // `text.replace("hydra/allow", "")`: what is left can spell `hydra/` across a cut.
    let mut left = Vec::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = find(rest, allow.code_points()) {
        left.extend_from_slice(&rest[..at]);
        rest = &rest[at + allow.len()..];
    }
    left.extend_from_slice(rest);
    find(&left, voice.code_points()).is_some()
}

/// `value.get(key)` on a dict.
fn at<'a>(dict: &'a Dict, key: &str) -> &'a Value {
    dict.get(key).unwrap_or(&NULL)
}

fn as_dict(value: &Value) -> Option<&Dict> {
    match value {
        Value::Dict(dict) => Some(dict),
        _ => None,
    }
}

/// `value if isinstance(value, str) else None`.
fn as_str(value: &Value) -> Option<&PyStr> {
    match value {
        Value::Str(text) => Some(text),
        _ => None,
    }
}

/// `parse_ts`: a line's `ts`, `None` when it is not text or does not read.
fn parse_ts(line: &Dict) -> Option<DateTime> {
    let ts = as_str(at(line, "ts"))?;
    let mut stamp = PyStr::new();
    for code in ts.code_points() {
        if *code == u32::from('Z') {
            stamp.push_str("+00:00");
        } else {
            stamp.push_code(*code);
        }
    }
    isotime::fromisoformat(&stamp)
}

/// `int(value)` on a float, which is exact however large the float is.
fn int_of_float(x: f64) -> Result<Value> {
    if x.is_nan() {
        bail!("ValueError: cannot convert float NaN to integer");
    }
    if x.is_infinite() {
        bail!("OverflowError: cannot convert float infinity to integer");
    }
    if x.abs() < 1e38 {
        return Ok(Value::Int(x.trunc() as i128));
    }
    // Past 2^53 a float is a whole number, and `{:.0}` prints every digit of it.
    Ok(Value::BigInt(format!("{x:.0}")))
}

/// `output_tokens`: the latest usage report on this frame, or null when it carries none.
///
/// Providers park the object in different places: on the frame (`claude`'s `result`,
/// `codex`'s `turn.completed`), under `result`, under `step_update` (`agy`) and under
/// `message` (a harvested `claude` transcript). The last one on the frame wins. A running
/// total and a per-step count are not added.
fn output_tokens(raw: &Dict) -> Result<Value> {
    let mut found = Value::Null;
    let mut containers = vec![at(raw, "usage")];
    for key in ["result", "step_update", "message"] {
        if let Some(inner) = as_dict(at(raw, key)) {
            containers.push(at(inner, "usage"));
        }
    }
    for container in containers {
        let Some(container) = as_dict(container) else {
            continue;
        };
        let value = container
            .get("output_tokens")
            .unwrap_or_else(|| at(container, "outputTokens"));
        match value {
            Value::Int(_) | Value::BigInt(_) => found = value.clone(),
            Value::Float(x) => found = int_of_float(*x)?,
            _ => {}
        }
    }
    Ok(found)
}

fn ends_with_tool_call(key: &PyStr) -> bool {
    key.code_points()
        .ends_with(PyStr::from(TOOL_CALL).code_points())
}

/// `cursor_name`: the tool a `tool_call` object names, null when it names none.
fn cursor_name(tool_call: &Dict) -> Value {
    for (key, name) in CURSOR_TOOLS {
        if tool_call.get(key).is_some() {
            return Value::text(name);
        }
    }
    for (key, _) in tool_call.iter() {
        if ends_with_tool_call(key) {
            return Value::Str(key.head(key.len() - TOOL_CALL.len()));
        }
    }
    Value::Null
}

/// `cursor_args`: the input of the first `*ToolCall` that is an object.
fn cursor_args(tool_call: &Dict) -> Value {
    for (key, value) in tool_call.iter() {
        if let Value::Dict(body) = value
            && ends_with_tool_call(key)
        {
            // The call id rides inside the args and differs on every call, so a repeat
            // would never match on it.
            return match at(body, "args") {
                Value::Dict(args) => {
                    let mut kept = Dict::new();
                    for (name, item) in args.iter().filter(|(name, _)| *name != "toolCallId") {
                        kept.push(name.clone(), item.clone());
                    }
                    Value::Dict(kept)
                }
                other => other.clone(),
            };
        }
    }
    Value::Null
}

fn cursor_denied(tool_call: &Dict) -> bool {
    for (_, value) in tool_call.iter() {
        let Some(body) = as_dict(value) else {
            continue;
        };
        let result = match at(body, "result") {
            Value::Str(text) if is_denial(text.code_points()) => return true,
            Value::Dict(result) => result,
            _ => continue,
        };
        for part in ["error", "failure"] {
            if let Some(found) = result.get(part)
                && is_denial(PyStr::from(json::dumps_default(found)).code_points())
            {
                return true;
            }
        }
    }
    false
}

/// `hook_denial`: the denied tool, or `""` when the hook denied and named no tool. `None`
/// means this frame is not a denial. A following `tool_result` repeats the same denial in
/// prose; the hook is the one that counts.
fn hook_denial(raw: &Dict) -> Result<Option<Value>> {
    if !py::eq_str(at(raw, "subtype"), "hook_response") {
        return Ok(None);
    }
    let parsed = match at(raw, "output") {
        Value::Dict(output) => Value::Dict(output.clone()),
        Value::Str(output) => match json::loads_explained(output) {
            Ok(parsed) => parsed,
            Err((ParseError::Decode, _)) => return Ok(None),
            // An integer past 4,300 digits is a `ValueError` the Python does not catch.
            Err((err, message)) => bail!("{}: {message}", err.class()),
        },
        _ => return Ok(None),
    };
    let Some(parsed) = as_dict(&parsed) else {
        return Ok(None);
    };
    let specific = match at(parsed, "hookSpecificOutput") {
        inner if py::truthy(inner) => inner,
        _ => &Value::Dict(parsed.clone()),
    };
    let denied = as_dict(specific)
        .is_some_and(|specific| py::eq_str(at(specific, "permissionDecision"), "deny"));
    if !denied {
        return Ok(None);
    }
    // `":" in hook_name`, then `hook_name.split(":", 1)[1]`, on whatever the frame holds.
    let no_split = |value: &Value| {
        anyhow!(
            "AttributeError: '{}' object has no attribute 'split'",
            py::type_name(value)
        )
    };
    match at(raw, "hook_name") {
        name if !py::truthy(name) => Ok(Some(Value::text(""))),
        Value::Str(name) => {
            let codes = name.code_points();
            let tool = match codes.iter().position(|code| *code == u32::from(':')) {
                Some(colon) => {
                    let mut tool = PyStr::new();
                    codes[colon + 1..]
                        .iter()
                        .for_each(|code| tool.push_code(*code));
                    tool
                }
                None => PyStr::new(),
            };
            Ok(Some(Value::Str(tool)))
        }
        name @ Value::List(items) => {
            if items.iter().any(|item| py::eq_str(item, ":")) {
                return Err(no_split(name));
            }
            Ok(Some(Value::text("")))
        }
        name @ Value::Dict(dict) => {
            if dict.get(":").is_some() {
                return Err(no_split(name));
            }
            Ok(Some(Value::text("")))
        }
        other => bail!(
            "TypeError: argument of type '{}' is not a container or iterable",
            py::type_name(other)
        ),
    }
}

/// `call_key`: the tool name plus a hash of its input. No input keys on the name alone,
/// and that key is the name as the frame held it, so `1` and `true` are one call.
fn call_key(name: &Value, args: &Value) -> Value {
    if matches!(args, Value::Null) {
        return name.clone();
    }
    let mut key = py::str(name);
    let digest: String = Sha256::digest(baseline::dumps_sorted(args).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    key.push_str(&format!(":{}", &digest[..16]));
    Value::Str(key)
}

/// The content blocks of a `claude` frame's message.
fn claude_content(raw: &Dict) -> &[Value] {
    match as_dict(at(raw, "message")).map(|message| at(message, "content")) {
        Some(Value::List(content)) => content,
        _ => &[],
    }
}

/// `claude_results`: each `tool_result` block's call id and whether it is a denial.
fn claude_results(raw: &Dict) -> Result<Vec<(Option<PyStr>, bool)>> {
    let mut found = Vec::new();
    for block in claude_content(raw).iter().filter_map(as_dict) {
        if !py::eq_str(at(block, "type"), "tool_result") {
            continue;
        }
        let denial = match at(block, "content") {
            Value::Str(text) => is_denial(text.code_points()),
            Value::List(parts) => {
                // `" ".join(part.get("text", "") for part in text if isinstance(part, dict))`.
                let mut joined = PyStr::new();
                for (n, part) in parts.iter().filter_map(as_dict).enumerate() {
                    if n > 0 {
                        joined.push_str(" ");
                    }
                    match part.get("text") {
                        Some(Value::Str(text)) => joined.push(text),
                        Some(other) => bail!(
                            "TypeError: sequence item {n}: expected str instance, {} found",
                            py::type_name(other)
                        ),
                        None => {}
                    }
                }
                is_denial(joined.code_points())
            }
            _ => false,
        };
        let denied = py::truthy(at(block, "is_error")) && denial;
        found.push((as_str(at(block, "tool_use_id")).cloned(), denied));
    }
    Ok(found)
}

/// `Prefix`: what the walk over a record's first lines has counted so far.
#[derive(Default)]
pub(crate) struct Walk {
    tool_calls: usize,
    refusals: usize,
    /// Null until a refusal names a tool.
    first_refused: Option<Value>,
    tools: Vec<Value>,
    /// Null until a frame reports usage.
    output_tokens: Option<Value>,
    seat: bool,
    /// Open calls by call id, in the order they opened.
    pending: Vec<(PyStr, Value)>,
    /// Open calls that came with no id.
    anon: Vec<Value>,
    opened: BTreeSet<PyStr>,
    /// The keys seen that are text, which is all of them but a bare name that is not.
    seen_keys: BTreeSet<PyStr>,
    seen_names: Vec<Value>,
    repeated: usize,
    last_key: Option<Value>,
    run: usize,
    longest_run: usize,
}

impl Walk {
    /// Counts triage compares before and after observing a frame.
    pub(crate) fn triage_counts(&self) -> (usize, usize, usize) {
        (
            self.refusals,
            self.tool_calls,
            self.pending.len() + self.anon.len(),
        )
    }
    fn call(&mut self, name: &Value, call_id: Option<&PyStr>, args: &Value) -> Result<()> {
        self.tool_calls += 1;
        py::hashable(name)?;
        if !self.tools.iter().any(|known| py::same_key(known, name)) {
            self.tools.push(name.clone());
        }
        let key = call_key(name, args);
        let seen = match &key {
            Value::Str(text) => !self.seen_keys.insert(text.clone()),
            other => {
                let seen = self
                    .seen_names
                    .iter()
                    .any(|known| py::same_key(known, other));
                if !seen {
                    self.seen_names.push(other.clone());
                }
                seen
            }
        };
        if seen {
            self.repeated += 1;
        }
        self.run = if self
            .last_key
            .as_ref()
            .is_some_and(|last| py::eq(last, &key))
        {
            self.run + 1
        } else {
            1
        };
        self.last_key = Some(key);
        self.longest_run = self.longest_run.max(self.run);
        match call_id.filter(|id| !id.is_empty()) {
            Some(id) => {
                match self.pending.iter_mut().find(|(known, _)| known == id) {
                    Some(slot) => slot.1 = name.clone(),
                    None => self.pending.push((id.clone(), name.clone())),
                }
                self.opened.insert(id.clone());
            }
            None => self.anon.push(name.clone()),
        }
        Ok(())
    }

    fn refuse(&mut self, name: &Value) {
        if self.refusals == 0 {
            self.first_refused = py::truthy(name).then(|| name.clone());
        }
        self.refusals += 1;
    }

    /// Close one open call. A denial counts once, when the call is still open or when the
    /// frame names a tool the prefix never saw opened.
    fn finish(&mut self, call_id: Option<&PyStr>, denied: bool, name: &Value) {
        let named = py::truthy(name);
        let open = call_id.and_then(|id| self.pending.iter().position(|(known, _)| known == id));
        let mut resolved = None;
        if let Some(open) = open {
            resolved = Some(self.pending.remove(open).1);
        } else if named {
            // The last open call of this name. A hook names the tool, not the call id, and
            // it belongs to the call that just happened.
            let last = self
                .pending
                .iter()
                .rposition(|(_, known)| py::eq(known, name));
            if let Some(last) = last {
                resolved = Some(self.pending.remove(last).1);
            } else if self.anon.iter().any(|known| py::same_key(known, name))
                && let Some(last) = self.anon.iter().rposition(|known| py::eq(known, name))
            {
                resolved = Some(self.anon.remove(last));
            }
        }
        if denied && (resolved.is_some() || named) {
            let who = resolved.filter(py::truthy).unwrap_or_else(|| name.clone());
            self.refuse(&who);
        } else if denied && call_id.is_none() {
            self.refuse(&Value::Null);
        }
    }

    pub(crate) fn observe(&mut self, line: &Dict) -> Result<()> {
        if as_dict(at(line, "hydra"))
            .is_some_and(|hydra| py::eq_str(at(hydra, "type"), "seat_request"))
        {
            self.seat = true;
        }
        let Some(raw) = as_dict(at(line, "raw")) else {
            return Ok(());
        };
        let tokens = output_tokens(raw)?;
        if !matches!(tokens, Value::Null) {
            self.output_tokens = Some(tokens);
        }
        self.calls(raw)
    }

    fn calls(&mut self, raw: &Dict) -> Result<()> {
        if py::eq_str(at(raw, "type"), "tool_call") {
            return self.cursor(raw);
        }
        if py::eq_str(at(raw, "type"), "item.completed") {
            return self.codex(raw);
        }
        if py::eq_str(at(raw, "event"), "step_update") {
            return self.agy(raw);
        }
        for block in claude_content(raw).iter().filter_map(as_dict) {
            let name = at(block, "name");
            if py::eq_str(at(block, "type"), "tool_use") && py::truthy(name) {
                self.call(name, as_str(at(block, "id")), at(block, "input"))?;
            }
        }
        if let Some(tool) = hook_denial(raw)? {
            self.finish(None, true, &tool);
        }
        for (call_id, denied) in claude_results(raw)? {
            // The hook already closed this call. The result echoes it.
            if let Some(id) = &call_id
                && !self.pending.iter().any(|(known, _)| known == id)
            {
                continue;
            }
            self.finish(call_id.as_ref(), denied, &Value::Null);
        }
        Ok(())
    }

    fn cursor(&mut self, raw: &Dict) -> Result<()> {
        let empty = Dict::new();
        let tool_call = as_dict(at(raw, "tool_call")).unwrap_or(&empty);
        let call_id = match at(raw, "call_id") {
            given if py::truthy(given) => given,
            _ => at(tool_call, "toolCallId"),
        };
        let call_id = as_str(call_id);
        let name = cursor_name(tool_call);
        let named = py::truthy(&name);
        let unopened = call_id.is_some_and(|id| !self.opened.contains(id));
        if py::eq_str(at(raw, "subtype"), "started") {
            if named && (call_id.is_none() || unopened) {
                self.call(&name, call_id, &cursor_args(tool_call))?;
            }
            return Ok(());
        }
        if !py::eq_str(at(raw, "subtype"), "completed") {
            return Ok(());
        }
        if (named && unopened)
            || (named && call_id.is_none() && self.anon.is_empty() && self.pending.is_empty())
        {
            self.call(&name, call_id, &cursor_args(tool_call))?;
        }
        self.finish(call_id, cursor_denied(tool_call), &name);
        Ok(())
    }

    fn codex(&mut self, raw: &Dict) -> Result<()> {
        let Some(item) = as_dict(at(raw, "item")) else {
            return Ok(());
        };
        let kind = at(item, "type");
        py::hashable(kind)?;
        let mut name = CODEX_TOOLS
            .iter()
            .find(|(known, _)| py::eq_str(kind, known))
            .map_or(Value::Null, |(_, name)| Value::text(name));
        if py::eq_str(kind, "mcp_tool_call") {
            name = match at(item, "name") {
                given @ Value::Str(_) => given.clone(),
                _ => Value::text("mcp"),
            };
        }
        if !py::truthy(&name) {
            return Ok(());
        }
        let call_id = as_str(at(item, "id"));
        let output = as_str(at(item, "aggregated_output")).or_else(|| as_str(at(item, "output")));
        let denied = output.is_some_and(|output| is_denial(output.code_points()));
        let args = if py::eq_str(kind, "command_execution") {
            at(item, "command")
        } else {
            &NULL
        };
        self.call(&name, call_id, args)?;
        self.finish(call_id, denied, &name);
        Ok(())
    }

    fn agy(&mut self, raw: &Dict) -> Result<()> {
        let Some(step) = as_dict(at(raw, "step_update")) else {
            return Ok(());
        };
        if !py::eq_str(at(step, "step_type"), "tool") {
            return Ok(());
        }
        let empty = Dict::new();
        let info = as_dict(at(step, "tool_info")).unwrap_or(&empty);
        // `step.get("tool_name") or info.get("name") or "tool"`, and text or nothing.
        let name = [at(step, "tool_name"), at(info, "name")]
            .into_iter()
            .find(|given| py::truthy(given))
            .filter(|given| matches!(given, Value::Str(_)))
            .cloned()
            .unwrap_or_else(|| Value::text("tool"));
        let mut call_id = PyStr::from("agy:");
        call_id.push(&py::str(at(step, "step_index")));
        if !self.opened.contains(&call_id) {
            self.call(&name, Some(&call_id), at(info, "parameters"))?;
        }
        if py::eq_str(at(step, "state"), "ACTIVE") {
            return Ok(());
        }
        let message = as_dict(at(info, "error")).and_then(|error| as_str(at(error, "message")));
        let denied = py::eq_str(at(step, "state"), "ERROR")
            && message.is_some_and(|message| is_denial(message.code_points()));
        self.finish(Some(&call_id), denied, &name);
        Ok(())
    }
}

/// `summarize`: what the first `fraction` of a record's lines say, as the dict the Python
/// builds. A line that is not an object counts as an event and is otherwise stepped over.
pub fn summarize(lines: &[Value], fraction: f64) -> Result<Dict> {
    let prefix = &lines[..prefix_len(lines.len(), fraction)];
    let mut walk = Walk::default();
    for line in prefix.iter().filter_map(as_dict) {
        walk.observe(line)?;
    }
    let stamps: Vec<DateTime> = prefix
        .iter()
        .filter_map(as_dict)
        .filter_map(parse_ts)
        .collect();
    let mut elapsed = 0.0;
    if let [first, .., last] = stamps.as_slice() {
        let Some(seconds) = isotime::seconds_between(first, last) else {
            bail!("TypeError: can't subtract offset-naive and offset-aware datetimes");
        };
        // `max(0.0, seconds)`.
        if seconds > 0.0 {
            elapsed = seconds;
        }
    }
    let count = |n: usize| Value::Int(n as i128);
    let mut out = Dict::new();
    out.set("events_so_far", count(prefix.len()));
    out.set("tool_calls_so_far", count(walk.tool_calls));
    out.set("refusals_so_far", count(walk.refusals));
    out.set(
        "first_refused_tool",
        walk.first_refused.unwrap_or(Value::Null),
    );
    out.set("distinct_tools", count(walk.tools.len()));
    out.set("repeated_calls_so_far", count(walk.repeated));
    out.set("longest_repeat_run", count(walk.longest_run));
    out.set(
        "output_tokens_so_far",
        walk.output_tokens.unwrap_or(Value::Null),
    );
    out.set("seconds_elapsed", Value::Float(elapsed));
    out.set("seat_requested", Value::Bool(walk.seat));
    Ok(out)
}

/// `raw_features`: the baseline's features plus the prefix counts. The first refused tool
/// is one-hot, the same shape as `provider=`. The two repeat counts are printed by
/// `features` and left out here: in the fitted vector they lowered the outcome AUC at every
/// fraction.
struct Features {
    base: baseline::Features,
    prefix: [f64; 7],
    first_refused: Option<PyStr>,
}

impl Features {
    fn of(row: &Value) -> Result<Features> {
        let base = baseline::Features::of(row)?;
        // `row.get("_prefix") or {}`.
        let prefix = py::get(row, "_prefix", Value::Null)?;
        let prefix = if py::truthy(&prefix) {
            prefix
        } else {
            Value::Dict(Dict::new())
        };
        let mut numbers = [0.0; 7];
        for (slot, name) in numbers.iter_mut().zip(PREFIX_NAMES) {
            let value = py::get(&prefix, name, Value::Null)?;
            if !py::truthy(&value) {
                continue;
            }
            *slot = if name == "seat_requested" {
                1.0
            } else {
                // `float(value or 0)`.
                baseline::number(&value)?
            };
        }
        let tool = py::get(&prefix, "first_refused_tool", Value::Null)?;
        let first_refused = py::truthy(&tool).then(|| {
            let mut name = PyStr::from("first_refused=");
            name.push(&py::str(&tool));
            name
        });
        Ok(Features {
            base,
            prefix: numbers,
            first_refused,
        })
    }

    fn names(&self) -> Vec<PyStr> {
        let mut names = self.base.names();
        names.extend(PREFIX_NAMES.iter().map(|name| PyStr::from(*name)));
        names.extend(self.first_refused.clone());
        names
    }

    /// `feats.get(name, 0.0)`. A later key of the Python's dict wins over an earlier one.
    fn value(&self, name: &PyStr) -> f64 {
        if self.first_refused.as_ref() == Some(name) {
            return 1.0;
        }
        match PREFIX_NAMES.iter().position(|known| name == known) {
            Some(at) => self.prefix[at],
            None => self.base.value(name),
        }
    }
}

/// `join`: dataset rows that have a readable record, with `_prefix` attached. A row with
/// no file, or a file that is not a run, is left out. The label stays on the dataset row:
/// the record supplies the prefix only.
pub fn join(
    rows: Vec<Value>,
    records_dir: &Path,
    fraction: f64,
    stderr: &mut dyn Write,
) -> Result<Vec<Value>> {
    let mut joined = Vec::new();
    for row in rows {
        let mut name = py::str(py::item(&row, "run_id")?);
        name.push_str(".jsonl");
        // A name with a lone surrogate is no file a record was written under.
        let Some(name) = name.to_utf8() else {
            continue;
        };
        let path = records_dir.join(name);
        if !path.is_file() {
            continue;
        }
        let Some(item) = records::scan_record(&path, stderr)? else {
            continue;
        };
        let prefix = summarize(&item.lines, fraction)?;
        let Value::Dict(mut row) = row else {
            unreachable!("a row with a run id is a dict");
        };
        row.set("_prefix", Value::Dict(prefix));
        joined.push(Value::Dict(row));
    }
    Ok(joined)
}

/// `x ** 2` on a float: the square, or the `OverflowError` Python raises when a finite `x`
/// has none. An `x` that is already infinite squares to infinity without one.
fn squared(x: f64) -> Result<f64> {
    let square = math::pow(x, 2.0);
    if square.is_infinite() && x.is_finite() {
        bail!("OverflowError: (34, 'Result too large')");
    }
    Ok(square)
}

/// `scale_of`: the mean and deviation of each scaled name over the fit rows. A count so
/// large its spread has no square ends the training here, before anything is written, as
/// it ends the Python's (build ticket 220).
fn scale_of(feats: &[Features]) -> Result<Vec<(PyStr, Vec<Value>)>> {
    let n = feats.len() as f64;
    SCALE_NAMES
        .iter()
        .map(|name| {
            let name = PyStr::from(*name);
            let values: Vec<f64> = feats.iter().map(|f| f.value(&name)).collect();
            let mean = fsum_like_python(&values) / n;
            let spread: Vec<f64> = values
                .iter()
                .map(|v| squared(v - mean))
                .collect::<Result<_>>()?;
            let mut std = math::sqrt(fsum_like_python(&spread) / n);
            if std == 0.0 {
                std = 1.0;
            }
            Ok((name, vec![Value::Float(mean), Value::Float(std)]))
        })
        .collect()
}

/// `train`: the fitted model as the file holds it, or the refusal when either slice has
/// fewer than ten rows of its rarer class. `rows` are already joined to their prefixes.
pub fn train(rows: &[Value], label: Label, test_from: &DateTime, fraction: f64) -> Result<Value> {
    let parts = baseline::slices(rows, label, test_from)?;
    for (name, part) in [("fit", &parts.fit), ("test", &parts.test)] {
        let ones = baseline::positives(part);
        let minority = ones.min(part.len() - ones);
        if minority < baseline::MIN_MINORITY {
            let label = label.name();
            return Err(Refusal(format!(
                "monitor: {} labelled rows for '{label}' had a record; the {name} slice has {} \
                 of them and {minority} of its rarer class, and {} is the floor. Label more \
                 runs before training on '{label}'.",
                parts.labelled,
                part.len(),
                baseline::MIN_MINORITY
            ))
            .into());
        }
    }

    let feats: Vec<Features> = parts
        .fit
        .iter()
        .map(|(row, _)| Features::of(row))
        .collect::<Result<_>>()?;
    let mut vocab: Vec<PyStr> = feats.iter().flat_map(Features::names).collect();
    vocab.sort();
    vocab.dedup();
    let vocab: Vec<Value> = vocab.into_iter().map(Value::Str).collect();
    let scale = scale_of(&feats)?;
    let xs: Vec<Vec<f64>> = feats
        .iter()
        .map(|f| baseline::vector(&|name| f.value(name), &vocab, &scale))
        .collect::<Result<_>>()?;
    let ys: Vec<u8> = parts.fit.iter().map(|(_, y)| *y).collect();
    let (weights, bias) = baseline::fit(&xs, &ys);

    let mut body = Dict::new();
    body.set("label", Value::text(label.name()));
    body.set("positive", Value::text(label.positive()));
    body.set("at", Value::Float(fraction));
    body.set("vocab", Value::List(vocab));
    let mut scales = Dict::new();
    for (name, pair) in scale {
        scales.set_key(name, Value::List(pair));
    }
    body.set("scale", Value::Dict(scales));
    body.set("weights", baseline::floats(&weights));
    body.set("bias", Value::Float(bias));
    body.set("test_from", Value::text(&test_from.isoformat()));
    let mut counts = Dict::new();
    counts.set("fit", Value::Int(parts.fit.len() as i128));
    counts.set("test", Value::Int(parts.test.len() as i128));
    body.set("rows", Value::Dict(counts));

    let digest: String =
        Sha256::digest(baseline::dumps_sorted(&Value::Dict(body.clone())).as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
    body.set(
        "version",
        Value::text(&format!(
            "monitor-{}-{}-{}",
            label.name(),
            fraction_token(fraction),
            &digest[..12]
        )),
    );
    Ok(Value::Dict(body))
}

/// `predict`: the model's probability for one row that carries its `_prefix`.
pub fn predict(model: &Value, row: &Value) -> Result<f64> {
    baseline::predict_with(model, || {
        let feats = Features::of(row)?;
        Ok(move |name: &PyStr| feats.value(name))
    })
}

/// `report`: how a model `train` just fitted does on its test slice. The baseline's
/// report, scored by this file's `predict`.
pub fn report(model: &Value, rows: &[Value], label: Label) -> Result<Value> {
    baseline::report_with(model, rows, label, &|row| predict(model, row))
}

/// `load_record`: the record at `path`, or the refusal when it is not a run. `shown` is the
/// path as the Python prints it.
fn load_record(path: &Path, shown: &str, stderr: &mut dyn Write) -> Result<records::Scanned> {
    records::scan_record(path, stderr)?
        .ok_or_else(|| Refusal(format!("monitor: {shown} is not a run or resume record")).into())
}

/// `feature_view`: what `features` prints, the prefix counts and then the pre-launch
/// features of the same record. The pre-launch fields come from the reader `score` uses,
/// so the printed vector and a later score agree.
pub fn feature_view(
    path: &Path,
    shown: &str,
    fraction: f64,
    aliases: &dyn Fn() -> Result<Vec<(Pair, Pair)>>,
    stderr: &mut dyn Write,
) -> Result<String> {
    let item = load_record(path, shown, stderr)?;
    let prefix = summarize(&item.lines, fraction)?;
    let events_read = at(&prefix, "events_so_far").clone();
    let mut features = prefix;
    let base = baseline::Features::of(&baseline::row_of(&item, aliases)?)?;
    for name in base.names() {
        let value = base.value(&name);
        features.set_key(name, Value::Float(value));
    }
    let mut view = Dict::new();
    view.set("run_id", item.run_id.clone());
    view.set("at", Value::Float(fraction));
    view.set("events", Value::Int(item.lines.len() as i128));
    view.set("events_read", events_read);
    view.set("features", Value::Dict(features));
    Ok(json::dumps(&Value::Dict(view), Some(2)))
}

/// The fraction a model file was trained at, `None` when it holds none of the three.
pub fn model_fraction(model: &Value) -> Result<Option<f64>> {
    let stored = py::get(model, "at", Value::Null)?;
    match stored {
        Value::Null | Value::Str(_) | Value::List(_) | Value::Dict(_) => Ok(None),
        number => Ok(allowed(baseline::number(&number)?)),
    }
}

/// What `score` prints: one line of JSON with the model's probability for the record, read
/// as far as the fraction stored on the model.
pub fn score_line(
    model: &Value,
    fraction: f64,
    path: &Path,
    shown: &str,
    aliases: &dyn Fn() -> Result<Vec<(Pair, Pair)>>,
    stderr: &mut dyn Write,
) -> Result<String> {
    let item = load_record(path, shown, stderr)?;
    let prefix = summarize(&item.lines, fraction)?;
    let events_read = at(&prefix, "events_so_far").clone();
    let Value::Dict(mut row) = baseline::row_of(&item, aliases)? else {
        unreachable!("a row is a dict");
    };
    row.set("_prefix", Value::Dict(prefix));
    let row = Value::Dict(row);
    let mut out = Dict::new();
    out.set("run_id", py::item(&row, "run_id")?.clone());
    out.set("label", py::item(model, "label")?.clone());
    out.set("positive", py::item(model, "positive")?.clone());
    out.set("p", Value::Float(predict(model, &row)?));
    out.set("model_version", py::item(model, "version")?.clone());
    out.set("at", Value::Float(fraction));
    out.set("events_read", events_read);
    Ok(json::dumps_default(&Value::Dict(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn denial(text: &str) -> bool {
        is_denial(PyStr::from(text).code_points())
    }

    #[test]
    fn a_square_past_a_float_is_an_overflow_and_an_infinite_one_is_not() {
        assert_eq!(squared(3.0).unwrap(), 9.0);
        let err = squared(-5e199).unwrap_err().to_string();
        assert_eq!(err, "OverflowError: (34, 'Result too large')");
        assert_eq!(squared(f64::NEG_INFINITY).unwrap(), f64::INFINITY);
        assert!(squared(f64::NAN).unwrap().is_nan());
    }

    #[test]
    fn a_prefix_is_at_least_one_line_and_never_more_than_the_record() {
        assert_eq!(prefix_len(0, 0.5), 0);
        assert_eq!(prefix_len(1, 0.1), 1);
        assert_eq!(prefix_len(10, 0.1), 1);
        assert_eq!(prefix_len(10, 0.25), 2);
        assert_eq!(prefix_len(10, 0.5), 5);
        assert_eq!(prefix_len(7, 0.5), 3);
    }

    #[test]
    fn the_gate_allowing_a_call_is_not_a_denial() {
        assert!(denial("hydra/capability: no"));
        assert!(!denial("hydra/allow"));
        assert!(!denial("nothing of ours"));
        assert!(denial("hydra/allow then hydra/path"));
        // What is left after the cut spells the voice again.
        assert!(denial("hydrahydra/allow/"));
    }

    #[test]
    fn a_fraction_reads_as_python_reads_a_float() {
        assert_eq!(parse_fraction("0.5"), Some(0.5));
        assert_eq!(parse_fraction(" .25\n"), Some(0.25));
        assert_eq!(parse_fraction("1e-1"), Some(0.1));
        assert_eq!(parse_fraction("0_0.5"), Some(0.5));
        assert_eq!(parse_fraction("\u{ff10}.\u{ff15}"), Some(0.5));
        assert_eq!(parse_fraction("0.5000000001"), Some(0.5));
        for refused in [
            "0.3", "", "_0.5", "0._5", "0.5_", "nan", "inf", "0x1", "0.5 x",
        ] {
            assert_eq!(parse_fraction(refused), None, "{refused:?}");
        }
    }

    #[test]
    fn a_whole_float_of_any_size_is_its_own_integer() {
        assert!(matches!(int_of_float(12.9), Ok(Value::Int(12))));
        assert!(matches!(int_of_float(-0.5), Ok(Value::Int(0))));
        let Ok(Value::BigInt(digits)) = int_of_float(1e40) else {
            panic!("a wide integer");
        };
        assert_eq!(digits, "10000000000000000303786028427003666890752");
        assert!(int_of_float(f64::NAN).is_err());
    }
}
