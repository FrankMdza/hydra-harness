//! `grader.py`: one pass over the run records, and `records.refresh_show_answer`.
//!
//! A pass scans every record, asks `hydra show <id> --events` about each one whose bytes or
//! label changed since the cursor, and rewrites `dataset.jsonl`, `cursor.json` and
//! `show-cache.json` under the grader home. Each rewritten row is then scored with every
//! model under `models/`, one line per run and model version appended to `scores.jsonl`.
//!
//! The order of the writes is the Python's, so a pass that fails leaves what the Python
//! leaves: nothing when a goal log cannot be shown or `hydra show` fails, the cache alone
//! when a row cannot be built.

use std::collections::HashSet;
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::file::os_error;
use hydra_pyjson::json::{self, Dict, PyStr, Value};
use hydra_pyjson::models::Pair;
use hydra_pyjson::py;
use sha2::{Digest, Sha256};

use crate::baseline;
use crate::isotime::DateTime;
use crate::labels::{self, Env, Keyed, glob, os_error_text, return_code};
use crate::records::{self, Scanned};
use crate::redact;

/// The stack a thread that reads records gets: room for a line nested as deep as
/// `json::MAX_NESTING`, which the reader recurses through once per level.
pub const STACK_BYTES: usize = 1 << 30;

/// `utc_now()`: this second, in UTC, as `2026-10-01T12:00:00Z`.
pub fn utc_now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    DateTime::from_unix_millis(seconds * 1000)
        .map(|now| now.isoformat().replace("+00:00", "Z"))
        .unwrap_or_default()
}

fn io_error(err: &std::io::Error, path: &Path) -> anyhow::Error {
    anyhow!("{}: {}", os_error(err), path.display())
}

fn empty() -> Value {
    Value::Dict(Dict::new())
}

/// `value or {}`.
fn or_empty(value: Value) -> Value {
    if py::truthy(&value) { value } else { empty() }
}

fn get(value: &Value, key: &str) -> Result<Value> {
    py::get(value, key, Value::Null)
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

/// The value with every object's keys in the order `sort_keys=True` prints them.
pub(crate) fn sorted_keys(value: &Value) -> Value {
    match value {
        Value::Dict(dict) => {
            let mut pairs: Vec<(&PyStr, &Value)> = dict.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Dict::new();
            for (key, item) in pairs {
                out.push(key.clone(), sorted_keys(item));
            }
            Value::Dict(out)
        }
        Value::List(items) => Value::List(items.iter().map(sorted_keys).collect()),
        other => other.clone(),
    }
}

/// `label_digest`: sixteen hex digits that change when a model-written label does.
pub fn label_digest(label: &Value) -> String {
    let text = json::dumps_default(&sorted_keys(label));
    let digest: String = Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    digest[..16].to_string()
}

/// `d.get(key)` on a value read from one of the grader's own JSON files, whose keys are all
/// strings: a key that is not a string is found nowhere, and one Python cannot hash is an
/// error.
fn lookup(dict: &Value, key: &Value) -> Result<Option<Value>> {
    let Value::Dict(dict) = dict else {
        bail!(
            "AttributeError: '{}' object has no attribute 'get'",
            py::type_name(dict)
        );
    };
    py::hashable(key)?;
    Ok(match key {
        Value::Str(key) => dict.get_key(key).cloned(),
        _ => None,
    })
}

/// A directory that is removed when this goes, as `tempfile.TemporaryDirectory` removes its.
struct TempDir(PathBuf);

impl TempDir {
    fn new(env: &Env) -> Result<TempDir> {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let base = ["TMPDIR", "TEMP", "TMP"]
            .iter()
            .find_map(|name| env.named(name))
            .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let path = base.join(format!(
            "hydra-grader-{}-{}-{nanos}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|err| io_error(&err, &path))?;
        Ok(TempDir(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// What a signal is called in `CalledProcessError`'s sentence.
fn signal_name(signal: i32) -> Option<&'static str> {
    Some(match signal {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        6 => "SIGABRT",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        _ => return None,
    })
}

/// What Python raises for a program with an empty name, which is what an empty `HYDRA_BIN`
/// is: it looks for `""` in every directory of `PATH`, and a directory that is there cannot
/// be run, which outranks one that is missing.
fn no_program(env: &Env) -> anyhow::Error {
    let path = env.get("PATH").map_or_else(
        || "/bin:/usr/bin".into(),
        |path| path.to_string_lossy().into_owned(),
    );
    let any = path.split(':').any(|dir| {
        let dir = if dir.is_empty() { "." } else { dir };
        Path::new(dir).is_dir()
    });
    if any {
        anyhow!("PermissionError: [Errno 13] Permission denied: ''")
    } else {
        anyhow!("FileNotFoundError: [Errno 2] No such file or directory: ''")
    }
}

/// `refresh_show_answer`: ask `hydra show <run_id> --events` about one record under `runs`
/// and return what a dataset row reads of the answer, which is all the cache keeps. The
/// first version cached the whole answer, every event of every record unredacted, which put
/// a raw copy of the corpus beside the redacted dataset (the security review of goal
/// ea69da1f).
///
/// `hydra` is told where the records are through `HYDRA_ROOT`: the directory above `runs`,
/// or, when `runs` has another name, a scratch directory holding a link called `runs`.
pub fn refresh_show_answer(run_id: &Value, runs: &Path, hydra: &str, env: &Env) -> Result<Value> {
    let resolved = std::fs::canonicalize(runs).map_err(|err| io_error(&err, runs))?;
    let mut root = resolved.parent().unwrap_or(Path::new("/")).to_path_buf();
    let mut scratch = None;
    if runs.file_name().is_none_or(|name| name != "runs") {
        let dir = TempDir::new(env)?;
        let link = dir.0.join("runs");
        std::os::unix::fs::symlink(&resolved, &link).map_err(|err| io_error(&err, &link))?;
        root = dir.0.clone();
        scratch = Some(dir);
    }
    let Value::Str(run_id) = run_id else {
        bail!(
            "TypeError: expected str, bytes or os.PathLike object, not {}",
            py::type_name(run_id)
        );
    };
    let Some(run_id) = run_id.to_utf8() else {
        bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in the run id");
    };
    if run_id.contains('\0') || hydra.contains('\0') {
        bail!("ValueError: embedded null byte");
    }
    if hydra.is_empty() {
        return Err(no_program(env));
    }
    let done = Command::new(hydra)
        .arg("show")
        .arg(&run_id)
        .arg("--events")
        .env_clear()
        .envs(env.pairs())
        .env("HYDRA_ROOT", &root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| anyhow!("{}: {}", os_error(&err), os_error_text(&err, hydra)))?;
    drop(scratch);
    if !done.status.success() {
        let argv: Vec<Value> = [hydra, "show", &run_id, "--events"]
            .iter()
            .map(|arg| Value::text(arg))
            .collect();
        let argv = py::repr(&Value::List(argv));
        let code = return_code(&done.status);
        if code >= 0 {
            bail!("CalledProcessError: Command '{argv}' returned non-zero exit status {code}.");
        }
        match signal_name(-code) {
            Some(name) => {
                bail!(
                    "CalledProcessError: Command '{argv}' died with <Signals.{name}: {}>.",
                    -code
                )
            }
            None => bail!(
                "CalledProcessError: Command '{argv}' died with unknown signal {}.",
                -code
            ),
        }
    }
    let Ok(stdout) = String::from_utf8(done.stdout) else {
        bail!("UnicodeDecodeError: 'utf-8' codec can't decode what hydra show printed");
    };
    let stdout = stdout.replace("\r\n", "\n").replace('\r', "\n");
    let full = match json::loads_explained(&PyStr::from(stdout)) {
        Ok(full) => full,
        Err((err, message)) => bail!("{}: {message}", err.class()),
    };
    let projection = or_empty(get(&full, "projection")?);
    let outcome = get(&projection, "outcome")?;
    let usage = or_empty(get(&projection, "usage")?);
    let counts = or_empty(get(&usage, "counts")?);
    // Only what a dataset row reads.
    let mut kept_usage = Dict::new();
    kept_usage.set("counts", counts);
    let mut kept = Dict::new();
    kept.set("outcome", outcome);
    kept.set("usage", Value::Dict(kept_usage));
    let mut answer = Dict::new();
    answer.set("projection", Value::Dict(kept));
    Ok(Value::Dict(answer))
}

/// `dataset_row`: one line of the dataset. Puppets names the run: it wins. Otherwise a class
/// a model wrote fills the row, marked "llm".
pub fn dataset_row(
    raw: &Value,
    label: Option<&Value>,
    collected_at: &str,
    llm_label: Option<&Value>,
    preamble_text: Option<&PyStr>,
) -> Result<Value> {
    let task = get(raw, "task")?;
    let task = if py::truthy(&task) {
        task
    } else {
        Value::text("")
    };
    let Value::Str(task) = &task else {
        bail!(
            "AttributeError: '{}' object has no attribute 'startswith'",
            py::type_name(&task)
        );
    };
    let (stripped, has_preamble) = records::strip_preamble(task, preamble_text);
    let mut task_text = Value::Str(stripped);
    if let Some(label) = label {
        let text = get(label, "text")?;
        if py::truthy(&text) {
            task_text = text;
        }
    }
    let Value::Str(task_text) = &task_text else {
        bail!(
            "AttributeError: '{}' object has no attribute 'find'",
            py::type_name(&task_text)
        );
    };
    let llm = if label.is_some() { None } else { llm_label };
    let of_llm = |key: &str| -> Result<Value> {
        match llm {
            Some(llm) => get(llm, key),
            None => Ok(Value::Null),
        }
    };
    let either = |ours: &str, theirs: &str| -> Result<Value> {
        match label {
            Some(label) => get(label, ours),
            None => of_llm(theirs),
        }
    };

    let mut row = Dict::new();
    row.set("run_id", py::item(raw, "run_id")?.clone());
    row.set("provider", py::item(raw, "provider")?.clone());
    row.set("model", py::item(raw, "canonical_model")?.clone());
    row.set("recorded_model", py::item(raw, "recorded_model")?.clone());
    row.set("policy", py::item(raw, "policy")?.clone());
    row.set("task_text", Value::Str(redact::redact_string(task_text)));
    row.set("preamble", Value::Bool(has_preamble));
    row.set("task_class", either("class", "task_class")?);
    row.set("grade", either("grade", "grade")?);
    row.set("area", of_llm("area")?);
    row.set("kind", of_llm("kind")?);
    row.set("outcome", py::item(raw, "outcome")?.clone());
    row.set(
        "human_verdict",
        match label {
            Some(label) => get(label, "human_verdict")?,
            None => Value::Null,
        },
    );
    row.set("tokens", py::item(raw, "tokens")?.clone());
    row.set("duration_s", py::item(raw, "duration_seconds")?.clone());
    row.set(
        "labels_source",
        match (label, llm) {
            (Some(_), _) => Value::text("puppets"),
            (None, Some(_)) => Value::text("llm"),
            (None, None) => Value::Null,
        },
    );
    row.set("collected_at", Value::text(collected_at));
    if let Some(llm) = llm
        && py::truthy(&get(llm, "unsure")?)
    {
        row.set("label_unsure", Value::Bool(true));
    }
    Ok(Value::Dict(row))
}

/// The `(run_id, model_version)` pairs `scores.jsonl` already holds.
#[derive(Default)]
struct Scored {
    strings: HashSet<(Vec<u32>, Vec<u32>)>,
    /// The pairs with a member that is not a string.
    others: Vec<(Value, Value)>,
}

impl Scored {
    fn add(&mut self, run_id: &Value, version: &Value) -> Result<()> {
        py::hashable(run_id)?;
        py::hashable(version)?;
        match (run_id, version) {
            (Value::Str(a), Value::Str(b)) => {
                self.strings
                    .insert((a.code_points().to_vec(), b.code_points().to_vec()));
            }
            _ => self.others.push((run_id.clone(), version.clone())),
        }
        Ok(())
    }

    fn holds(&self, run_id: &Value, version: &Value) -> Result<bool> {
        py::hashable(run_id)?;
        py::hashable(version)?;
        Ok(match (run_id, version) {
            (Value::Str(a), Value::Str(b)) => self
                .strings
                .contains(&(a.code_points().to_vec(), b.code_points().to_vec())),
            _ => self
                .others
                .iter()
                .any(|(a, b)| py::same_key(a, run_id) && py::same_key(b, version)),
        })
    }
}

/// `score_rows`: score each row with every model under `home/models/` and append one line
/// per run and model version to `home/scores.jsonl`. `hydra-puppets queue` and `board` read
/// the verdict scores when ordering reviews. The ranking does not, so the model still does
/// not choose which agent is assigned (build 143, D6). Returns how many lines were appended.
pub fn score_rows(home: &Path, rows: &[Value], scored_at: &str) -> Result<usize> {
    let paths = glob(&home.join("models"), "", ".json");
    if paths.is_empty() || rows.is_empty() {
        return Ok(0);
    }
    let scores_path = home.join("scores.jsonl");
    let mut done = Scored::default();
    for line in records::read_jsonl(&scores_path)? {
        done.add(
            py::item(&line, "run_id")?,
            py::item(&line, "model_version")?,
        )?;
    }
    let mut fresh = Vec::new();
    for path in paths {
        let model = records::read_json(&path)?.unwrap_or(Value::Null);
        for row in rows {
            let run_id = py::item(row, "run_id")?;
            let version = py::item(&model, "version")?;
            if done.holds(run_id, version)? {
                continue;
            }
            let mut score = Dict::new();
            score.set("run_id", run_id.clone());
            score.set("label", py::item(&model, "label")?.clone());
            score.set("p", Value::Float(baseline::predict(&model, row)?));
            score.set("model_version", version.clone());
            score.set("scored_at", Value::text(scored_at));
            fresh.push(Value::Dict(score));
        }
    }
    if !fresh.is_empty() {
        // The file is opened before the first line is made, so a score that cannot be
        // written leaves it there, empty if it was not there before.
        let mut handle = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&scores_path)
            .map_err(|err| io_error(&err, &scores_path))?;
        let mut text = String::new();
        for score in &fresh {
            if let Some(x) = out_of_range(score) {
                bail!(
                    "ValueError: Out of range float values are not JSON compliant: {}",
                    json::float_repr(x)
                );
            }
            text.push_str(&json::dumps_default(score));
            text.push('\n');
        }
        handle
            .write_all(text.as_bytes())
            .map_err(|err| io_error(&err, &scores_path))?;
    }
    Ok(fresh.len())
}

/// What one pass is told. `None` is the Python's default: the runs under the root, the
/// grader home, and `HYDRA_BIN` or `hydra`.
#[derive(Clone, Debug, Default)]
pub struct Pass {
    pub runs: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub hydra: Option<String>,
    /// How many `hydra show` run at once. Less than one is one.
    pub jobs: usize,
    /// Drop the cursor and the dataset, and write every row again.
    pub rebuild: bool,
}

/// `item["run_id"] in ticket_verdicts`, and then `ticket_verdicts[item["run_id"]]`, for a
/// file that holds whatever somebody left in it.
fn ticket_verdict(verdicts: &Value, run_id: &Value) -> Result<Option<Value>> {
    let inside = match verdicts {
        Value::Dict(_) => return lookup(verdicts, run_id),
        Value::List(items) => items.iter().any(|item| py::same_key(item, run_id)),
        Value::Str(text) => match run_id {
            Value::Str(run_id) => {
                run_id.is_empty()
                    || text
                        .code_points()
                        .windows(run_id.len())
                        .any(|window| window == run_id.code_points())
            }
            other => bail!(
                "TypeError: 'in <string>' requires string as left operand, not {}",
                py::type_name(other)
            ),
        },
        other => bail!(
            "TypeError: argument of type '{}' is not a container or iterable",
            py::type_name(other)
        ),
    };
    if inside {
        bail!(
            "TypeError: {} indices must be integers, not str",
            py::type_name(verdicts)
        );
    }
    Ok(None)
}

/// Whether `hydra show` has to be asked about `item`: not when the cache holds an answer
/// for these bytes, which is the case when only the label changed.
fn cached(cache: &Value, item: &Scanned) -> Result<bool> {
    let entry = lookup(cache, &item.run_id)?.unwrap_or_else(empty);
    Ok(py::eq_str(&get(&entry, "fingerprint")?, &item.fingerprint))
}

fn remember(cache: &mut Value, item: &Scanned, answer: Value) {
    let (Value::Dict(cache), Value::Str(run_id)) = (cache, &item.run_id) else {
        unreachable!("an answer comes from a cache that is a dict and a run id that is text");
    };
    let mut entry = Dict::new();
    entry.set("fingerprint", Value::text(&item.fingerprint));
    entry.set("answer", answer);
    cache.set_key(run_id.clone(), Value::Dict(entry));
}

/// The answers for `stale`, asked `workers` at a time. Every record is asked about even
/// when one fails, as the Python's pool goes on; the first failure in the order given is
/// the one returned, and the cache fills in that order too. The Python's fills in the
/// order the answers arrive, which is why the differential test runs with one worker.
fn refresh_together(
    stale: &[&Scanned],
    workers: usize,
    runs: &Path,
    hydra: &str,
    cache: &mut Value,
    env: &Env,
) -> Result<()> {
    let needed: Vec<Result<bool>> = stale
        .iter()
        .map(|item| cached(cache, item).map(|hit| !hit))
        .collect();
    let asked: Vec<usize> = (0..stale.len())
        .filter(|at| matches!(needed[*at], Ok(true)))
        .collect();
    let answers: Mutex<Vec<Option<Result<Value>>>> =
        Mutex::new((0..stale.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..workers.min(asked.len()) {
            let work = || {
                while let Some(at) = asked.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let answer = refresh_show_answer(&stale[*at].run_id, runs, hydra, env);
                    answers.lock().expect("no worker panics")[*at] = Some(answer);
                }
            };
            // A worker that cannot be started leaves its share to the others, and with
            // none started the answers below are missing, which is an error there.
            let _ = std::thread::Builder::new()
                .stack_size(STACK_BYTES)
                .spawn_scoped(scope, work);
        }
    });
    let mut answers = answers.into_inner().expect("no worker panics");
    for (at, need) in needed.into_iter().enumerate() {
        if need? {
            let Some(answer) = answers[at].take() else {
                bail!("RuntimeError: can't start new thread");
            };
            let answer = answer?;
            remember(cache, stale[at], answer);
        }
    }
    Ok(())
}

/// `collect`: refresh the stale rows and return how many were written.
pub fn collect(
    pass: &Pass,
    env: &Env,
    aliases: &dyn Fn() -> Result<Vec<(Pair, Pair)>>,
    stderr: &mut dyn Write,
) -> Result<usize> {
    let lookup_env = |name: &str| env.get(name);
    let root = || records::hydra_root_in(&lookup_env);
    let runs = match &pass.runs {
        Some(runs) => runs.clone(),
        None => root()?.join("runs"),
    };
    let home = match &pass.home {
        Some(home) => home.clone(),
        None => records::grader_home_in(&lookup_env)?,
    };
    let hydra = match &pass.hydra {
        Some(hydra) if !hydra.is_empty() => hydra.clone(),
        _ => env.get("HYDRA_BIN").map_or_else(
            || "hydra".to_string(),
            |bin| bin.to_string_lossy().into_owned(),
        ),
    };
    let cursor_path = home.join("cursor.json");
    let dataset_path = home.join("dataset.jsonl");

    let mut cursor = empty();
    let mut dataset = Keyed::new();
    if !pass.rebuild {
        cursor = records::read_json(&cursor_path)?.unwrap_or_else(empty);
        for row in records::read_jsonl(&dataset_path)? {
            dataset.set(py::item(&row, "run_id")?.clone(), row)?;
        }
    }
    let puppets_labels = labels::collect(&labels::puppets_home(env)?, env, stderr)?;
    let llm_labels = labels::collect_llm(&home)?;

    // The record's fingerprint, plus a digest of the model-written label when one fills the
    // row, so a new or changed class file makes the run stale.
    let effective = |item: &Scanned| -> Result<String> {
        if puppets_labels.contains(&item.run_id)? {
            return Ok(item.fingerprint.clone());
        }
        Ok(match llm_labels.get(&item.run_id)? {
            Some(label) => format!("{}:llm-{}", item.fingerprint, label_digest(label)),
            None => item.fingerprint.clone(),
        })
    };

    let mut scanned = Vec::new();
    for path in glob(&runs, "", ".jsonl") {
        if let Some(item) = records::scan_record(&path, stderr)? {
            scanned.push(item);
        }
    }

    let mut stale = Vec::new();
    for item in &scanned {
        let is_stale = pass.rebuild || {
            let seen = lookup(&cursor, &item.run_id)?.unwrap_or(Value::Null);
            !py::eq_str(&seen, &effective(item)?)
        };
        if is_stale {
            stale.push(item);
        }
    }
    if stale.is_empty() {
        return Ok(0);
    }
    let mut stale_ids = Keyed::new();
    for item in &stale {
        stale_ids.set(item.run_id.clone(), Value::Null)?;
    }

    let aliases = aliases()?;
    let cache_path = home.join("show-cache.json");
    let mut cache = records::read_json(&cache_path)?.unwrap_or_else(empty);
    let workers = pass.jobs.max(1);
    if workers == 1 {
        for item in &stale {
            if cached(&cache, item)? {
                continue; // only the label changed
            }
            let answer = refresh_show_answer(&item.run_id, &runs, &hydra, env)?;
            remember(&mut cache, item, answer);
        }
    } else {
        refresh_together(&stale, workers, &runs, &hydra, &mut cache, env)?;
    }
    records::write_json(&cache_path, &cache)?;

    // Verdicts the human gave through `baseline.py label-import`, kept so a rewritten line
    // does not lose one.
    let ticket_verdicts =
        records::read_json(&home.join("ticket-verdicts.json"))?.unwrap_or_else(empty);
    let collected_at = utc_now();
    let preamble_text = records::current_preamble(&root()?)?;
    for item in &scanned {
        let (Value::Dict(cursor), Value::Str(run_id)) = (&mut cursor, &item.run_id) else {
            bail!("TypeError: the cursor is not an object or a run id is not text");
        };
        cursor.set_key(run_id.clone(), Value::text(&effective(item)?));
        if pass.rebuild || stale_ids.contains(&item.run_id)? {
            let entry = match &cache {
                Value::Dict(cache) => cache.get_key(run_id),
                _ => None,
            };
            let Some(entry) = entry else {
                bail!("KeyError: {}", py::repr(&item.run_id));
            };
            let answer = py::item(entry, "answer")?;
            let raw = records::record_row(&item.first, &item.lines, answer, &aliases)?;
            let mut row = dataset_row(
                &raw,
                puppets_labels.get(&item.run_id)?,
                &collected_at,
                llm_labels.get(&item.run_id)?,
                preamble_text.as_ref(),
            )?;
            if let Some(verdict) = ticket_verdict(&ticket_verdicts, &item.run_id)?
                && let Value::Dict(row) = &mut row
            {
                row.set("human_verdict", verdict);
            }
            dataset.set(item.run_id.clone(), row)?;
        }
    }

    records::write_jsonl(&dataset_path, &dataset.sorted_values()?)?;
    records::write_json(&cursor_path, &cursor)?;
    let mut rewritten = Keyed::new();
    for (run_id, _) in stale_ids.iter() {
        let row = dataset.get(run_id)?.expect("every stale run has a row");
        rewritten.set(run_id.clone(), row.clone())?;
    }
    score_rows(&home, &rewritten.sorted_values()?, &collected_at)?;
    Ok(stale.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loads(text: &str) -> Value {
        json::loads_str(text).unwrap()
    }

    #[test]
    fn now_is_a_second_of_utc_that_ends_in_z() {
        let now = utc_now();
        assert_eq!(now.len(), 20, "{now}");
        assert!(now.ends_with('Z') && now.as_bytes()[10] == b'T', "{now}");
        assert!(now.as_str() > "2026-01-01T00:00:00Z", "{now}");
    }

    #[test]
    fn a_label_digest_does_not_depend_on_the_order_of_its_keys() {
        let a = loads(r#"{"run_id": "r", "task_class": "build", "deep": {"b": 1, "a": [2.5]}}"#);
        let b = loads(r#"{"deep": {"a": [2.5], "b": 1}, "task_class": "build", "run_id": "r"}"#);
        assert_eq!(label_digest(&a), label_digest(&b));
        // sha256 of `{"a": 1}`, which is what `json.dumps(..., sort_keys=True)` prints.
        assert_eq!(label_digest(&loads(r#"{"a": 1}"#)), "f9d86028c6e0d64e");
    }

    #[test]
    fn a_verdict_file_of_any_shape_is_read_as_python_reads_it() {
        let id = Value::text("r1");
        let found = ticket_verdict(&loads(r#"{"r1": "fixed"}"#), &id).unwrap();
        assert!(py::eq_str(&found.unwrap(), "fixed"));
        assert!(
            ticket_verdict(&loads(r#"{"r2": "fixed"}"#), &id)
                .unwrap()
                .is_none()
        );
        assert!(ticket_verdict(&loads(r#"["r2"]"#), &id).unwrap().is_none());
        assert!(ticket_verdict(&loads(r#""xx""#), &id).unwrap().is_none());
        for held in [r#"["r1"]"#, r#""an r1 inside""#, "null", "7"] {
            let err = ticket_verdict(&loads(held), &id).unwrap_err();
            assert!(err.to_string().starts_with("TypeError"), "{held}: {err}");
        }
    }
}
