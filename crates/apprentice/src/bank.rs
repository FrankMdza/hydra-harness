//! `apprentice.py` lines 61 to 319: the facts a task asks about, read from one record, the
//! templates, and `bank build`, which writes `bank.jsonl`.
//!
//! A record is whatever its writer left, so where the Python raises on a value of the wrong
//! shape this returns an error whose text starts with the class Python raises, and nothing
//! is written. Where it raises halfway through a write, the `.tmp` it left is left here too,
//! which is `hydra_records::records::write_jsonl`'s rule.
//!
//! Each template's runs are ordered by the SHA-256 of `apprentice-1:<template>:<run id>`, so
//! the bank does not depend on the order the records are listed in, and the templates of a
//! class take turns until `n` tasks are taken or every pool is empty.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use hydra_pyjson::json::{Dict, PyStr, Value};
use hydra_pyjson::py;
use hydra_records::records::{self, Scanned};
use hydra_records::redact;
use sha2::{Digest, Sha256};

use crate::dump::dump_sorted;

const SEED: &str = "apprentice-1";
const EXCERPT_BUDGET: usize = 3000;

/// `home()`: where the apprentice keeps its files, under the grader's home.
pub fn home(grader_home: &Path) -> PathBuf {
    grader_home.join("apprentice")
}

/// `cut`: `text` with what is past `budget` code points replaced by a line that counts them.
fn cut(text: &PyStr, budget: usize) -> PyStr {
    if text.len() <= budget {
        return text.clone();
    }
    let mut out = text.head(budget);
    out.push_str(&format!("\n[cut: {} more characters]", text.len() - budget));
    out
}

/// `safe`: redacted, then cut.
fn safe(text: &PyStr) -> PyStr {
    cut(&redact::redact_string(text), EXCERPT_BUDGET)
}

/// `str(Path(text))`: `pathlib` drops a `.` part, a doubled slash and a trailing one, and
/// keeps a leading pair of slashes but not three.
fn pure_path(text: &str) -> String {
    let lead = if text.starts_with("//") && !text.starts_with("///") {
        "//"
    } else if text.starts_with('/') {
        "/"
    } else {
        ""
    };
    let parts: Vec<&str> = text
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    match (lead, parts.is_empty()) {
        ("", true) => ".".into(),
        (lead, _) => format!("{lead}{}", parts.join("/")),
    }
}

/// `Path(base) / name`, spelled as `pathlib` spells it.
fn joined(base: &Path, name: &str) -> PathBuf {
    match pure_path(&base.to_string_lossy()).as_str() {
        "." => PathBuf::from(name),
        base => Path::new(base).join(name),
    }
}

/// `sorted(runs.glob("*.jsonl"))`: every entry whose name ends in `.jsonl`, a hidden one, a
/// directory and a dangling link included, in code point order of the name. Nothing when
/// `runs` is not a directory that can be listed.
fn list_paths(runs: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(runs) else {
        return Vec::new();
    };
    let mut names: Vec<_> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name()))
        .filter(|name| name.as_encoded_bytes().ends_with(b".jsonl"))
        .collect();
    names.sort_by(|a, b| a.as_encoded_bytes().cmp(b.as_encoded_bytes()));
    names.into_iter().map(|name| runs.join(name)).collect()
}

/// `list_records`: every record `scan_record` takes, in the order of their names.
fn list_records(runs: &Path, stderr: &mut dyn Write) -> Result<Vec<Scanned>> {
    let mut items = Vec::new();
    for path in list_paths(runs) {
        if let Some(item) = records::scan_record(&path, stderr)? {
            items.push(item);
        }
    }
    Ok(items)
}

fn get(value: &Value, key: &str) -> Result<Value> {
    py::get(value, key, Value::Null)
}

/// `value or {}`.
fn or_empty(value: Value) -> Value {
    if py::truthy(&value) {
        value
    } else {
        Value::Dict(Dict::new())
    }
}

/// `(line.get("hydra") or {}).get("type") == kind`, and the `hydra` field when it holds.
fn hydra_of(line: &Value, kind: &str) -> Result<Option<Value>> {
    let hydra = or_empty(get(line, "hydra")?);
    Ok(py::eq_str(&get(&hydra, "type")?, kind).then_some(hydra))
}

/// `_blocks`: the dicts of `line["raw"]["message"]["content"]`, or none.
fn blocks(line: &Value) -> Result<Vec<Value>> {
    let raw = get(line, "raw")?;
    let message = match &raw {
        Value::Dict(_) => get(&raw, "message")?,
        _ => Value::Null,
    };
    let content = match &message {
        Value::Dict(_) => get(&message, "content")?,
        _ => Value::Null,
    };
    Ok(match content {
        Value::List(items) => items
            .into_iter()
            .filter(|block| matches!(block, Value::Dict(_)))
            .collect(),
        _ => Vec::new(),
    })
}

/// `_text_of`: a tool result's text, from a string or from the `text` of each dict in a
/// list, joined by newlines.
fn text_of(content: &Value) -> Result<PyStr> {
    match content {
        Value::Str(text) => Ok(text.clone()),
        Value::List(items) => {
            let mut out = PyStr::new();
            for (n, block) in items
                .iter()
                .filter(|block| matches!(block, Value::Dict(_)))
                .enumerate()
            {
                let text = py::get(block, "text", Value::text(""))?;
                let Value::Str(text) = text else {
                    bail!(
                        "TypeError: sequence item {n}: expected str instance, {} found",
                        py::type_name(&text)
                    );
                };
                if n > 0 {
                    out.push_str("\n");
                }
                out.push(&text);
            }
            Ok(out)
        }
        _ => Ok(PyStr::new()),
    }
}

/// `int(value)` for a seat count: an int as it is, a bool as 0 or 1, a float cut towards
/// zero, a string as `int()` reads one. An integer past 128 bits is an error here, where
/// Python carries on.
fn int_of(value: &Value) -> Result<i128> {
    match value {
        Value::Int(n) => Ok(*n),
        Value::Bool(b) => Ok(i128::from(*b)),
        Value::Float(x) if x.is_nan() => bail!("ValueError: cannot convert float NaN to integer"),
        Value::Float(x) if x.is_infinite() => {
            bail!("OverflowError: cannot convert float infinity to integer")
        }
        Value::Float(x) if x.trunc().abs() < 1.7e38 => Ok(x.trunc() as i128),
        Value::Str(text) => match text.to_utf8().as_deref().and_then(py::int_value) {
            Some(Value::Int(n)) => Ok(n),
            Some(_) => bail!("OverflowError: a seat count past 128 bits is not ported"),
            None => bail!(
                "ValueError: invalid literal for int() with base 10: {}",
                py::repr(value)
            ),
        },
        Value::BigInt(_) | Value::Float(_) => {
            bail!("OverflowError: a seat count past 128 bits is not ported")
        }
        other => bail!(
            "TypeError: int() argument must be a string, a bytes-like object or a real number, not '{}'",
            py::type_name(other)
        ),
    }
}

/// One refusal the gate wrote: the tool it refused, by the id of its call, and its text.
struct Refused {
    tool: Value,
    text: PyStr,
}

/// `facts`: what the templates ask about one record.
struct Facts {
    run_id: Value,
    provider: Value,
    model: Value,
    policy: Value,
    interactive: bool,
    outcome: &'static str,
    task: PyStr,
    start: Dict,
    end: Option<Dict>,
    refused: Vec<Refused>,
    final_text: Value,
    seat_requests: Vec<Value>,
    seats_launched: i128,
}

/// `{k: source.get(k) for k in keys}`.
fn subset(source: &Value, keys: &[&str]) -> Result<Dict> {
    let mut out = Dict::new();
    for key in keys {
        out.set(key, get(source, key)?);
    }
    Ok(out)
}

/// `facts`: read from the record's own lines. Refusals and the last assistant text are read
/// from `claude`'s frame shape only, so other providers get none.
fn facts(item: &Scanned, root: &Path) -> Result<Facts> {
    let (first, lines) = (&item.first, &item.lines);
    let start = or_empty(get(first, "hydra")?);
    let mut end = None;
    for line in lines.iter().rev() {
        if hydra_of(line, "run_end")?.is_some() {
            end = Some(py::item(line, "hydra")?.clone());
            break;
        }
    }
    let outcome = match &end {
        None => "unfinished",
        Some(end) if py::truthy(&get(end, "refusal")?) => "refused",
        Some(end) if py::eq(&get(end, "exit_code")?, &Value::Int(0)) => "success",
        Some(_) => "failed",
    };
    let mut model = get(first, "model")?;
    if !py::truthy(&model) {
        model = Value::Null;
        for line in lines.iter().take(6).skip(1) {
            let raw = get(line, "raw")?;
            if matches!(raw, Value::Dict(_)) && matches!(get(&raw, "model")?, Value::Str(_)) {
                model = get(&raw, "model")?;
                break;
            }
        }
    }
    let mut tools: Vec<(Value, Value)> = Vec::new();
    let mut refused = Vec::new();
    let mut final_text = Value::Null;
    if py::eq_str(&get(first, "provider")?, "claude") {
        for line in lines {
            for block in blocks(line)? {
                let kind = get(&block, "type")?;
                if py::eq_str(&kind, "tool_use") {
                    let id = get(&block, "id")?;
                    py::hashable(&id)?;
                    let name = get(&block, "name")?;
                    match tools.iter_mut().find(|(key, _)| py::same_key(key, &id)) {
                        Some(slot) => slot.1 = name,
                        None => tools.push((id, name)),
                    }
                } else if py::eq_str(&kind, "tool_result") && py::truthy(&get(&block, "is_error")?)
                {
                    let text = text_of(&get(&block, "content")?)?;
                    if text
                        .code_points()
                        .starts_with(PyStr::from("hydra/").code_points())
                    {
                        let id = get(&block, "tool_use_id")?;
                        py::hashable(&id)?;
                        let tool = tools
                            .iter()
                            .find(|(key, _)| py::same_key(key, &id))
                            .map_or(Value::Null, |(_, name)| name.clone());
                        refused.push(Refused { tool, text });
                    }
                } else if py::eq_str(&kind, "text")
                    && py::eq_str(&get(&or_empty(get(line, "raw")?), "type")?, "assistant")
                {
                    final_text = get(&block, "text")?;
                }
            }
        }
    }
    let mut seat_requests = Vec::new();
    for line in lines {
        if hydra_of(line, "seat_request")?.is_some() {
            seat_requests.push(py::item(line, "hydra")?.clone());
        }
    }
    let mut launched: i128 = 0;
    for request in &seat_requests {
        if py::eq_str(&get(request, "decision")?, "launched") {
            let seats = match get(request, "runs")? {
                Value::List(runs) => runs.len() as i128,
                _ => {
                    let seats = get(request, "seats")?;
                    int_of(&if py::truthy(&seats) {
                        seats
                    } else {
                        Value::Int(0)
                    })?
                }
            };
            launched = launched.checked_add(seats).ok_or_else(|| {
                anyhow::anyhow!("OverflowError: a seat count past 128 bits is not ported")
            })?;
        }
    }
    let task = get(&start, "task")?;
    let task = if py::truthy(&task) {
        task
    } else {
        Value::text("")
    };
    // The preamble is read for every record, before the task is looked at, as the
    // arguments of `strip_preamble` are.
    let current = records::current_preamble(root)?;
    let Value::Str(task) = task else {
        bail!(
            "AttributeError: '{}' object has no attribute 'startswith'",
            py::type_name(&task)
        );
    };
    let (task, _) = records::strip_preamble(&task, current.as_ref());
    let start_shown = subset(&start, &["command", "policy", "launch_mode", "task"])?;
    let end_shown = match &end {
        Some(end) => Some(subset(end, &["exit_code", "error", "refusal"])?),
        None => None,
    };
    Ok(Facts {
        run_id: item.run_id.clone(),
        provider: py::item(first, "provider")?.clone(),
        model,
        policy: get(&start, "policy")?,
        interactive: py::eq_str(&get(&start, "launch_mode")?, "interactive"),
        outcome,
        task: py::strip(&task),
        start: start_shown,
        end: end_shown,
        refused,
        final_text,
        seat_requests,
        seats_launched: launched,
    })
}

/// A dict of `pairs`, in the order given.
fn dict(pairs: Vec<(&str, Value)>) -> Value {
    let mut out = Dict::new();
    for (key, value) in pairs {
        out.set(key, value);
    }
    Value::Dict(out)
}

/// `_start_line`: the `run_start` fields a task shows, with the task itself hidden.
fn start_line(f: &Facts) -> PyStr {
    let mut shown = f.start.clone();
    let hidden = match shown.get("task") {
        Some(task) if py::truthy(task) => Value::text("(shown elsewhere)"),
        _ => Value::Null,
    };
    shown.set("task", hidden);
    dump_sorted(&dict(vec![("run_start", Value::Dict(shown))]))
}

/// Lines joined by newlines.
fn joined_lines(lines: impl Iterator<Item = PyStr>) -> PyStr {
    let mut out = PyStr::new();
    for (n, line) in lines.enumerate() {
        if n > 0 {
            out.push_str("\n");
        }
        out.push(&line);
    }
    out
}

fn refusal_lines(f: &Facts) -> PyStr {
    joined_lines(f.refused.iter().map(|r| {
        dump_sorted(&dict(vec![
            ("error_result", Value::Str(r.text.clone())),
            ("tool", r.tool.clone()),
        ]))
    }))
}

/// Text after a header line, as the open templates show a task.
fn headed(head: &str, text: &PyStr) -> PyStr {
    let mut out = PyStr::from(head);
    out.push(text);
    out
}

/// One template. `answer` is `None` for an open one.
struct Template {
    name: &'static str,
    check: &'static str,
    question: &'static str,
    applies: fn(&Facts) -> bool,
    excerpt: fn(&Facts) -> Result<PyStr>,
    answer: Option<fn(&Facts) -> Value>,
}

fn is_claude(f: &Facts) -> bool {
    py::eq_str(&f.provider, "claude")
}

const VERIFIABLE: [Template; 7] = [
    Template {
        name: "outcome",
        check: "exact",
        question: "How did this run end? Answer with one word: success (exit code 0, no refusal), failed (a nonzero exit code), refused (Hydra refused it) or unfinished (no run_end line).",
        applies: |_| true,
        excerpt: |f| {
            let mut out = start_line(f);
            out.push_str("\n");
            let end = f.end.clone().map_or(Value::Null, Value::Dict);
            out.push(&dump_sorted(&dict(vec![("run_end", end)])));
            Ok(out)
        },
        answer: Some(|f| Value::text(f.outcome)),
    },
    Template {
        name: "provider_model",
        check: "one_of",
        question: "Which provider and model ran this? Answer as provider/model, for example codex/gpt-5. When no model is recorded, answer with the provider alone.",
        applies: |_| true,
        excerpt: |f| {
            Ok(dump_sorted(&dict(vec![
                ("provider", f.provider.clone()),
                ("model", f.model.clone()),
            ])))
        },
        answer: Some(|f| {
            if py::truthy(&f.model) {
                let mut spelled = py::str(&f.provider);
                spelled.push_str("/");
                spelled.push(&py::str(&f.model));
                Value::List(vec![Value::Str(spelled)])
            } else {
                Value::List(vec![f.provider.clone()])
            }
        }),
    },
    Template {
        name: "refused_count",
        check: "number",
        question: "How many tool calls did the gate refuse in this run? A refusal is an error tool result whose text starts with hydra/. Answer with the number.",
        applies: is_claude,
        excerpt: |f| {
            let lines = refusal_lines(f);
            Ok(if lines.is_empty() {
                PyStr::from("(no error results)")
            } else {
                lines
            })
        },
        answer: Some(|f| Value::Int(f.refused.len() as i128)),
    },
    Template {
        name: "first_refused_tool",
        check: "exact",
        question: "Which tool did the gate refuse first in this run? Answer with the tool name alone.",
        applies: |f| is_claude(f) && f.refused.first().is_some_and(|r| py::truthy(&r.tool)),
        excerpt: |f| Ok(refusal_lines(f)),
        answer: Some(|f| f.refused[0].tool.clone()),
    },
    Template {
        name: "interactive",
        check: "exact",
        question: "Was this run an interactive session? Answer yes or no. launch_mode is interactive for a session and recorded for a run Hydra launched with a task.",
        applies: |_| true,
        excerpt: |f| Ok(start_line(f)),
        answer: Some(|f| Value::text(if f.interactive { "yes" } else { "no" })),
    },
    Template {
        name: "seats_launched",
        check: "number",
        question: "How many seats did the seat_request lines of this run launch in total? Only a request whose decision is launched counts, and each run in its runs list is one seat. Answer with the number.",
        applies: |f| f.interactive && !f.seat_requests.is_empty(),
        excerpt: |f| {
            Ok(joined_lines(f.seat_requests.iter().map(|r| {
                dump_sorted(&dict(vec![("seat_request", r.clone())]))
            })))
        },
        answer: Some(|f| Value::Int(f.seats_launched)),
    },
    Template {
        name: "policy",
        check: "exact",
        question: "Which policy was this run launched under? Answer with the policy name, or none when the run_start line has a null policy.",
        applies: |_| true,
        excerpt: |f| Ok(start_line(f)),
        answer: Some(|f| {
            if py::truthy(&f.policy) {
                f.policy.clone()
            } else {
                Value::text("none")
            }
        }),
    },
];

const OPEN: [Template; 4] = [
    Template {
        name: "run_line",
        check: "",
        question: "Write the `hydra run` command line that would launch this task text. Name the provider and the policy in it.",
        applies: |f| !f.task.is_empty(),
        excerpt: |f| Ok(headed("TASK TEXT:\n", &f.task)),
        answer: None,
    },
    Template {
        name: "provider_policy",
        check: "",
        question: "Pick a provider (claude, cursor-agent, agy or codex) and a policy for this task text, and say why in two sentences.",
        applies: |f| !f.task.is_empty(),
        excerpt: |f| Ok(headed("TASK TEXT:\n", &f.task)),
        answer: None,
    },
    Template {
        name: "summarize",
        check: "",
        question: "Say in one sentence what this run did.",
        applies: |f| !f.task.is_empty() && py::truthy(&f.final_text),
        excerpt: |f| {
            let Value::Str(final_text) = &f.final_text else {
                bail!(
                    "TypeError: can only concatenate str (not \"{}\") to str",
                    py::type_name(&f.final_text)
                );
            };
            let mut out = headed("TASK TEXT:\n", &f.task);
            out.push_str("\n\nFINAL TEXT:\n");
            out.push(final_text);
            Ok(out)
        },
        answer: None,
    },
    Template {
        name: "refusal_meaning",
        check: "",
        question: "Say what this refusal line means and what the agent should do next.",
        applies: |f| !f.refused.is_empty(),
        excerpt: |f| Ok(headed("REFUSAL LINE:\n", &f.refused[0].text)),
        answer: None,
    },
];

/// `f"{name}:{run_id}"` and the like: `str()` of the run id after a prefix.
fn spelled(prefix: &str, run_id: &Value) -> PyStr {
    let mut out = PyStr::from(prefix);
    out.push(&py::str(run_id));
    out
}

/// `_pick`: the facts of `pool` ordered by the hash of the seed, the template and the run id.
fn pick<'a>(pool: Vec<&'a Facts>, name: &str) -> Result<Vec<&'a Facts>> {
    let mut keyed = Vec::with_capacity(pool.len());
    for f in pool {
        let text = spelled(&format!("{SEED}:{name}:"), &f.run_id);
        let Some(text) = text.to_utf8() else {
            bail!(
                "UnicodeEncodeError: 'utf-8' codec can't encode a surrogate: surrogates not allowed"
            );
        };
        let hex: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        keyed.push((hex, f));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(keyed.into_iter().map(|(_, f)| f).collect())
}

/// `_make`: one task of `template` about `f`.
fn make(template: &Template, f: &Facts, class: &str) -> Result<Value> {
    let mut prompt = PyStr::from(template.question);
    prompt.push_str("\n\n");
    prompt.push(&safe(&(template.excerpt)(f)?));
    let mut task = Dict::new();
    task.set(
        "id",
        Value::Str(spelled(&format!("{}:", template.name), &f.run_id)),
    );
    task.set("template", Value::text(template.name));
    task.set("class", Value::text(class));
    task.set("run_id", f.run_id.clone());
    task.set(
        "harness",
        if py::truthy(&f.provider) {
            f.provider.clone()
        } else {
            Value::text("hydra")
        },
    );
    task.set("prompt", Value::Str(prompt));
    if let Some(answer) = template.answer {
        task.set("check", Value::text(template.check));
        task.set("answer", answer(f));
    }
    Ok(Value::Dict(task))
}

/// `int` `n` as the loop compares it: an integer past 128 bits is as good as infinite.
fn bound(n: &Value) -> i128 {
    match n {
        Value::Int(n) => *n,
        Value::BigInt(digits) if digits.starts_with('-') => i128::MIN,
        _ => i128::MAX,
    }
}

/// `build_bank`: `n` verifiable and `n` open tasks from the records under `runs`, sorted by
/// id. `root` is where `config/preamble.md` is read from.
pub fn build_bank(
    runs: &Path,
    root: &Path,
    n: &Value,
    stderr: &mut dyn Write,
) -> Result<Vec<Value>> {
    let n = bound(n);
    let mut all = Vec::new();
    for item in list_records(runs, stderr)? {
        all.push(facts(&item, root)?);
    }
    let mut bank = Vec::new();
    for (class, templates) in [("verifiable", &VERIFIABLE[..]), ("open", &OPEN[..])] {
        let mut pools = Vec::new();
        for template in templates {
            let pool = all.iter().filter(|f| (template.applies)(f)).collect();
            pools.push((template, pick(pool, template.name)?));
        }
        let mut taken = vec![0; pools.len()];
        let mut count: i128 = 0;
        while count < n {
            let mut progressed = false;
            for (i, (template, pool)) in pools.iter().enumerate() {
                if count >= n {
                    break;
                }
                if taken[i] < pool.len() {
                    bank.push(make(template, pool[taken[i]], class)?);
                    taken[i] += 1;
                    count += 1;
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
    }
    let id = |task: &Value| match task {
        Value::Dict(task) => match task.get("id") {
            Some(Value::Str(id)) => id.clone(),
            _ => PyStr::new(),
        },
        _ => PyStr::new(),
    };
    bank.sort_by_key(id);
    Ok(bank)
}

/// `bank_build`: write `bank.jsonl` under the apprentice's home and say what it holds.
pub fn bank_build(
    n: &Value,
    env: &dyn Fn(&str) -> Option<std::ffi::OsString>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<()> {
    let root = records::hydra_root_in(env)?;
    let runs = joined(&root, "runs");
    let bank = build_bank(&runs, &root, n, stderr)?;
    let grader_home = records::grader_home_in(env)?;
    records::write_jsonl(&home(&grader_home).join("bank.jsonl"), &bank)?;
    let verifiable = bank
        .iter()
        .filter(|task| matches!(task, Value::Dict(task) if task.get("class").is_some_and(|c| py::eq_str(c, "verifiable"))))
        .count();
    let _ = writeln!(
        stdout,
        "bank: {verifiable} verifiable and {} open tasks from {}",
        bank.len() - verifiable,
        runs.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_spelled_as_pathlib_spells_it() {
        for (given, want) in [
            ("a//b/./c/", "a/b/c"),
            ("//x", "//x"),
            ("///x", "/x"),
            ("./root//", "root"),
            (".", "."),
            ("/", "/"),
        ] {
            assert_eq!(pure_path(given), want, "{given}");
        }
        assert_eq!(joined(Path::new("."), "runs"), PathBuf::from("runs"));
    }

    #[test]
    fn cut_counts_code_points() {
        let text = PyStr::from("é".repeat(5).as_str());
        assert_eq!(cut(&text, 3), PyStr::from("ééé\n[cut: 2 more characters]"));
        assert_eq!(cut(&text, 5), text);
    }
}
