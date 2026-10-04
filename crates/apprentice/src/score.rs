//! `apprentice.py` lines 447 to 499: `score` prints, per model, how many verifiable tasks
//! its latest attempt at the current prompt got right, per template and overall, and the
//! same at the prompt before it.
//!
//! The Python keeps the attempts in dicts keyed by whatever the file holds, so the tables
//! here are lists searched by Python's key equality, and a key that cannot be hashed raises.

use std::io::Write;

use anyhow::{Result, bail};
use hydra_pyjson::json::{PyStr, Value};
use hydra_pyjson::py;
use hydra_pyjson::pysort;
use hydra_records::records;

use crate::attempt::Env;
use crate::bank::home;

/// A Python dict over arbitrary keys, in insertion order. Setting a key that is present
/// keeps the first key and its place, and replaces the value.
struct Table<T>(Vec<(Value, T)>);

impl<T> Table<T> {
    fn new() -> Table<T> {
        Table(Vec::new())
    }

    fn find(&self, key: &Value) -> Result<Option<usize>> {
        py::hashable(key)?;
        Ok(self.0.iter().position(|(held, _)| py::same_key(held, key)))
    }

    fn get(&self, key: &Value) -> Result<Option<&T>> {
        Ok(self.find(key)?.map(|at| &self.0[at].1))
    }

    fn set(&mut self, key: Value, value: T) -> Result<()> {
        match self.find(&key)? {
            Some(at) => self.0[at].1 = value,
            None => self.0.push((key, value)),
        }
        Ok(())
    }

    fn entry(&mut self, key: Value, default: T) -> Result<&mut T> {
        let at = match self.find(&key)? {
            Some(at) => at,
            None => {
                self.0.push((key, default));
                self.0.len() - 1
            }
        };
        Ok(&mut self.0[at].1)
    }
}

/// `_tally`: right and asked, per template name and `overall`, over the latest row of each
/// task at `sha` that was graded.
fn tally(rows: &[&Value], sha: &Value, templates: &Table<Value>) -> Result<Table<[i128; 2]>> {
    let mut latest: Table<&Value> = Table::new();
    for row in rows {
        let graded = matches!(row, Value::Dict(dict) if dict.get("correct").is_some());
        if py::eq(py::item(row, "prompt_sha")?, sha) && graded {
            latest.set(py::item(row, "task_id")?.clone(), row)?;
        }
    }
    let mut counts = Table::new();
    for (task_id, row) in &latest.0 {
        let mut name = templates.get(task_id)?.cloned().unwrap_or(Value::Null);
        if !py::truthy(&name) {
            name = py::get(row, "template", Value::Null)?;
        }
        if !py::truthy(&name) {
            let Value::Str(id) = task_id else {
                bail!(
                    "AttributeError: '{}' object has no attribute 'split'",
                    py::type_name(task_id)
                );
            };
            let colon = id
                .code_points()
                .iter()
                .position(|code| *code == u32::from(':'));
            name = Value::Str(id.head(colon.unwrap_or(id.len())));
        }
        let right = py::truthy(py::item(row, "correct")?);
        for key in [name, Value::text("overall")] {
            let cell = counts.entry(key, [0, 0])?;
            cell[0] += i128::from(right);
            cell[1] += 1;
        }
    }
    Ok(counts)
}

/// `value[:8]`, as an f-string prints it.
fn head8(value: &Value) -> Result<PyStr> {
    Ok(py::str(&py::head(value, 8)?))
}

/// `f"{x:.2f}"` and `f"{x:+.2f}"`.
fn fixed(x: f64, signed: bool) -> String {
    if signed {
        format!("{x:+.2}")
    } else {
        format!("{x:.2}")
    }
}

/// `_print_model`: one model's block.
fn print_model(
    rows: &[Value],
    sha: &Value,
    templates: &Table<Value>,
    model: &Value,
    stdout: &mut dyn Write,
) -> Result<()> {
    let mut mine = Vec::new();
    for row in rows {
        if py::eq(&py::get(row, "model", Value::Null)?, model) {
            mine.push(row);
        }
    }
    let mut previous = None;
    for row in mine.iter().rev() {
        let held = py::item(row, "prompt_sha")?;
        if !py::eq(held, sha) {
            previous = Some(held.clone());
            break;
        }
    }
    let now = tally(&mine, sha, templates)?;
    let before = match &previous {
        Some(previous) if py::truthy(previous) => tally(&mine, previous, templates)?,
        _ => Table::new(),
    };
    let mut out = PyStr::from("model ");
    out.push(&py::str(model));
    write_text(stdout, &out)?;
    let mut out = PyStr::from("prompt sha ");
    out.push(&head8(sha)?);
    if let Some(previous) = previous.as_ref().filter(|previous| py::truthy(previous)) {
        out.push_str(", previous ");
        out.push(&head8(previous)?);
    }
    write_text(stdout, &out)?;
    let overall = |key: &Value| !py::eq_str(key, "overall");
    let order = pysort::sorted(now.0.len(), |i, j| {
        let (a, b) = (&now.0[i].0, &now.0[j].0);
        // `(k != "overall", k)` against another such pair: the flags first, then the keys.
        match (overall(a), overall(b)) {
            (x, y) if x != y => Ok(!x & y),
            _ if py::same_key(a, b) => Ok(false),
            _ => py::less(a, b),
        }
    })?;
    for i in order {
        let (name, [got, total]) = &now.0[i];
        let ratio = *got as f64 / *total as f64;
        let mut line = py::ljust(name, 20)?;
        line.push_str(&format!("{got}/{total}  {}", fixed(ratio, false)));
        if let Some([pg, pt]) = before.get(name)? {
            let was = *pg as f64 / *pt as f64;
            line.push_str(&format!(
                "   previous {pg}/{pt}  {}   delta {}",
                fixed(was, false),
                fixed(ratio - was, true)
            ));
        }
        write_text(stdout, &line)?;
    }
    Ok(())
}

/// `print(text)`: stdout is UTF-8 and strict, so a lone surrogate raises.
fn write_text(stdout: &mut dyn Write, text: &PyStr) -> Result<()> {
    let Some(text) = text.to_utf8() else {
        bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate: surrogates not allowed");
    };
    let _ = writeln!(stdout, "{text}");
    Ok(())
}

/// `score`: one block per model that answered at the current prompt, or only `only`'s.
pub fn score(only: Option<&str>, env: Env, stdout: &mut dyn Write) -> Result<u8> {
    let base = home(&records::grader_home_in(env)?);
    let rows = records::read_jsonl(&base.join("attempts.jsonl"))?;
    let Some(last) = rows.last() else {
        let _ = writeln!(stdout, "score: no attempts yet.");
        return Ok(0);
    };
    let mut templates = Table::new();
    for task in records::read_jsonl(&base.join("bank.jsonl"))? {
        let id = py::item(&task, "id")?.clone();
        let template = py::item(&task, "template")?.clone();
        templates.set(id, template)?;
    }
    let current = py::item(last, "prompt_sha")?.clone();
    let mut models: Table<()> = Table::new();
    for row in &rows {
        if py::eq(py::item(row, "prompt_sha")?, &current) {
            models.set(py::item(row, "model")?.clone(), ())?;
        }
    }
    let models: Vec<Value> = models.0.into_iter().map(|(model, ())| model).collect();
    let order = pysort::sorted(models.len(), |i, j| py::less(&models[i], &models[j]))?;
    let mut models: Vec<Value> = order.into_iter().map(|i| models[i].clone()).collect();
    if let Some(only) = only {
        let only = Value::text(only);
        models = if models.iter().any(|model| py::eq(&only, model)) {
            vec![only]
        } else {
            Vec::new()
        };
    }
    if models.is_empty() {
        let _ = writeln!(stdout, "score: no attempts yet.");
        return Ok(0);
    }
    for model in &models {
        print_model(&rows, &current, &templates, model, stdout)?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_rounds_a_tie_to_even_as_python_does() {
        assert_eq!(fixed(0.125, false), "0.12");
        assert_eq!(fixed(0.375, false), "0.38");
        assert_eq!(fixed(-0.001, true), "-0.00");
        assert_eq!(fixed(0.0, true), "+0.00");
    }
}
