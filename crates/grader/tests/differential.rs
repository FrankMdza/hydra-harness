//! `hydra-grader` must give what the grader's Python gives on the same inputs.
//!
//! Build ticket 212 ported `records.py` (without the two functions that shell out) and
//! `redact.py`. Neither has a command line, so `record.py` called each function once on
//! every input here and wrote what it returned, wrote to disk and printed under
//! `tests/fixtures/`, with the input beside it. These tests call the Rust on the same
//! inputs and compare with those files. No test starts `python3`.
//!
//! What is compared, and it must be equal to the byte:
//! - `redact.json`: each redacted string, the OpenAI key spans of each, and each redacted
//!   JSON value as `json.dumps` prints it.
//! - `scan.json`: for the twelve records of `tests/fixtures/records/` and the synthesized
//!   ones (a torn tail, a garbled interior line, a line 0 that is not an object, invalid
//!   UTF-8 of every kind the decoder names), what `committed_lines` covers, what
//!   `scan_record` returns with its fingerprint, the skip line it prints on stderr, and
//!   `record_row` under nine answers of `hydra show`.
//! - `rows.json`: `record_row` on first lines of the wrong shape, and `duration_seconds`
//!   on some 2,200 pairs of timestamps.
//! - `files.json`: the bytes `write_json` and `write_jsonl` leave, the `.tmp` file a
//!   refused write leaves, and what `read_json` and `read_jsonl` return.
//! - `preamble.json`: `strip_preamble`, `current_preamble`, and the two homes.
//! - `isspace.txt`: every code point `str.isspace` accepts, which is what ends a secret.
//!
//! These are normalized, and only these:
//! - Where the Python raised, the Rust must return an error whose text starts with the
//!   same exception class. The message after it is not compared.
//! - A home is compared by its components. `pathlib` drops a trailing slash, a doubled one
//!   and a `.` when it builds a path, and a `PathBuf` keeps the spelling it was given.
//!
//! The record's directory, which the plan expected to need replacing, appears in nothing
//! compared: the skip line names the file alone, and each case is written here under the
//! file name the recorder used.
//!
//! The data file, `aliases.json`, is the frozen copy in `fixtures/data/`. To record again,
//! see `record.py`. `HYDRA_GRADER_FIXTURES` names another fixtures directory, which is how
//! a longer run of generated timestamps was compared once.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use hydra_pyjson::json::{self, PyStr, Value};
use hydra_pyjson::models::{self, Pair};
use hydra_pyjson::py;
use hydra_records::records;
use hydra_records::redact;

const RUST: &str = env!("CARGO_BIN_EXE_hydra-grader");

// Build ticket 213: `baseline.py`'s `train` and `score`, command by command.
#[path = "differential/baseline.rs"]
mod baseline;

// Build ticket 215: `monitor.py`'s `features`, `train` and `score`, command by command.
#[path = "differential/monitor.rs"]
mod monitor;
// Build ticket 214: `labels.py` and `grader.py`, function by function and command by command.
#[path = "differential/collect.rs"]
mod collect;

fn fixtures() -> PathBuf {
    match std::env::var_os("HYDRA_GRADER_FIXTURES") {
        Some(named) => PathBuf::from(named),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
    }
}

fn fixture(name: &str) -> Value {
    let path = fixtures().join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{name}: {err}"));
    json::loads_str(&text).unwrap_or_else(|err| panic!("{name}: {err:?}"))
}

fn aliases() -> Vec<(Pair, Pair)> {
    models::load_aliases(&fixtures().join("data/aliases.json")).unwrap()
}

/// A scratch directory under the system temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!("hydra-grader-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path.canonicalize().unwrap())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Executable stubs, each written once per test into `dir` and hard-linked where a case
/// plants it. macOS checks a file the first time it runs, about 170 ms for each new file
/// and nothing for a new name of a file it already ran, and these tests plant hundreds
/// (build ticket 304). A planter removes its path before writing, so no write reaches a
/// stub through a link.
struct Stubs(PathBuf);

impl Stubs {
    fn new<'a>(dir: PathBuf, stubs: impl IntoIterator<Item = (&'a str, &'a str)>) -> Stubs {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in stubs {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Stubs(dir)
    }

    /// Plant the stub `name` at `target`, which must not exist.
    fn link(&self, name: &str, target: &Path) {
        std::fs::hard_link(self.0.join(name), target).unwrap();
    }
}

/// The comparisons of one test: every one runs, and the test fails at the end with how many
/// differed, so breaking the Rust on purpose gives a count.
struct Tally {
    made: usize,
    differing: Vec<String>,
}

impl Tally {
    fn new() -> Tally {
        Tally {
            made: 0,
            differing: Vec::new(),
        }
    }

    fn check<T: PartialEq + std::fmt::Debug>(&mut self, label: &str, rust: T, python: T) {
        self.made += 1;
        if rust != python {
            self.differing
                .push(format!("{label}:\n  rust   {rust:?}\n  python {python:?}"));
        }
    }

    fn finish(self) {
        assert!(self.made > 0, "nothing was compared");
        println!("{} comparisons", self.made);
        assert!(
            self.differing.is_empty(),
            "{} of {} comparisons differ:\n{}",
            self.differing.len(),
            self.made,
            self.differing
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value {
        Value::Dict(dict) => dict.get(key).unwrap_or_else(|| panic!("no {key}")),
        other => panic!("{key} of {other:?}"),
    }
}

fn items(value: &Value) -> &[Value] {
    match value {
        Value::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

fn pystr(value: &Value) -> PyStr {
    match value {
        Value::Str(s) => s.clone(),
        other => panic!("not a string: {other:?}"),
    }
}

/// A string of the fixture that is known to hold no surrogate.
fn string(value: &Value) -> String {
    pystr(value).to_utf8().expect("a surrogate")
}

fn optional(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        other => Some(string(other)),
    }
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

/// A JSON source of the fixture, read as the recorder read it.
fn parsed(value: &Value) -> Value {
    json::loads_str(&string(value)).unwrap()
}

fn list(value: Value) -> Vec<Value> {
    match value {
        Value::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

/// The exception class an error names: its text up to the first colon.
fn class(err: &anyhow::Error) -> String {
    let text = err.to_string();
    text.split(':').next().unwrap_or_default().to_string()
}

/// What a call gave, as the recorder wrote it down: the value, or the class it raised.
fn outcome<T>(got: anyhow::Result<T>) -> (Option<T>, Option<String>) {
    match got {
        Ok(value) => (Some(value), None),
        Err(err) => (None, Some(class(&err))),
    }
}

#[test]
fn the_fixtures_were_recorded_from_python_3_14_7() {
    let version = std::fs::read_to_string(fixtures().join("version.txt")).unwrap();
    assert!(version.starts_with("3.14.7 "), "{version}");
}

#[test]
fn the_redactor_gives_what_the_python_gave() {
    let recorded = fixture("redact.json");
    let mut tally = Tally::new();
    for case in items(field(&recorded, "strings")) {
        let text = pystr(field(case, "text"));
        let label = format!("{:?}", text.lossy());
        let spans: Vec<(i128, i128)> = redact::openai_key_spans(&text)
            .into_iter()
            .map(|(at, end)| (at as i128, end as i128))
            .collect();
        let python: Vec<(i128, i128)> = items(field(case, "spans"))
            .iter()
            .map(|span| match items(span) {
                [Value::Int(at), Value::Int(end)] => (*at, *end),
                other => panic!("a span: {other:?}"),
            })
            .collect();
        tally.check(&format!("spans of {label}"), spans, python);
        tally.check(
            &format!("redacted {label}"),
            redact::redact_string(&text),
            pystr(field(case, "redacted")),
        );
    }
    for case in items(field(&recorded, "values")) {
        let source = string(field(case, "source"));
        tally.check(
            &format!("redacted value {source}"),
            json::dumps_default(&redact::redact_value(&parsed(field(case, "source")))),
            string(field(case, "redacted")),
        );
    }
    tally.finish();
}

#[test]
fn what_ends_a_secret_is_what_isspace_accepts() {
    let recorded = std::fs::read_to_string(fixtures().join("isspace.txt")).unwrap();
    let python: Vec<u32> = recorded
        .lines()
        .map(|line| u32::from_str_radix(line, 16).unwrap())
        .collect();
    let rust: Vec<u32> = (0..0x110000)
        .filter(|code| char::from_u32(*code).is_some_and(py::is_space))
        .collect();
    assert_eq!(rust, python);
}

#[test]
fn the_record_scan_gives_what_the_python_gave() {
    let recorded = fixture("scan.json");
    let aliases = aliases();
    let scratch = Scratch::new("scan");
    let mut tally = Tally::new();
    let mut corpus = 0;
    for case in items(&recorded) {
        let name = string(field(case, "name"));
        let data = unhex(&string(field(case, "data")));
        if name.ends_with(".jsonl") {
            corpus += 1;
        }

        let committed = field(case, "committed");
        let python = match committed {
            Value::Dict(dict) if dict.get("raises").is_some() => {
                format!("raises {}", string(field(committed, "raises")))
            }
            _ => format!(
                "{} bytes: {}",
                py::repr(field(committed, "covered")),
                string(field(committed, "lines"))
            ),
        };
        let rust = match records::committed_lines(&data) {
            Ok((lines, covered)) => {
                format!(
                    "{covered} bytes: {}",
                    json::dumps_default(&Value::List(lines))
                )
            }
            Err(bad) => format!("raises {}", bad.error.class()),
        };
        tally.check(&format!("{name}: committed_lines"), rust, python);

        let path = scratch.0.join(string(field(case, "file")));
        std::fs::write(&path, &data).unwrap();
        let mut stderr = Vec::new();
        let (scanned, raised) = outcome(records::scan_record(&path, &mut stderr));
        tally.check(
            &format!("{name}: what scan_record raises"),
            raised,
            optional(field(case, "raises")),
        );
        tally.check(
            &format!("{name}: stderr"),
            String::from_utf8(stderr).unwrap(),
            string(field(case, "stderr")),
        );
        let scanned = scanned.flatten();
        let python = field(case, "scanned");
        tally.check(
            &format!("{name}: whether it is a provider run"),
            scanned.is_some(),
            !matches!(python, Value::Null),
        );
        let Some(scanned) = scanned else { continue };
        if matches!(python, Value::Null) {
            continue;
        }
        assert_eq!(scanned.path, path);
        assert_eq!(scanned.data, data);
        tally.check(
            &format!("{name}: fingerprint"),
            scanned.fingerprint.clone(),
            string(field(python, "fingerprint")),
        );
        tally.check(
            &format!("{name}: run_id"),
            json::dumps_default(&scanned.run_id),
            string(field(python, "run_id")),
        );
        tally.check(
            &format!("{name}: first"),
            json::dumps_default(&scanned.first),
            string(field(python, "first")),
        );
        tally.check(
            &format!("{name}: lines"),
            json::dumps_default(&Value::List(scanned.lines.clone())),
            string(field(python, "lines")),
        );
        for row in items(field(case, "rows")) {
            let answer = parsed(field(row, "answer"));
            let got = records::record_row(&scanned.first, &scanned.lines, &answer, &aliases);
            let (value, raised) = outcome(got.map(|row| json::dumps_default(&row)));
            let label = format!("{name}: record_row under {}", string(field(row, "answer")));
            tally.check(&label, value, optional(field(row, "row")));
            tally.check(
                &format!("{label}, raised"),
                raised,
                optional(field(row, "raises")),
            );
        }
    }
    assert_eq!(corpus, 12, "the records of tests/fixtures/records/");
    tally.finish();
}

#[test]
fn rows_and_durations_are_what_the_python_gave() {
    let recorded = fixture("rows.json");
    let aliases = aliases();
    let mut tally = Tally::new();
    for case in items(field(&recorded, "rows")) {
        let first = parsed(field(case, "first"));
        let lines = list(parsed(field(case, "lines")));
        let answer = parsed(field(case, "answer"));
        let got = records::record_row(&first, &lines, &answer, &aliases);
        let (value, raised) = outcome(got.map(|row| json::dumps_default(&row)));
        let label = format!("record_row of {}", string(field(case, "first")));
        tally.check(&label, value, optional(field(case, "row")));
        tally.check(
            &format!("{label}, raised"),
            raised,
            optional(field(case, "raises")),
        );
    }
    for case in items(field(&recorded, "durations")) {
        let first = parsed(field(case, "first"));
        let lines = list(parsed(field(case, "lines")));
        let got = records::duration_seconds(&first, &lines);
        let (value, raised) = outcome(
            got.map(|seconds| json::dumps_default(&seconds.map_or(Value::Null, Value::Float))),
        );
        let label = format!(
            "duration_seconds of {} and {}",
            string(field(case, "first")),
            string(field(case, "lines"))
        );
        tally.check(&label, value, optional(field(case, "seconds")));
        tally.check(
            &format!("{label}, raised"),
            raised,
            optional(field(case, "raises")),
        );
    }
    tally.finish();
}

/// Every file under `root` as `path<TAB>text`, sorted by path.
fn listing(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, found);
            } else {
                let relative = path.strip_prefix(root).unwrap().display().to_string();
                let text = std::fs::read_to_string(&path).unwrap();
                found.push(format!("{relative}\t{text}"));
            }
        }
    }
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.sort();
    found
}

#[test]
fn the_files_written_and_read_are_what_the_python_gave() {
    let recorded = fixture("files.json");
    let scratch = Scratch::new("files");
    let mut tally = Tally::new();
    for (n, case) in items(field(&recorded, "writes")).iter().enumerate() {
        let root = scratch.0.join(format!("w{n}"));
        std::fs::create_dir_all(&root).unwrap();
        let kind = string(field(case, "kind"));
        let target = string(field(case, "target"));
        if let Some(before) = optional(field(case, "before")) {
            std::fs::write(root.join(&target), before).unwrap();
        }
        let value = parsed(field(case, "source"));
        let got = match kind.as_str() {
            "json" => records::write_json(&root.join(&target), &value),
            _ => records::write_jsonl(&root.join(&target), &list(value)),
        };
        let label = format!(
            "write_{kind} of {} to {target}",
            string(field(case, "source"))
        );
        tally.check(
            &format!("{label}, raised"),
            outcome(got).1,
            optional(field(case, "raises")),
        );
        let mut python: Vec<String> = items(field(case, "files"))
            .iter()
            .map(|file| {
                format!(
                    "{}\t{}",
                    string(field(file, "path")),
                    string(field(file, "text"))
                )
            })
            .collect();
        python.sort();
        tally.check(&format!("{label}, the files left"), listing(&root), python);
    }
    for (n, case) in items(field(&recorded, "reads")).iter().enumerate() {
        let kind = string(field(case, "kind"));
        let path = scratch.0.join(format!("r{n}.{kind}"));
        let data = optional(field(case, "data"));
        if let Some(data) = &data {
            std::fs::write(&path, unhex(data)).unwrap();
        }
        let got = match kind.as_str() {
            "jsonl" => {
                records::read_jsonl(&path).map(|rows| json::dumps_default(&Value::List(rows)))
            }
            _ => records::read_json(&path).map(|value| match value {
                Some(value) => json::dumps_default(&value),
                // The default the recorder passed.
                None => "{\"default\": 1}".to_string(),
            }),
        };
        let (value, raised) = outcome(got);
        let label = format!("read_{kind} of {data:?}");
        tally.check(&label, value, optional(field(case, "value")));
        tally.check(
            &format!("{label}, raised"),
            raised,
            optional(field(case, "raises")),
        );
    }
    tally.finish();
}

#[test]
fn the_preamble_and_the_homes_are_what_the_python_gave() {
    let recorded = fixture("preamble.json");
    let scratch = Scratch::new("preamble");
    let mut tally = Tally::new();
    for case in items(field(&recorded, "strips")) {
        let task = pystr(field(case, "task"));
        let current = match field(case, "current") {
            Value::Null => None,
            other => Some(pystr(other)),
        };
        let (text, stripped) = records::strip_preamble(&task, current.as_ref());
        let label = format!("strip_preamble of {:?}", task.lossy());
        tally.check(&label, text, pystr(field(case, "text")));
        tally.check(
            &format!("{label}, whether one was found"),
            stripped,
            matches!(field(case, "stripped"), Value::Bool(true)),
        );
    }
    for (n, case) in items(field(&recorded, "currents")).iter().enumerate() {
        let root = scratch.0.join(format!("c{n}"));
        let file = root.join("config").join("preamble.md");
        let data = optional(field(case, "data"));
        match data.as_deref() {
            None => std::fs::create_dir_all(&root).unwrap(),
            Some("directory") => std::fs::create_dir_all(&file).unwrap(),
            Some(data) => {
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(&file, unhex(data)).unwrap();
            }
        }
        let (value, raised) = outcome(records::current_preamble(&root));
        let label = format!("current_preamble of {data:?}");
        let python = match field(case, "preamble") {
            Value::Null => None,
            other => Some(pystr(other)),
        };
        tally.check(&label, value.flatten(), python);
        tally.check(
            &format!("{label}, raised"),
            raised,
            optional(field(case, "raises")),
        );
    }
    for case in items(field(&recorded, "homes")) {
        let Value::Dict(env) = field(case, "env") else {
            panic!("an environment");
        };
        let lookup = |name: &str| env.get(name).map(|value| OsString::from(string(value)));
        let by_components = |path: PathBuf| path.components().collect::<PathBuf>();
        let label = py::repr(field(case, "env"));
        tally.check(
            &format!("hydra_root under {label}"),
            by_components(records::hydra_root_in(&lookup).unwrap()),
            PathBuf::from(string(field(case, "root"))),
        );
        tally.check(
            &format!("grader_home under {label}"),
            by_components(records::grader_home_in(&lookup).unwrap()),
            PathBuf::from(string(field(case, "home"))),
        );
    }
    tally.finish();
}

#[path = "differential/jev.rs"]
mod jev;
#[path = "differential/queue.rs"]
mod queue;

#[path = "differential/triage.rs"]
mod triage;

// Build ticket 218: `judge.py`'s `label` and `agree`, against a stub model server.
#[path = "differential/judge.rs"]
mod judge;
#[path = "differential/sheet.rs"]
mod sheet;
