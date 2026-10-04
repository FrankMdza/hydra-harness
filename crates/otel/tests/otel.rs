//! `hydra-otel` must give what `record_to_otlp.py` gives.
//!
//! `record_otel.py` ran the Python once per case of `fixtures/otel.json`, from a scratch
//! directory and an empty environment, against a stub `hydra` that replays planted
//! `show --events` answers, and kept what it gave. This test plants the same files, runs
//! the built binary the same way, and compares. It starts no `python3`.
//!
//! What is compared, and it must be equal to the byte: the exit code, stdout, every file
//! the command left that was not planted or that it changed (`--out`, the batch directory,
//! the stub's argv log, the rewritten record), and stderr where the Python did not end in
//! a traceback.
//!
//! These are normalized, and only these:
//! - The scratch directory is `<root>` in both.
//! - The checkout is `<repo>` in a traceback.
//! - Where the Python ended in a traceback, the Rust's last line of stderr must start with
//!   the same exception class. What came before the traceback is compared whole when there
//!   is any.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use hydra_pyjson::json::{self, Value};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-otel");
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

fn told(stderr: &str, python: bool, traceback: bool) -> String {
    if !traceback {
        return stderr.to_string();
    }
    if python {
        let (before, rest) = stderr.split_once(TRACEBACK).unwrap_or(("", stderr));
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

fn run(case: &Value, root: &Path) -> Ran {
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    std::fs::create_dir_all(root.join("tmp")).unwrap();
    let mut originals = Vec::new();
    for (name, value) in pairs(field(case, "files")) {
        let path = root.join(&name);
        match bytes(&value) {
            None => std::fs::create_dir_all(&path).unwrap(),
            Some(data) => {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &data).unwrap();
                if name == "bin/hydra" {
                    let mut perm = std::fs::metadata(&path).unwrap().permissions();
                    perm.set_mode(0o755);
                    std::fs::set_permissions(&path, perm).unwrap();
                }
                originals.push((name.clone(), data));
            }
        }
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
            let data = std::fs::read(path).unwrap();
            let same = originals
                .iter()
                .any(|(planted, orig)| *planted == name && *orig == data);
            (!same).then(|| {
                let text = String::from_utf8(data)
                    .unwrap()
                    .replace(root_text, "<root>");
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
fn otel_gives_what_the_python_gave() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/otel.json");
    let fixture = json::loads_str(&std::fs::read_to_string(&here).unwrap()).unwrap();
    let Value::List(cases) = field(&fixture, "cases") else {
        panic!("no cases");
    };
    let root = std::env::temp_dir().join(format!("hydra-otel-{}", std::process::id()));
    let mut failures = Vec::new();
    for case in cases {
        let name = string(field(case, "name"));
        let ran = run(case, &root);
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
                .map(|file| {
                    let path = string(field(file, "path"));
                    let text = match field(file, "text") {
                        Value::Str(text) => text.to_utf8().expect("written text is UTF-8"),
                        _ => String::from_utf8(unhex(&string(field(file, "hex"))))
                            .expect("written hex is UTF-8"),
                    };
                    (path, text)
                })
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
}
