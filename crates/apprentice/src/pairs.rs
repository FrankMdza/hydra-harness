//! `apprentice.py` lines 505 to 543: `pairs` turns a graded file into `pairs.jsonl`, one
//! preference pair per open task and grader whose verdict on the local answer is `better` or
//! `worse`.
//!
//! The Python keys the pairs it keeps by a tuple of the task id and the grader, which can be
//! any value the file holds, so the table here is a list searched by Python's key equality,
//! and the order it is written in is `sorted` over those tuples.

use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use hydra_pyjson::json::{PyStr, Value};
use hydra_pyjson::py;
use hydra_pyjson::pysort;
use hydra_records::isotime::DateTime;
use hydra_records::{records, redact};

use crate::attempt::Env;
use crate::bank::{dict, home};
use crate::score::Table;

/// `utc_now`: the time to the second, as `2026-10-04T05:21:00Z`.
fn utc_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let now = DateTime::from_unix_millis(seconds * 1000).expect("the clock is before 9999");
    now.isoformat().replace("+00:00", "Z")
}

/// The key of a kept pair: `(task_id, graded_by)`.
type Key = (Value, Value);

/// `key in kept`, by the tuple's equality, after the hash a lookup takes first.
fn find(kept: &[(Key, Value)], key: &Key) -> Result<Option<usize>> {
    py::hashable(&key.0)?;
    py::hashable(&key.1)?;
    Ok(kept
        .iter()
        .position(|((id, by), _)| py::same_key(id, &key.0) && py::same_key(by, &key.1)))
}

/// `kept[key] = value`: a key already there keeps its first spelling and its place.
fn set(kept: &mut Vec<(Key, Value)>, key: Key, value: Value) -> Result<()> {
    match find(kept, &key)? {
        Some(at) => kept[at].1 = value,
        None => kept.push((key, value)),
    }
    Ok(())
}

/// `a < b` for two tuples: the first items that are not equal decide, by `<`.
fn tuple_less(a: &Key, b: &Key) -> Result<bool> {
    if !py::same_key(&a.0, &b.0) {
        return py::less(&a.0, &b.0);
    }
    if !py::same_key(&a.1, &b.1) {
        return py::less(&a.1, &b.1);
    }
    Ok(false)
}

/// The `(task_id, answer)` of every line in `attempts.jsonl`: a set in the Python, so both
/// members have to hash.
fn read_attempts(path: &Path) -> Result<Vec<(Value, Value)>> {
    let mut attempts = Vec::new();
    for row in records::read_jsonl(path)? {
        let id = py::item(&row, "task_id")?.clone();
        let answer = py::item(&row, "answer")?.clone();
        py::hashable(&id)?;
        py::hashable(&answer)?;
        attempts.push((id, answer));
    }
    Ok(attempts)
}

/// `(task_id, answer) in attempts`: the id compares as the bank's ids do, the answer as text.
fn attempted(attempts: &[(Value, Value)], id: &Value, answer: &PyStr) -> bool {
    attempts.iter().any(|(held_id, held)| {
        py::same_key(held_id, id) && matches!(held, Value::Str(text) if text == answer)
    })
}

/// `[{"role": role, "content": content}]`.
fn turn(role: &str, content: Value) -> Value {
    Value::List(vec![dict(vec![
        ("role", Value::text(role)),
        ("content", content),
    ])])
}

fn line(stdout: &mut dyn Write, text: &str) {
    let _ = writeln!(stdout, "{text}");
}

/// `pairs`: read `graded`, keep a pair for each line that makes one, and write them all.
pub fn pairs(graded: &Path, env: Env, stdout: &mut dyn Write) -> Result<u8> {
    let base = home(&records::grader_home_in(env)?);
    let mut bank = Table::new();
    for task in records::read_jsonl(&base.join("bank.jsonl"))? {
        let id = py::item(&task, "id")?.clone();
        bank.set(id, task)?;
    }
    let attempts = read_attempts(&base.join("attempts.jsonl"))?;
    let out_path = base.join("pairs.jsonl");
    let mut kept: Vec<(Key, Value)> = Vec::new();
    for pair in records::read_jsonl(&out_path)? {
        let key = (
            py::item(&pair, "task_id")?.clone(),
            py::item(&pair, "graded_by")?.clone(),
        );
        set(&mut kept, key, pair)?;
    }
    let stamp = utc_now();
    let (mut added, mut skipped) = (0, 0);
    for row in records::read_jsonl(graded)? {
        let verdict = py::get(&row, "verdict", Value::Null)?;
        if py::eq_str(&verdict, "same") {
            continue;
        }
        let task_id = py::get(&row, "task_id", Value::Null)?;
        let task = bank.get(&task_id)?;
        let open = match task {
            Some(task) => py::eq_str(py::item(task, "class")?, "open"),
            None => false,
        };
        let judged = py::eq_str(&verdict, "better") || py::eq_str(&verdict, "worse");
        let Some(task) = task.filter(|_| open && judged) else {
            let why = if open {
                "verdict is not better, same or worse"
            } else {
                "not an open task in the bank"
            };
            line(
                stdout,
                &format!(
                    "pairs: skipped {}, verdict {}: {why}",
                    py::repr(&task_id),
                    py::repr(&verdict)
                ),
            );
            skipped += 1;
            continue;
        };
        let (local, better) = (
            py::get(&row, "local_answer", Value::Null)?,
            py::get(&row, "better_answer", Value::Null)?,
        );
        let (Value::Str(local), Value::Str(better)) = (local, better) else {
            let task_id = py::item(&row, "task_id")?;
            line(
                stdout,
                &format!("pairs: skipped {}, an answer is missing", py::repr(task_id)),
            );
            skipped += 1;
            continue;
        };
        let task_id = py::item(&row, "task_id")?;
        let unrecorded = [("local_answer", &local), ("better_answer", &better)]
            .into_iter()
            .find(|(_, answer)| !attempted(&attempts, task_id, answer));
        if let Some((field, _)) = unrecorded {
            line(
                stdout,
                &format!(
                    "pairs: skipped {}, {field} is not a recorded attempt",
                    py::repr(task_id)
                ),
            );
            skipped += 1;
            continue;
        }
        // The verdict is on the local answer. `better` keeps it as chosen, `worse` swaps in the
        // other recorded attempt, which `better_answer` names since build 351.
        let (chosen, rejected) = if py::eq_str(&verdict, "better") {
            (local, better)
        } else {
            (better, local)
        };
        let task_id = py::item(&row, "task_id")?.clone();
        let graded_by = Value::Str(py::str(&py::get(&row, "graded_by", Value::Null)?));
        let key = (task_id.clone(), graded_by.clone());
        if find(&kept, &key)?.is_none() {
            added += 1;
        }
        let value = dict(vec![
            ("task_id", task_id),
            ("prompt", turn("user", py::item(task, "prompt")?.clone())),
            (
                "chosen",
                turn("assistant", Value::Str(redact::redact_string(&chosen))),
            ),
            (
                "rejected",
                turn("assistant", Value::Str(redact::redact_string(&rejected))),
            ),
            ("harness", py::item(task, "harness")?.clone()),
            ("graded_by", graded_by),
            ("graded_at", Value::text(&stamp)),
            ("source", Value::text("apprentice")),
        ]);
        set(&mut kept, key, value)?;
    }
    let order = pysort::sorted(kept.len(), |i, j| tuple_less(&kept[i].0, &kept[j].0))?;
    let rows: Vec<Value> = order.into_iter().map(|i| kept[i].1.clone()).collect();
    records::write_jsonl(&out_path, &rows)?;
    line(
        stdout,
        &format!(
            "pairs: {added} new, {} in pairs.jsonl, {skipped} skipped",
            rows.len()
        ),
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stamp_is_to_the_second_in_utc() {
        let stamp = utc_now();
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert!(
            stamp.ends_with('Z') && stamp.as_bytes()[10] == b'T',
            "{stamp}"
        );
    }

    #[test]
    fn tuples_compare_by_the_first_items_that_differ() {
        let key = |id: Value, by: &str| (id, Value::text(by));
        assert!(tuple_less(&key(Value::Int(1), "b"), &key(Value::Float(2.0), "a")).unwrap());
        assert!(tuple_less(&key(Value::Bool(true), "a"), &key(Value::Float(1.0), "b")).unwrap());
        assert!(!tuple_less(&key(Value::Int(1), "a"), &key(Value::Bool(true), "a")).unwrap());
        assert!(tuple_less(&key(Value::text("a"), "x"), &key(Value::Int(1), "x")).is_err());
    }
}
