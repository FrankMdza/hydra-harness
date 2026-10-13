//! Builder acceptance and token cost from goal logs.
use crate::fold;
use crate::json::{self, Dict, Value};
use crate::log::{self, Home};
use crate::{py, sheet};
use anyhow::Result;
use std::io::Write;

#[derive(Clone, Default)]
struct Counts {
    tasks: i128,
    first_pass: i128,
    accepted: i128,
    rejections: i128,
    attempts: i128,
    tokens: i128,
    no_verdict: i128,
}
impl Counts {
    fn add(&mut self, other: &Self) {
        self.tasks += other.tasks;
        self.first_pass += other.first_pass;
        self.accepted += other.accepted;
        self.rejections += other.rejections;
        self.attempts += other.attempts;
        self.tokens += other.tokens;
        self.no_verdict += other.no_verdict;
    }
}
struct Row {
    class: Value,
    provider: Value,
    model: Value,
    counts: Counts,
}
struct TaskRuns {
    task: Value,
    class: Value,
    provider: Value,
    model: Value,
    runs: Vec<Value>,
}
fn field(value: &Value, key: &str) -> Value {
    match value {
        Value::Dict(d) => d.get(key).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
fn add_row(rows: &mut Vec<Row>, class: &Value, provider: &Value, model: &Value, counts: &Counts) {
    if let Some(row) = rows.iter_mut().find(|r| {
        py::eq(&r.class, class) && py::eq(&r.provider, provider) && py::eq(&r.model, model)
    }) {
        row.counts.add(counts);
    } else {
        rows.push(Row {
            class: class.clone(),
            provider: provider.clone(),
            model: model.clone(),
            counts: counts.clone(),
        });
    }
}
fn ratio(n: i128, d: i128) -> Value {
    if d == 0 {
        Value::Null
    } else {
        Value::Float(n as f64 / d as f64)
    }
}
fn object(row: &Row, total: bool) -> Value {
    let c = &row.counts;
    let mut d = Dict::new();
    d.set("total", Value::Bool(total));
    d.set("class", row.class.clone());
    d.set("provider", row.provider.clone());
    d.set("model", row.model.clone());
    for (key, n) in [
        ("tasks", c.tasks),
        ("first_pass_accepted", c.first_pass),
        ("eventually_accepted", c.accepted),
        ("rejections", c.rejections),
        ("attempts", c.attempts),
        ("total_tokens", c.tokens),
        ("no_verdict", c.no_verdict),
    ] {
        d.set(key, Value::Int(n));
    }
    d.set("first_pass_acceptance", ratio(c.first_pass, c.tasks));
    d.set("eventual_acceptance", ratio(c.accepted, c.tasks));
    d.set("attempts_per_task", ratio(c.attempts, c.tasks));
    d.set("tokens_per_accepted_task", ratio(c.tokens, c.accepted));
    Value::Dict(d)
}
fn print_row(row: &Row, total: bool, as_json: bool, out: &mut dyn Write) -> Result<()> {
    if as_json {
        writeln!(out, "{}", json::dumps_default(&object(row, total)))?;
    } else {
        let c = &row.counts;
        let rate = |n, d| {
            if d == 0 {
                "-".to_string()
            } else {
                format!("{:.2}", n as f64 / d as f64)
            }
        };
        let class = if total {
            "TOTAL".to_string()
        } else {
            py::str(&row.class).lossy()
        };
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            class,
            py::str(&row.provider).lossy(),
            py::str(&row.model).lossy(),
            c.tasks,
            c.first_pass,
            rate(c.first_pass, c.tasks),
            c.accepted,
            rate(c.accepted, c.tasks),
            c.rejections,
            c.attempts,
            rate(c.attempts, c.tasks),
            c.tokens,
            rate(c.tokens, c.accepted),
            c.no_verdict
        )?;
    }
    Ok(())
}
/// Read every top-level goal log, including complete lines before a torn tail.
pub fn builders(
    home: &Home,
    class: Option<&str>,
    model: Option<&str>,
    as_json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32> {
    let mut names = Vec::new();
    match std::fs::read_dir(&home.puppets) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                if entry.file_type()?.is_file()
                    && entry.path().extension().is_some_and(|e| e == "jsonl")
                {
                    names.push(entry.file_name());
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    names.sort();
    let mut rows = Vec::new();
    for name in names {
        let goal = name
            .to_string_lossy()
            .strip_suffix(".jsonl")
            .unwrap()
            .to_string();
        let events = log::read_log(home, &goal)?;
        if log::torn_tail(&std::fs::read(home.goal_log_path(&goal))?).is_some() {
            writeln!(
                err,
                "goal log {goal}: unterminated tail; counted complete lines"
            )?;
        }
        let state = fold::fold(&goal, &events)?;
        let mut groups: Vec<TaskRuns> = Vec::new();
        for (i, event) in events.iter().enumerate() {
            if !py::eq_str(&field(event, "event"), "run") {
                continue;
            }
            // Use the assignment in force at launch, even after reassignment.
            let at_launch = fold::fold(&goal, &events[..=i])?;
            let task_id = field(event, "task");
            let Some(task) = at_launch.task(&task_id)? else {
                continue;
            };
            let Some(assignment) = &task.assignment else {
                continue;
            };
            let provider = assignment.get("provider").cloned().unwrap_or(Value::Null);
            let model = assignment.get("model").cloned().unwrap_or(Value::Null);
            let task_class = state
                .task(&task_id)?
                .map(|t| t.task_class.clone())
                .unwrap_or_else(|| task.task_class.clone());
            let at = groups.iter().position(|g| {
                py::same_key(&g.task, &task_id)
                    && py::eq(&g.provider, &provider)
                    && py::eq(&g.model, &model)
            });
            let at = at.unwrap_or_else(|| {
                groups.push(TaskRuns {
                    task: task_id.clone(),
                    class: task_class,
                    provider,
                    model,
                    runs: Vec::new(),
                });
                groups.len() - 1
            });
            let mut run = match event {
                Value::Dict(d) => d.clone(),
                _ => unreachable!(),
            };
            run.set("task_id", task_id);
            // Goal logs number attempts rather than carrying a parent link.
            if matches!(field(event, "attempt"), Value::Int(n) if n > 1)
                && !py::truthy(&field(event, "parent_run_id"))
            {
                run.set("parent_run_id", Value::Bool(true));
            }
            if !groups[at]
                .runs
                .iter()
                .any(|r| py::eq(&field(r, "run_id"), &field(event, "run_id")))
            {
                groups[at].runs.push(Value::Dict(run));
            }
        }
        for task in &state.tasks {
            if groups.iter().any(|g| py::same_key(&g.task, &task.key)) {
                continue;
            }
            if let Some(a) = &task.assignment {
                groups.push(TaskRuns {
                    task: task.key.clone(),
                    class: task.task_class.clone(),
                    provider: a.get("provider").cloned().unwrap_or(Value::Null),
                    model: a.get("model").cloned().unwrap_or(Value::Null),
                    runs: Vec::new(),
                });
            }
        }
        for group in groups {
            if class.is_some_and(|c| !py::eq_str(&group.class, c))
                || model.is_some_and(|m| !py::eq_str(&group.model, m))
            {
                continue;
            }
            let mut c = Counts {
                tasks: 1,
                attempts: group.runs.len() as i128,
                ..Counts::default()
            };
            let mut seen = Vec::new();
            let mut accepted = false;
            let mut rejected_runs = Vec::new();
            let mut ended_runs = Vec::new();
            for event in &events {
                let Some(run) = group
                    .runs
                    .iter()
                    .find(|r| py::eq(&field(r, "run_id"), &field(event, "run_id")))
                else {
                    continue;
                };
                let id = field(run, "run_id");
                if py::eq_str(&field(event, "event"), "run_end")
                    && !ended_runs.iter().any(|r| py::eq(r, &id))
                {
                    if let Value::Int(tokens) = field(event, "total_tokens") {
                        c.tokens += tokens;
                    }
                    ended_runs.push(id.clone());
                }
                if !py::eq_str(&field(event, "event"), "human_verdict") {
                    continue;
                }
                if let Some(result) = sheet::first_builder_verdict(&mut seen, event, run)? {
                    c.first_pass = i128::from(py::eq_str(&result, "applied"));
                }
                if let Some(result) = sheet::builder_verdict(event, run)? {
                    accepted |= py::eq_str(&result, "applied") || py::eq_str(&result, "fixed");
                    if py::eq_str(&result, "rejected")
                        && !rejected_runs.iter().any(|r| py::eq(r, &id))
                    {
                        c.rejections += 1;
                        rejected_runs.push(id);
                    }
                }
            }
            c.accepted = i128::from(accepted);
            c.no_verdict = i128::from(seen.is_empty());
            add_row(&mut rows, &group.class, &group.provider, &group.model, &c);
        }
    }
    rows.sort_by_key(|r| {
        (
            py::str(&r.class).lossy(),
            py::str(&r.provider).lossy(),
            py::str(&r.model).lossy(),
        )
    });
    if !as_json {
        writeln!(
            out,
            "class\tprovider\tmodel\ttasks\tfirst_pass_accepted\tfirst_pass_acceptance\teventually_accepted\teventual_acceptance\trejections\tattempts\tattempts_per_task\ttotal_tokens\ttokens_per_accepted_task\tno_verdict"
        )?;
    }
    let mut totals = Vec::new();
    for row in &rows {
        print_row(row, false, as_json, out)?;
        add_row(
            &mut totals,
            &Value::Null,
            &row.provider,
            &row.model,
            &row.counts,
        );
    }
    totals.sort_by_key(|r| (py::str(&r.provider).lossy(), py::str(&r.model).lossy()));
    for row in &totals {
        print_row(row, true, as_json, out)?;
    }
    Ok(0)
}
