//! A goal's state, folded from its log the way `puppets.py`'s `fold` does, and the
//! questions the read commands ask of it: which approvals are live, which launches would be
//! refused, and what the helper is waiting on.

use std::path::Path;

use anyhow::{Result, bail};

use crate::json::{self, Dict, ParseError, PyStr, Value};
use crate::py;

/// One task as the fold leaves it. Fields hold whatever the log held, `Null` for Python's None.
#[derive(Clone, Debug)]
pub struct TaskState {
    /// The key `state.tasks` holds this task under: the first id seen for it, which a later
    /// `task` event with an equal id (`1` after `1.0`) does not replace.
    pub key: Value,
    pub id: Value,
    pub text: Value,
    pub task_class: Value,
    pub grade: Value,
    pub done: Value,
    pub deps: Vec<Value>,
    pub paths: Vec<Value>,
    pub assignment: Option<Dict>,
    pub status: Value,
    pub run_id: Value,
    pub attempt: Value,
    pub outcome: Value,
    pub human_result: Value,
    pub review_verdict: Value,
    pub review_degraded: Value,
    pub review_substitutions: Vec<Value>,
}

impl TaskState {
    fn new(id: Value, text: Value, task_class: Value, grade: Value) -> TaskState {
        TaskState {
            key: id.clone(),
            id,
            text,
            task_class,
            grade,
            done: Value::Null,
            deps: Vec::new(),
            paths: Vec::new(),
            assignment: None,
            status: Value::Str("pending".into()),
            run_id: Value::Null,
            attempt: Value::Null,
            outcome: Value::Null,
            human_result: Value::Null,
            review_verdict: Value::Null,
            review_degraded: Value::Bool(false),
            review_substitutions: Vec::new(),
        }
    }

    fn assignment_field(&self, key: &str) -> Value {
        self.assignment
            .as_ref()
            .and_then(|a| a.get(key))
            .cloned()
            .unwrap_or(Value::Null)
    }
}

/// A goal folded from its log.
#[derive(Clone, Debug)]
pub struct GoalState {
    pub goal_id: String,
    pub goal: Option<Dict>,
    /// Wide enough that no log that fits in memory saturates it: a `repair` adds at most
    /// 1,000,000,001.
    pub revision: i128,
    pub plans: Vec<Dict>,
    pub chosen_plan: Option<Dict>,
    pub human_plan: Option<Dict>,
    /// In first-seen order, keyed by `id` as a Python dict keys them.
    pub tasks: Vec<TaskState>,
    pub approvals: Vec<Dict>,
    pub dangling_intents: Vec<Value>,
    pub pending_runs: Vec<Dict>,
}

impl GoalState {
    /// `state.tasks.get(id)`.
    pub fn task(&self, id: &Value) -> Result<Option<&TaskState>> {
        py::hashable(id)?;
        Ok(self.tasks.iter().find(|t| py::same_key(&t.key, id)))
    }

    fn task_index(&self, id: &Value) -> Result<Option<usize>> {
        py::hashable(id)?;
        Ok(self.tasks.iter().position(|t| py::same_key(&t.key, id)))
    }
}

fn s(text: &str) -> Value {
    Value::text(text)
}

/// The most bad lines one `repair` event is believed about. `puppets.py` has the same number.
pub const MAX_REPAIR_REMOVED: i128 = 1_000_000_000;

fn dict(pairs: Vec<(&str, Value)>) -> Dict {
    let mut out = Dict::new();
    for (key, value) in pairs {
        out.set(key, value);
    }
    out
}

/// A Python set or dict keyed by log values, found the way a dict finds a key.
struct Keyed<T>(Vec<(Vec<Value>, T)>);

impl<T> Keyed<T> {
    fn position(&self, key: &[Value]) -> Result<Option<usize>> {
        for part in key {
            py::hashable(part)?;
        }
        Ok(self.0.iter().position(|(k, _)| {
            k.len() == key.len() && k.iter().zip(key).all(|(a, b)| py::same_key(a, b))
        }))
    }

    fn contains(&self, key: &[Value]) -> Result<bool> {
        Ok(self.position(key)?.is_some())
    }

    fn insert(&mut self, key: Vec<Value>, value: T) -> Result<()> {
        match self.position(&key)? {
            Some(at) => self.0[at].1 = value,
            None => self.0.push((key, value)),
        }
        Ok(())
    }

    fn remove(&mut self, key: &[Value]) -> Result<()> {
        if let Some(at) = self.position(key)? {
            self.0.remove(at);
        }
        Ok(())
    }
}

/// Fold a goal's events into its state.
pub fn fold(goal_id: &str, events: &[Value]) -> Result<GoalState> {
    let mut state = GoalState {
        goal_id: goal_id.to_string(),
        goal: None,
        revision: 0,
        plans: Vec::new(),
        chosen_plan: None,
        human_plan: None,
        tasks: Vec::new(),
        approvals: Vec::new(),
        dangling_intents: Vec::new(),
        pending_runs: Vec::new(),
    };
    let mut revision: i128 = 0;
    let mut consumed: Keyed<()> = Keyed(Vec::new());
    let mut intents: Keyed<Value> = Keyed(Vec::new());
    let mut runs_by_attempt: Keyed<Value> = Keyed(Vec::new());

    for (index, event) in events.iter().enumerate() {
        let name = py::item(event, "event")?;
        let seq = py::get(event, "seq", Value::Int(index as i128 + 1))?;
        // An event name is ASCII, so a name that is not a string or not UTF-8 matches none.
        let Value::Str(name) = name else { continue };
        let Some(name) = name.to_utf8() else { continue };
        match name.as_str() {
            "goal" => {
                state.goal = Some(dict(vec![
                    ("text", py::item(event, "text")?.clone()),
                    ("repo", py::item(event, "repo")?.clone()),
                    ("base_sha", py::item(event, "base_sha")?.clone()),
                ]));
            }
            "plan_proposed" => {
                state.plans.push(dict(vec![
                    ("provider", py::item(event, "provider")?.clone()),
                    ("model", py::get(event, "model", Value::Null)?),
                    ("run_id", py::item(event, "run_id")?.clone()),
                ]));
            }
            "plan_ready" => {
                for plan in &mut state.plans {
                    let run_id = plan.get("run_id").cloned().unwrap_or(Value::Null);
                    if py::eq(&run_id, py::item(event, "run_id")?) {
                        plan.set("path", py::item(event, "path")?.clone());
                    }
                }
            }
            "panel_verdict" => {
                state.chosen_plan = Some(dict(vec![
                    (
                        "judge_run_ids",
                        py::get(event, "judge_run_ids", Value::List(Vec::new()))?,
                    ),
                    ("chosen", py::item(event, "chosen")?.clone()),
                    ("path", py::get(event, "path", Value::Null)?),
                    ("degraded", py::get(event, "degraded", Value::Bool(false))?),
                    (
                        "substitutions",
                        py::get(event, "substitutions", Value::List(Vec::new()))?,
                    ),
                ]));
            }
            "human_plan" => {
                state.human_plan = Some(dict(vec![
                    ("accepted", py::item(event, "accepted")?.clone()),
                    ("note", py::get(event, "note", Value::Null)?),
                ]));
            }
            "task" => {
                revision += 1;
                let id = py::item(event, "id")?.clone();
                let mut task = TaskState::new(
                    id.clone(),
                    py::item(event, "text")?.clone(),
                    py::item(event, "class")?.clone(),
                    py::item(event, "grade")?.clone(),
                );
                task.done = py::get(event, "done", Value::Null)?;
                task.deps = py::list(&py::get(event, "deps", Value::List(Vec::new()))?)?;
                task.paths = py::list(&py::get(event, "paths", Value::List(Vec::new()))?)?;
                match state.task_index(&id)? {
                    Some(at) => {
                        // A dict keeps the first key it saw; only the value is replaced.
                        task.key = state.tasks[at].key.clone();
                        state.tasks[at] = task;
                    }
                    None => state.tasks.push(task),
                }
            }
            "assignment" => {
                revision += 1;
                let id = py::item(event, "task")?.clone();
                let at = match state.task_index(&id)? {
                    Some(at) => at,
                    None => {
                        state
                            .tasks
                            .push(TaskState::new(id, s(""), s("batch_scripting"), s("low")));
                        state.tasks.len() - 1
                    }
                };
                let assignment = dict(vec![
                    ("provider", py::item(event, "provider")?.clone()),
                    ("model", py::get(event, "model", s(""))?),
                    ("policy", py::item(event, "policy")?.clone()),
                    ("source", py::item(event, "source")?.clone()),
                ]);
                let task = &mut state.tasks[at];
                task.assignment = Some(assignment);
                if py::eq_str(&task.status, "pending") {
                    task.status = s("assigned");
                }
            }
            "approval" => {
                let kind = py::item(event, "kind")?.clone();
                let mut entry = dict(vec![
                    ("kind", kind.clone()),
                    ("ref", py::get(event, "ref", Value::Null)?),
                    ("run_id", py::get(event, "run_id", Value::Null)?),
                    ("revision", py::item(event, "revision")?.clone()),
                    ("yes", py::item(event, "yes")?.clone()),
                    ("seq", seq),
                    ("consumed", Value::Bool(false)),
                ]);
                if py::eq_str(&kind, "degraded") {
                    entry.set("missing", py::item(event, "missing")?.clone());
                    entry.set("substitute", py::item(event, "substitute")?.clone());
                    let model = py::get(event, "model", Value::Null)?;
                    if py::truthy(&model) {
                        entry.set("model", model);
                    }
                }
                state.approvals.push(entry);
            }
            "launch_intent" => {
                let key = vec![
                    py::item(event, "task")?.clone(),
                    py::item(event, "attempt")?.clone(),
                ];
                intents.insert(key, event.clone())?;
            }
            "run" => {
                let task_id = py::item(event, "task")?.clone();
                let attempt = py::item(event, "attempt")?.clone();
                let run_id = py::item(event, "run_id")?.clone();
                runs_by_attempt.insert(vec![task_id.clone(), attempt.clone()], run_id.clone())?;
                let at = state.task_index(&task_id)?;
                if let Some(at) = at {
                    let task = &mut state.tasks[at];
                    task.status = s("running");
                    task.run_id = run_id.clone();
                    task.attempt = attempt.clone();
                }
                state.pending_runs.push(dict(vec![
                    ("task", task_id.clone()),
                    ("attempt", attempt),
                    ("run_id", run_id),
                ]));
                // A table approval covers every launch at its revision (decision C); only
                // launch and diff approvals are spent by one use.
                if let Some(at) = at
                    && needs_launch_approval(&state.tasks[at])
                {
                    for approval in &state.approvals {
                        if field_eq_str(approval, "kind", "launch")
                            && py::eq(&field(approval, "ref"), &task_id)
                            && py::eq(&field(approval, "revision"), &Value::Int(revision))
                            && py::truthy(&field(approval, "yes"))
                            && !consumed.contains(&[field(approval, "seq")])?
                        {
                            consumed.insert(vec![field(approval, "seq")], ())?;
                            break;
                        }
                    }
                }
            }
            "run_end" => {
                // Python reads `run_id` only where it compares it: for a task it knows, and
                // once per pending run. With neither, an event without one is not an error.
                let task_id = py::item(event, "task")?.clone();
                if let Some(at) = state.task_index(&task_id)? {
                    let run_id = py::item(event, "run_id")?;
                    let task = &mut state.tasks[at];
                    if py::eq(&task.run_id, run_id) {
                        task.status = s("ended");
                        task.outcome = py::get(event, "outcome", Value::Null)?;
                    }
                }
                if !state.pending_runs.is_empty() {
                    let run_id = py::item(event, "run_id")?;
                    state
                        .pending_runs
                        .retain(|pending| !py::eq(&field(pending, "run_id"), run_id));
                }
            }
            "human_verdict" => {
                let task_id = py::item(event, "task")?.clone();
                if let Some(at) = state.task_index(&task_id)? {
                    let result = py::item(event, "result")?.clone();
                    let task = &mut state.tasks[at];
                    task.human_result = result.clone();
                    task.status = result;
                }
                let run_id = py::get(event, "run_id", Value::Null)?;
                for approval in &state.approvals {
                    if field_eq_str(approval, "kind", "diff")
                        && py::eq(&field(approval, "ref"), py::item(event, "task")?)
                        && py::eq(&field(approval, "run_id"), &run_id)
                        && py::eq(&field(approval, "revision"), &Value::Int(revision))
                        && py::truthy(&field(approval, "yes"))
                        && !consumed.contains(&[field(approval, "seq")])?
                    {
                        consumed.insert(vec![field(approval, "seq")], ())?;
                    }
                }
            }
            "intent_abandoned" => {
                let key = [
                    py::item(event, "task")?.clone(),
                    py::item(event, "attempt")?.clone(),
                ];
                intents.remove(&key)?;
            }
            "review" => {
                let task_id = py::item(event, "task")?.clone();
                if let Some(at) = state.task_index(&task_id)? {
                    let verdict = py::get(event, "verdict", Value::Null)?;
                    let degraded = py::get(event, "degraded", Value::Bool(false))?;
                    let substitutions = py::get(event, "substitutions", Value::Null)?;
                    let task = &mut state.tasks[at];
                    task.review_verdict = verdict;
                    task.review_degraded = degraded;
                    task.review_substitutions = if py::truthy(&substitutions) {
                        py::list(&substitutions)?
                    } else {
                        Vec::new()
                    };
                }
            }
            "repair" => {
                // Past every revision an approval before the repair could name. A `removed`
                // that is not an int from 1 to MAX_REPAIR_REMOVED counts as zero, so the
                // event still bumps by one. A bool is not an int here, as in the Python.
                let removed = match py::get(event, "removed", Value::Int(0))? {
                    Value::Int(n) if (1..=MAX_REPAIR_REMOVED).contains(&n) => n,
                    _ => 0,
                };
                revision += removed + 1;
            }
            _ => {}
        }
    }

    state.revision = revision;
    for (key, intent) in &intents.0 {
        if !runs_by_attempt.contains(key)? {
            state.dangling_intents.push(intent.clone());
        }
    }
    for approval in &mut state.approvals {
        if consumed.contains(&[field(approval, "seq")])? {
            approval.set("consumed", Value::Bool(true));
        } else if !field_eq_str(approval, "kind", "plan")
            && !py::eq(&field(approval, "revision"), &Value::Int(revision))
        {
            approval.set("voided", Value::Bool(true));
        }
    }
    Ok(state)
}

fn field(d: &Dict, key: &str) -> Value {
    d.get(key).cloned().unwrap_or(Value::Null)
}

fn field_eq_str(d: &Dict, key: &str, want: &str) -> bool {
    d.get(key).is_some_and(|v| py::eq_str(v, want))
}

/// `approval_available`: the first live `yes` approval of `kind` at the current revision.
/// `reference` is the task id for `launch` and `diff` and the missing provider for
/// `degraded`; `run_id` of `Null` does not narrow a `diff`.
pub fn approval_available<'a>(
    state: &'a GoalState,
    kind: &str,
    reference: &Value,
    run_id: &Value,
) -> Option<&'a Dict> {
    let revision = Value::Int(state.revision);
    state.approvals.iter().find(|approval| {
        if py::truthy(&field(approval, "consumed"))
            || py::truthy(&field(approval, "voided"))
            || !py::truthy(&field(approval, "yes"))
            || !field_eq_str(approval, "kind", kind)
        {
            return false;
        }
        // A plan approval is about the chosen plan, not the task table.
        if kind != "plan" && !py::eq(&field(approval, "revision"), &revision) {
            return false;
        }
        if kind == "degraded" && !py::eq(&field(approval, "missing"), reference) {
            return false;
        }
        if (kind == "launch" || kind == "diff") && !py::eq(&field(approval, "ref"), reference) {
            return false;
        }
        if kind == "diff"
            && !matches!(run_id, Value::Null)
            && !py::eq(&field(approval, "run_id"), run_id)
        {
            return false;
        }
        true
    })
}

/// Codex's gate can fail open on a dead hook, and claude's when it fails to start; those
/// launches need their own approval at `high` and `agi` for claude and always for codex.
pub fn needs_launch_approval(task: &TaskState) -> bool {
    let Some(assignment) = &task.assignment else {
        return false;
    };
    let provider = assignment.get("provider").cloned().unwrap_or(Value::Null);
    if py::eq_str(&provider, "codex") {
        return true;
    }
    py::eq_str(&provider, "claude")
        && (py::eq_str(&task.grade, "high") || py::eq_str(&task.grade, "agi"))
}

/// Why `launch` would refuse this task, from the fold alone. Only the start of the text is
/// read here, so a lone surrogate in a task id is spelled U+FFFD in it.
pub fn launch_refusal(state: &GoalState, task_id: &Value) -> Result<Option<String>> {
    if let Some(first) = state.dangling_intents.first() {
        let attempt = py::str(py::item(first, "attempt")?).lossy();
        return Ok(Some(format!(
            "ambiguous launch intent attempt {attempt}; reconcile with hydra status before launching again"
        )));
    }
    let Some(task) = state.task(task_id)? else {
        return Ok(Some(format!("unknown task {}", py::str(task_id).lossy())));
    };
    let id = py::str(task_id).lossy();
    if task.assignment.is_none() {
        return Ok(Some(format!("task {id} has no assignment")));
    }
    if approval_available(state, "table", &Value::Null, &Value::Null).is_none() {
        return Ok(Some(format!(
            "missing table approval at revision {}; approve with: approve {} --table",
            state.revision, state.goal_id
        )));
    }
    if needs_launch_approval(task)
        && approval_available(state, "launch", task_id, &Value::Null).is_none()
    {
        let reason = if py::eq_str(&task.assignment_field("provider"), "codex") {
            "codex gate can fail open on a dead hook"
        } else {
            "claude gate can fail open when it fails to start"
        };
        return Ok(Some(format!(
            "missing launch approval for task {id} at revision {} ({reason}); approve with: approve {} --launch {id}",
            state.revision, state.goal_id
        )));
    }
    Ok(None)
}

/// The panel seats `puppets.py` sits, in order, by provider.
pub const PANEL_PROVIDERS: [&str; 3] = ["claude", "codex", "cursor-agent"];

/// The text `approve` prints for a seat that is down; not itself a substitute.
pub const DEGRADED_SUBSTITUTE_PLACEHOLDER: &str = "<substitute-provider[:model]>";

/// Whether `vendors.json` marks a provider available. A file that is missing or not JSON
/// counts as available; a provider it does not name is available.
pub fn provider_available(vendors: &Path, provider: &str) -> Result<bool> {
    let raw = match std::fs::read(vendors) {
        Ok(raw) => raw,
        Err(_) => return Ok(true),
    };
    let text =
        std::str::from_utf8(&raw).map_err(|err| anyhow::anyhow!("UnicodeDecodeError: {err}"))?;
    let rows = match json::loads_str(text) {
        Ok(Value::List(rows)) => rows,
        // Iterating an empty dict or string finds no rows. A dict's keys and a string's
        // characters are strings, which have no `get`; nothing else iterates.
        Ok(Value::Dict(d)) if d.is_empty() => return Ok(true),
        Ok(Value::Str(t)) if t.is_empty() => return Ok(true),
        Ok(Value::Dict(_) | Value::Str(_)) => {
            bail!("AttributeError: 'str' object has no attribute 'get'")
        }
        Ok(_) => bail!("TypeError: vendors.json is not iterable"),
        Err(ParseError::Decode) => return Ok(true),
        Err(err) => bail!("{}: vendors.json", err.class()),
    };
    let mut matching = Vec::new();
    for row in &rows {
        if py::eq_str(&py::get(row, "provider", Value::Null)?, provider) {
            matching.push(row);
        }
    }
    let Some(first) = matching.first() else {
        return Ok(true);
    };
    let mut chosen = *first;
    for row in &matching {
        let model = py::get(row, "model", Value::Null)?;
        if matches!(model, Value::Null) || py::eq_str(&model, "") {
            chosen = row;
            break;
        }
    }
    Ok(py::truthy(&py::get(
        chosen,
        "available",
        Value::Bool(true),
    )?))
}

/// The missing provider `resolve_panel_seats` would ask the human to approve, if any: the
/// first seat it cannot fill. A degraded approval with an empty substitute stops it with a
/// refusal that is not an approval wait.
pub fn degraded_provider_waiting(state: &GoalState, vendors: &Path) -> Result<Option<String>> {
    for provider in PANEL_PROVIDERS {
        if provider_available(vendors, provider)? {
            continue;
        }
        let Some(approval) = approval_available(state, "degraded", &s(provider), &Value::Null)
        else {
            return Ok(Some(provider.to_string()));
        };
        if !py::truthy(&field(approval, "substitute")) {
            return Ok(None);
        }
    }
    Ok(None)
}

/// Whether a panel command would consult the seats: two proposals with paths, or a run.
pub fn panel_gate_reachable(state: &GoalState) -> bool {
    let ready = state
        .plans
        .iter()
        .filter(|p| py::truthy(&field(p, "path")))
        .count();
    ready >= 2 || state.tasks.iter().any(|t| py::truthy(&t.run_id))
}

/// `approve_argv`: the command that records one approval at the revision it was drawn from.
/// `helper` is the `hydra-puppets` that listed it (build 196).
pub fn approve_argv(helper: &str, goal_id: &str, revision: i128, args: &[Value]) -> Value {
    let mut argv = vec![s(helper), s("approve"), s(goal_id)];
    argv.extend(args.iter().cloned());
    argv.push(s("--expect-revision"));
    argv.push(s(&revision.to_string()));
    Value::List(argv)
}

/// `pending_approvals`: what the helper is waiting on, in the order `puppets.py` lists it.
pub fn pending_approvals(state: &GoalState, vendors: &Path, helper: &str) -> Result<Vec<Value>> {
    let item = |kind: &str, reference: Value, args: Vec<Value>| {
        Value::Dict(dict(vec![
            ("kind", s(kind)),
            ("ref", reference),
            ("revision", Value::Int(state.revision)),
            (
                "argv",
                approve_argv(helper, &state.goal_id, state.revision, &args),
            ),
        ]))
    };
    let mut items = Vec::new();
    if let Some(chosen) = &state.chosen_plan
        && py::truthy(&field(chosen, "path"))
        && approval_available(state, "plan", &Value::Null, &Value::Null).is_none()
    {
        items.push(item(
            "plan",
            Value::Null,
            vec![s("--plan"), s("--expect-plan"), field(chosen, "path")],
        ));
    }
    let mut seen_table = false;
    for task in &state.tasks {
        let Some(refusal) = launch_refusal(state, &task.id)? else {
            continue;
        };
        if refusal.starts_with("missing table approval") && !seen_table {
            items.push(item("table", Value::Null, vec![s("--table")]));
            seen_table = true;
        } else if refusal.starts_with("missing launch approval")
            && py::eq_str(&task.status, "assigned")
        {
            items.push(item(
                "launch",
                task.id.clone(),
                vec![s("--launch"), task.id.clone()],
            ));
        }
    }
    for task in &state.tasks {
        if !py::truthy(&task.run_id) || !matches!(task.human_result, Value::Null) {
            continue;
        }
        if approval_available(state, "diff", &task.id, &task.run_id).is_none() {
            items.push(item(
                "diff",
                task.id.clone(),
                vec![
                    s("--diff"),
                    task.id.clone(),
                    s("--expect-run"),
                    task.run_id.clone(),
                ],
            ));
        }
    }
    if panel_gate_reachable(state)
        && let Some(provider) = degraded_provider_waiting(state, vendors)?
    {
        let spec = format!("{provider}={DEGRADED_SUBSTITUTE_PLACEHOLDER}");
        items.push(item(
            "degraded",
            s(&provider),
            vec![s("--degraded"), s(&spec)],
        ));
    }
    Ok(items)
}

/// `state_to_dict`, the shape `show` prints.
pub fn state_to_value(state: &GoalState) -> Value {
    let opt = |d: &Option<Dict>| d.clone().map_or(Value::Null, Value::Dict);
    let mut tasks = Dict::new();
    for task in &state.tasks {
        let body = dict(vec![
            ("id", task.id.clone()),
            ("text", task.text.clone()),
            ("class", task.task_class.clone()),
            ("grade", task.grade.clone()),
            ("deps", Value::List(task.deps.clone())),
            ("paths", Value::List(task.paths.clone())),
            ("assignment", opt(&task.assignment)),
            ("status", task.status.clone()),
            ("run_id", task.run_id.clone()),
            ("attempt", task.attempt.clone()),
            ("outcome", task.outcome.clone()),
            ("human_result", task.human_result.clone()),
        ]);
        tasks.push(json_key(&task.key), Value::Dict(body));
    }
    Value::Dict(dict(vec![
        ("goal_id", s(&state.goal_id)),
        ("goal", opt(&state.goal)),
        ("revision", Value::Int(state.revision)),
        (
            "plans",
            Value::List(state.plans.iter().cloned().map(Value::Dict).collect()),
        ),
        ("chosen_plan", opt(&state.chosen_plan)),
        ("human_plan", opt(&state.human_plan)),
        ("tasks", Value::Dict(tasks)),
        (
            "approvals",
            Value::List(state.approvals.iter().cloned().map(Value::Dict).collect()),
        ),
        (
            "dangling_intents",
            Value::List(state.dangling_intents.clone()),
        ),
        (
            "pending_runs",
            Value::List(
                state
                    .pending_runs
                    .iter()
                    .cloned()
                    .map(Value::Dict)
                    .collect(),
            ),
        ),
    ]))
}

/// How `json.dumps` spells a non-string dict key.
fn json_key(key: &Value) -> PyStr {
    match key {
        Value::Str(text) => text.clone(),
        Value::Null => "null".into(),
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => "false".into(),
        other => json::dumps(other, None).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOAL: &str = r#"{"event":"goal","text":"x","repo":"/r","base_sha":"a"}"#;
    const TASK: &str = r#"{"event":"task","id":"t1","text":"t","class":"c","grade":"low"}"#;

    fn folded(lines: &[&str]) -> Result<GoalState> {
        let events: Vec<Value> = lines
            .iter()
            .map(|line| json::loads_str(line).unwrap())
            .collect();
        fold("g1", &events)
    }

    /// Build 199, item 12, the rule `puppets.py` has too: `removed` counts when it is an int
    /// from 1 to a billion.
    #[test]
    fn a_repair_counts_removed_only_from_one_to_a_billion() {
        let revision = |removed: &str| {
            let repair = format!(r#"{{"event":"repair","removed":{removed}}}"#);
            folded(&[GOAL, &repair]).unwrap().revision
        };
        assert_eq!(revision("1"), 2);
        assert_eq!(revision("7"), 8);
        assert_eq!(revision("1000000000"), 1_000_000_001);
        for ignored in [
            "1000000001",
            "9223372036854775807",
            "1606938044258990275541962092341162602522202993782792835301376",
            "0",
            "-3",
            "true",
            "2.0",
            "\"2\"",
            "null",
            "[2]",
        ] {
            assert_eq!(revision(ignored), 1, "{ignored}");
        }
        assert_eq!(
            folded(&[GOAL, r#"{"event":"repair"}"#]).unwrap().revision,
            1
        );
    }

    /// Two repairs that claim `i64::MAX` bad lines used to pin the revision at that value,
    /// so an approval recorded between them at that revision stayed live.
    #[test]
    fn an_approval_between_two_wide_repairs_is_void() {
        let wide = r#"{"event":"repair","removed":9223372036854775807}"#;
        for at in ["2", "9223372036854775807"] {
            let approval =
                format!(r#"{{"event":"approval","kind":"table","revision":{at},"yes":true}}"#);
            let state = folded(&[GOAL, TASK, wide, &approval, wide]).unwrap();
            assert_eq!(state.revision, 3);
            assert!(
                state.approvals[0].get("voided").is_some_and(py::truthy),
                "{at}"
            );
            assert!(approval_available(&state, "table", &Value::Null, &Value::Null).is_none());
        }
    }

    /// Build 199, item 6: Python reads `run_id` for a task it knows and once per pending
    /// run, so an event without one is an error only where one of those exists.
    #[test]
    fn run_end_reads_run_id_only_where_python_does() {
        let run_end = r#"{"event":"run_end","task":"missing"}"#;
        assert!(folded(&[GOAL, run_end]).is_ok());
        let pending = r#"{"event":"run","task":"t9","attempt":1,"run_id":"r"}"#;
        let err = folded(&[GOAL, pending, run_end]).unwrap_err();
        assert_eq!(err.to_string(), "KeyError: 'run_id'");
        let known = r#"{"event":"run_end","task":"t1"}"#;
        let err = folded(&[GOAL, TASK, known]).unwrap_err();
        assert_eq!(err.to_string(), "KeyError: 'run_id'");
        let err = folded(&[GOAL, r#"{"event":"run_end"}"#]).unwrap_err();
        assert_eq!(err.to_string(), "KeyError: 'task'");
    }
}
