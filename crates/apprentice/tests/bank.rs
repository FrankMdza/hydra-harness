//! `hydra-apprentice bank build` must give what `apprentice.py bank build` gives.
//!
//! `record_bank.py` ran the Python once per case of `fixtures/bank.json`, from a scratch
//! directory and an empty environment, and kept what it gave. This test plants the same
//! files, runs the built binary the same way, and compares. It starts no `python3`. The
//! cases are the twelve records of `tests/fixtures/records/`, copied into the fixture, with
//! records synthesized beside them (seat requests, refusals, an interactive start, secrets,
//! a task past the excerpt budget, both preambles), records of the wrong shape, the homes
//! the environment can name, and command lines `argparse` takes and refuses.
//!
//! What is compared, and it must be equal to the byte: the exit code, stdout, every file
//! the command left that was not planted (`bank.jsonl`, or the `.tmp` a refused write
//! leaves), and stderr where the Python did not end in a traceback.
//!
//! These are normalized, and only these:
//! - The scratch directory is `<root>` in both, which is the runs path in the stdout line
//!   when the environment names it absolutely.
//! - The program's name: the Python's usage and errors say `apprentice.py`, the Rust's say
//!   `hydra-apprentice`.
//! - Where the Python ended in a traceback, the Rust's last line of stderr must start with
//!   the same exception class. What came before the traceback, the skip lines, is compared
//!   whole.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use hydra_pyjson::json::{self, Value};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-apprentice");
const TRACEBACK: &str = "Traceback (most recent call last):\n";

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

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

/// One planted file: its bytes, or `None` for a directory.
fn bytes(value: &Value) -> Option<Vec<u8>> {
    match value {
        Value::Null => None,
        Value::Str(_) => Some(string(value).into_bytes()),
        hexed => Some(unhex(&string(field(hexed, "hex")))),
    }
}

fn pairs(value: &Value) -> Vec<(String, Value)> {
    match value {
        Value::Dict(dict) => dict
            .iter()
            .map(|(key, item)| (key.to_utf8().unwrap(), item.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            walk(&path, found);
        } else if kind.is_file() {
            found.push(path);
        }
    }
}

/// The line of a traceback that names the exception. Since 3.11 notes may follow it, and
/// `json.dumps` adds one for each object it was inside of.
fn exception_class(stderr: &str) -> String {
    let line = stderr
        .trim_end_matches('\n')
        .rsplit('\n')
        .find(|line| !line.starts_with("when serializing "))
        .unwrap_or("");
    line.split_once(':')
        .map_or(line, |(class, _)| class)
        .to_string()
}

/// What of a stderr is compared: all of it, or with a traceback what came before it and
/// the exception class.
fn told(stderr: &str, python: bool, traceback: bool) -> String {
    if !traceback {
        return stderr.to_string();
    }
    if python {
        let (before, rest) = stderr.split_once(TRACEBACK).unwrap();
        return format!("{before}{}", exception_class(rest));
    }
    let body = stderr.trim_end_matches('\n');
    let (before, last) = match body.rsplit_once('\n') {
        Some((before, last)) => (format!("{before}\n"), last),
        None => (String::new(), body),
    };
    format!("{before}{}", exception_class(last))
}

struct Ran {
    exit: i128,
    stdout: String,
    stderr: String,
    written: Vec<(String, String)>,
}

fn run(case: &Value, corpus: &Value, root: &Path) -> Ran {
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let mut planted = Vec::new();
    let mut files = if matches!(field(case, "corpus"), Value::Bool(true)) {
        pairs(corpus)
    } else {
        Vec::new()
    };
    files.extend(pairs(field(case, "files")));
    for (name, value) in &files {
        let path = root.join(name);
        match bytes(value) {
            None => std::fs::create_dir_all(&path).unwrap(),
            Some(data) => {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, data).unwrap();
            }
        }
        planted.push(name.clone());
    }
    for (link, target) in pairs(field(case, "links")) {
        std::os::unix::fs::symlink(string(&target), root.join(link)).unwrap();
    }
    let root_text = root.to_str().unwrap();
    let args: Vec<String> = match field(case, "argv") {
        Value::List(items) => items.iter().map(string).collect(),
        _ => Vec::new(),
    };
    let mut command = Command::new(RUST);
    command
        .args(&args)
        .current_dir(root)
        .env_clear()
        .stdin(Stdio::null());
    for (key, value) in pairs(field(case, "env")) {
        command.env(key, string(&value).replace("<root>", root_text));
    }
    let out = command.output().unwrap();
    let plain = |data: &[u8]| {
        String::from_utf8(data.to_vec())
            .unwrap()
            .replace(root_text, "<root>")
            .replace("hydra-apprentice", "apprentice.py")
    };
    let mut found = Vec::new();
    walk(root, &mut found);
    found.sort();
    let written = found
        .iter()
        .filter_map(|path| {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            (!planted.contains(&name)).then(|| {
                let text = String::from_utf8(std::fs::read(path).unwrap()).unwrap();
                (name, text)
            })
        })
        .collect();
    Ran {
        exit: i128::from(out.status.code().unwrap_or(-1)),
        stdout: plain(&out.stdout),
        stderr: plain(&out.stderr),
        written,
    }
}

#[test]
fn bank_build_gives_what_the_python_gave() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bank.json");
    let fixture = json::loads_str(&std::fs::read_to_string(here).unwrap()).unwrap();
    let corpus = field(&fixture, "corpus");
    let Value::List(cases) = field(&fixture, "cases") else {
        panic!("no cases");
    };
    let root = std::env::temp_dir().join(format!("hydra-apprentice-bank-{}", std::process::id()));
    let mut failures = Vec::new();
    for case in cases {
        let name = string(field(case, "name"));
        let ran = run(case, corpus, &root);
        let Value::Int(exit) = field(case, "exit") else {
            panic!("{name}: no exit code");
        };
        let python_err = string(field(case, "stderr"));
        let traceback = python_err.contains(TRACEBACK);
        if ran.exit != *exit {
            failures.push(format!(
                "{name}: exit {} where the Python gave {exit}",
                ran.exit
            ));
        }
        if ran.stdout != string(field(case, "stdout")) {
            failures.push(format!(
                "{name}: stdout\n  rust:   {:?}\n  python: {:?}",
                ran.stdout,
                string(field(case, "stdout"))
            ));
        }
        let (rust_told, python_told) = (
            told(&ran.stderr, false, traceback),
            told(&python_err, true, traceback),
        );
        if rust_told != python_told {
            failures.push(format!(
                "{name}: stderr\n  rust:   {rust_told:?}\n  python: {python_told:?}"
            ));
        }
        let want: Vec<(String, String)> = match field(case, "written") {
            Value::List(items) => items
                .iter()
                .map(|file| (string(field(file, "path")), string(field(file, "text"))))
                .collect(),
            _ => Vec::new(),
        };
        if ran.written != want {
            let names = |files: &[(String, String)]| {
                files
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>()
            };
            let first_difference = ran
                .written
                .iter()
                .zip(&want)
                .flat_map(|((_, a), (_, b))| a.lines().zip(b.lines()))
                .find(|(a, b)| a != b);
            failures.push(format!(
                "{name}: files {:?} where the Python left {:?}, first line that differs: {first_difference:?}",
                names(&ran.written),
                names(&want),
            ));
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    println!("{} cases compared", cases.len());
    assert!(
        failures.is_empty(),
        "{} of {} comparisons failed:\n{}",
        failures.len(),
        cases.len() * 4,
        failures.join("\n")
    );
    assert!(cases.len() > 60, "{} cases", cases.len());
}
