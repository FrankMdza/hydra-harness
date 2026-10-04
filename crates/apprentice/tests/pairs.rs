//! `hydra-apprentice pairs` must give what `apprentice.py pairs` gives.
//!
//! `record_pairs.py` ran the Python on each step of `fixtures/pairs.json`: the files it
//! planted, the arguments and the environment. Most cases run `pairs` twice on the same
//! graded file. This runs the built binary on the same steps. It starts no `python3`.
//!
//! Equal to the byte: the exit code; stdout; every file a step wrote or changed, which is
//! `pairs.jsonl` and the `.tmp` a refused write leaves, but for the stamps; and stderr where
//! the Python wrote a sentence. A file counts as changed when its text differs once the
//! stamps are read, so a second run that lands in a later second changes nothing.
//!
//! Normalized, and only these:
//! - Each `graded_at` a step stamped is the wall clock. It must read `YYYY-MM-DDTHH:MM:SSZ`
//!   and be no earlier than the test's start, and is then read as `<graded_at>`. A planted
//!   pair's stamp from 2000 is compared as it is.
//! - The scratch directory is `<root>`.
//! - The program's name: the Python says `apprentice.py`, the Rust `hydra-apprentice`.
//! - Where the Python ended in a traceback, the Rust's last line of stderr must start with
//!   the same exception class, without the module the Python names it by.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use hydra_pyjson::json::{self, Value};
use hydra_records::isotime::DateTime;

const RUST: &str = env!("CARGO_BIN_EXE_hydra-apprentice");
const OLD: &str = "2000-01-01T00:00:00Z";

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value {
        Value::Dict(dict) => dict.get(key).unwrap_or(&Value::Null),
        _ => &Value::Null,
    }
}

fn string(value: &Value) -> String {
    match value {
        Value::Str(text) => text.to_utf8().expect("fixture text is UTF-8"),
        other => panic!("not a string: {other:?}"),
    }
}

fn items(value: &Value) -> &[Value] {
    match value {
        Value::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

/// The time now, as the Python's `utc_now` spells it.
fn now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    DateTime::from_unix_millis(seconds * 1000)
        .unwrap()
        .isoformat()
        .replace("+00:00", "Z")
}

/// `text` with each stamp a step wrote checked and read as `<graded_at>`, as the recorder
/// wrote it. A stamp that is not the clock's stays as it is, and fails the comparison.
fn stamped(text: &str, start: &str) -> String {
    const MARKER: &str = "\"graded_at\": \"";
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(MARKER) {
        let from = at + MARKER.len();
        let end = rest[from..].find('"').map_or(rest.len(), |end| from + end);
        let stamp = &rest[from..end];
        let shape = stamp.len() == 20
            && stamp.bytes().enumerate().all(|(i, b)| match i {
                4 | 7 => b == b'-',
                10 => b == b'T',
                13 | 16 => b == b':',
                19 => b == b'Z',
                _ => b.is_ascii_digit(),
            });
        out.push_str(&rest[..from]);
        if stamp != OLD && shape && stamp >= start {
            out.push_str("<graded_at>");
        } else {
            out.push_str(stamp);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// The class of the exception a stderr ends with: in the Python, the first line under the
/// last traceback that is not indented, since 3.14 prints notes after it; in the Rust, its
/// last line. A class the Python names with its module is compared without it.
fn exception_class(stderr: &str, python: bool) -> String {
    let lines: Vec<&str> = stderr.lines().collect();
    let raised = if python {
        let from = lines
            .iter()
            .rposition(|line| line.starts_with("Traceback "))
            .map_or(0, |at| at + 1);
        lines[from..]
            .iter()
            .find(|line| !line.starts_with(' '))
            .copied()
            .unwrap_or_default()
    } else {
        lines.last().copied().unwrap_or_default()
    };
    let class = raised.split(':').next().unwrap_or_default();
    class.rsplit('.').next().unwrap_or_default().to_string()
}

/// What of a stderr is compared: all of it, or with a traceback what came before it and
/// the exception class.
fn told(stderr: &str, python: bool, traceback: bool) -> String {
    if !traceback {
        return stderr.to_string();
    }
    let before = if python {
        stderr.split("Traceback ").next().unwrap_or_default()
    } else {
        stderr
            .trim_end_matches('\n')
            .rsplit_once('\n')
            .map_or("", |(before, _)| before)
    };
    format!("{before}|{}", exception_class(stderr, python))
}

struct Ran {
    exit: i128,
    stdout: String,
    stderr: String,
    written: Vec<(String, String)>,
}

fn walk(root: &Path, dir: &Path, found: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, found);
        } else {
            let name = path.strip_prefix(root).unwrap().display().to_string();
            found.insert(name, std::fs::read(&path).unwrap());
        }
    }
}

/// A file as a step is judged by: its text with the root and the stamps read, or its bytes
/// where it is not UTF-8. A second run that wrote the same pairs a second later wrote
/// nothing new, as the recorder judged it.
fn seen(data: &[u8], root: &str, start: &str) -> Vec<u8> {
    match std::str::from_utf8(data) {
        Ok(text) => stamped(&text.replace(root, "<root>"), start).into_bytes(),
        Err(_) => data.to_vec(),
    }
}

/// One step: plant its files, run the binary, and keep what it gave. `given` is every file
/// as the last step left it, as `seen` reads it.
fn step(root: &Path, step: &Value, start: &str, given: &mut BTreeMap<String, Vec<u8>>) -> Ran {
    let root_text = root.to_str().unwrap();
    if let Value::Dict(planted) = field(step, "planted") {
        for (name, content) in planted.iter() {
            let name = name.to_utf8().unwrap();
            let path = root.join(&name);
            let data = match content {
                Value::Null => {
                    std::fs::create_dir_all(&path).unwrap();
                    continue;
                }
                Value::Str(_) => string(content).into_bytes(),
                hexed => unhex(&string(field(hexed, "hex"))),
            };
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &data).unwrap();
            given.insert(name, seen(&data, root_text, start));
        }
    }
    let fill = |text: String| text.replace("<root>", root_text);
    let args: Vec<String> = items(field(step, "argv"))
        .iter()
        .map(|arg| fill(string(arg)))
        .collect();
    let env: Vec<(String, String)> = match field(step, "env") {
        Value::Dict(env) => env
            .iter()
            .map(|(key, value)| (key.to_utf8().unwrap(), fill(string(value))))
            .collect(),
        _ => Vec::new(),
    };
    let out = Command::new(RUST)
        .args(&args)
        .current_dir(root)
        .env_clear()
        .envs(env)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let plain = |data: &[u8]| {
        String::from_utf8(data.to_vec())
            .unwrap()
            .replace(root_text, "<root>")
            .replace("hydra-apprentice", "apprentice.py")
    };
    let mut found = BTreeMap::new();
    walk(root, root, &mut found);
    let mut written = Vec::new();
    for (name, data) in found {
        let data = seen(&data, root_text, start);
        if given.get(&name) != Some(&data) {
            written.push((name.clone(), String::from_utf8(data.clone()).unwrap()));
            given.insert(name, data);
        }
    }
    Ran {
        exit: i128::from(out.status.code().unwrap_or(-1)),
        stdout: plain(&out.stdout),
        stderr: plain(&out.stderr),
        written,
    }
}

#[test]
fn pairs_gives_what_the_python_gave() {
    let start = now();
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pairs.json");
    let fixture = json::loads_str(&std::fs::read_to_string(here).unwrap()).unwrap();
    let root = std::env::temp_dir().join(format!("hydra-apprentice-pairs-{}", std::process::id()));
    let mut failures = Vec::new();
    let mut compared = 0;
    let cases = items(field(&fixture, "cases"));
    for case in cases {
        let name = string(field(case, "name"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let mut given = BTreeMap::new();
        for (n, recorded) in items(field(case, "steps")).iter().enumerate() {
            let name = format!("{name}, step {n}");
            let ran = step(&root, recorded, &start, &mut given);
            let mut check = |what: &str, same: bool, detail: String| {
                compared += 1;
                if !same {
                    failures.push(format!("{name}: {what}\n{detail}"));
                }
            };
            let Value::Int(exit) = field(recorded, "exit") else {
                panic!("{name}: no exit code");
            };
            check(
                "exit",
                ran.exit == *exit,
                format!("  rust {} python {exit}", ran.exit),
            );
            let stdout = string(field(recorded, "stdout"));
            check(
                "stdout",
                ran.stdout == stdout,
                format!("  rust:   {:?}\n  python: {stdout:?}", ran.stdout),
            );
            let python_err = string(field(recorded, "stderr"));
            let traceback = python_err.contains("Traceback (most recent call last):\n");
            let (rust, python) = (
                told(&ran.stderr, false, traceback),
                told(&python_err, true, traceback),
            );
            check(
                "stderr",
                rust == python,
                format!("  rust:   {rust:?}\n  python: {python:?}"),
            );
            let want: Vec<(String, String)> = items(field(recorded, "written"))
                .iter()
                .map(|file| (string(field(file, "path")), string(field(file, "text"))))
                .collect();
            check(
                "files",
                ran.written == want,
                format!("  rust:   {:?}\n  python: {want:?}", ran.written),
            );
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    println!("{} cases, {compared} comparisons", cases.len());
    assert!(
        failures.is_empty(),
        "{} of {compared} comparisons failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(cases.len() > 100, "{} cases", cases.len());
}

#[test]
fn a_stamp_is_read_only_when_it_is_the_clocks() {
    let start = "2026-10-04T05:00:00Z";
    let text = format!(
        "{{\"graded_at\": \"2026-10-04T05:00:01Z\"}} {{\"graded_at\": \"{OLD}\"}} \
         {{\"graded_at\": \"2026-10-04T04:59:59Z\"}} {{\"graded_at\": \"2026-10-04T05:00:01+00:00\"}}"
    );
    assert_eq!(
        stamped(&text, start),
        format!(
            "{{\"graded_at\": \"<graded_at>\"}} {{\"graded_at\": \"{OLD}\"}} \
             {{\"graded_at\": \"2026-10-04T04:59:59Z\"}} {{\"graded_at\": \"2026-10-04T05:00:01+00:00\"}}"
        )
    );
}
