//! `records.py`: read one run record for grading, and the grader's own files.
//!
//! A record is whatever its writer left, so each function here does what the Python does
//! with a value of the wrong shape: it returns an error whose text starts with the class
//! Python raises, where the Python lets the exception through, and it skips the record with
//! the Python's line on stderr, where the Python catches it.
//!
//! `refresh_show_answer` and `iter_records` shell out to `hydra show` and are not here.
//! They arrive with `collect`.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::file::{os_error, read_text};
use hydra_pyjson::json::{self, Dict, ParseError, PyStr, Value};
use hydra_pyjson::models::{Pair, canonical};
use hydra_pyjson::py;
use sha2::{Digest, Sha256};

use crate::isotime;

/// `hydra_root` given the environment: `HYDRA_ROOT` when it is set and not empty, else
/// `hydra` under the home directory.
pub fn hydra_root_in(env: &dyn Fn(&str) -> Option<OsString>) -> Result<PathBuf> {
    if let Some(named) = env("HYDRA_ROOT").filter(|named| !named.is_empty()) {
        return Ok(PathBuf::from(named));
    }
    // `Path.home()`: `HOME` when it is set, and an empty one is the root directory.
    let home = match env("HOME") {
        Some(home) if home.is_empty() => PathBuf::from("/"),
        Some(home) => PathBuf::from(home),
        None => std::env::home_dir()
            .ok_or_else(|| anyhow!("RuntimeError: Could not determine home directory."))?,
    };
    Ok(home.join("hydra"))
}

/// `grader_home` given the environment: `HYDRA_GRADER_HOME` when it is set and not empty,
/// else `grader` under the root.
pub fn grader_home_in(env: &dyn Fn(&str) -> Option<OsString>) -> Result<PathBuf> {
    if let Some(named) = env("HYDRA_GRADER_HOME").filter(|named| !named.is_empty()) {
        return Ok(PathBuf::from(named));
    }
    Ok(hydra_root_in(env)?.join("grader"))
}

/// `hydra_root()`.
pub fn hydra_root() -> Result<PathBuf> {
    hydra_root_in(&|name| std::env::var_os(name))
}

/// `grader_home()`.
pub fn grader_home() -> Result<PathBuf> {
    grader_home_in(&|name| std::env::var_os(name))
}

/// `read_json`: the document at `path`, or `None` when nothing is there, which is where the
/// Python returns its default.
pub fn read_json(path: &Path) -> Result<Option<Value>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = read_text(path)?;
    match json::loads_explained(&PyStr::from(text)) {
        Ok(value) => Ok(Some(value)),
        Err((err, message)) => bail!("{}: {message}", err.class()),
    }
}

/// What a text-mode read does to line ends: `\r\n` and a lone `\r` both become `\n`.
fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// `current_preamble`: what `config/preamble.md` under `root` puts in front of a task, with
/// exactly one blank line after it. `None` when there is no such file.
pub fn current_preamble(root: &Path) -> Result<Option<PyStr>> {
    let path = root.join("config").join("preamble.md");
    if !path.is_file() {
        return Ok(None);
    }
    let text = universal_newlines(&read_text(&path)?);
    Ok(Some(PyStr::from(format!(
        "{}\n\n",
        text.trim_end_matches('\n')
    ))))
}

fn find(hay: &[u32], needle: &[u32]) -> Option<usize> {
    hay.windows(needle.len())
        .position(|window| window == needle)
}

fn text(codes: &[u32]) -> PyStr {
    let mut out = PyStr::new();
    for code in codes {
        out.push_code(*code);
    }
    out
}

/// `strip_preamble`: the task without the preamble Hydra put in front of it, and whether
/// one was found. The preamble is today's, or an older wording told by its first sentence
/// and its last.
pub fn strip_preamble(task_text: &PyStr, current: Option<&PyStr>) -> (PyStr, bool) {
    let task = task_text.code_points();
    if let Some(current) = current.filter(|current| !current.is_empty())
        && task.starts_with(current.code_points())
    {
        return (text(&task[current.len()..]), true);
    }

    let first_line = PyStr::from("Hydra launched this run and wrote its instructions at spawn.");
    let marker = PyStr::from("say so and stop.\n\n");
    if task.starts_with(first_line.code_points())
        && let Some(at) = find(task, marker.code_points())
    {
        return (text(&task[at + marker.len()..]), true);
    }

    (task_text.clone(), false)
}

/// `allow_nan=False`: the float `json.dumps` refuses, if the value holds one.
fn out_of_range(value: &Value) -> Option<f64> {
    match value {
        Value::Float(x) if !x.is_finite() => Some(*x),
        Value::List(items) => items.iter().find_map(out_of_range),
        Value::Dict(dict) => dict.iter().find_map(|(_, item)| out_of_range(item)),
        _ => None,
    }
}

pub fn refuse_nan(value: &Value) -> Result<()> {
    match out_of_range(value) {
        Some(x) => bail!(
            "ValueError: Out of range float values are not JSON compliant: {}",
            json::float_repr(x)
        ),
        None => Ok(()),
    }
}

/// The file a write goes to first: the target's name with `.tmp` after it.
fn temp_beside(path: &Path) -> Result<PathBuf> {
    let mut name = path
        .file_name()
        .ok_or_else(|| anyhow!("ValueError: {} has an empty name", path.display()))?
        .to_os_string();
    name.push(".tmp");
    Ok(path.with_file_name(name))
}

fn io_error(err: &std::io::Error, path: &Path) -> anyhow::Error {
    anyhow!("{}: {}", os_error(err), path.display())
}

fn make_parent(path: &Path) -> Result<()> {
    match path.parent() {
        Some(parent) => std::fs::create_dir_all(parent).map_err(|err| io_error(&err, parent)),
        None => Ok(()),
    }
}

fn replace(temp: &Path, path: &Path) -> Result<()> {
    std::fs::rename(temp, path).map_err(|err| io_error(&err, path))
}

/// `write_json`: `value` indented by two, written beside `path` and renamed over it. A NaN
/// or an infinity is refused before anything but the directory is made.
pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    make_parent(path)?;
    let temp = temp_beside(path)?;
    refuse_nan(value)?;
    let text = json::dumps(value, Some(2)) + "\n";
    std::fs::write(&temp, text).map_err(|err| io_error(&err, &temp))?;
    replace(&temp, path)
}

/// `read_jsonl`: every complete line of `path`, parsed. No file is no rows; a complete line
/// that does not parse is an error.
pub fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let data = std::fs::read(path).map_err(|err| io_error(&err, path))?;
    match committed_lines(&data) {
        Ok((lines, _)) => Ok(lines),
        Err(bad) => Err(bad.raised()),
    }
}

/// `write_jsonl`: one row a line, written beside `path` and renamed over it. A row with a
/// NaN or an infinity stops the write there: the rows before it stay in the `.tmp` file and
/// `path` is not touched, which is what the Python leaves when `json.dumps` raises inside
/// its `with` block.
pub fn write_jsonl(path: &Path, rows: &[Value]) -> Result<()> {
    make_parent(path)?;
    let temp = temp_beside(path)?;
    let mut text = String::new();
    let mut refused = Ok(());
    for row in rows {
        refused = refuse_nan(row);
        if refused.is_err() {
            break;
        }
        text.push_str(&json::dumps_default(row));
        text.push('\n');
    }
    std::fs::write(&temp, text).map_err(|err| io_error(&err, &temp))?;
    refused?;
    replace(&temp, path)
}

/// A complete line `json.loads` refused: the exception's class, and `str(err)` for the two
/// classes `scan_record` catches.
#[derive(Debug)]
pub struct BadLine {
    pub error: ParseError,
    pub message: String,
}

impl BadLine {
    /// The error a caller that does not catch it ends with.
    pub fn raised(&self) -> anyhow::Error {
        anyhow!("{}: {}", self.error.class(), self.message)
    }
}

/// `bytes.splitlines(keepends=True)`: a line ends at `\n`, at `\r\n` or at a lone `\r`, and
/// at nothing else. Not at a vertical tab or a form feed, which end a line of a `str`.
fn split_lines(data: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < data.len() {
        let end = match data[i] {
            b'\n' => i + 1,
            b'\r' if data.get(i + 1) == Some(&b'\n') => i + 2,
            b'\r' => i + 1,
            _ => {
                i += 1;
                continue;
            }
        };
        lines.push(&data[start..end]);
        start = end;
        i = end;
    }
    if start < data.len() {
        lines.push(&data[start..]);
    }
    lines
}

/// `committed_lines`: the parsed complete lines and how many bytes they cover. The scan
/// stops at the first line that does not end in `\n`, a torn tail or a line cut by a lone
/// carriage return. A complete line that does not parse is an error, and the caller decides
/// what to do with the record.
pub fn committed_lines(data: &[u8]) -> Result<(Vec<Value>, usize), BadLine> {
    let mut lines = Vec::new();
    let mut covered = 0;
    for line in split_lines(data) {
        if !line.ends_with(b"\n") {
            break;
        }
        let value = json::loads_bytes_explained(line)
            .map_err(|(error, message)| BadLine { error, message })?;
        lines.push(value);
        covered += line.len();
    }
    Ok((lines, covered))
}

/// `value.get(key)` where the Python calls it on whatever the record holds.
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

/// `str(value).replace("Z", "+00:00")`, which is how the Python makes a `Z` readable to a
/// `fromisoformat` that once did not take it.
fn stamp(value: &Value) -> PyStr {
    let mut out = PyStr::new();
    for code in py::str(value).code_points() {
        if *code == u32::from('Z') {
            out.push_str("+00:00");
        } else {
            out.push_code(*code);
        }
    }
    out
}

/// `duration_seconds`: from the first line's `ts` to the `ended_at` of the last `run_end`.
/// `None` when there is no end or when either time does not read. An error when one time
/// has an offset and the other has none, which Hydra's own `Z` timestamps never are.
pub fn duration_seconds(first: &Value, lines: &[Value]) -> Result<Option<f64>> {
    let mut end = Value::Dict(Dict::new());
    for row in lines.iter().rev() {
        let hydra = py::get(row, "hydra", Value::Dict(Dict::new()))?;
        if py::eq_str(&get(&hydra, "type")?, "run_end") {
            end = hydra;
            break;
        }
    }
    let ended_at = get(&end, "ended_at")?;
    if !py::truthy(&ended_at) {
        return Ok(None);
    }
    let Some(started) = isotime::fromisoformat(&stamp(&get(first, "ts")?)) else {
        return Ok(None);
    };
    let Some(ended) = isotime::fromisoformat(&stamp(&ended_at)) else {
        return Ok(None);
    };
    // The subtraction is outside the Python's `try`, so this one is raised.
    match isotime::seconds_between(&started, &ended) {
        Some(seconds) => Ok(Some(seconds)),
        None => bail!("TypeError: can't subtract offset-naive and offset-aware datetimes"),
    }
}

/// `record_row`: what the dataset keeps of one record, given the answer of `hydra show`.
pub fn record_row(
    first: &Value,
    lines: &[Value],
    answer: &Value,
    aliases: &[(Pair, Pair)],
) -> Result<Value> {
    let start = py::get(first, "hydra", Value::Dict(Dict::new()))?;
    let projection = or_empty(get(answer, "projection")?);
    let usage = or_empty(get(&projection, "usage")?);
    let recorded_model = get(first, "model")?;
    let provider = py::item(first, "provider")?;
    let (canonical_provider, canonical_model) = canonical(provider, &recorded_model, aliases)?;

    let mut row = Dict::new();
    row.set("run_id", py::item(first, "run_id")?.clone());
    row.set("provider", provider.clone());
    row.set("recorded_model", recorded_model);
    row.set("canonical_provider", canonical_provider);
    row.set("canonical_model", canonical_model);
    row.set("policy", get(&start, "policy")?);
    row.set("task", get(&start, "task")?);
    row.set("outcome", get(&projection, "outcome")?);
    row.set("tokens", or_empty(get(&usage, "counts")?));
    row.set(
        "duration_seconds",
        duration_seconds(first, lines)?.map_or(Value::Null, Value::Float),
    );
    Ok(Value::Dict(row))
}

/// One record `scan_record` took: a provider run or a resume.
#[derive(Debug)]
pub struct Scanned {
    pub path: PathBuf,
    /// The whole file, torn tail included.
    pub data: Vec<u8>,
    /// Its complete lines.
    pub lines: Vec<Value>,
    /// Line 0, an object.
    pub first: Value,
    pub run_id: Value,
    /// The SHA-256 of the bytes the complete lines cover, which is what says a record
    /// changed since `hydra show` was last asked about it.
    pub fingerprint: String,
}

/// `scan_record`: one record, or `None` when it is not a provider run or cannot be read. A
/// garbled line skips this record and names it on `stderr` rather than ending the pass for
/// every other one. What the Python does not catch is an error here: an integer past 4,300
/// digits, or a `hydra` field that is neither an object nor empty.
pub fn scan_record(path: &Path, stderr: &mut dyn Write) -> Result<Option<Scanned>> {
    let data = std::fs::read(path).map_err(|err| io_error(&err, path))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (lines, covered) = match committed_lines(&data) {
        Ok(read) => read,
        Err(bad) if matches!(bad.error, ParseError::Decode | ParseError::Unicode) => {
            let _ = writeln!(stderr, "grader: skipping {name}: {}", bad.message);
            return Ok(None);
        }
        Err(bad) => return Err(bad.raised()),
    };
    let Some(first) = lines.first().cloned() else {
        return Ok(None);
    };
    if !matches!(first, Value::Dict(_)) {
        let _ = writeln!(stderr, "grader: skipping {name}: line 0 is not an object");
        return Ok(None);
    }
    let start = or_empty(get(&first, "hydra")?);
    let run_id = get(&first, "run_id")?;
    if !py::truthy(&get(&first, "provider")?) || !py::truthy(&run_id) {
        return Ok(None);
    }
    let command = get(&start, "command")?;
    if !py::eq_str(&command, "run") && !py::eq_str(&command, "resume") {
        return Ok(None);
    }
    let fingerprint = Sha256::digest(&data[..covered])
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(Some(Scanned {
        path: path.to_path_buf(),
        data,
        lines,
        first,
        run_id,
        fingerprint,
    }))
}
