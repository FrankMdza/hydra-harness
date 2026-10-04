//! `apprentice.py` lines 325 to 444: `attempt` asks the local model each task of the bank it
//! has not answered at this prompt, and writes one line of `attempts.jsonl` per answer.
//!
//! The one network call is `loopback_http::post_json`, which goes to this machine and nowhere
//! else. The Python calls whatever `HYDRA_LOCAL_LLM_URL` names; here a URL that names any
//! other host ends the pass with one sentence and exit 1, as Ollama being down does (map 95,
//! question C).
//!
//! A bank or an attempts file is whatever was left there, so where the Python raises on a
//! value of the wrong shape this returns an error whose text starts with the class Python
//! raises.

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use hydra_pyjson::file::os_error;
use hydra_pyjson::json::{self, Dict, ParseError, PyStr, Value};
use hydra_pyjson::py;
use hydra_records::loopback_http::{self, Failure};
use hydra_records::records;
use sha2::{Digest, Sha256};

use crate::bank::home;

const DEFAULT_MODEL: &str = "gemma3:1b-it-qat";
const DEFAULT_URL: &str = "http://127.0.0.1:11434/api/generate";
const NUM_PREDICT: i128 = 300;
const TIMEOUT: Duration = Duration::from_secs(300);
const UNPARSED_BUDGET: usize = 500;

pub const INSTRUCTIONS: &str =
    "You answer questions about records of coding-agent runs. Below is one task.

First write your reasoning in one or two sentences. Then give the answer. Answer with one JSON \
object and nothing else, with the keys in this order:
{\"reason\": \"<one or two sentences>\", \"answer\": \"<your answer>\"}

When the task wants a number, the answer is the number alone. When the task lists the possible \
answers, the answer is one of them, spelled exactly. When the task wants a command or a piece of \
writing, the answer is that text.";

/// The environment as `os.environ` gives it.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<OsString>;

fn environ(env: Env, name: &str, default: &str) -> String {
    env(name).map_or_else(
        || default.to_string(),
        |value| value.to_string_lossy().into_owned(),
    )
}

/// `ollama_url()`.
pub fn ollama_url(env: Env) -> String {
    environ(env, "HYDRA_LOCAL_LLM_URL", DEFAULT_URL)
}

/// `ollama_model()`.
pub fn ollama_model(env: Env) -> String {
    environ(env, "HYDRA_LOCAL_MODEL", DEFAULT_MODEL)
}

/// `prompt_sha()`: the SHA-256 of the instructions, which is what tells one prompt's
/// attempts from another's.
pub fn prompt_sha() -> String {
    Sha256::digest(INSTRUCTIONS.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `Unavailable`: Ollama did not answer, does not have the model, or is not on this machine.
#[derive(Debug)]
pub struct Unavailable(pub String);

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Unavailable {}

/// The body of one call, as `json.dumps` writes it.
pub fn request_body(model: &str, prompt: &PyStr) -> String {
    let mut options = Dict::new();
    options.set("temperature", Value::Int(0));
    options.set("num_predict", Value::Int(NUM_PREDICT));
    let mut body = Dict::new();
    body.set("model", Value::text(model));
    body.set("prompt", Value::Str(prompt.clone()));
    body.set("stream", Value::Bool(false));
    body.set("format", Value::text("json"));
    body.set("options", Value::Dict(options));
    json::dumps_default(&Value::Dict(body))
}

/// `generate`: the `response` of one call, or the whole reply when it is not an object that
/// has one.
pub fn generate(prompt: &PyStr, env: Env) -> Result<Value> {
    let (model, url) = (ollama_model(env), ollama_url(env));
    let body = request_body(&model, prompt);
    let unavailable = |text: String| Err(Unavailable(text).into());
    let raw = match loopback_http::post_json(&url, body.as_bytes(), TIMEOUT) {
        Ok(raw) => raw,
        Err(Failure::Status(404, _)) => {
            return unavailable(format!("Ollama does not have the model {model}"));
        }
        Err(Failure::Status(code, _)) => {
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

/// `parse_reply`: the answer, as text, and the reason, or `None` when the reply is not an
/// object with an answer that is text or a number.
pub fn parse_reply(text: &Value) -> Result<Option<(PyStr, PyStr)>> {
    // `json.loads` of anything but text raises `TypeError`, which the Python catches.
    let Value::Str(text) = text else {
        return Ok(None);
    };
    let reply = match json::loads_explained(text) {
        Ok(reply) => reply,
        Err((ParseError::Decode, _)) => return Ok(None),
        Err((err, message)) => bail!("{}: {message}", err.class()),
    };
    let Value::Dict(reply) = reply else {
        return Ok(None);
    };
    let answer = match reply.get("answer") {
        Some(answer @ (Value::Str(_) | Value::Int(_) | Value::BigInt(_) | Value::Float(_))) => {
            py::str(answer)
        }
        _ => return Ok(None),
    };
    let reason = match reply.get("reason") {
        Some(Value::Str(reason)) => reason.clone(),
        _ => PyStr::new(),
    };
    Ok(Some((answer, reason)))
}

/// `str.lower()`. A lone surrogate has no case and is kept.
fn lower(text: &PyStr) -> PyStr {
    let mut out = PyStr::new();
    let mut run = String::new();
    for &code in text.code_points() {
        match char::from_u32(code) {
            Some(c) => run.push(c),
            None => {
                out.push_str(&run.to_lowercase());
                run.clear();
                out.push_code(code);
            }
        }
    }
    out.push_str(&run.to_lowercase());
    out
}

/// `text` without any of `chars` at its start, when `start`, and at its end.
fn strip_set(text: &PyStr, chars: &[char], start: bool) -> PyStr {
    let codes = text.code_points();
    let drop = |code: &u32| chars.iter().any(|c| u32::from(*c) == *code);
    let from = if start {
        codes.iter().position(|c| !drop(c)).unwrap_or(codes.len())
    } else {
        0
    };
    let to = codes
        .iter()
        .rposition(|c| !drop(c))
        .map_or(from, |at| at + 1);
    let mut out = PyStr::new();
    for code in &codes[from..to.max(from)] {
        out.push_code(*code);
    }
    out
}

/// `_norm`: `str(text).strip().strip("`\"'").rstrip(".").strip().lower()`.
fn norm(text: &Value) -> PyStr {
    let text = py::strip(&py::str(text));
    let text = strip_set(&text, &['`', '"', '\''], true);
    let text = strip_set(&text, &['.'], false);
    lower(&py::strip(&text))
}

/// The digits `re.fullmatch(r"\s*(-?\d+)\s*\.?\s*", text)` captures. `\s` and `\d` are
/// Unicode in a `str` pattern: `str.isspace` and every decimal digit of any script.
fn number_in(text: &PyStr) -> Option<String> {
    let codes: Vec<Option<char>> = text
        .code_points()
        .iter()
        .map(|code| char::from_u32(*code))
        .collect();
    let space = |at: usize| codes.get(at).copied().flatten().is_some_and(py::is_space);
    let mut at = 0;
    while space(at) {
        at += 1;
    }
    let mut group = String::new();
    if codes.get(at) == Some(&Some('-')) {
        group.push('-');
        at += 1;
    }
    let digits = group.len();
    while let Some(Some(c)) = codes.get(at)
        && py::decimal_digit(*c).is_some()
    {
        group.push(*c);
        at += 1;
    }
    if group.len() == digits {
        return None;
    }
    while space(at) {
        at += 1;
    }
    if codes.get(at) == Some(&Some('.')) {
        at += 1;
    }
    while space(at) {
        at += 1;
    }
    (at == codes.len()).then_some(group)
}

const INT_DIGITS: &str =
    "ValueError: Exceeds the limit (4300 digits) for integer string conversion";

/// `int(value)` for what a bank's `answer` may hold.
fn int_of(value: &Value) -> Result<Value> {
    Ok(match value {
        Value::Bool(b) => Value::Int(i128::from(*b)),
        Value::Int(_) | Value::BigInt(_) => value.clone(),
        Value::Float(f) if f.is_nan() => bail!("ValueError: cannot convert float NaN to integer"),
        Value::Float(f) if f.is_infinite() => {
            bail!("OverflowError: cannot convert float infinity to integer")
        }
        Value::Float(f) => match py::int_value(&format!("{:.0}", f.trunc())) {
            Some(n) => n,
            None => bail!("{INT_DIGITS}"),
        },
        Value::Str(text) => match text.to_utf8().as_deref().and_then(py::int_value) {
            Some(n) => n,
            None => bail!(
                "ValueError: invalid literal for int() with base 10: {}",
                py::repr(value)
            ),
        },
        other => bail!(
            "TypeError: int() argument must be a string, a bytes-like object or a real number, not '{}'",
            py::type_name(other)
        ),
    })
}

/// `grade`: whether `answer` is right by the task's `check`.
pub fn grade(task: &Value, answer: &PyStr) -> Result<bool> {
    let (check, want) = (py::item(task, "check")?, py::item(task, "answer")?);
    let answer = Value::Str(answer.clone());
    if py::eq_str(check, "exact") {
        return Ok(norm(&answer) == norm(want));
    }
    if py::eq_str(check, "one_of") {
        let mine = norm(&answer);
        return Ok(py::list(want)?.iter().any(|w| norm(w) == mine));
    }
    if py::eq_str(check, "number") {
        let Value::Str(text) = &answer else {
            unreachable!("the answer is text");
        };
        let Some(group) = number_in(text) else {
            return Ok(false);
        };
        let Some(found) = py::int_value(&group) else {
            bail!("{INT_DIGITS}");
        };
        return Ok(py::eq(&found, &int_of(want)?));
    }
    bail!("ValueError: unknown check {}", py::repr(check))
}

/// The `(task_id, prompt_sha, model)` of every attempt already written: a set in the Python,
/// so each member has to hash, and a lookup finds a member by `is` or `==`.
struct Done(Vec<[Value; 3]>);

impl Done {
    fn read(path: &Path) -> Result<Done> {
        let mut done = Vec::new();
        for row in records::read_jsonl(path)? {
            let triple = [
                py::item(&row, "task_id")?.clone(),
                py::item(&row, "prompt_sha")?.clone(),
                py::item(&row, "model")?.clone(),
            ];
            triple.iter().try_for_each(py::hashable)?;
            done.push(triple);
        }
        Ok(Done(done))
    }

    fn holds(&self, triple: &[Value; 3]) -> Result<bool> {
        triple.iter().try_for_each(py::hashable)?;
        Ok(self
            .0
            .iter()
            .any(|held| held.iter().zip(triple).all(|(a, b)| py::same_key(a, b))))
    }
}

/// `raw[:500]` on whatever `generate` returned.
fn unparsed_head(raw: &Value) -> Result<Value> {
    py::head(raw, UNPARSED_BUDGET)
}

/// `round(seconds, 2)`.
fn round2(seconds: f64) -> f64 {
    format!("{seconds:.2}").parse().unwrap_or(seconds)
}

/// `format(x, "g")`: six significant digits, the exponent form below 1e-4 and from 1e6,
/// and no trailing zeros.
pub fn format_g(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.into();
    }
    let sign = if x.is_sign_negative() { "-" } else { "" };
    if x == 0.0 {
        return format!("{sign}0");
    }
    let sci = format!("{:.5e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').expect("{:e} has an exponent");
    let exp: i32 = exp.parse().expect("{:e} exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let digits = digits.trim_end_matches('0');
    if !(-4..6).contains(&exp) {
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let esign = if exp < 0 { '-' } else { '+' };
        return format!("{sign}{first}{rest}e{esign}{:02}", exp.abs());
    }
    let point = exp + 1;
    let text = if point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}", "0".repeat(point as usize - digits.len()))
    } else {
        let (head, tail) = digits.split_at(point as usize);
        format!("{head}.{tail}")
    };
    format!("{sign}{text}")
}

/// `limit` as the loop compares it: an integer past 128 bits is as good as infinite.
fn bound(limit: &Value) -> i128 {
    match limit {
        Value::Int(n) => *n,
        Value::BigInt(digits) if digits.starts_with('-') => i128::MIN,
        _ => i128::MAX,
    }
}

/// One line of `attempts.jsonl`, appended as `json.dumps(line, allow_nan=False)`. The file
/// is opened before the line is written, so a line Python refuses leaves the file there.
fn append(path: &Path, line: &Value) -> Result<()> {
    let mut handle = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|err| anyhow::anyhow!("{}: {}", os_error(&err), path.display()))?;
    records::refuse_nan(line)?;
    let mut text = json::dumps_default(line);
    text.push('\n');
    handle
        .write_all(text.as_bytes())
        .map_err(|err| anyhow::anyhow!("{}: {}", os_error(&err), path.display()))
}

/// `attempt`: ask the model each task it has not answered at this prompt, up to `limit`
/// answers and for about `minutes`. The exit code, or an error where the Python raises.
pub fn attempt(
    limit: Option<&Value>,
    minutes: Option<f64>,
    env: Env,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<u8> {
    let base = home(&records::grader_home_in(env)?);
    let bank = records::read_jsonl(&base.join("bank.jsonl"))?;
    if bank.is_empty() {
        let _ = writeln!(
            stderr,
            "attempt: no bank yet, run `hydra-apprentice bank build` first."
        );
        return Ok(1);
    }
    let path = base.join("attempts.jsonl");
    let sha = Value::text(&prompt_sha());
    let model = Value::text(&ollama_model(env));
    let done = Done::read(&path)?;
    let began = Instant::now();
    let mut written: i128 = 0;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| anyhow::anyhow!("{}: {}", os_error(&err), parent.display()))?;
    }
    for task in &bank {
        let id = py::item(task, "id")?;
        if done.holds(&[id.clone(), sha.clone(), model.clone()])? {
            continue;
        }
        if limit.is_some_and(|limit| written >= bound(limit)) {
            break;
        }
        if let Some(minutes) = minutes
            && began.elapsed().as_secs_f64() >= minutes * 60.0
        {
            let _ = writeln!(
                stdout,
                "attempt: stopped at the {} minute budget after {written} attempts.",
                format_g(minutes)
            );
            return Ok(0);
        }
        let mut prompt = PyStr::from(INSTRUCTIONS);
        prompt.push_str("\n\nTASK:\n");
        prompt.push(&py::str(py::item(task, "prompt")?));
        prompt.push_str("\n");
        let t0 = Instant::now();
        let raw = match generate(&prompt, env) {
            Ok(raw) => raw,
            Err(err) => match err.downcast::<Unavailable>() {
                Ok(Unavailable(why)) => {
                    let _ = writeln!(
                        stderr,
                        "attempt: {why}, so the pass stopped and kept the {written} attempts it had written."
                    );
                    return Ok(1);
                }
                Err(err) => return Err(err),
            },
        };
        let parsed = parse_reply(&raw)?;
        let (answer, reason) = parsed.clone().unwrap_or_default();
        let mut line = Dict::new();
        line.set("task_id", id.clone());
        line.set("template", py::item(task, "template")?.clone());
        line.set("prompt_sha", sha.clone());
        line.set("answer", Value::Str(answer.clone()));
        line.set("reason", Value::Str(reason));
        line.set("model", model.clone());
        line.set("seconds", Value::Float(round2(t0.elapsed().as_secs_f64())));
        if parsed.is_none() {
            line.set("unparsed", unparsed_head(&raw)?);
        }
        if py::eq_str(py::item(task, "class")?, "verifiable") {
            let correct = parsed.is_some() && grade(task, &answer)?;
            line.set("correct", Value::Bool(correct));
        }
        append(&path, &Value::Dict(line))?;
        written += 1;
    }
    let _ = writeln!(stdout, "attempt: wrote {written} attempts");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_formats_as_python_does() {
        for (x, want) in [
            (0.0, "0"),
            (-0.0, "-0"),
            (30.0, "30"),
            (0.5, "0.5"),
            (-1.5e10, "-1.5e+10"),
            (1e-300, "1e-300"),
            (123456789.0, "1.23457e+08"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (100000.0, "100000"),
            (1e6, "1e+06"),
            (999999.5, "1e+06"),
            (f64::INFINITY, "inf"),
        ] {
            assert_eq!(format_g(x), want, "{x}");
        }
    }

    #[test]
    fn a_number_is_any_script_digits_with_space_around() {
        let found = |text: &str| number_in(&PyStr::from(text));
        assert_eq!(found(" -12 . ").as_deref(), Some("-12"));
        assert_eq!(found("\u{1c}٣\u{2003}").as_deref(), Some("٣"));
        assert_eq!(found("12.5"), None);
        assert_eq!(found("- 1"), None);
        assert_eq!(found(""), None);
        assert_eq!(found("1.."), None);
    }

    #[test]
    fn norm_strips_quotes_and_a_final_point_and_lowers() {
        let normed = |text: &str| norm(&Value::text(text)).lossy();
        // The point is stripped after the quotes, so quotes before it stay. Each `want` is
        // what CPython 3.14.8 printed.
        assert_eq!(normed("  `\"Claude\"`. "), "claude\"`");
        assert_eq!(normed(" \"x.\". "), "x.\"");
        assert_eq!(normed("'ΟΔΟΣ'"), "\u{3bf}\u{3b4}\u{3bf}\u{3c2}");
        assert_eq!(normed("İ"), "i\u{307}");
    }
}
