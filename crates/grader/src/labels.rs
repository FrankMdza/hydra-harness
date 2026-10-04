//! `labels.py`: which run a Puppets goal log names, and what it says of it.
//!
//! A goal log is folded by `hydra-puppets show -- <goal>` and by nothing here (map 93). This
//! module runs that helper by an absolute path that never comes from `PATH`, checks the
//! shape of what it printed, and turns the state and the log's events into one label per
//! run id. Anything the helper cannot be asked, or answers wrongly, is a `LabelsError`, and
//! the pass that meets one writes nothing.
//!
//! The environment is handed in as an `Env` and never read from the process, so a test
//! names its own.
//!
//! How `pathlib` spells a path is here and in `baseline_cli.rs`, copied in build ticket 214.
//! Where `aliases.json` is found is `data.rs`, and the reading of `argparse` is
//! `argparse.rs`, one copy of each since build ticket 220.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::file::{os_error, read_text};
use hydra_pyjson::json::{self, Dict, ParseError, PyStr, Value};
use hydra_pyjson::py;

use crate::records;

/// `LabelsError`: `hydra-puppets show` could not be run, failed, or printed something this
/// module does not know how to read.
#[derive(Debug)]
pub struct LabelsError(pub String);

impl std::fmt::Display for LabelsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LabelsError {}

/// `os.environ`, as a command was started with it or as a test wrote it.
#[derive(Clone, Debug, Default)]
pub struct Env(Vec<(OsString, OsString)>);

impl Env {
    /// The environment of this process.
    pub fn process() -> Env {
        Env(std::env::vars_os().collect())
    }

    pub fn from_pairs<K: Into<OsString>, V: Into<OsString>>(
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Env {
        Env(pairs
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect())
    }

    /// `os.environ.get(name)`.
    pub fn get(&self, name: &str) -> Option<OsString> {
        self.0
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    /// `os.environ.get(name)` where the Python goes on only with a value that is not empty.
    pub fn named(&self, name: &str) -> Option<OsString> {
        self.get(name).filter(|value| !value.is_empty())
    }

    /// Every pair, which is what a child is started with.
    pub fn pairs(&self) -> impl Iterator<Item = (&OsString, &OsString)> {
        self.0.iter().map(|(key, value)| (key, value))
    }
}

/// `str(Path(text))`: no `.` component, no doubled or trailing slash, and `.` for nothing.
pub(crate) fn shown(path: &Path) -> String {
    let clean: PathBuf = path
        .components()
        .filter(|part| !matches!(part, std::path::Component::CurDir))
        .collect();
    if clean.as_os_str().is_empty() {
        ".".to_string()
    } else {
        clean.display().to_string()
    }
}

/// `sorted(dir.glob(prefix + "*" + suffix))`: the entries of `dir` with such a name, hidden
/// ones included, in the order of their names. No directory is no entries.
pub(crate) fn glob(dir: &Path, prefix: &str, suffix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<OsString> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name()))
        .filter(|name| {
            let name = name.as_bytes();
            name.len() >= prefix.len() + suffix.len()
                && name.starts_with(prefix.as_bytes())
                && name.ends_with(suffix.as_bytes())
        })
        .collect();
    names.sort();
    names.into_iter().map(|name| dir.join(name)).collect()
}

/// A Python dict whose keys are whatever JSON held: insertion order, a key set again keeps
/// its place, and `3`, `3.0` and `True`-like numbers that compare equal are one key.
#[derive(Clone, Debug, Default)]
pub struct Keyed {
    pairs: Vec<(Value, Value)>,
    /// Where each string key is.
    strings: HashMap<Vec<u32>, usize>,
    /// Where the keys that are not strings are.
    others: Vec<usize>,
}

impl Keyed {
    pub fn new() -> Keyed {
        Keyed::default()
    }

    fn position(&self, key: &Value) -> Result<Option<usize>> {
        py::hashable(key)?;
        Ok(match key {
            Value::Str(text) => self.strings.get(text.code_points()).copied(),
            other => self
                .others
                .iter()
                .copied()
                .find(|at| py::same_key(&self.pairs[*at].0, other)),
        })
    }

    /// `d.get(key)`. An error for a key Python cannot hash.
    pub fn get(&self, key: &Value) -> Result<Option<&Value>> {
        Ok(self.position(key)?.map(|at| &self.pairs[at].1))
    }

    /// `key in d`.
    pub fn contains(&self, key: &Value) -> Result<bool> {
        Ok(self.position(key)?.is_some())
    }

    /// `d[key] = value`.
    pub fn set(&mut self, key: Value, value: Value) -> Result<()> {
        match self.position(&key)? {
            Some(at) => self.pairs[at].1 = value,
            None => {
                match &key {
                    Value::Str(text) => {
                        self.strings
                            .insert(text.code_points().to_vec(), self.pairs.len());
                    }
                    _ => self.others.push(self.pairs.len()),
                }
                self.pairs.push((key, value));
            }
        }
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.pairs.iter().map(|(key, value)| (key, value))
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// `[d[key] for key in sorted(d)]`: an error where Python cannot order two keys.
    pub fn sorted_values(&self) -> Result<Vec<Value>> {
        let order = hydra_pyjson::pysort::sorted(self.pairs.len(), |i, j| {
            py::less(&self.pairs[i].0, &self.pairs[j].0)
        })?;
        Ok(order
            .into_iter()
            .map(|at| self.pairs[at].1.clone())
            .collect())
    }
}

/// `puppets_home()`: `HYDRA_PUPPETS_HOME`, else `puppets` under the root.
pub fn puppets_home(env: &Env) -> Result<PathBuf> {
    if let Some(named) = env.named("HYDRA_PUPPETS_HOME") {
        return Ok(PathBuf::from(named));
    }
    Ok(records::hydra_root_in(&|name| env.get(name))?.join("puppets"))
}

/// The checkout `exe` was built in: the first directory above it that holds
/// `integrations/puppets`, which is the checkout for a binary under its `target/`. The
/// Python used the checkout its own file was in.
fn checkout(exe: Option<&Path>) -> Option<PathBuf> {
    exe?.ancestors()
        .find(|dir| dir.join("integrations/puppets").is_dir())
        .map(Path::to_path_buf)
}

/// `helper_bin()`: the `hydra-puppets` that folds a goal log. Always an absolute path and
/// never a search of `PATH`: `HYDRA_PUPPETS_BIN`, else the one beside `HYDRA_BIN` when that
/// is absolute, else the release build of the checkout this binary is in. The first that is
/// configured is the answer, so a missing binary is an error and not a reason to try the
/// next.
///
/// A binary installed outside a checkout, with neither variable naming an absolute path,
/// has no helper, where the Python found its checkout's. The error names both variables
/// (build ticket 220), and the launchd job sets `HYDRA_BIN` to an absolute path.
pub fn helper_bin(env: &Env) -> Result<PathBuf> {
    helper_bin_from(env, std::env::current_exe().ok().as_deref())
}

fn helper_bin_from(env: &Env, exe: Option<&Path>) -> Result<PathBuf> {
    if let Some(named) = env.named("HYDRA_PUPPETS_BIN") {
        let path = PathBuf::from(&named);
        if !path.is_absolute() {
            return Err(LabelsError(format!(
                "HYDRA_PUPPETS_BIN is not an absolute path: {}",
                named.to_string_lossy()
            ))
            .into());
        }
        return Ok(PathBuf::from(shown(&path)));
    }
    if let Some(hydra) = env.named("HYDRA_BIN").map(PathBuf::from)
        && hydra.is_absolute()
    {
        let hydra = PathBuf::from(shown(&hydra));
        let beside = hydra.parent().unwrap_or(Path::new("/"));
        return Ok(beside.join("hydra-puppets"));
    }
    match checkout(exe) {
        Some(checkout) => Ok(checkout.join("target/release/hydra-puppets")),
        None => Err(LabelsError(
            "no checkout above this binary to find hydra-puppets in; set HYDRA_PUPPETS_BIN to \
             its absolute path, or HYDRA_BIN to the absolute path of the hydra it sits beside"
                .to_string(),
        )
        .into()),
    }
}

/// `str(err)` of the `OSError` Python raises when `path` cannot be started.
pub(crate) fn os_error_text(err: &std::io::Error, path: &str) -> String {
    let said = err.to_string();
    let said = match said.rfind(" (os error ") {
        Some(at) => &said[..at],
        None => &said,
    };
    match err.raw_os_error() {
        Some(errno) => format!("[Errno {errno}] {said}: {}", py::repr(&Value::text(path))),
        None => said.to_string(),
    }
}

/// What a child's `returncode` is: its exit code, or minus the signal that ended it.
pub(crate) fn return_code(status: &std::process::ExitStatus) -> i32 {
    status
        .code()
        .unwrap_or_else(|| -status.signal().unwrap_or(0))
}

/// What `show_goal` returns: the state, and every task `show` printed, in order.
#[derive(Debug)]
pub struct Shown {
    /// The state as a dict, where a key written twice holds what was written last.
    pub state: Value,
    /// Each pair of `tasks`. `show` prints its tasks under `str(id)`, so the ids `3` and
    /// `"3"` are one key twice and a plain dict keeps one.
    pub tasks: Vec<Value>,
}

/// The dict Python builds from the pairs of an object, all the way down.
fn as_dict(raw: &Value) -> Value {
    match raw {
        Value::Dict(pairs) => {
            let mut out = Dict::new();
            for (key, value) in pairs.iter() {
                out.set_key(key.clone(), as_dict(value));
            }
            Value::Dict(out)
        }
        Value::List(items) => Value::List(items.iter().map(as_dict).collect()),
        other => other.clone(),
    }
}

const TASK_FIELDS: [&str; 6] = ["id", "class", "grade", "text", "run_id", "human_result"];

fn has(value: &Value, key: &str) -> bool {
    matches!(value, Value::Dict(dict) if dict.get(key).is_some())
}

/// `show_goal`: what `hydra-puppets show <goal>` prints for the log under `home`, with the
/// shape `labels_for_goal` reads checked. A `LabelsError` when the binary is missing, exits
/// non-zero (a damaged log does), or prints anything else.
pub fn show_goal(goal_id: &OsStr, home: &Path, env: &Env) -> Result<Shown> {
    let binary = helper_bin(env)?;
    let binary_text = binary.display().to_string();
    let goal_text = goal_id.to_string_lossy().into_owned();
    let done = Command::new(&binary)
        .arg("show")
        .arg("--")
        .arg(goal_id)
        .env_clear()
        .envs(env.pairs())
        .env("HYDRA_PUPPETS_HOME", shown(home))
        .stdin(Stdio::null())
        .output()
        .map_err(|err| {
            LabelsError(format!(
                "cannot run {binary_text}: {}",
                os_error_text(&err, &binary_text)
            ))
        })?;
    if !done.status.success() {
        let said = String::from_utf8_lossy(&done.stderr);
        return Err(LabelsError(format!(
            "{binary_text} show {goal_text} exited {}: {}",
            return_code(&done.status),
            py::strip_str(&said)
        ))
        .into());
    }
    let raw = match json::loads_bytes_pairs_explained(&done.stdout) {
        Ok(raw) => raw,
        // Not a `ValueError`, so the Python lets it through.
        Err((ParseError::Recursion, _)) => {
            bail!("RecursionError: maximum recursion depth exceeded")
        }
        Err((_, message)) => {
            return Err(LabelsError(format!(
                "{binary_text} show {goal_text} did not print JSON: {message}"
            ))
            .into());
        }
    };
    let refuse = |what: String| -> anyhow::Error {
        LabelsError(format!("{binary_text} show {goal_text}: {what}")).into()
    };

    let state = as_dict(&raw);
    let (Value::Dict(dict), Value::Dict(raw)) = (&state, &raw) else {
        return Err(refuse("the state is not an object".into()));
    };
    for key in ["goal", "tasks", "plans"] {
        if dict.get(key).is_none() {
            return Err(refuse(format!("no `{key}`")));
        }
    }
    // `show` prints the empty state, exit 0, for a goal with no log. An answer about any
    // goal but the one asked for is not read as this one's.
    let named = dict.get("goal_id").cloned().unwrap_or(Value::Null);
    if !goal_id
        .to_str()
        .is_some_and(|goal| py::eq_str(&named, goal))
    {
        return Err(refuse(format!(
            "`goal_id` is {}, not the goal asked for",
            py::repr(&named)
        )));
    }
    let goal = dict.get("goal").expect("checked above");
    if !matches!(goal, Value::Null) && !has(goal, "text") {
        return Err(refuse(
            "`goal` is neither null nor an object with `text`".into(),
        ));
    }
    // The pairs of the `tasks` a dict would keep, which is the one written last.
    let tasks = raw
        .iter()
        .filter(|(key, _)| *key == "tasks")
        .map(|(_, value)| value)
        .last()
        .expect("checked above");
    let Value::Dict(tasks) = tasks else {
        return Err(refuse("`tasks` is not an object".into()));
    };
    let tasks: Vec<Value> = tasks.iter().map(|(_, task)| as_dict(task)).collect();
    if tasks
        .iter()
        .any(|task| TASK_FIELDS.iter().any(|field| !has(task, field)))
    {
        return Err(refuse(format!(
            "a task lacks one of {}",
            TASK_FIELDS.join(", ")
        )));
    }
    let Value::List(plans) = dict.get("plans").expect("checked above") else {
        return Err(refuse("`plans` is not a list".into()));
    };
    if plans.iter().any(|plan| !has(plan, "run_id")) {
        return Err(refuse("a plan has no `run_id`".into()));
    }
    Ok(Shown { state, tasks })
}

/// `committed_events`: every newline-terminated line of a goal log, by the helper's rule:
/// only `\n` ends a line and an unterminated tail is not an event. Call it only after
/// `show_goal` read the same log, which is what refuses a blank line or one that is not
/// JSON.
pub fn committed_events(path: &Path) -> Result<Vec<Value>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let data =
        std::fs::read(path).map_err(|err| anyhow!("{}: {}", os_error(&err), path.display()))?;
    let mut lines: Vec<&[u8]> = data.split(|byte| *byte == b'\n').collect();
    lines.pop();
    let mut events = Vec::new();
    for line in lines {
        // Each line goes to the parser with its newline, as the helper hands it over: a
        // line in UTF-16 ends in the two bytes of one.
        let mut whole = line.to_vec();
        whole.push(b'\n');
        match json::loads_bytes_explained(&whole) {
            Ok(event) => events.push(event),
            Err((err, message)) => bail!("{}: {message}", err.class()),
        }
    }
    Ok(events)
}

fn label(task_id: Value, class: Value, grade: Value, verdict: Value, text: Value) -> Value {
    let mut out = Dict::new();
    out.set("task_id", task_id);
    out.set("class", class);
    out.set("grade", grade);
    out.set("human_verdict", verdict);
    out.set("text", text);
    Value::Dict(out)
}

/// `for x in value`, for what a log line can hold: the items of a list, the characters of a
/// string, the keys of an object.
fn iterate(value: &Value) -> Result<Vec<Value>> {
    Ok(match value {
        Value::List(items) => items.clone(),
        Value::Str(text) => text
            .code_points()
            .iter()
            .map(|code| {
                let mut one = PyStr::new();
                one.push_code(*code);
                Value::Str(one)
            })
            .collect(),
        Value::Dict(dict) => dict
            .iter()
            .map(|(key, _)| Value::Str(key.clone()))
            .collect(),
        other => bail!(
            "TypeError: '{}' object is not iterable",
            py::type_name(other)
        ),
    })
}

/// `labels_for_goal`: run id to label fields, from the state `show_goal` returned and the
/// log's events.
pub fn labels_for_goal(shown: &Shown, events: &[Value]) -> Result<Keyed> {
    let goal = py::item(&shown.state, "goal")?;
    let goal_text = match goal {
        Value::Null => Value::text(""),
        goal => py::item(goal, "text")?.clone(),
    };
    let mut labels = Keyed::new();
    // As the fold keeps them: by id, where `3` and `3.0` are one task and the later wins.
    let mut tasks = Keyed::new();
    for task in &shown.tasks {
        tasks.set(py::item(task, "id")?.clone(), task.clone())?;
    }

    for (_, task) in tasks.iter() {
        let run_id = py::item(task, "run_id")?;
        if !py::truthy(run_id) {
            continue;
        }
        labels.set(
            run_id.clone(),
            label(
                py::item(task, "id")?.clone(),
                py::item(task, "class")?.clone(),
                py::item(task, "grade")?.clone(),
                py::item(task, "human_result")?.clone(),
                py::item(task, "text")?.clone(),
            ),
        )?;
    }

    let role = |class: &str| {
        label(
            Value::Null,
            Value::text(class),
            Value::Null,
            Value::Null,
            goal_text.clone(),
        )
    };
    for plan in py::list(py::item(&shown.state, "plans")?)? {
        labels.set(py::item(&plan, "run_id")?.clone(), role("plan_proposer"))?;
    }

    let nothing = || Value::List(Vec::new());
    for event in events {
        let name = py::get(event, "event", Value::Null)?;
        if py::eq_str(&name, "panel_verdict") {
            for run_id in iterate(&py::get(event, "judge_run_ids", nothing())?)? {
                labels.set(run_id, role("plan_judge"))?;
            }
        } else if py::eq_str(&name, "review") {
            let id = py::get(event, "task", Value::Null)?;
            let task = tasks.get(&id)?;
            let of = |field: &str, absent: Value| -> Result<Value> {
                match task {
                    Some(task) => Ok(py::item(task, field)?.clone()),
                    None => Ok(absent),
                }
            };
            for run_id in iterate(&py::get(event, "judge_run_ids", nothing())?)? {
                labels.set(
                    run_id,
                    label(
                        id.clone(),
                        of("class", Value::Null)?,
                        of("grade", Value::Null)?,
                        Value::Null,
                        of("text", Value::text(""))?,
                    ),
                )?;
            }
        }
    }
    Ok(labels)
}

/// `collect`: ask `hydra-puppets show` for every goal log under `home` and return run id to
/// label fields. One goal that cannot be shown is a `LabelsError`, and no label is
/// returned. A file named `.jsonl` alone has no goal id and is skipped with a line on
/// `stderr`. A name that starts with a dot is a goal like any other: the helper reads
/// `.real.jsonl` for the goal `.real`.
pub fn collect(home: &Path, env: &Env, stderr: &mut dyn Write) -> Result<Keyed> {
    let mut labels = Keyed::new();
    for path in glob(home, "", ".jsonl") {
        // The id plus `.jsonl` is the name of the file read below.
        let name = path.file_name().expect("a globbed name").as_bytes();
        let goal_id = OsStr::from_bytes(&name[..name.len() - ".jsonl".len()]);
        if goal_id.is_empty() {
            let _ = writeln!(
                stderr,
                "labels: skipping {}: its name is not a goal id",
                shown(&path)
            );
            continue;
        }
        let state = show_goal(goal_id, home, env)?;
        for (run_id, row) in labels_for_goal(&state, &committed_events(&path)?)?.iter() {
            labels.set(run_id.clone(), row.clone())?;
        }
    }
    Ok(labels)
}

/// `collect_llm`: every `task-classes-*.jsonl` under the grader home, oldest name first, so
/// a later file wins on the same run id. A model wrote these, not a human.
pub fn collect_llm(home: &Path) -> Result<Keyed> {
    let mut found = Keyed::new();
    for path in glob(home, "task-classes-", ".jsonl") {
        let text = read_text(&path)?;
        for line in py::splitlines(&text) {
            if py::strip_str(line).is_empty() {
                continue;
            }
            let item = match json::loads_explained(&PyStr::from(line)) {
                Ok(item) => item,
                Err((ParseError::Decode, _)) => continue,
                Err((err, message)) => bail!("{}: {message}", err.class()),
            };
            if !matches!(item, Value::Dict(_)) {
                continue;
            }
            let run_id = py::get(&item, "run_id", Value::Null)?;
            if py::truthy(&run_id) {
                found.set(run_id, item)?;
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::text(value)
    }

    #[test]
    fn a_keyed_dict_holds_one_key_for_numbers_that_are_equal() {
        let mut dict = Keyed::new();
        dict.set(Value::Int(3), text("a")).unwrap();
        dict.set(text("3"), text("b")).unwrap();
        dict.set(Value::Float(3.0), text("c")).unwrap();
        dict.set(Value::Bool(true), text("d")).unwrap();
        dict.set(Value::Int(1), text("e")).unwrap();
        let got: Vec<String> = dict
            .iter()
            .map(|(key, value)| format!("{}={}", py::repr(key), py::repr(value)))
            .collect();
        assert_eq!(got, ["3='c'", "'3'='b'", "True='e'"]);
        assert!(dict.contains(&Value::Float(1.0)).unwrap());
        assert!(!dict.contains(&text("1")).unwrap());
        let err = dict.get(&Value::List(Vec::new())).unwrap_err();
        assert!(err.to_string().starts_with("TypeError"), "{err}");
    }

    /// Build 220: a binary installed outside the checkout has no helper unless one of two
    /// variables names it, and the error says which two.
    #[test]
    fn a_binary_outside_a_checkout_is_told_the_two_variables_that_name_the_helper() {
        let exe = Path::new("/opt/hydra/bin/hydra-grader");
        for env in [
            Env::default(),
            // A `HYDRA_BIN` that is not absolute names no directory to look in.
            Env::from_pairs([("HYDRA_BIN", "hydra")]),
        ] {
            let err = helper_bin_from(&env, Some(exe)).unwrap_err();
            let said = &err.downcast_ref::<LabelsError>().unwrap().0;
            for name in ["HYDRA_PUPPETS_BIN", "HYDRA_BIN to"] {
                assert!(said.contains(name), "{said}");
            }
        }
        let env = Env::from_pairs([("HYDRA_BIN", "/opt/hydra/bin/hydra")]);
        assert_eq!(
            helper_bin_from(&env, Some(exe)).unwrap(),
            Path::new("/opt/hydra/bin/hydra-puppets")
        );
    }

    #[test]
    fn keys_sort_as_python_sorts_them_or_not_at_all() {
        let mut dict = Keyed::new();
        for key in ["b", "a", "é", "B"] {
            dict.set(text(key), text(key)).unwrap();
        }
        let got: Vec<String> = dict
            .sorted_values()
            .unwrap()
            .iter()
            .map(|value| py::str(value).lossy())
            .collect();
        assert_eq!(got, ["B", "a", "b", "é"]);
        dict.set(Value::Int(1), text("one")).unwrap();
        let err = dict.sorted_values().unwrap_err();
        assert!(err.to_string().starts_with("TypeError"), "{err}");
    }

    #[test]
    fn text_splits_where_python_splits_it() {
        assert_eq!(
            py::splitlines("a\r\nb\rc\n\nd\x0be\u{2028}f\u{85}g\x1ch"),
            ["a", "b", "c", "", "d", "e", "f", "g", "h"]
        );
        assert_eq!(py::splitlines("a\n"), ["a"]);
        assert!(py::splitlines("").is_empty());
        assert_eq!(py::splitlines("a\x1fb\tc"), ["a\x1fb\tc"]);
    }

    #[test]
    fn the_helper_is_named_by_an_absolute_path_or_not_at_all() {
        let env = |pairs: &[(&str, &str)]| Env::from_pairs(pairs.iter().copied());
        let named = helper_bin(&env(&[("HYDRA_PUPPETS_BIN", "/a//b/./helper")])).unwrap();
        assert_eq!(named, Path::new("/a/b/helper"));
        let err = helper_bin(&env(&[("HYDRA_PUPPETS_BIN", "bin/helper")])).unwrap_err();
        assert_eq!(
            err.downcast_ref::<LabelsError>().unwrap().0,
            "HYDRA_PUPPETS_BIN is not an absolute path: bin/helper"
        );
        let beside = helper_bin(&env(&[("HYDRA_BIN", "/opt/bin/hydra")])).unwrap();
        assert_eq!(beside, Path::new("/opt/bin/hydra-puppets"));
        let root = helper_bin(&env(&[("HYDRA_BIN", "/")])).unwrap();
        assert_eq!(root, Path::new("/hydra-puppets"));
        // A relative `HYDRA_BIN` names nothing, and `PATH` is never searched.
        let built = helper_bin(&env(&[("HYDRA_BIN", "hydra"), ("PATH", "/usr/bin")])).unwrap();
        assert!(built.is_absolute(), "{}", built.display());
        assert!(built.ends_with("target/release/hydra-puppets"));
    }
}
