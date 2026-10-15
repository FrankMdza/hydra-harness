//! The read commands: `show`, `board`, `pending --json` and `queue`. Each writes what
//! the Python helper printed to `out`, line by line, so a value Python would choke on stops the
//! output at the same line, and returns the exit code or the error Python would raise.

use std::io::Write;
use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::fold::{self, GoalState, TaskState, approval_available, needs_launch_approval};
use crate::json::{self, ParseError, PyStr, Value};
use crate::log::{self, Home};
use crate::{py, pysort};

/// Something an f-string would interpolate into a line.
pub(crate) trait Piece {
    fn put(&self, line: &mut PyStr);
}

impl Piece for &str {
    fn put(&self, line: &mut PyStr) {
        line.push_str(self);
    }
}

impl Piece for String {
    fn put(&self, line: &mut PyStr) {
        line.push_str(self);
    }
}

impl Piece for PyStr {
    fn put(&self, line: &mut PyStr) {
        line.push(self);
    }
}

/// An f-string: its pieces, evaluated left to right and joined. The result is a Python
/// string, which a Rust `format!` could not hold when a log value carries a lone surrogate.
macro_rules! text {
    ($($piece:expr),* $(,)?) => {{
        let mut line = PyStr::new();
        $( Piece::put(&$piece, &mut line); )*
        line
    }};
}
pub(crate) use text;

/// Python's stdout defaults, before an explicit PYTHONIOENCODING override.
fn stdout_settings() -> (String, String) {
    let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| "C".into());
    let c_locale = matches!(locale.as_str(), "C" | "POSIX");
    let utf8 = match std::env::var("PYTHONUTF8").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        _ => c_locale,
    };
    let coerced = locale == "C"
        && std::env::var("LC_ALL").unwrap_or_default().is_empty()
        && std::env::var("PYTHONCOERCECLOCALE").as_deref() != Ok("0");
    let mut encoding = if c_locale && !utf8 && !coerced {
        "ascii"
    } else {
        "utf-8"
    }
    .to_string();
    let mut errors = if c_locale || utf8 {
        "surrogateescape"
    } else {
        "strict"
    }
    .to_string();
    if let Ok(setting) = std::env::var("PYTHONIOENCODING") {
        let (codec, handler) = setting.split_once(':').unwrap_or((&setting, ""));
        if !codec.is_empty() {
            encoding = codec.to_lowercase().replace('_', "-");
            errors = "strict".into();
        }
        if !handler.is_empty() {
            errors = handler.into();
        }
    }
    (encoding, errors)
}

/// Encode the entire line before writing, as Python's TextIOWrapper does.
pub(crate) fn print(out: &mut dyn Write, line: &PyStr) -> Result<()> {
    let (encoding, errors) = stdout_settings();
    let limit = match encoding.as_str() {
        "utf-8" | "utf8" => 0x10ffff,
        "ascii" | "us-ascii" => 0x7f,
        "latin-1" | "latin1" | "iso8859-1" | "iso-8859-1" => 0xff,
        _ => bail!("LookupError: unsupported stdout encoding {encoding}"),
    };
    let mut bytes = Vec::new();
    for &code in line.code_points() {
        if errors == "surrogateescape" && (0xdc80..=0xdcff).contains(&code) {
            bytes.push((code - 0xdc00) as u8);
        } else if let Some(ch) = char::from_u32(code).filter(|_| code <= limit) {
            if limit < 0x100 {
                bytes.push(code as u8);
            } else {
                bytes.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
            }
        } else {
            match errors.as_str() {
                "ignore" => {}
                "replace" => bytes.push(b'?'),
                "backslashreplace" => bytes.extend_from_slice(
                    if code <= 0xff {
                        format!("\\x{code:02x}")
                    } else if code <= 0xffff {
                        format!("\\u{code:04x}")
                    } else {
                        format!("\\U{code:08x}")
                    }
                    .as_bytes(),
                ),
                "surrogatepass" if limit == 0x10ffff && (0xd800..=0xdfff).contains(&code) => {
                    bytes.extend_from_slice(&[
                        (0xe0 | (code >> 12)) as u8,
                        (0x80 | ((code >> 6) & 0x3f)) as u8,
                        (0x80 | (code & 0x3f)) as u8,
                    ]);
                }
                _ => bail!("UnicodeEncodeError: '{encoding}' codec can't encode character"),
            }
        }
    }
    bytes.push(b'\n');
    out.write_all(&bytes)?;
    Ok(())
}

/// `", ".join(parts)`.
fn join(parts: &[PyStr]) -> PyStr {
    let mut out = PyStr::new();
    for (n, part) in parts.iter().enumerate() {
        if n > 0 {
            out.push_str(", ");
        }
        out.push(part);
    }
    out
}

/// `show`: the folded state as indented JSON. An unknown goal prints its empty state.
pub fn show(home: &Home, goal_id: &str, out: &mut dyn Write) -> Result<i32> {
    let state = fold::fold(goal_id, &log::read_log(home, goal_id)?)?;
    let Value::Dict(mut shown) = fold::state_to_value(&state) else {
        unreachable!();
    };
    let mut tasks = json::Dict::new();
    if let Some(Value::Dict(folded)) = shown.get("tasks") {
        for ((key, body), task) in folded.iter().zip(&state.tasks) {
            let Value::Dict(mut body) = body.clone() else {
                unreachable!();
            };
            body.set(
                "task_text",
                Value::Str(crate::launch::worker_task_text(task)),
            );
            tasks.push(key.clone(), Value::Dict(body));
        }
    }
    shown.set("tasks", Value::Dict(tasks));
    let shown = Value::Dict(shown);
    writeln!(out, "{}", json::dumps(&shown, Some(2)))?;
    Ok(0)
}

/// `pending --json`. `helper` begins each `argv`, the command that records the approval:
/// the path of the `hydra-puppets` that is answering.
pub fn pending(
    home: &Home,
    goal_id: &str,
    helper: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32> {
    let state = fold::fold(goal_id, &log::read_log(home, goal_id)?)?;
    if state.goal.is_none() {
        writeln!(err, "unknown goal {goal_id}")?;
        return Ok(1);
    }
    let items = if fold::panel_gate_reachable(&state) {
        fold::pending_approvals(&state, &home.vendors_file()?, helper)?
    } else {
        // `pending_approvals` reads `vendors` only when the degraded branch runs, which needs
        // `panel_gate_reachable`.
        fold::pending_approvals(&state, Path::new(""), helper)?
    };
    writeln!(out, "{}", json::dumps(&Value::List(items), Some(2)))?;
    Ok(0)
}

/// Verdict-model probabilities from `scores.jsonl`, for the newest verdict `model_version`
/// only: the verdict line with the latest `scored_at`, a later line breaking a tie.
#[derive(Debug, Default)]
pub struct VerdictScores {
    pub model_version: Option<PyStr>,
    pub by_run: Vec<(PyStr, f64)>,
}

impl VerdictScores {
    /// `scores.by_run.get(run_id)`.
    fn get(&self, run_id: &Value) -> Result<Option<f64>> {
        py::hashable(run_id)?;
        let Value::Str(run_id) = run_id else {
            return Ok(None);
        };
        Ok(self
            .by_run
            .iter()
            .find(|(r, _)| r == run_id)
            .map(|(_, p)| *p))
    }
}

/// `load_verdict_scores`. A missing file is an empty score set; nothing is written.
pub fn load_verdict_scores(path: &Path) -> Result<VerdictScores> {
    if !path.is_file() {
        return Ok(VerdictScores::default());
    }
    let raw = std::fs::read(path)?;
    let text = std::str::from_utf8(&raw).map_err(|e| anyhow!("UnicodeDecodeError: {e}"))?;
    let mut rows: Vec<(PyStr, PyStr, f64)> = Vec::new();
    let mut best: Option<(PyStr, usize)> = None;
    let mut best_version: Option<PyStr> = None;
    for (index, line) in py::splitlines(text).into_iter().enumerate() {
        let line = py::strip_str(line);
        if line.is_empty() {
            continue;
        }
        let row = match json::loads_str(line) {
            Ok(row) => row,
            Err(ParseError::Decode) => continue,
            // Python catches `JSONDecodeError` here and nothing wider.
            Err(err) => bail!("{}: scores.jsonl line {}", err.class(), index + 1),
        };
        let Value::Dict(row) = row else { continue };
        if !row.get("label").is_some_and(|l| py::eq_str(l, "verdict")) {
            continue;
        }
        let (Some(Value::Str(version)), Some(Value::Str(run_id))) =
            (row.get("model_version"), row.get("run_id"))
        else {
            continue;
        };
        let p = match row.get("p") {
            Some(Value::Int(n)) => *n as f64,
            Some(Value::Float(f)) => *f,
            Some(Value::BigInt(digits)) => {
                let f: f64 = digits.parse().unwrap_or(f64::INFINITY);
                if f.is_infinite() {
                    return Err(anyhow!("OverflowError: int too large to convert to float"));
                }
                f
            }
            _ => continue,
        };
        let scored_at = match row.get("scored_at") {
            Some(Value::Str(at)) => at.clone(),
            _ => PyStr::new(),
        };
        let key = (scored_at, index);
        if best.as_ref().is_none_or(|b| key > *b) {
            best = Some(key);
            best_version = Some(version.clone());
        }
        rows.push((version.clone(), run_id.clone(), p));
    }
    let Some(version) = best_version else {
        return Ok(VerdictScores::default());
    };
    let mut by_run: Vec<(PyStr, f64)> = Vec::new();
    for (row_version, run_id, p) in rows {
        if row_version != version {
            continue;
        }
        match by_run.iter_mut().find(|(r, _)| *r == run_id) {
            Some(slot) => slot.1 = p,
            None => by_run.push((run_id, p)),
        }
    }
    Ok(VerdictScores {
        model_version: Some(version),
        by_run,
    })
}

/// Run ids that already have a human verdict, per task. Only string ids count.
fn human_verdict_runs(events: &[Value]) -> Vec<(PyStr, Vec<PyStr>)> {
    let mut found: Vec<(PyStr, Vec<PyStr>)> = Vec::new();
    for event in events {
        let Value::Dict(event) = event else { continue };
        if !event
            .get("event")
            .is_some_and(|e| py::eq_str(e, "human_verdict"))
        {
            continue;
        }
        let (Some(Value::Str(task)), Some(Value::Str(run))) =
            (event.get("task"), event.get("run_id"))
        else {
            continue;
        };
        match found.iter_mut().find(|(t, _)| t == task) {
            Some((_, runs)) => runs.push(run.clone()),
            None => found.push((task.clone(), vec![run.clone()])),
        }
    }
    found
}

/// The task's current run has ended and no human verdict names it.
fn awaits_review(task: &TaskState, verdicts: &[(PyStr, Vec<PyStr>)]) -> Result<bool> {
    if !py::eq_str(&task.status, "ended") || !py::truthy(&task.run_id) {
        return Ok(false);
    }
    py::hashable(&task.id)?;
    let Value::Str(id) = &task.id else {
        return Ok(true);
    };
    // `run_id not in verdicts.get(id, ())`: the run id is hashed only when the task has a
    // set of verdicts to look in; the empty tuple is searched by comparison.
    let Some((_, runs)) = verdicts.iter().find(|(t, _)| t == id) else {
        return Ok(true);
    };
    py::hashable(&task.run_id)?;
    let Value::Str(run) = &task.run_id else {
        return Ok(true);
    };
    Ok(!runs.contains(run))
}

fn format_p(p: Option<f64>) -> String {
    match p {
        None => "no score".into(),
        Some(p) if p.is_nan() => "nan".into(),
        Some(p) if p.is_infinite() => if p > 0.0 { "inf" } else { "-inf" }.into(),
        Some(p) => format!("{p:.2}"),
    }
}

/// `task.run_id or ""`.
fn run_or_empty(task: &TaskState) -> Value {
    if py::truthy(&task.run_id) {
        task.run_id.clone()
    } else {
        Value::Str(PyStr::new())
    }
}

/// The tuple `ordered_review_tasks` sorts by: `(0, -p, id)` with a score, `(1, 0.0, id)`
/// without.
struct ReviewKey<'a> {
    unscored: bool,
    neg_p: f64,
    id: &'a Value,
}

/// `a < b` on two of those tuples, as Python compares tuples: find the first position where
/// the items are not equal, then compare those two items. A NaN score is equal to nothing
/// and less than nothing, so the ids behind it are never looked at.
fn review_key_less(a: &ReviewKey, b: &ReviewKey) -> Result<bool> {
    if a.unscored != b.unscored {
        return Ok(!a.unscored);
    }
    if a.neg_p != b.neg_p {
        return Ok(a.neg_p < b.neg_p);
    }
    if py::eq(a.id, b.id) {
        return Ok(false);
    }
    py::less(a.id, b.id)
}

/// Waiting tasks, highest `p` first, unscored last, then by task id, in the order Python's
/// `list.sort` leaves them.
fn ordered_review_tasks<'a>(
    state: &'a GoalState,
    events: &[Value],
    scores: &VerdictScores,
) -> Result<Vec<&'a TaskState>> {
    let verdicts = human_verdict_runs(events);
    let mut waiting = Vec::new();
    for task in &state.tasks {
        if awaits_review(task, &verdicts)? {
            waiting.push(task);
        }
    }
    // `list.sort` computes every key before it compares any.
    let mut keys = Vec::new();
    for task in &waiting {
        keys.push(match scores.get(&run_or_empty(task))? {
            None => ReviewKey {
                unscored: true,
                neg_p: 0.0,
                id: &task.id,
            },
            Some(p) => ReviewKey {
                unscored: false,
                neg_p: -p,
                id: &task.id,
            },
        });
    }
    let order = pysort::sorted(keys.len(), |i, j| review_key_less(&keys[i], &keys[j]))?;
    Ok(order.into_iter().map(|i| waiting[i]).collect())
}

/// `board`: the goal at a glance, and what the human still owes.
pub fn board(home: &Home, goal_id: &str, out: &mut dyn Write, err: &mut dyn Write) -> Result<i32> {
    let events = log::read_log(home, goal_id)?;
    let state = fold::fold(goal_id, &events)?;
    let Some(goal) = &state.goal else {
        writeln!(err, "unknown goal {goal_id}")?;
        return Ok(1);
    };
    let scores = load_verdict_scores(&home.grader.join("scores.jsonl"))?;
    let verdicts = human_verdict_runs(&events);
    writeln!(out, "goal {}  revision {}", state.goal_id, state.revision)?;
    print(
        out,
        &text!["  ", py::str(goal.get("text").unwrap_or(&Value::Null))],
    )?;
    if !state.plans.is_empty() {
        writeln!(out, "plans")?;
        for plan in &state.plans {
            let ready = if plan.get("path").is_some_and(py::truthy) {
                "ready"
            } else {
                "running"
            };
            let provider = py::ljust(plan.get("provider").unwrap_or(&Value::Null), 13)?;
            let run = py::str(&py::head(plan.get("run_id").unwrap_or(&Value::Null), 8)?);
            print(out, &text!["  ", provider, " ", run, "  ", ready])?;
        }
    }
    if let Some(chosen) = &state.chosen_plan {
        let null = Value::Null;
        if chosen.get("degraded").is_some_and(py::truthy) {
            let subs = chosen.get("substitutions").cloned().unwrap_or(Value::Null);
            let subs = if py::truthy(&subs) {
                py::list(&subs)?
            } else {
                Vec::new()
            };
            let mut parts = Vec::new();
            for sub in &subs {
                parts.push(text![
                    py::str(py::item(sub, "missing")?),
                    "->",
                    py::str(py::item(sub, "substitute")?)
                ]);
            }
            let name = py::str(chosen.get("chosen").unwrap_or(&null));
            let judges = py::len(chosen.get("judge_run_ids").unwrap_or(&null))?;
            print(
                out,
                &text![
                    "chosen (degraded panel)  ",
                    name,
                    "  judges ",
                    judges.to_string(),
                    "  ",
                    join(&parts)
                ],
            )?;
        } else {
            let name = py::str(chosen.get("chosen").unwrap_or(&null));
            let judges = py::len(chosen.get("judge_run_ids").unwrap_or(&null))?;
            print(
                out,
                &text!["chosen  ", name, "  judges ", judges.to_string()],
            )?;
        }
    }
    if !state.tasks.is_empty() {
        writeln!(out, "tasks")?;
        writeln!(
            out,
            "  {:<12} {:<21} {:<6} {:<28} {:<9} {:<8} run",
            "id", "class", "grade", "agent", "status", "p"
        )?;
        for task in &state.tasks {
            let mut agent = PyStr::from("(unassigned)");
            if let Some(assignment) = &task.assignment
                && !assignment.is_empty()
            {
                let provider = py::str(assignment.get("provider").unwrap_or(&Value::Null));
                let model = assignment
                    .get("model")
                    .filter(|m| py::truthy(m))
                    .map(py::str)
                    .unwrap_or_default();
                agent = py::strip(&text![provider, " ", model]);
            }
            let run = py::str(&py::head(&run_or_empty(task), 8)?);
            let mut verdict = if py::truthy(&task.human_result) {
                text!["  ", py::str(&task.human_result)]
            } else {
                PyStr::new()
            };
            if py::truthy(&task.review_verdict) && !py::truthy(&task.human_result) {
                let mut label = py::str(&task.review_verdict);
                if py::truthy(&task.review_degraded) {
                    label.push_str(" (degraded panel)");
                }
                verdict = text!["  review ", label];
            }
            let mut p_cell = String::new();
            if awaits_review(task, &verdicts)? {
                p_cell = format_p(scores.get(&run_or_empty(task))?);
            }
            print(
                out,
                &text![
                    "  ",
                    py::ljust(&task.id, 12)?,
                    " ",
                    py::ljust(&task.task_class, 21)?,
                    " ",
                    py::ljust(&task.grade, 6)?,
                    " ",
                    py::pad(&agent, 28),
                    " ",
                    py::ljust(&task.status, 9)?,
                    " ",
                    format!("{p_cell:<8}"),
                    " ",
                    run,
                    verdict
                ],
            )?;
        }
    }
    let none = Value::Null;
    let mut waiting: Vec<PyStr> = Vec::new();
    if !state.plans.is_empty()
        && state.chosen_plan.as_ref().is_some_and(|c| !c.is_empty())
        && approval_available(&state, "plan", &none, &none).is_none()
    {
        waiting.push("plan".into());
    }
    if !state.tasks.is_empty() && approval_available(&state, "table", &none, &none).is_none() {
        waiting.push("table".into());
    }
    for task in &state.tasks {
        if task.assignment.as_ref().is_some_and(|a| !a.is_empty())
            && needs_launch_approval(task)
            && py::eq_str(&task.status, "assigned")
            && approval_available(&state, "launch", &task.id, &none).is_none()
        {
            waiting.push(text!["launch ", py::str(&task.id)]);
        }
        if py::eq_str(&task.status, "ended")
            && approval_available(&state, "diff", &task.id, &task.run_id).is_none()
        {
            waiting.push(text!["diff ", py::str(&task.id)]);
        }
    }
    if !state.dangling_intents.is_empty() {
        let mut parts = Vec::new();
        for intent in &state.dangling_intents {
            parts.push(text![
                py::str(py::item(intent, "task")?),
                "#",
                py::str(py::item(intent, "attempt")?)
            ]);
        }
        waiting.push(text!["reconcile ", join(&parts)]);
    }
    let waiting = if waiting.is_empty() {
        PyStr::from("nothing")
    } else {
        join(&waiting)
    };
    print(out, &text!["waiting on you: ", waiting])?;
    Ok(0)
}

/// `queue`: tasks whose current run has ended with no human verdict, highest `p` first.
pub fn queue(home: &Home, goal_id: &str, out: &mut dyn Write, err: &mut dyn Write) -> Result<i32> {
    let events = log::read_log(home, goal_id)?;
    let state = fold::fold(goal_id, &events)?;
    if state.goal.is_none() {
        writeln!(err, "unknown goal {goal_id}")?;
        return Ok(1);
    }
    let scores = load_verdict_scores(&home.grader.join("scores.jsonl"))?;
    let rows = ordered_review_tasks(&state, &events, &scores)?;
    let mut missing = 0;
    for task in &rows {
        if scores.get(&run_or_empty(task))?.is_none() {
            missing += 1;
        }
    }
    // `scores.model_version or "none"`: an empty version prints as none too.
    let version = match &scores.model_version {
        Some(version) if !version.is_empty() => version.clone(),
        _ => PyStr::from("none"),
    };
    let noun = if missing == 1 { "task" } else { "tasks" };
    print(
        out,
        &text![
            "verdict ",
            version,
            "  ",
            missing.to_string(),
            " ",
            noun,
            " with no score"
        ],
    )?;
    writeln!(
        out,
        "{:<12} {:<14} {:<16} {:<36} {:<8} attempt",
        "task", "provider", "model", "run", "p"
    )?;
    for task in rows {
        let cell = |key: &str| -> PyStr {
            let value = task
                .assignment
                .as_ref()
                .and_then(|a| a.get(key))
                .cloned()
                .unwrap_or(Value::Null);
            if py::truthy(&value) {
                py::str(&value)
            } else {
                PyStr::new()
            }
        };
        let provider = cell("provider");
        let model = cell("model");
        let p_cell = format_p(scores.get(&run_or_empty(task))?);
        let attempt = if matches!(task.attempt, Value::Null) {
            PyStr::new()
        } else {
            py::str(&task.attempt)
        };
        print(
            out,
            &text![
                py::ljust(&task.id, 12)?,
                " ",
                py::pad(&provider, 14),
                " ",
                py::pad(&model, 16),
                " ",
                py::ljust(&run_or_empty(task), 36)?,
                " ",
                format!("{p_cell:<8}"),
                " ",
                attempt
            ],
        )?;
    }
    Ok(0)
}

/// What a goal log recorded as a verdict on one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunVerdict {
    pub goal_id: String,
    pub task_id: String,
    pub value: String,
    pub by: String,
    pub at: String,
}

fn field_text(event: &Value, key: &str) -> Option<String> {
    let Value::Dict(dict) = event else {
        return None;
    };
    dict.get(key).map(|value| py::str(value).lossy())
}

fn names_run(event: &Value, run_id: &str) -> bool {
    let Value::Dict(dict) = event else {
        return false;
    };
    dict.get("run_id").is_some_and(|id| py::eq_str(id, run_id))
}

fn is_verdict_event(event: &Value) -> bool {
    let Value::Dict(dict) = event else {
        return false;
    };
    dict.get("event")
        .is_some_and(|name| py::eq_str(name, "human_verdict") || py::eq_str(name, "verdict"))
}

fn given_by(event: &Value) -> String {
    if let Some(by) = field_text(event, "by").filter(|by| !by.is_empty()) {
        return by;
    }
    match field_text(event, "event").as_deref() {
        Some("human_verdict") => "human".into(),
        Some("panel_verdict") | Some("review") => {
            let judges = match event {
                Value::Dict(dict) => dict.get("judge_run_ids"),
                _ => None,
            };
            match judges {
                Some(Value::List(items)) if !items.is_empty() => "panel".into(),
                Some(value) if py::truthy(value) => "panel".into(),
                _ => "session".into(),
            }
        }
        _ => "session".into(),
    }
}

fn verdict_value(event: &Value) -> String {
    field_text(event, "result")
        .filter(|value| !value.is_empty())
        .or_else(|| field_text(event, "verdict"))
        .unwrap_or_default()
}

/// The latest verdict event that names `run_id`, walking every goal log under
/// `home` the way [`crate::report::goal_ids`] and [`log::read_log`] do. A
/// damaged log is skipped so one torn file does not hide another goal's
/// verdict. Nothing is written.
pub fn verdict_for_run(home: &Home, run_id: &str) -> Result<Option<RunVerdict>> {
    let mut found = None;
    for goal_id in crate::report::goal_ids(home)? {
        if goal_id.is_empty() {
            continue;
        }
        let events = match log::read_log(home, &goal_id) {
            Ok(events) => events,
            Err(_) => continue,
        };
        for event in events {
            if !is_verdict_event(&event) || !names_run(&event, run_id) {
                continue;
            }
            found = Some(RunVerdict {
                goal_id: goal_id.clone(),
                task_id: field_text(&event, "task").unwrap_or_default(),
                value: verdict_value(&event),
                by: given_by(&event),
                at: field_text(&event, "ts").unwrap_or_default(),
            });
        }
    }
    Ok(found)
}

#[cfg(test)]
mod task_text_tests {
    use super::*;

    #[test]
    fn show_composes_each_tasks_worker_text() {
        let dir = std::path::PathBuf::from(format!("show-task-text-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fixture = std::fs::read("tests/fixtures/task-text.jsonl").unwrap();
        std::fs::write(dir.join("fixture.jsonl"), fixture).unwrap();
        let home = Home {
            puppets: dir.clone(),
            grader: dir.clone(),
            root: dir.clone(),
            vendors: None,
        };
        let state = fold::fold("fixture", &log::read_log(&home, "fixture").unwrap()).unwrap();
        let mut out = Vec::new();
        assert_eq!(show(&home, "fixture", &mut out).unwrap(), 0);
        let Value::Dict(shown) = json::loads_bytes(&out).unwrap() else {
            panic!("show must print an object");
        };
        let Some(Value::Dict(tasks)) = shown.get("tasks") else {
            panic!("show must print tasks");
        };
        assert_eq!(tasks.len(), 3);
        for ((_, value), task) in tasks.iter().zip(&state.tasks) {
            let Value::Dict(value) = value else {
                panic!("task must be an object");
            };
            let Some(Value::Str(task_text)) = value.get("task_text") else {
                panic!("task text must be a string");
            };
            assert_eq!(task_text, &crate::launch::worker_task_text(task));
            assert!(py::eq(value.get("run_id").unwrap(), &task.run_id));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
