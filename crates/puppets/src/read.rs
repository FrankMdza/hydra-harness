//! The read commands: `show`, `board`, `pending --json` and `queue`. Each writes what
//! `puppets.py` prints to `out`, line by line, so a value Python would choke on stops the
//! output at the same line, and returns the exit code or the error Python would raise.

use std::io::Write;
use std::path::Path;

use anyhow::{Result, anyhow};

use crate::fold::{self, GoalState, TaskState, approval_available, needs_launch_approval};
use crate::json::{self, ParseError, Value};
use crate::log::{self, Home};
use crate::py;

/// `show`: the folded state as indented JSON. An unknown goal prints its empty state.
pub fn show(home: &Home, goal_id: &str, out: &mut dyn Write) -> Result<i32> {
    let state = fold::fold(goal_id, &log::read_log(home, goal_id)?)?;
    writeln!(
        out,
        "{}",
        json::dumps(&fold::state_to_value(&state), Some(2))
    )?;
    Ok(0)
}

/// `pending --json`. `python` and `script` begin each `argv`, the command that records the
/// approval, which is the Python helper's until `approve` moves to this crate.
pub fn pending(
    home: &Home,
    goal_id: &str,
    python: &str,
    script: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32> {
    let state = fold::fold(goal_id, &log::read_log(home, goal_id)?)?;
    if state.goal.is_none() {
        writeln!(err, "unknown goal {goal_id}")?;
        return Ok(1);
    }
    let items = fold::pending_approvals(&state, &home.vendors, python, script)?;
    writeln!(out, "{}", json::dumps(&Value::List(items), Some(2)))?;
    Ok(0)
}

/// Verdict-model probabilities from `scores.jsonl`, for the newest verdict `model_version`
/// only: the verdict line with the latest `scored_at`, a later line breaking a tie.
#[derive(Debug, Default)]
pub struct VerdictScores {
    pub model_version: Option<String>,
    pub by_run: Vec<(String, f64)>,
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

/// `str.splitlines()`, which also breaks on the separators below `\x20` and above `\x7f`.
fn str_splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let breaks = matches!(
            c,
            '\n' | '\r'
                | '\u{b}'
                | '\u{c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !breaks {
            continue;
        }
        lines.push(&text[start..i]);
        let mut end = i + c.len_utf8();
        if c == '\r'
            && let Some((j, '\n')) = chars.peek().copied()
        {
            chars.next();
            end = j + 1;
        }
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// `load_verdict_scores`. A missing file is an empty score set; nothing is written.
pub fn load_verdict_scores(path: &Path) -> Result<VerdictScores> {
    if !path.is_file() {
        return Ok(VerdictScores::default());
    }
    let raw = std::fs::read(path)?;
    let text = std::str::from_utf8(&raw).map_err(|e| anyhow!("UnicodeDecodeError: {e}"))?;
    let mut rows: Vec<(String, String, f64)> = Vec::new();
    let mut best: Option<(String, usize)> = None;
    let mut best_version: Option<String> = None;
    for (index, line) in str_splitlines(text).into_iter().enumerate() {
        let line = py::strip(line);
        if line.is_empty() {
            continue;
        }
        let row = match json::loads(line) {
            Ok(row) => row,
            Err(ParseError::Decode) => continue,
            Err(ParseError::Unsupported) => {
                return Err(anyhow!("scores.jsonl holds a lone surrogate"));
            }
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
            _ => String::new(),
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
    let mut by_run: Vec<(String, f64)> = Vec::new();
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
fn human_verdict_runs(events: &[Value]) -> Vec<(String, Vec<String>)> {
    let mut found: Vec<(String, Vec<String>)> = Vec::new();
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
fn awaits_review(task: &TaskState, verdicts: &[(String, Vec<String>)]) -> Result<bool> {
    if !py::eq_str(&task.status, "ended") || !py::truthy(&task.run_id) {
        return Ok(false);
    }
    py::hashable(&task.id)?;
    py::hashable(&task.run_id)?;
    let (Value::Str(id), Value::Str(run)) = (&task.id, &task.run_id) else {
        return Ok(true);
    };
    Ok(!verdicts
        .iter()
        .any(|(t, runs)| t == id && runs.contains(run)))
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
        Value::Str(String::new())
    }
}

/// Waiting tasks, highest `p` first, unscored last, then by task id.
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
    let mut keyed = Vec::new();
    for task in waiting {
        let key = match scores.get(&run_or_empty(task))? {
            None => (1, 0.0),
            Some(p) => (0, -p),
        };
        keyed.push((key, task));
    }
    // Python's sort is stable and compares ids only on a tie; a pairwise error surfaces.
    let mut error = None;
    keyed.sort_by(|(ka, a), (kb, b)| {
        let by_key =
            ka.0.cmp(&kb.0)
                .then(ka.1.partial_cmp(&kb.1).unwrap_or(std::cmp::Ordering::Equal));
        by_key.then_with(|| match (py::less(&a.id, &b.id), py::less(&b.id, &a.id)) {
            (Ok(true), _) => std::cmp::Ordering::Less,
            (Ok(false), Ok(true)) => std::cmp::Ordering::Greater,
            (Ok(false), Ok(false)) => std::cmp::Ordering::Equal,
            (Err(e), _) | (_, Err(e)) => {
                error.get_or_insert(e);
                std::cmp::Ordering::Equal
            }
        })
    });
    if let Some(e) = error {
        return Err(e);
    }
    Ok(keyed.into_iter().map(|(_, task)| task).collect())
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
    writeln!(
        out,
        "  {}",
        py::str(goal.get("text").unwrap_or(&Value::Null))
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
            writeln!(out, "  {provider} {run}  {ready}")?;
        }
    }
    if let Some(chosen) = &state.chosen_plan {
        let null = Value::Null;
        let name = py::str(chosen.get("chosen").unwrap_or(&null));
        let judges = py::len(chosen.get("judge_run_ids").unwrap_or(&null))?;
        if chosen.get("degraded").is_some_and(py::truthy) {
            let subs = chosen.get("substitutions").cloned().unwrap_or(Value::Null);
            let subs = if py::truthy(&subs) {
                py::list(&subs)?
            } else {
                Vec::new()
            };
            let mut parts = Vec::new();
            for sub in &subs {
                parts.push(format!(
                    "{}->{}",
                    py::str(py::item(sub, "missing")?),
                    py::str(py::item(sub, "substitute")?)
                ));
            }
            writeln!(
                out,
                "chosen (degraded panel)  {name}  judges {judges}  {}",
                parts.join(", ")
            )?;
        } else {
            writeln!(out, "chosen  {name}  judges {judges}")?;
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
            let mut agent = "(unassigned)".to_string();
            if let Some(assignment) = &task.assignment
                && !assignment.is_empty()
            {
                let provider = py::str(assignment.get("provider").unwrap_or(&Value::Null));
                let model = assignment
                    .get("model")
                    .filter(|m| py::truthy(m))
                    .map(py::str)
                    .unwrap_or_default();
                agent = py::strip(&format!("{provider} {model}")).to_string();
            }
            let run = py::str(&py::head(&run_or_empty(task), 8)?);
            let mut verdict = if py::truthy(&task.human_result) {
                format!("  {}", py::str(&task.human_result))
            } else {
                String::new()
            };
            if py::truthy(&task.review_verdict) && !py::truthy(&task.human_result) {
                let mut label = py::str(&task.review_verdict);
                if py::truthy(&task.review_degraded) {
                    label = format!("{label} (degraded panel)");
                }
                verdict = format!("  review {label}");
            }
            let mut p_cell = String::new();
            if awaits_review(task, &verdicts)? {
                p_cell = format_p(scores.get(&run_or_empty(task))?);
            }
            writeln!(
                out,
                "  {} {} {} {} {} {} {run}{verdict}",
                py::ljust(&task.id, 12)?,
                py::ljust(&task.task_class, 21)?,
                py::ljust(&task.grade, 6)?,
                py::pad(&agent, 28),
                py::ljust(&task.status, 9)?,
                py::pad(&p_cell, 8),
            )?;
        }
    }
    let none = Value::Null;
    let mut waiting: Vec<String> = Vec::new();
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
            waiting.push(format!("launch {}", py::str(&task.id)));
        }
        if py::eq_str(&task.status, "ended")
            && approval_available(&state, "diff", &task.id, &task.run_id).is_none()
        {
            waiting.push(format!("diff {}", py::str(&task.id)));
        }
    }
    if !state.dangling_intents.is_empty() {
        let mut parts = Vec::new();
        for intent in &state.dangling_intents {
            parts.push(format!(
                "{}#{}",
                py::str(py::item(intent, "task")?),
                py::str(py::item(intent, "attempt")?)
            ));
        }
        waiting.push(format!("reconcile {}", parts.join(", ")));
    }
    let waiting = if waiting.is_empty() {
        "nothing".to_string()
    } else {
        waiting.join(", ")
    };
    writeln!(out, "waiting on you: {waiting}")?;
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
    let version = scores.model_version.as_deref().unwrap_or("none");
    let noun = if missing == 1 { "task" } else { "tasks" };
    writeln!(out, "verdict {version}  {missing} {noun} with no score")?;
    writeln!(
        out,
        "{:<12} {:<14} {:<16} {:<36} {:<8} attempt",
        "task", "provider", "model", "run", "p"
    )?;
    for task in rows {
        let cell = |key: &str| -> String {
            let value = task
                .assignment
                .as_ref()
                .and_then(|a| a.get(key))
                .cloned()
                .unwrap_or(Value::Null);
            if py::truthy(&value) {
                py::str(&value)
            } else {
                String::new()
            }
        };
        let provider = cell("provider");
        let model = cell("model");
        let p_cell = format_p(scores.get(&run_or_empty(task))?);
        let attempt = if matches!(task.attempt, Value::Null) {
            String::new()
        } else {
            py::str(&task.attempt)
        };
        writeln!(
            out,
            "{} {provider:<14} {model:<16} {} {p_cell:<8} {attempt}",
            py::ljust(&task.id, 12)?,
            py::ljust(&run_or_empty(task), 36)?,
        )?;
    }
    Ok(0)
}
