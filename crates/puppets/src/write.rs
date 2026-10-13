//! The commands that write a goal log and launch nothing: `new`, `set`, `approve`,
//! `verdict`, `choose`, `tasks`, `assign` and `reconcile`. Each appends the lines
//! the Python helper appended, prints what it printed, and returns its exit code or the
//! error Python would raise.
//!
//! `choose --judge` is the one path here that launches runs. It lives in `panel.rs`.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};

use crate::fold::{self, GoalState, approval_available};
use crate::json::{self, Dict, PyStr, Value};
use crate::log::{self, Home};
use crate::py;
use crate::read::{Piece, print, text};
use crate::sheet;

const TASK_CLASSES: [&str; 7] = [
    "batch_scripting",
    "rust_off_launch_path",
    "rust_launch_path",
    "security_review",
    "research",
    "docs",
    "probe",
];
const TASK_GRADES: [&str; 4] = ["low", "medium", "high", "agi"];

/// `DEFAULT_ASSIGNMENTS`: a task class, then the provider and model it goes to when no
/// ranking has a verdict behind it. The policy is `standard` for every row.
const DEFAULT_ASSIGNMENTS: [(&str, &str, &str); 7] = [
    ("batch_scripting", "cursor-agent", "cursor-grok-4.6-high"),
    ("docs", "cursor-agent", "cursor-grok-4.6-high"),
    (
        "rust_off_launch_path",
        "cursor-agent",
        "cursor-grok-4.6-high",
    ),
    ("rust_launch_path", "claude", "sonnet"),
    ("security_review", "codex", ""),
    ("probe", "agy", "gemini-3.1-pro-high"),
    ("research", "cursor-agent", "cursor-grok-4.6-high"),
];

/// A command-line argument as the Python string `sys.argv` holds: UTF-8, with each byte
/// that does not decode carried as a lone surrogate (`surrogateescape`). `json.dumps`
/// writes such a surrogate as `\udcXX`, so the log line is the same in both.
pub fn arg(raw: &OsStr) -> PyStr {
    let mut out = PyStr::new();
    for chunk in raw.as_bytes().utf8_chunks() {
        out.push_str(chunk.valid());
        for byte in chunk.invalid() {
            out.push_code(0xdc00 + u32::from(*byte));
        }
    }
    out
}

/// `os.fsencode(text)`: the bytes `arg` read, or `UnicodeEncodeError` on a surrogate that
/// did not come from one.
pub(crate) fn fs_path(text: &PyStr) -> Result<PathBuf> {
    let mut bytes = Vec::new();
    for &code in text.code_points() {
        match char::from_u32(code) {
            Some(c) => bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
            None if (0xdc80..=0xdcff).contains(&code) => bytes.push((code - 0xdc00) as u8),
            None => bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate"),
        }
    }
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

/// `print(line, file=sys.stderr)`. Python's stderr never raises on a surrogate: it writes
/// `\udXXX` (`backslashreplace`).
pub(crate) fn eprint(err: &mut dyn Write, line: &PyStr) -> Result<()> {
    let mut out = String::new();
    for &code in line.code_points() {
        match char::from_u32(code) {
            Some(c) => out.push(c),
            None => out.push_str(&format!("\\u{code:04x}")),
        }
    }
    writeln!(err, "{out}")?;
    Ok(())
}

pub(crate) fn body(pairs: Vec<(&str, Value)>) -> Dict {
    let mut out = Dict::new();
    for (key, value) in pairs {
        out.set(key, value);
    }
    out
}

/// `append_event`: one line, under the log's lock.
pub(crate) fn append_event(home: &Home, goal_id: &str, body: Dict) -> Result<()> {
    log::locked_update(home, goal_id, |_events| Ok(Some(body)))?;
    Ok(())
}

pub(crate) fn folded(home: &Home, goal_id: &str) -> Result<GoalState> {
    fold::fold(goal_id, &log::read_log(home, goal_id)?)
}

pub(crate) fn field(d: &Dict, key: &str) -> Value {
    d.get(key).cloned().unwrap_or(Value::Null)
}

/// `str(uuid.uuid4())`.
fn uuid4() -> Result<String> {
    use std::io::Read;
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut b))
        .context("reading /dev/urandom")?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

/// One `git rev-parse` from the working directory, stripped. `None` where the Python
/// catches the failure: git exits nonzero, or there is no git. Git's own stderr goes to
/// ours, as `subprocess.check_output` leaves it.
fn git_rev_parse(what: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["rev-parse", what])
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output();
    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(anyhow!("OSError: running git: {err}")),
    };
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|err| anyhow!("UnicodeDecodeError: git rev-parse {what}: {err}"))?;
    Ok(Some(py::strip_str(&text).to_string()))
}

/// `git_repo_and_sha`: the repo's top level and its HEAD, or the working directory and an
/// empty sha when either question fails.
fn git_repo_and_sha() -> Result<(PyStr, String)> {
    if let Some(repo) = git_rev_parse("--show-toplevel")?
        && let Some(sha) = git_rev_parse("HEAD")?
    {
        return Ok((PyStr::from(repo), sha));
    }
    let cwd = std::env::current_dir().map_err(|err| anyhow!("FileNotFoundError: {err}"))?;
    Ok((arg(cwd.as_os_str()), String::new()))
}

/// `new`: start a goal log with its `goal` event and print the goal id.
pub fn new(home: &Home, goal_text: &PyStr, out: &mut dyn Write) -> Result<i32> {
    let goal_id = uuid4()?;
    let (repo, base_sha) = git_repo_and_sha()?;
    append_event(
        home,
        &goal_id,
        body(vec![
            ("event", Value::text("goal")),
            ("text", Value::Str(goal_text.clone())),
            ("repo", Value::Str(repo)),
            ("base_sha", Value::text(&base_sha)),
        ]),
    )?;
    writeln!(out, "{goal_id}")?;
    Ok(0)
}

/// `set`: the human's assignment for one task. An empty model is left out of the event.
pub fn set(
    home: &Home,
    goal_id: &str,
    task: &PyStr,
    provider: &PyStr,
    model: &PyStr,
    policy: &PyStr,
) -> Result<i32> {
    let mut event = body(vec![
        ("event", Value::text("assignment")),
        ("task", Value::Str(task.clone())),
        ("provider", Value::Str(provider.clone())),
        ("policy", Value::Str(policy.clone())),
        ("source", Value::text("human")),
    ]);
    if !model.is_empty() {
        event.set("model", Value::Str(model.clone()));
    }
    append_event(home, goal_id, event)?;
    Ok(0)
}

/// What `approve` was asked for. An option given as the empty string counts as not given
/// wherever the Python tests its truth.
#[derive(Clone, Debug, Default)]
pub struct Approve {
    pub plan: bool,
    pub table: bool,
    pub launch: Option<PyStr>,
    pub diff: Option<PyStr>,
    pub degraded: Option<PyStr>,
    pub expect_revision: Option<Value>,
    pub expect_run: Option<PyStr>,
    pub expect_plan: Option<PyStr>,
}

/// The approval one `approve` would record from a state, or its refusal: the exit code
/// and the line for stderr.
enum Prepared {
    Refused(i32, PyStr),
    Body(Dict),
}

fn refused(code: i32, message: impl Piece) -> Result<Prepared> {
    Ok(Prepared::Refused(code, text!(message)))
}

fn given(option: &Option<PyStr>) -> Option<&PyStr> {
    option.as_ref().filter(|text| !text.is_empty())
}

pub(crate) fn slice(codes: &[u32]) -> PyStr {
    let mut out = PyStr::new();
    for code in codes {
        out.push_code(*code);
    }
    out
}

/// `parse_degraded_substitution`: `missing=substitute` or `missing=substitute:model`.
fn parse_degraded_substitution(spec: &PyStr) -> Result<(PyStr, PyStr, Option<PyStr>), String> {
    let codes = spec.code_points();
    let Some(eq) = codes.iter().position(|c| *c == u32::from('=')) else {
        return Err("expected missing=substitute-provider[:model]".into());
    };
    let missing = py::strip(&slice(&codes[..eq]));
    if missing.is_empty() {
        return Err("missing provider name required".into());
    }
    let rhs = &codes[eq + 1..];
    let (substitute, model) = match rhs.iter().position(|c| *c == u32::from(':')) {
        Some(colon) => {
            let model = py::strip(&slice(&rhs[colon + 1..]));
            (
                py::strip(&slice(&rhs[..colon])),
                (!model.is_empty()).then_some(model),
            )
        }
        None => (py::strip(&slice(rhs)), None),
    };
    if substitute.is_empty() || substitute.code_points()[0] == u32::from('<') {
        return Err(format!(
            "substitute provider required; replace {} with a provider",
            fold::DEGRADED_SUBSTITUTE_PLACEHOLDER
        ));
    }
    Ok((missing, substitute, model))
}

/// `_duplicate_refusal`: exit 3 when the log already holds this approval, same kind, ref
/// and run, at this revision. A consumed or voided one still counts.
fn duplicate(state: &GoalState, kind: &str, reference: &Value, run_id: &Value) -> Option<Prepared> {
    let revision = Value::Int(state.revision);
    let matches = |approval: &Dict| {
        if !py::truthy(&field(approval, "yes")) || !py::eq_str(&field(approval, "kind"), kind) {
            return false;
        }
        if !py::eq(&field(approval, "revision"), &revision) {
            return false;
        }
        let existing = field(approval, if kind == "degraded" { "missing" } else { "ref" });
        py::eq(&existing, reference) && py::eq(&field(approval, "run_id"), run_id)
    };
    state.approvals.iter().any(matches).then(|| {
        Prepared::Refused(
            3,
            PyStr::from(format!(
                "already approved {kind} at revision {}",
                state.revision
            )),
        )
    })
}

/// `prepare_approval`. Exit 2 is an `--expect-*` mismatch, 3 a duplicate, 1 any other
/// refusal. A degraded approval's ref is the missing provider; a diff's run is the task's
/// current run id.
fn prepare_approval(args: &Approve, state: &GoalState) -> Result<Prepared> {
    if let Some(expected) = &args.expect_revision
        && !py::eq(expected, &Value::Int(state.revision))
    {
        return refused(
            2,
            format!(
                "expected revision {}, current revision {}",
                py::repr(expected),
                state.revision
            ),
        );
    }
    let (launch, diff) = (given(&args.launch), given(&args.diff));
    if args.expect_run.is_some() && diff.is_none() {
        return refused(1, "use --expect-run with --diff");
    }
    if args.expect_plan.is_some() && !args.plan {
        return refused(1, "use --expect-plan with --plan");
    }
    if let Some(spec) = given(&args.degraded) {
        if args.plan || args.table || launch.is_some() || diff.is_some() {
            return refused(
                1,
                "use --degraded without --plan, --table, --launch or --diff",
            );
        }
        let (missing, substitute, model) = match parse_degraded_substitution(spec) {
            Ok(parsed) => parsed,
            Err(message) => return refused(1, message),
        };
        let missing = Value::Str(missing);
        if let Some(refusal) = duplicate(state, "degraded", &missing, &Value::Null) {
            return Ok(refusal);
        }
        let mut event = body(vec![
            ("event", Value::text("approval")),
            ("kind", Value::text("degraded")),
            ("missing", missing),
            ("substitute", Value::Str(substitute)),
            ("revision", Value::Int(state.revision)),
            ("yes", Value::Bool(true)),
        ]);
        if let Some(model) = model {
            event.set("model", Value::Str(model));
        }
        return Ok(Prepared::Body(event));
    }
    let none = || PyStr::from("none");
    let (kind, reference) = if args.plan {
        if let Some(expected) = &args.expect_plan {
            let current = state
                .chosen_plan
                .as_ref()
                .map_or(Value::Null, |chosen| field(chosen, "path"));
            if !py::eq(&current, &Value::Str(expected.clone())) {
                let shown = if py::truthy(&current) {
                    py::str(&current)
                } else {
                    none()
                };
                return Ok(Prepared::Refused(
                    2,
                    text!("expected plan ", expected.clone(), ", current plan ", shown),
                ));
            }
        }
        ("plan", Value::Null)
    } else if args.table {
        ("table", Value::Null)
    } else if let Some(task) = launch {
        ("launch", Value::Str(task.clone()))
    } else if let Some(task) = diff {
        let reference = Value::Str(task.clone());
        if let Some(expected) = &args.expect_run {
            let current = state
                .task(&reference)?
                .map_or(Value::Null, |task| task.run_id.clone());
            if !py::eq(&current, &Value::Str(expected.clone())) {
                let shown = if py::truthy(&current) {
                    py::str(&current)
                } else {
                    none()
                };
                return Ok(Prepared::Refused(
                    2,
                    text!("expected run ", expected.clone(), ", current run ", shown),
                ));
            }
        }
        ("diff", reference)
    } else {
        return refused(1, "specify --plan, --table, --launch, --diff or --degraded");
    };
    let mut run_id = Value::Null;
    if let Value::Str(name) = &reference {
        let Some(task) = state.task(&reference)? else {
            return Ok(Prepared::Refused(1, text!("unknown task ", name.clone())));
        };
        if kind == "diff" {
            if !py::truthy(&task.run_id) {
                return Ok(Prepared::Refused(
                    1,
                    text!("task ", name.clone(), " has no run to approve a diff for"),
                ));
            }
            run_id = task.run_id.clone();
        }
    }
    if let Some(refusal) = duplicate(state, kind, &reference, &run_id) {
        return Ok(refusal);
    }
    let mut event = body(vec![
        ("event", Value::text("approval")),
        ("kind", Value::text(kind)),
        ("ref", reference),
        ("revision", Value::Int(state.revision)),
        ("yes", Value::Bool(true)),
    ]);
    if !matches!(run_id, Value::Null) {
        event.set("run_id", run_id);
    }
    Ok(Prepared::Body(event))
}

/// `approve`: record one approval. The first fold can be stale, so every `--expect-*` is
/// checked again on the lines read under the log's lock, and the event carries that
/// fold's revision and run, not the ones the first read saw.
pub fn approve(home: &Home, goal_id: &str, args: &Approve, err: &mut dyn Write) -> Result<i32> {
    if let Prepared::Refused(code, message) = prepare_approval(args, &folded(home, goal_id)?)? {
        eprint(err, &message)?;
        return Ok(code);
    }
    let mut held = None;
    let written = log::locked_update(home, goal_id, |events| {
        match prepare_approval(args, &fold::fold(goal_id, events)?)? {
            Prepared::Refused(code, message) => {
                held = Some((code, message));
                Ok(None)
            }
            Prepared::Body(event) => Ok(Some(event)),
        }
    })?;
    if written.is_none() {
        let Some((code, message)) = held else {
            bail!("RuntimeError: approve wrote nothing for goal {goal_id}");
        };
        eprint(err, &message)?;
        return Ok(code);
    }
    Ok(0)
}

/// `verdict`: the human's word on a task's current run. `applied` and `fixed` need a live
/// diff approval for that run; `rejected` does not.
pub fn verdict(
    home: &Home,
    goal_id: &str,
    task_id: &PyStr,
    result: &str,
    note: &PyStr,
    err: &mut dyn Write,
) -> Result<i32> {
    let state = folded(home, goal_id)?;
    let reference = Value::Str(task_id.clone());
    let run_id = match state.task(&reference)? {
        Some(task) if py::truthy(&task.run_id) => task.run_id.clone(),
        _ => {
            eprint(err, &text!("task ", task_id.clone(), " has no run"))?;
            return Ok(1);
        }
    };
    if (result == "applied" || result == "fixed")
        && approval_available(&state, "diff", &reference, &run_id).is_none()
    {
        eprint(
            err,
            &text!(
                "missing diff approval for task ",
                task_id.clone(),
                " run ",
                py::str(&run_id),
                " at revision ",
                state.revision.to_string(),
                "; approve with: approve ",
                state.goal_id.as_str(),
                " --diff ",
                task_id.clone(),
            ),
        )?;
        return Ok(1);
    }
    append_event(
        home,
        goal_id,
        body(vec![
            ("event", Value::text("human_verdict")),
            ("task", reference),
            ("run_id", run_id),
            ("result", Value::text(result)),
            ("note", Value::Str(note.clone())),
        ]),
    )?;
    Ok(0)
}

/// `os.path.normpath` on an absolute or relative POSIX path.
fn normpath(path: &[u8]) -> Vec<u8> {
    if path.is_empty() {
        return b".".to_vec();
    }
    // POSIX keeps exactly two leading slashes apart from one, and from three or more.
    let slashes = match path {
        [b'/', b'/', b'/', ..] => 1,
        [b'/', b'/', ..] => 2,
        [b'/', ..] => 1,
        _ => 0,
    };
    let mut parts: Vec<&[u8]> = Vec::new();
    for part in path.split(|b| *b == b'/') {
        if part.is_empty() || part == b"." {
            continue;
        }
        if part != b".."
            || (slashes == 0 && parts.is_empty())
            || parts.last().is_some_and(|last| *last == b"..")
        {
            parts.push(part);
        } else {
            parts.pop();
        }
    }
    let mut out = vec![b'/'; slashes];
    out.extend_from_slice(&parts.join(&b'/'));
    if out.is_empty() {
        out.push(b'.');
    }
    out
}

/// `os.path.abspath`: joined to the working directory when relative, then `normpath`.
/// Nothing is resolved on disk.
fn abspath(path: &OsStr) -> Result<Vec<u8>> {
    let path = path.as_bytes();
    if path.starts_with(b"/") {
        return Ok(normpath(path));
    }
    let cwd = std::env::current_dir().map_err(|err| anyhow!("FileNotFoundError: {err}"))?;
    let mut joined = cwd.into_os_string().into_vec();
    if !joined.ends_with(b"/") {
        joined.push(b'/');
    }
    joined.extend_from_slice(path);
    Ok(normpath(&joined))
}

/// `Path(path).stem`: the last component without its last suffix. A name's leading dots
/// are not a suffix, and from Python 3.14 a trailing dot is one.
fn stem(path: &[u8]) -> &[u8] {
    let name = path.rsplit(|b| *b == b'/').next().unwrap_or(path);
    let Some(dot) = name.iter().rposition(|b| *b == b'.') else {
        return name;
    };
    if name[..dot].iter().all(|b| *b == b'.') {
        return name;
    }
    &name[..dot]
}

/// `choose`: record which plan was chosen. A choice that names a file is recorded by its
/// absolute path, with the file's stem as the name; anything else is a provider's name.
///
/// With `judge` the panel is launched and its answer recorded, and a `choice` is not read.
pub fn choose(
    home: &Home,
    goal_id: &str,
    choice: Option<&OsStr>,
    judge: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32> {
    if judge {
        return crate::panel::choose_judge(home, goal_id, out, err);
    }
    let Some(choice) = choice.filter(|choice| !choice.is_empty()) else {
        writeln!(err, "specify a provider, a plan path, or --judge")?;
        return Ok(1);
    };
    let (chosen, path) = if std::path::Path::new(choice).is_file() {
        let path = abspath(choice)?;
        (
            arg(OsStr::from_bytes(stem(&path))),
            Value::Str(arg(OsStr::from_bytes(&path))),
        )
    } else {
        (arg(choice), Value::Null)
    };
    append_event(
        home,
        goal_id,
        body(vec![
            ("event", Value::text("panel_verdict")),
            ("judge_run_ids", Value::List(Vec::new())),
            ("chosen", Value::Str(chosen)),
            ("path", path),
        ]),
    )?;
    Ok(0)
}

/// The match of `` ```(?:json)?\s*(\[[\s\S]*?\])\s*``` ``: the first fence, from the left,
/// that opens on a `[`, and the shortest run from that `[` to a `]` with the closing fence
/// after it.
pub(crate) fn fenced_array(chars: &[char]) -> Option<std::ops::RangeInclusive<usize>> {
    const FENCE: [char; 3] = ['`'; 3];
    let spaces = |mut at: usize| {
        while chars.get(at).is_some_and(|c| py::is_space(*c)) {
            at += 1;
        }
        at
    };
    for start in 0..chars.len() {
        if !chars[start..].starts_with(&FENCE) {
            continue;
        }
        let mut open = start + FENCE.len();
        if chars[open..].starts_with(&['j', 's', 'o', 'n']) {
            open += 4;
        }
        open = spaces(open);
        if chars.get(open) != Some(&'[') {
            continue;
        }
        for close in open + 1..chars.len() {
            if chars[close] == ']' && chars[spaces(close + 1)..].starts_with(&FENCE) {
                return Some(open..=close);
            }
        }
    }
    None
}

/// `extract_tasks_json`: the plan's fenced JSON array.
fn extract_tasks_json(plan: &str) -> Result<Vec<Value>> {
    let chars: Vec<char> = plan.chars().collect();
    let Some(found) = fenced_array(&chars) else {
        bail!("ValueError: no fenced JSON task array found in plan");
    };
    let found: String = chars[found].iter().collect();
    match json::loads_str(&found) {
        Ok(Value::List(tasks)) => Ok(tasks),
        Ok(_) => bail!("ValueError: plan JSON block is not an array"),
        Err(err) => bail!("{}: the plan's task array", err.class()),
    }
}

pub(crate) use hydra_pyjson::file::{os_error, read_text};

const TASK_KEYS: [&str; 4] = ["id", "text", "class", "grade"];

/// Which of the four keys a plan task lacks, the way `k not in task` answers for whatever
/// JSON the task is: a dict's keys, a list's items, a string's substrings.
fn missing_keys(task: &Value) -> Result<Vec<&'static str>> {
    let holds = |key: &str| -> Result<bool> {
        match task {
            Value::Dict(dict) => Ok(dict.get(key).is_some()),
            Value::List(items) => Ok(items.iter().any(|item| py::eq_str(item, key))),
            Value::Str(text) => {
                let needle = PyStr::from(key);
                let (hay, needle) = (text.code_points(), needle.code_points());
                Ok(hay.windows(needle.len()).any(|window| window == needle))
            }
            other => bail!(
                "TypeError: argument of type '{}' is not a container or iterable",
                py::type_name(other)
            ),
        }
    };
    let mut missing = Vec::new();
    for key in TASK_KEYS {
        if !holds(key)? {
            missing.push(key);
        }
    }
    Ok(missing)
}

/// `value not in names`, for a frozenset of strings: Python hashes the value first.
fn named(value: &Value, names: &[&str]) -> Result<bool> {
    py::hashable(value)?;
    Ok(names.iter().any(|name| py::eq_str(value, name)))
}

/// Validate keys with the fold's equality, including numeric aliases and NaN. Walk
/// iteratively so a long dependency chain cannot exhaust the call stack.
fn task_graph_offences(tasks: &[Value]) -> Result<Vec<PyStr>> {
    let mut offences = Vec::new();
    let mut ids = Vec::new();
    for task in tasks {
        let id = py::item(task, "id")?;
        py::hashable(id)?;
        if ids.iter().any(|other| py::same_key(other, id)) {
            offences.push(text!("duplicate task id ", py::repr(id)));
        } else {
            ids.push(id.clone());
        }
    }
    let mut edges = vec![Vec::new(); ids.len()];
    for task in tasks {
        let id = py::item(task, "id")?;
        let from = ids
            .iter()
            .position(|other| py::same_key(other, id))
            .unwrap();
        for dep in py::list(&py::get(task, "deps", Value::List(Vec::new()))?)? {
            py::hashable(&dep)?;
            if let Some(to) = ids.iter().position(|other| py::same_key(other, &dep)) {
                edges[from].push(to);
            } else {
                offences.push(text!(
                    "task ",
                    py::repr(id),
                    " has unknown dep ",
                    py::repr(&dep)
                ));
            }
        }
    }
    // A task is in a cycle exactly when following its deps can reach itself.
    // Checking each root reports every cyclic task, without blaming tasks that
    // merely depend on a cycle or enumerating exponentially many cycle paths.
    for (root, id) in ids.iter().enumerate() {
        let mut seen = vec![false; ids.len()];
        let mut pending = edges[root].clone();
        while let Some(next) = pending.pop() {
            if next == root {
                offences.push(text!("dependency cycle includes task ", py::repr(id)));
                break;
            }
            if !seen[next] {
                seen[next] = true;
                pending.extend(&edges[next]);
            }
        }
    }
    Ok(offences)
}

/// `tasks`: import the chosen plan's task array, one `task` event each. Every task is
/// checked before the first is written.
pub fn tasks(home: &Home, goal_id: &str, err: &mut dyn Write) -> Result<i32> {
    let state = folded(home, goal_id)?;
    let path = state
        .chosen_plan
        .as_ref()
        .map_or(Value::Null, |chosen| field(chosen, "path"));
    if !py::truthy(&path) {
        writeln!(err, "no chosen plan with a path")?;
        return Ok(1);
    }
    if approval_available(&state, "plan", &Value::Null, &Value::Null).is_none() {
        writeln!(
            err,
            "missing plan approval at revision {}; approve with: approve {goal_id} --plan",
            state.revision
        )?;
        return Ok(1);
    }
    let Value::Str(path) = &path else {
        bail!(
            "TypeError: expected str, bytes or os.PathLike object, not {}",
            py::type_name(&path)
        );
    };
    let plan = read_text(&fs_path(path)?)?;
    let tasks = extract_tasks_json(&plan)?;
    for task in &tasks {
        let missing = missing_keys(task)?;
        if !missing.is_empty() {
            let id = py::str(&py::get(task, "id", Value::text("?"))?);
            eprint(err, &text!("task ", id, " lacks ", missing.join(", ")))?;
            return Ok(1);
        }
        let known = named(py::item(task, "class")?, &TASK_CLASSES)?
            && named(py::item(task, "grade")?, &TASK_GRADES)?;
        if !known {
            let id = py::str(py::item(task, "id")?);
            eprint(err, &text!("task ", id, " has an unknown class or grade"))?;
            return Ok(1);
        }
    }
    let offences = task_graph_offences(&tasks)?;
    if !offences.is_empty() {
        for offence in offences {
            eprint(err, &offence)?;
        }
        return Ok(1);
    }
    for task in &tasks {
        append_event(
            home,
            goal_id,
            body(vec![
                ("event", Value::text("task")),
                ("id", py::item(task, "id")?.clone()),
                ("text", py::item(task, "text")?.clone()),
                ("class", py::item(task, "class")?.clone()),
                ("grade", py::item(task, "grade")?.clone()),
                ("done", py::get(task, "done", Value::Null)?),
                ("deps", py::get(task, "deps", Value::List(Vec::new()))?),
                ("paths", py::get(task, "paths", Value::List(Vec::new()))?),
            ]),
        )?;
    }
    Ok(0)
}

/// `assign`: give every task without an assignment a provider. The ranking over the agent
/// sheets wins when a verdict stands behind its first row; otherwise the default table
/// does. An `agi` task, and a class the table does not hold, wait for `set`.
pub fn assign(home: &Home, goal_id: &str, out: &mut dyn Write, err: &mut dyn Write) -> Result<i32> {
    let events = log::read_log(home, goal_id)?;
    let state = fold::fold(goal_id, &events)?;
    for task in &state.tasks {
        if task.assignment.is_some() {
            continue;
        }
        let id = py::str(&task.key);
        if py::eq_str(&task.grade, "agi") {
            eprint(err, &text!("task ", id, " needs a human assignment (agi)"))?;
            continue;
        }
        let mut source = "ranking";
        let mut row = sheet::ranked_assignment(home, task, &events)?;
        if row.is_none() {
            source = "default";
            py::hashable(&task.task_class)?;
            row = DEFAULT_ASSIGNMENTS
                .iter()
                .find(|(class, _, _)| py::eq_str(&task.task_class, class))
                .map(|(_, provider, model)| (Value::text(provider), Value::text(model)));
        }
        let Some((provider, model)) = row.filter(|(provider, _)| py::truthy(provider)) else {
            eprint(err, &text!("task ", id, " needs a human assignment"))?;
            continue;
        };
        let mut event = body(vec![
            ("event", Value::text("assignment")),
            ("task", task.key.clone()),
            ("provider", provider.clone()),
            ("policy", Value::text("standard")),
            ("source", Value::text(source)),
        ]);
        let shown = if py::truthy(&model) {
            event.set("model", model.clone());
            py::str(&model)
        } else {
            PyStr::new()
        };
        append_event(home, goal_id, event)?;
        print(
            out,
            &text!(id, ": ", py::str(&provider), " ", shown, " (", source, ")"),
        )?;
    }
    Ok(0)
}

/// `reconcile`: settle a launch intent no run followed, with the run id the human found or
/// with abandonment. An empty `run_id` abandons, as in the Python. Nothing is launched.
pub fn reconcile(
    home: &Home,
    goal_id: &str,
    task: &PyStr,
    attempt: &Value,
    run_id: Option<&PyStr>,
    err: &mut dyn Write,
) -> Result<i32> {
    let state = folded(home, goal_id)?;
    let (task_value, attempt_value) = (Value::Str(task.clone()), attempt.clone());
    let mut dangling = false;
    for intent in &state.dangling_intents {
        if py::eq(py::item(intent, "task")?, &task_value)
            && py::eq(py::item(intent, "attempt")?, &attempt_value)
        {
            dangling = true;
        }
    }
    if !dangling {
        eprint(
            err,
            &text!(
                "no dangling intent for task ",
                task.clone(),
                " attempt ",
                py::repr(attempt)
            ),
        )?;
        return Ok(1);
    }
    let event = match run_id.filter(|run_id| !run_id.is_empty()) {
        Some(run_id) => body(vec![
            ("event", Value::text("run")),
            ("task", task_value),
            ("attempt", attempt_value),
            ("run_id", Value::Str(run_id.clone())),
        ]),
        None => body(vec![
            ("event", Value::text("intent_abandoned")),
            ("task", task_value),
            ("attempt", attempt_value),
        ]),
    };
    append_event(home, goal_id, event)?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grok_defaults_pin_every_cursor_task_class() {
        let rows: Vec<_> = DEFAULT_ASSIGNMENTS
            .iter()
            .copied()
            .filter(|(_, provider, _)| *provider == "cursor-agent")
            .collect();
        assert_eq!(
            rows,
            vec![
                ("batch_scripting", "cursor-agent", "cursor-grok-4.6-high"),
                ("docs", "cursor-agent", "cursor-grok-4.6-high"),
                (
                    "rust_off_launch_path",
                    "cursor-agent",
                    "cursor-grok-4.6-high"
                ),
                ("research", "cursor-agent", "cursor-grok-4.6-high"),
            ]
        );
    }

    fn path(text: &str) -> String {
        String::from_utf8(normpath(text.as_bytes())).unwrap()
    }

    #[test]
    fn normpath_is_lexical_and_keeps_two_leading_slashes() {
        assert_eq!(path("/a/./b/../c/"), "/a/c");
        assert_eq!(path("/../a"), "/a");
        assert_eq!(path("//a//b"), "//a/b");
        assert_eq!(path("///a"), "/a");
        assert_eq!(path("a/../.."), "..");
        assert_eq!(path("../a/.."), "..");
        assert_eq!(path("a/.."), ".");
        assert_eq!(path(""), ".");
        assert_eq!(path("/"), "/");
    }

    #[test]
    fn the_stem_drops_one_suffix_and_never_the_leading_dots() {
        let stem = |text: &str| String::from_utf8(stem(text.as_bytes()).to_vec()).unwrap();
        assert_eq!(stem("/p/plan.md"), "plan");
        assert_eq!(stem("/p/a.b.md"), "a.b");
        assert_eq!(stem("/p/.hidden"), ".hidden");
        assert_eq!(stem("/p/..md"), "..md");
        assert_eq!(stem("/p/.a.md"), ".a");
        assert_eq!(stem("/p/plain"), "plain");
        assert_eq!(stem("/p/trailing."), "trailing");
    }

    #[test]
    fn an_argument_that_is_not_utf8_round_trips_through_surrogates() {
        let raw = OsStr::from_bytes(b"a\xffb\xc3\xa9\xed\xa0\x80");
        let text = arg(raw);
        assert_eq!(
            text.code_points(),
            [0x61, 0xdcff, 0x62, 0xe9, 0xdced, 0xdca0, 0xdc80]
        );
        assert_eq!(fs_path(&text).unwrap().as_os_str(), raw);
        let mut lone = PyStr::new();
        lone.push_code(0xd800);
        assert!(fs_path(&lone).is_err());
        let mut err = Vec::new();
        eprint(&mut err, &text).unwrap();
        assert_eq!(
            String::from_utf8(err).unwrap(),
            "a\\udcffb\u{e9}\\udced\\udca0\\udc80\n"
        );
    }

    #[test]
    fn the_fence_match_is_the_first_that_opens_on_a_bracket_and_the_shortest() {
        let found = |text: &str| -> Option<String> {
            let chars: Vec<char> = text.chars().collect();
            fenced_array(&chars).map(|found| chars[found].iter().collect())
        };
        assert_eq!(found("```json\n[1]\n```").as_deref(), Some("[1]"));
        assert_eq!(found("```[1]```").as_deref(), Some("[1]"));
        assert_eq!(
            found("```json[[1], [2]]\u{1f}```").as_deref(),
            Some("[[1], [2]]")
        );
        // The first `]` with a fence after it ends the match, whatever the JSON says.
        assert_eq!(found("```\n[\"]```\"]\n```").as_deref(), Some("[\"]"));
        // A fence that opens on something else is passed over, not a failure.
        assert_eq!(
            found("```rust\n[1]\n```\n```\n[2]\n```").as_deref(),
            Some("[2]")
        );
        assert_eq!(found("```jsonl\n[1]\n```"), None);
        assert_eq!(found("[1]"), None);
        assert_eq!(found("```json\n[1]\n"), None);
    }

    #[test]
    fn a_degraded_spec_names_a_missing_provider_and_a_substitute() {
        let parse = |spec: &str| {
            parse_degraded_substitution(&PyStr::from(spec)).map(|(missing, substitute, model)| {
                (
                    missing.lossy(),
                    substitute.lossy(),
                    model.map(|model| model.lossy()),
                )
            })
        };
        assert_eq!(
            parse(" codex = claude : opus "),
            Ok(("codex".into(), "claude".into(), Some("opus".into())))
        );
        assert_eq!(
            parse("codex=claude:"),
            Ok(("codex".into(), "claude".into(), None))
        );
        assert_eq!(parse("a=b=c"), Ok(("a".into(), "b=c".into(), None)));
        assert!(parse("codex").is_err());
        assert!(parse("=claude").is_err());
        assert!(parse("codex=").is_err());
        assert!(parse("codex=<substitute-provider[:model]>").is_err());
    }

    /// A puppets home in a temp directory, removed on drop.
    struct Scratch(PathBuf, Home);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir = std::env::temp_dir()
                .join(format!("hydra-puppets-write-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let home = Home::at(&dir);
            std::fs::create_dir_all(&home.puppets).unwrap();
            Scratch(dir, home)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const GOAL: &str = "{\"event\":\"goal\",\"text\":\"x\",\"repo\":\"/r\",\"base_sha\":\"a\"}\n";
    const TASK: &str =
        "{\"event\":\"task\",\"id\":\"t1\",\"text\":\"t\",\"class\":\"docs\",\"grade\":\"low\"}\n";

    /// Run `approve` while another writer holds the log's lock, let that writer add a task
    /// before it lets go, and return what `approve` said and the log it left.
    fn approve_behind_a_writer(name: &str, args: Approve) -> (i32, String, String) {
        use std::os::fd::AsRawFd;
        let scratch = Scratch::new(name);
        let home = &scratch.1;
        let path = home.goal_log_path("g1");
        std::fs::write(&path, [GOAL, TASK].concat()).unwrap();
        let holder = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        // SAFETY: `flock` reads only the descriptor, which `holder` keeps open.
        assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_EX) }, 0);
        let (code, err) = std::thread::scope(|scope| {
            let waiting = scope.spawn(|| {
                let mut err = Vec::new();
                let code = approve(home, "g1", &args, &mut err).unwrap();
                (code, String::from_utf8(err).unwrap())
            });
            // Long enough for `approve` to fold the log at revision 1 and block on the lock.
            std::thread::sleep(std::time::Duration::from_millis(300));
            assert!(!waiting.is_finished());
            (&holder)
                .write_all(TASK.replace("t1", "t2").as_bytes())
                .unwrap();
            drop(holder);
            waiting.join().unwrap()
        });
        (code, err, std::fs::read_to_string(&path).unwrap())
    }

    /// The first fold of `approve` reads without the lock. What it records, and what it
    /// refuses, come from the lines read under the lock.
    #[test]
    fn approve_checks_the_revision_it_expects_again_under_the_lock() {
        let args = Approve {
            table: true,
            expect_revision: Some(Value::Int(1)),
            ..Approve::default()
        };
        let (code, err, log) = approve_behind_a_writer("expect", args);
        assert_eq!(code, 2);
        assert_eq!(err, "expected revision 1, current revision 2\n");
        assert_eq!(log, [GOAL, TASK, &TASK.replace("t1", "t2")].concat());
    }

    #[test]
    fn approve_records_the_revision_read_under_the_lock() {
        let args = Approve {
            table: true,
            ..Approve::default()
        };
        let (code, err, log) = approve_behind_a_writer("revision", args);
        assert_eq!((code, err.as_str()), (0, ""));
        let last = log.lines().last().unwrap();
        assert!(last.starts_with("{\"seq\":4,\"ts\":\""), "{last}");
        assert!(
            last.ends_with(
                "\",\"event\":\"approval\",\"kind\":\"table\",\"ref\":null,\"revision\":2,\"yes\":true}"
            ),
            "{last}"
        );
    }

    #[test]
    fn a_goal_id_is_a_version_4_uuid() {
        let id = uuid4().unwrap();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert!(matches!(&id[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(id, uuid4().unwrap());
    }
}
