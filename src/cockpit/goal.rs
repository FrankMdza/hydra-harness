//! The task table of one goal, folded from its Master of Puppets log.
//!
//! **This is `puppets.py`'s `fold` narrowed to the table `board` prints**, and
//! it reads the JSONL itself rather than shelling out to Python. The events and
//! their fields are the ones `integrations/puppets/README.md` documents; the
//! rules below are `fold`'s, event by event, and only the ones that change a
//! row. Plans, approvals and intents are the helper's business and the cockpit
//! shows none of them.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

/// One row of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRow {
    pub id: String,
    pub class: String,
    pub grade: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    /// `pending`, `assigned`, `running`, `ended`, or the human's verdict.
    pub status: String,
    pub run_id: Option<String>,
    /// The plan's definition of done, when it stated one.
    pub done: Option<String>,
}

/// One `review` event: the panel's verdict on a task's diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    pub task: String,
    /// `pass` or `block`, or `None` when the event carries none.
    pub verdict: Option<String>,
    /// Whether a substitute filled a missing seat.
    pub degraded: bool,
}

impl Goal {
    /// How many reviews of `task` the log holds, and the last one.
    pub fn reviews_of(&self, task: &str) -> (usize, Option<&Review>) {
        let mut of = self.reviews.iter().filter(|r| r.task == task);
        let count = of.clone().count();
        (count, of.next_back())
    }
}

/// One goal, as far as the table needs it.
#[derive(Clone, Debug, Default)]
pub struct Goal {
    pub id: String,
    pub text: Option<String>,
    /// In the order `fold` keeps them: a task is where it first appeared, even
    /// after a later `task` event replaces it.
    pub rows: Vec<TaskRow>,
    /// The `review` events `puppets.py review` appended, in log order, for
    /// tasks the table has. Build ticket 183 reads the last one per task.
    pub reviews: Vec<Review>,
    /// Complete lines that were not JSON or lacked a field `fold` needs.
    /// `fold` raises on those; a viewer counts them and carries on.
    pub skipped: usize,
}

/// Reads and folds `<home>/<id>.jsonl`.
pub fn read(path: &Path) -> Result<Goal> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let id = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(fold(&id, &bytes))
}

/// The goal log under `home` that changed last, which is the one a human
/// running a single goal means.
pub fn latest(home: &Path) -> Result<Option<PathBuf>> {
    let entries = match std::fs::read_dir(home) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("reading {}", home.display())),
    };
    let newest = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let changed = path.metadata().and_then(|meta| meta.modified()).ok()?;
            Some((changed, path))
        })
        .max();
    Ok(newest.map(|(_, path)| path))
}

/// Folds a goal log's bytes into rows.
///
/// A last line with no newline is a write in progress and is left for the
/// next read, as `read_log` does.
pub fn fold(id: &str, bytes: &[u8]) -> Goal {
    let mut goal = Goal {
        id: id.to_string(),
        ..Goal::default()
    };
    let mut lines: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    // `split` yields what follows the last newline as a final piece: empty
    // when the log ends cleanly, a partial line when it does not.
    lines.pop();
    for line in lines {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let applied = serde_json::from_slice::<Value>(line)
            .ok()
            .and_then(|event| apply(&mut goal, &event));
        if applied.is_none() {
            goal.skipped += 1;
        }
    }
    goal
}

/// One event. `None` when a field `fold` would index is missing.
fn apply(goal: &mut Goal, event: &Value) -> Option<()> {
    let text = |key: &str| event[key].as_str().map(str::to_string);
    match event["event"].as_str()? {
        "goal" => goal.text = Some(text("text")?),
        "task" => {
            // `fold` builds a fresh `TaskState`, so a repeated `task` event
            // resets the assignment and status too. The dict keeps its slot.
            let row = TaskRow {
                id: text("id")?,
                class: text("class")?,
                grade: text("grade")?,
                provider: None,
                model: None,
                status: "pending".to_string(),
                run_id: None,
                done: text("done"),
            };
            match goal.rows.iter_mut().find(|r| r.id == row.id) {
                Some(slot) => *slot = row,
                None => goal.rows.push(row),
            }
        }
        "assignment" => {
            let task = text("task")?;
            let provider = text("provider")?;
            let model = text("model").filter(|m| !m.is_empty());
            let row = row_or_default(goal, &task);
            row.provider = Some(provider);
            row.model = model;
            if row.status == "pending" {
                row.status = "assigned".to_string();
            }
        }
        "run" => {
            let task = text("task")?;
            let run_id = text("run_id")?;
            if let Some(row) = goal.rows.iter_mut().find(|r| r.id == task) {
                row.status = "running".to_string();
                row.run_id = Some(run_id);
            }
        }
        "run_end" => {
            let task = text("task")?;
            let run_id = text("run_id")?;
            if let Some(row) = goal.rows.iter_mut().find(|r| r.id == task)
                && row.run_id.as_deref() == Some(run_id.as_str())
            {
                row.status = "ended".to_string();
            }
        }
        "review" => {
            // `fold` indexes `task` and reads the rest with `get`.
            let task = text("task")?;
            if goal.rows.iter().any(|r| r.id == task) {
                goal.reviews.push(Review {
                    task,
                    verdict: text("verdict"),
                    degraded: event["degraded"].as_bool().unwrap_or(false),
                });
            }
        }
        "human_verdict" => {
            let task = text("task")?;
            let result = text("result")?;
            if let Some(row) = goal.rows.iter_mut().find(|r| r.id == task) {
                row.status = result;
            }
        }
        _ => {}
    }
    Some(())
}

/// An assignment for a task nobody defined makes one, with `fold`'s defaults.
fn row_or_default<'a>(goal: &'a mut Goal, task: &str) -> &'a mut TaskRow {
    let index = match goal.rows.iter().position(|r| r.id == task) {
        Some(index) => index,
        None => {
            goal.rows.push(TaskRow {
                id: task.to_string(),
                class: "batch_scripting".to_string(),
                grade: "low".to_string(),
                provider: None,
                model: None,
                status: "pending".to_string(),
                run_id: None,
                done: None,
            });
            goal.rows.len() - 1
        }
    };
    &mut goal.rows[index]
}
