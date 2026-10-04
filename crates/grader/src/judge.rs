//! `judge.py`: a local model labels the dataset's runs, and `agree` measures it against the
//! human.
//!
//! `label` walks `dataset.jsonl`, asks the model on this machine about each run it has not
//! asked about under this model and this prompt, and appends one line per run to
//! `judge-labels.jsonl`. A run that hit a quota or the gate gets a line from a rule and no
//! call. `agree` counts the runs that hold both a human verdict and a judge's.
//!
//! The one network call is `judge_http::post_json`, which goes to this machine and nowhere
//! else. Everything the Python does with a row, an answer or a reply of the wrong shape is
//! done here too: an error whose text starts with the class Python raises where the Python
//! lets it through, and the Python's fallback where it catches.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::file::os_error;
use hydra_pyjson::json::{self, Dict, ParseError, PyStr, Value};
use hydra_pyjson::marker;
use hydra_pyjson::py;
use sha2::{Digest, Sha256};

use crate::collect::{sorted_keys, utc_now};
use crate::judge_http::{self, Failure};
use crate::labels::{Env, Keyed};
use crate::records;
use crate::redact;

pub const DEFAULT_MODEL: &str = "gemma3:1b-it-qat";
pub const DEFAULT_URL: &str = "http://127.0.0.1:11434/api/generate";
const TASK_BUDGET: usize = 3000;
const RESULT_BUDGET: usize = 4000;
const UNPARSED_BUDGET: usize = 500;
const NUM_PREDICT: i128 = 200;
const TIMEOUT: Duration = Duration::from_secs(300);
const CLASSES: [&str; 5] = ["quota", "gate", "agent", "task", "none"];
const LOOSE_BELOW: usize = 50;

const INSTRUCTIONS: &str = "You are checking the work of a coding agent. Below are the task it \
was given, the text it ended with, and how much it changed.

First write one sentence saying what the agent did and whether it did what the task asked. Then \
give the verdict.

Answer with one JSON object and nothing else, with the keys in this order:
{\"reason\": \"<one sentence>\", \"result_ok\": true or false}

result_ok is true when the result does what the task asked and a human could use it as it is. \
result_ok is false when a human would have to fix it first.";

/// `Unavailable`: the model server did not answer, or does not have the model.
#[derive(Debug)]
pub struct Unavailable(pub String);

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Unavailable {}

/// `os.environ.get(name, default)`: a variable set to nothing is nothing, not the default.
fn named(env: &Env, name: &str, default: &str) -> String {
    env.get(name)
        .map_or_else(|| default.to_string(), |v| v.to_string_lossy().into_owned())
}

/// `ollama_url()`.
pub fn ollama_url(env: &Env) -> String {
    named(env, "HYDRA_LOCAL_LLM_URL", DEFAULT_URL)
}

/// `ollama_model()`.
pub fn ollama_model(env: &Env) -> String {
    named(env, "HYDRA_LOCAL_MODEL", DEFAULT_MODEL)
}

fn io_error(err: &std::io::Error, path: &Path) -> anyhow::Error {
    anyhow!("{}: {}", os_error(err), path.display())
}

/// `cut`: the first `budget` code points, and a line saying how many were left out.
pub fn cut(text: &PyStr, budget: usize) -> PyStr {
    if text.len() <= budget {
        return text.clone();
    }
    let mut out = text.head(budget);
    out.push_str(&format!("\n[cut: {} more characters]", text.len() - budget));
    out
}

/// `Path.resolve()`: absolute, links followed, and a part that is not there kept as written.
fn resolve(path: &Path) -> PathBuf {
    let absolute = match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path.to_path_buf(),
    };
    let mut out = PathBuf::new();
    let mut real = true;
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(name) => {
                out.push(name);
                match (real, std::fs::canonicalize(&out)) {
                    (true, Ok(found)) => out = found,
                    _ => real = false,
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// `show_projection`: the run's `hydra show` projection, or an empty dict when the run
/// cannot be shown. Read fresh and never cached: the grader's cache leaves `result_text`
/// out on purpose.
///
/// The Python links a scratch `runs` when the directory has another name. Its only caller
/// passes `runs` under the root, so that branch never runs and is not here.
pub fn show_projection(run_id: &Value, runs: &Path, hydra: &str, env: &Env) -> Result<Value> {
    let resolved = resolve(runs);
    let root = resolved.parent().unwrap_or(Path::new("/"));
    let Value::Str(run_id) = run_id else {
        bail!(
            "TypeError: expected str, bytes or os.PathLike object, not {}",
            py::type_name(run_id)
        );
    };
    let Some(run_id) = run_id.to_utf8() else {
        bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in the run id");
    };
    if run_id.contains('\0') {
        bail!("ValueError: embedded null byte");
    }
    let empty = || Ok(Value::Dict(Dict::new()));
    let done = Command::new(hydra)
        .arg("show")
        .arg(&run_id)
        .env_clear()
        .envs(env.pairs())
        .env("HYDRA_ROOT", root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let Ok(done) = done else {
        return empty();
    };
    // Both streams are read as text before the exit code is looked at.
    let (Ok(stdout), Ok(_)) = (
        String::from_utf8(done.stdout),
        std::str::from_utf8(&done.stderr),
    ) else {
        bail!("UnicodeDecodeError: 'utf-8' codec can't decode what hydra show printed");
    };
    if !done.status.success() {
        return empty();
    }
    let stdout = stdout.replace("\r\n", "\n").replace('\r', "\n");
    let answer = match json::loads_explained(&PyStr::from(stdout)) {
        // Map 105 D3: `result_text` arrives inside a boundary, and the verdict parser and
        // the prompt read it bare.
        Ok(answer) => marker::unmark_show(&answer),
        Err((ParseError::Decode, _)) => return empty(),
        Err((err, message)) => bail!("{}: {message}", err.class()),
    };
    let projection = py::get(&answer, "projection", Value::Null)?;
    Ok(if py::truthy(&projection) {
        projection
    } else {
        answer
    })
}

/// `redact.redact_string(value)` on whatever a row or a projection held there.
fn redacted(value: &Value) -> Result<PyStr> {
    match value {
        Value::Str(text) => Ok(redact::redact_string(text)),
        other => bail!(
            "AttributeError: '{}' object has no attribute 'find'",
            py::type_name(other)
        ),
    }
}

/// `value or default`.
fn or_text(value: Value, default: &str) -> Value {
    if py::truthy(&value) {
        value
    } else {
        Value::text(default)
    }
}

/// `build_prompt`: the instructions, then the task, the result text and the diff stat, each
/// redacted, the first two cut to their budgets.
pub fn build_prompt(row: &Value, projection: &Value) -> Result<PyStr> {
    let task = or_text(py::get(row, "task_text", Value::Null)?, "");
    let task = cut(&redacted(&task)?, TASK_BUDGET);
    let result = or_text(
        py::get(projection, "result_text", Value::Null)?,
        "(none recorded)",
    );
    let result = cut(&redacted(&result)?, RESULT_BUDGET);
    let stat = py::get(projection, "diff_stat", Value::Null)?;
    let stat = if py::truthy(&stat) {
        redact::redact_string(&PyStr::from(json::dumps_default(&sorted_keys(&stat))))
    } else {
        PyStr::from("(none recorded)")
    };
    let mut prompt = PyStr::from(INSTRUCTIONS);
    prompt.push_str("\n\nTASK:\n");
    prompt.push(&task);
    prompt.push_str("\n\nRESULT TEXT:\n");
    prompt.push(&result);
    prompt.push_str("\n\nDIFF STAT:\n");
    prompt.push(&stat);
    prompt.push_str("\n");
    Ok(prompt)
}

/// The body of one call, as `json.dumps` writes it.
pub fn request_body(model: &str, prompt: &str) -> String {
    let mut options = Dict::new();
    options.set("temperature", Value::Int(0));
    options.set("num_predict", Value::Int(NUM_PREDICT));
    let mut body = Dict::new();
    body.set("model", Value::text(model));
    body.set("prompt", Value::text(prompt));
    body.set("stream", Value::Bool(false));
    body.set("format", Value::text("json"));
    body.set("options", Value::Dict(options));
    json::dumps_default(&Value::Dict(body))
}

/// `generate`: the `response` of one call, or the whole reply when it is not an object that
/// has one. `Unavailable` when the server is down, answers with an error or is not on this
/// machine.
pub fn generate(prompt: &str, model: &str, url: &str) -> Result<Value> {
    let body = request_body(model, prompt);
    let unavailable = |text: String| Err(Unavailable(text).into());
    let raw = match judge_http::post_json(url, body.as_bytes(), TIMEOUT) {
        Ok(raw) => raw,
        Err(Failure::Status(404)) => {
            return unavailable(format!("Ollama does not have the model {model}"));
        }
        Err(Failure::Status(code)) => {
            return unavailable(format!("Ollama at {url} answered HTTP {code}"));
        }
        Err(Failure::NotAnswering) => {
            return unavailable(format!("Ollama is not answering at {url}"));
        }
        // No `urllib` equivalent: the Python calls whatever the URL names.
        Err(Failure::NotLocal(why)) => {
            return unavailable(format!(
                "the model server has to be on this machine and {why}"
            ));
        }
        Err(Failure::Raised(text)) => bail!("{text}"),
    };
    let raw = PyStr::from(String::from_utf8_lossy(&raw).into_owned());
    match json::loads_explained(&raw) {
        Ok(Value::Dict(reply)) => Ok(reply.get("response").cloned().unwrap_or(Value::Str(raw))),
        Ok(_) | Err((ParseError::Decode, _)) => Ok(Value::Str(raw)),
        Err((err, message)) => bail!("{}: {message}", err.class()),
    }
}

/// What the model said about one run.
#[derive(Debug)]
pub struct Answer {
    pub needs_correction: bool,
    pub failure_class: Value,
    pub reason: Value,
}

/// `parse_answer`: the model's verdict, or `None` when its text is not one.
pub fn parse_answer(text: &Value) -> Result<Option<Answer>> {
    let Value::Str(text) = text else {
        return Ok(None);
    };
    let answer = match json::loads_explained(text) {
        Ok(answer) => answer,
        Err((ParseError::Decode, _)) => return Ok(None),
        Err((err, message)) => bail!("{}: {message}", err.class()),
    };
    let Value::Dict(answer) = answer else {
        return Ok(None);
    };
    let of = |key: &str| answer.get(key).cloned().unwrap_or(Value::Null);
    // The prompt asks for the reason first and for `result_ok`, the positive of what the
    // file stores. Asked for `needs_correction` and then a reason, the 1B model answered
    // true and wrote "no correction needed" in 9 of 18 runs; asked this way, 0 of 19
    // (2026-09-29).
    if let (Value::Bool(ok), reason @ Value::Str(_)) = (of("result_ok"), of("reason")) {
        return Ok(Some(Answer {
            needs_correction: !ok,
            failure_class: Value::Null,
            reason,
        }));
    }
    let (need, class, reason) = (of("needs_correction"), of("failure_class"), of("reason"));
    let Value::Bool(need) = need else {
        return Ok(None);
    };
    if !CLASSES.iter().any(|known| py::eq_str(&class, known)) || !matches!(reason, Value::Str(_)) {
        return Ok(None);
    }
    Ok(Some(Answer {
        needs_correction: need,
        failure_class: class,
        reason,
    }))
}

/// `raw[:UNPARSED_BUDGET]` on whatever `generate` returned.
fn unparsed_head(raw: &Value) -> Result<Value> {
    match raw {
        Value::Str(text) => Ok(Value::Str(text.head(UNPARSED_BUDGET))),
        Value::List(items) => Ok(Value::List(
            items.iter().take(UNPARSED_BUDGET).cloned().collect(),
        )),
        Value::Dict(_) => bail!("KeyError: slice(None, {UNPARSED_BUDGET}, None)"),
        other => bail!(
            "TypeError: '{}' object is not subscriptable",
            py::type_name(other)
        ),
    }
}

/// `label_line`: one line of `judge-labels.jsonl`.
pub fn label_line(
    run_id: &Value,
    model: &str,
    prompt_sha: Option<&str>,
    answer: Option<&Answer>,
    raw: &Value,
    judged_at: &str,
) -> Result<Value> {
    let part = |pick: fn(&Answer) -> Value| answer.map_or(Value::Null, pick);
    let mut line = Dict::new();
    line.set("run_id", run_id.clone());
    line.set(
        "needs_correction",
        part(|answer| Value::Bool(answer.needs_correction)),
    );
    line.set("failure_class", part(|answer| answer.failure_class.clone()));
    line.set("reason", part(|answer| answer.reason.clone()));
    line.set("judge_model", Value::text(model));
    line.set("prompt_sha256", prompt_sha.map_or(Value::Null, Value::text));
    line.set("judged_at", Value::text(judged_at));
    if answer.is_none() && model != "rule" {
        line.set("unparsed", unparsed_head(raw)?);
    }
    Ok(Value::Dict(line))
}

/// A triple of text, with a sha that may be null, by its code points.
type Plain = (Vec<u32>, Vec<u32>, Option<Vec<u32>>);

/// The `(run_id, judge_model, prompt_sha256)` of every line `judge-labels.jsonl` holds.
#[derive(Default)]
struct Done {
    strings: HashSet<Plain>,
    /// The triples with a member that is neither text nor, in the last place, null.
    others: Vec<[Value; 3]>,
}

impl Done {
    fn plain(triple: &[Value; 3]) -> Option<Plain> {
        let sha = match &triple[2] {
            Value::Null => None,
            Value::Str(sha) => Some(sha.code_points().to_vec()),
            _ => return None,
        };
        match (&triple[0], &triple[1]) {
            (Value::Str(a), Value::Str(b)) => {
                Some((a.code_points().to_vec(), b.code_points().to_vec(), sha))
            }
            _ => None,
        }
    }

    fn add(&mut self, triple: [Value; 3]) -> Result<()> {
        triple.iter().try_for_each(py::hashable)?;
        match Done::plain(&triple) {
            Some(key) => {
                self.strings.insert(key);
            }
            None => self.others.push(triple),
        }
        Ok(())
    }

    fn holds(&self, run_id: &Value, model: &str, sha: Option<&str>) -> Result<bool> {
        py::hashable(run_id)?;
        let triple = [
            run_id.clone(),
            Value::text(model),
            sha.map_or(Value::Null, Value::text),
        ];
        Ok(match Done::plain(&triple) {
            Some(key) => self.strings.contains(&key),
            None => self
                .others
                .iter()
                .any(|held| held.iter().zip(&triple).all(|(a, b)| py::same_key(a, b))),
        })
    }
}

/// `written >= limit`, for a limit of any width.
fn reached(written: usize, limit: &Value) -> bool {
    match limit {
        Value::Int(limit) => written as i128 >= *limit,
        Value::BigInt(digits) => digits.starts_with('-'),
        _ => false,
    }
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The class a rule gives a run the agent did not fail: `RULE_CLASS[outcome]`.
fn rule_class(outcome: &Value) -> Result<Option<&'static str>> {
    py::hashable(outcome)?;
    Ok(if py::eq_str(outcome, "rate_limited") {
        Some("quota")
    } else if py::eq_str(outcome, "gated") {
        Some("gate")
    } else {
        None
    })
}

/// `label`: one pass over the dataset. Returns the exit code: 1 when the model server
/// stopped the pass, after saying so on `stderr`.
pub fn label(
    limit: Option<&Value>,
    only_unlabelled: bool,
    env: &Env,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<u8> {
    let lookup = |name: &str| env.get(name);
    let home = records::grader_home_in(&lookup)?;
    let labels_path = home.join("judge-labels.jsonl");
    let rows = records::read_jsonl(&home.join("dataset.jsonl"))?;
    let mut done = Done::default();
    for line in records::read_jsonl(&labels_path)? {
        done.add([
            py::item(&line, "run_id")?.clone(),
            py::item(&line, "judge_model")?.clone(),
            py::item(&line, "prompt_sha256")?.clone(),
        ])?;
    }
    let runs = records::hydra_root_in(&lookup)?.join("runs");
    let hydra = named(env, "HYDRA_BIN", "hydra");
    let model = ollama_model(env);
    let url = ollama_url(env);
    let (mut written, mut unparsed, mut skipped) = (0usize, 0usize, 0usize);
    std::fs::create_dir_all(&home).map_err(|err| io_error(&err, &home))?;

    for row in &rows {
        if only_unlabelled && !matches!(py::get(row, "human_verdict", Value::Null)?, Value::Null) {
            continue;
        }
        if limit.is_some_and(|limit| reached(written, limit)) {
            break;
        }
        let run_id = py::item(row, "run_id")?;
        let outcome = py::get(row, "outcome", Value::Null)?;
        let line = if let Some(class) = rule_class(&outcome)? {
            if done.holds(run_id, "rule", None)? {
                skipped += 1;
                continue;
            }
            let outcome = py::str(py::item(row, "outcome")?).lossy();
            let mut line = Dict::new();
            line.set("run_id", run_id.clone());
            line.set("needs_correction", Value::Null);
            line.set("failure_class", Value::text(class));
            line.set(
                "reason",
                Value::text(&format!("outcome {outcome} is not a failure of the agent")),
            );
            line.set("judge_model", Value::text("rule"));
            line.set("prompt_sha256", Value::Null);
            line.set("judged_at", Value::text(&utc_now()));
            Value::Dict(line)
        } else {
            let projection = show_projection(run_id, &runs, &hydra, env)?;
            let prompt = build_prompt(row, &projection)?;
            let Some(prompt) = prompt.to_utf8() else {
                bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in the prompt");
            };
            let sha = hex(&Sha256::digest(prompt.as_bytes()));
            if done.holds(run_id, &model, Some(&sha))? {
                skipped += 1;
                continue;
            }
            let raw = match generate(&prompt, &model, &url) {
                Ok(raw) => raw,
                Err(err) => match err.downcast_ref::<Unavailable>() {
                    Some(down) => {
                        let _ = writeln!(
                            stderr,
                            "judge: {down}, so the pass stopped and kept the {written} labels it \
                             had written."
                        );
                        return Ok(1);
                    }
                    None => return Err(err),
                },
            };
            let answer = parse_answer(&raw)?;
            unparsed += usize::from(answer.is_none());
            label_line(
                run_id,
                &model,
                Some(&sha),
                answer.as_ref(),
                &raw,
                &utc_now(),
            )?
        };
        // The file is opened before the line is made, so a line that cannot be written
        // leaves it there, empty if it was not there before.
        let mut handle = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&labels_path)
            .map_err(|err| io_error(&err, &labels_path))?;
        records::refuse_nan(&line)?;
        let text = json::dumps_default(&line) + "\n";
        handle
            .write_all(text.as_bytes())
            .map_err(|err| io_error(&err, &labels_path))?;
        written += 1;
    }
    let _ = writeln!(
        stdout,
        "labelled {written}, unparsed {unparsed}, already labelled {skipped}"
    );
    Ok(0)
}

/// `agree`: what it prints, a count of the runs that hold a human verdict and a judge's.
pub fn agree(env: &Env) -> Result<String> {
    let home = records::grader_home_in(&|name| env.get(name))?;
    let mut dataset = Keyed::new();
    for row in records::read_jsonl(&home.join("dataset.jsonl"))? {
        dataset.set(py::item(&row, "run_id")?.clone(), row)?;
    }
    let mut judged = Keyed::new();
    for line in records::read_jsonl(&home.join("judge-labels.jsonl"))? {
        if !py::eq_str(&py::get(&line, "judge_model", Value::Null)?, "rule") {
            judged.set(py::item(&line, "run_id")?.clone(), line)?;
        }
    }
    let mut unparsed = 0;
    for (_, line) in judged.iter() {
        if matches!(py::get(line, "needs_correction", Value::Null)?, Value::Null) {
            unparsed += 1;
        }
    }
    // Each row is judge true, judge false.
    let (mut fixed, mut applied) = ([0usize; 2], [0usize; 2]);
    for (run_id, line) in judged.iter() {
        let human = match dataset.get(run_id)? {
            Some(row) if py::truthy(row) => py::get(row, "human_verdict", Value::Null)?,
            _ => Value::Null,
        };
        let need = py::get(line, "needs_correction", Value::Null)?;
        py::hashable(&human)?;
        let cells = if py::eq_str(&human, "fixed") {
            &mut fixed
        } else if py::eq_str(&human, "applied") {
            &mut applied
        } else {
            continue;
        };
        if let Value::Bool(need) = need {
            cells[usize::from(!need)] += 1;
        }
    }
    let both = fixed[0] + fixed[1] + applied[0] + applied[1];
    let agreed = fixed[0] + applied[1];
    let mut out = format!("runs with both: {both}\nagree: {agreed}\n");
    if both > 0 {
        out.push_str(&format!(
            "agreement: {agreed}/{both} = {:.2}\n",
            agreed as f64 / both as f64
        ));
    } else {
        out.push_str("agreement: none, no run has both labels\n");
    }
    if both < LOOSE_BELOW {
        out.push_str(&format!(
            "This number is loose: fewer than {LOOSE_BELOW} runs have both labels.\n"
        ));
    }
    for (name, [yes, no]) in [("fixed", fixed), ("applied", applied)] {
        out.push_str(&format!(
            "human {name:<7}  judge true {yes}    judge false {no}\n"
        ));
    }
    out.push_str(&format!("unparsed answers: {unparsed}\n"));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loads(text: &str) -> Value {
        json::loads_str(text).unwrap()
    }

    #[test]
    fn a_long_text_is_cut_by_code_points() {
        let text = PyStr::from("\u{e9}\u{1f600}abc");
        assert_eq!(cut(&text, 5), text);
        assert_eq!(
            cut(&text, 2).lossy(),
            "\u{e9}\u{1f600}\n[cut: 3 more characters]"
        );
    }

    #[test]
    fn an_answer_is_read_in_either_shape_and_nothing_else_is_one() {
        let parse = |text: &str| parse_answer(&Value::text(text)).unwrap();
        let answer = parse(r#"{"reason": "fine", "result_ok": true}"#).unwrap();
        assert!(!answer.needs_correction);
        assert!(matches!(answer.failure_class, Value::Null));
        let answer =
            parse(r#"{"needs_correction": true, "failure_class": "agent", "reason": "r"}"#)
                .unwrap();
        assert!(answer.needs_correction && py::eq_str(&answer.failure_class, "agent"));
        for text in [
            "not json",
            "[]",
            r#"{"reason": "r", "result_ok": 1}"#,
            r#"{"needs_correction": true, "failure_class": "weather", "reason": "r"}"#,
            r#"{"needs_correction": "yes", "failure_class": "agent", "reason": "r"}"#,
            r#"{"needs_correction": true, "failure_class": "agent", "reason": 7}"#,
        ] {
            assert!(parse(text).is_none(), "{text}");
        }
        assert!(parse_answer(&loads("{}")).unwrap().is_none());
    }

    #[test]
    fn a_path_resolves_past_what_is_not_there() {
        let cwd = std::fs::canonicalize(".").unwrap();
        assert_eq!(
            resolve(Path::new("no-such/../none/./runs")),
            cwd.join("none/runs")
        );
    }

    #[test]
    fn a_model_server_on_another_machine_is_unavailable_and_never_called() {
        let err = generate("p", "m", "http://example.com/api/generate").unwrap_err();
        let down = err.downcast_ref::<Unavailable>().expect("unavailable");
        assert!(down.0.contains("\"example.com\""), "{down}");
    }
}
