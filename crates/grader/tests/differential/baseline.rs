//! `hydra-grader baseline train` and `score` against `baseline.py` (build ticket 213).
//!
//! `record_baseline.py` ran the Python once per case, as a child with nothing in its
//! environment but what the case names, in a scratch directory holding the case's files
//! under relative paths. `baseline.json` keeps each case's command line, files, exit code,
//! stdout, stderr and every file the command wrote. This test builds the same directory,
//! runs the binary there with the same arguments and environment, and compares the four.
//!
//! Equal to the byte: the exit code, stdout (the report, the score line), the model file
//! with its `version` digest, any other file left behind, and stderr when the Python
//! refused with a sentence.
//!
//! Normalized, and only these:
//! - Where the Python ended in a traceback, the Rust's last line of stderr must start with
//!   the same exception class. What stderr held before the traceback is compared whole.
//! - On a wrong command line, exit code 2, only the last line of stderr is compared, and
//!   the Python's `baseline.py` at its start is read as `hydra-grader baseline`. The usage
//!   lines above it name a different program and wrap differently.
//! - Where the Python printed its help, the Rust must exit 0 and print a usage line.
//!
//! `HYDRA_PUPPETS_DATA` is added to the environment so the binary reads the frozen
//! `aliases.json`, which is the file the recorder made the Python read.

use std::collections::BTreeMap;
use std::process::Stdio;

use super::*;

fn bytes_of(name: &str) -> Vec<u8> {
    let path = fixtures().join(name);
    std::fs::read(&path).unwrap_or_else(|err| panic!("{name}: {err}"))
}

/// The twelve records of `tests/fixtures/records/` as `scan.json` froze them.
fn corpus() -> BTreeMap<String, Vec<u8>> {
    items(&fixture("scan.json"))
        .iter()
        .map(|case| (string(field(case, "name")), case))
        .filter(|(name, _)| name.ends_with(".jsonl"))
        .map(|(name, case)| (name, unhex(&string(field(case, "data")))))
        .collect()
}

fn has(value: &Value, key: &str) -> bool {
    matches!(value, Value::Dict(dict) if dict.get(key).is_some())
}

/// A file of a case, from whichever of the four places its bytes are kept.
fn file_bytes(
    spec: &Value,
    corpus: &BTreeMap<String, Vec<u8>>,
    models: &BTreeMap<String, String>,
) -> Vec<u8> {
    if has(spec, "dataset") {
        let data = bytes_of(&format!(
            "baseline/{}.jsonl",
            string(field(spec, "dataset"))
        ));
        if !has(spec, "line") {
            return data;
        }
        // The dataset with one line replaced.
        let Value::Int(at) = field(spec, "line") else {
            panic!("a line number");
        };
        let mut lines: Vec<&[u8]> = data.split_inclusive(|byte| *byte == b'\n').collect();
        let with = unhex(&string(field(spec, "with")));
        lines[*at as usize] = &with;
        lines.concat()
    } else if has(spec, "scan") {
        corpus[&string(field(spec, "scan"))].clone()
    } else if has(spec, "model") {
        models[&string(field(spec, "model"))].clone().into_bytes()
    } else {
        unhex(&string(field(spec, "hex")))
    }
}

/// Every file under `root` that is not one of `given` unchanged, as `path<TAB>text`.
fn left_behind(root: &Path, given: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, found);
            } else {
                let relative = path.strip_prefix(root).unwrap().display().to_string();
                found.push((relative, std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.sort();
    found
        .into_iter()
        .filter(|(path, data)| given.get(path) != Some(data))
        .map(|(path, data)| format!("{path}\t{}", String::from_utf8_lossy(&data)))
        .collect()
}

const TRACEBACK: &str = "Traceback (most recent call last):\n";

/// What of a stderr is compared: all of it, or with a traceback what came before it and
/// the exception class its last line names, without the module in front.
fn told(stderr: &str, traceback: bool) -> String {
    if !traceback {
        return stderr.to_string();
    }
    let (before, rest) = match stderr.split_once(TRACEBACK) {
        Some((before, rest)) => (before.to_string(), rest),
        // The Rust's: every line but the last came before.
        None => {
            let body = stderr.trim_end_matches('\n');
            match body.rsplit_once('\n') {
                Some((before, last)) => (format!("{before}\n"), last),
                None => (String::new(), body),
            }
        }
    };
    let last = rest
        .trim_end_matches('\n')
        .rsplit('\n')
        .next()
        .unwrap_or("");
    let class = last.split(':').next().unwrap_or("");
    let class = class.rsplit('.').next().unwrap_or("");
    format!("{before}{class}")
}

fn last_line(stderr: &str) -> String {
    let line = stderr
        .trim_end_matches('\n')
        .rsplit('\n')
        .next()
        .unwrap_or("");
    match line.strip_prefix("baseline.py") {
        Some(rest) => format!("hydra-grader baseline{rest}"),
        None => line.to_string(),
    }
}

#[test]
fn the_baseline_trains_reports_and_scores_as_the_python_did() {
    let recorded = fixture("baseline.json");
    let corpus = corpus();
    assert_eq!(corpus.len(), 12, "the records of tests/fixtures/records/");
    // A model a later case scores with is the file the Python wrote, never the Rust's.
    let mut models = BTreeMap::new();
    for case in items(&recorded) {
        for file in items(field(case, "written")) {
            if string(field(file, "path")).ends_with(".json") {
                models.insert(string(field(case, "name")), string(field(file, "text")));
            }
        }
    }

    let scratch = Scratch::new("baseline");
    let mut tally = Tally::new();
    let (mut trained, mut scored) = (0, 0);
    for (n, case) in items(&recorded).iter().enumerate() {
        let name = string(field(case, "name"));
        let root = scratch.0.join(format!("c{n}"));
        let mut given = BTreeMap::new();
        for spec in items(field(case, "files")) {
            let path = string(field(spec, "path"));
            let data = file_bytes(spec, &corpus, &models);
            std::fs::create_dir_all(root.join(&path).parent().unwrap()).unwrap();
            std::fs::write(root.join(&path), &data).unwrap();
            given.insert(path, data);
        }
        std::fs::create_dir_all(&root).unwrap();

        let argv: Vec<String> = items(field(case, "argv")).iter().map(string).collect();
        let Value::Dict(env) = field(case, "env") else {
            panic!("an environment");
        };
        let ran = Command::new(RUST)
            .arg("baseline")
            .args(&argv)
            .current_dir(&root)
            .env_clear()
            .envs(env.iter().map(|(key, value)| (key.lossy(), string(value))))
            .env("HYDRA_PUPPETS_DATA", fixtures().join("data"))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let stdout = String::from_utf8(ran.stdout).unwrap();
        let stderr = String::from_utf8(ran.stderr).unwrap();

        let Value::Int(exit) = field(case, "exit") else {
            panic!("an exit code");
        };
        let python_out = string(field(case, "stdout"));
        let python_err = string(field(case, "stderr"));
        tally.check(
            &format!("{name}: exit code"),
            ran.status.code().map(i128::from),
            Some(*exit),
        );
        if *exit == 0 && python_out.starts_with("usage: ") {
            tally.check(
                &format!("{name}: prints a usage line"),
                stdout.starts_with("usage: hydra-grader baseline"),
                true,
            );
        } else {
            tally.check(&format!("{name}: stdout"), stdout, python_out.clone());
        }
        if *exit == 2 {
            tally.check(
                &format!("{name}: the error line"),
                last_line(&stderr),
                last_line(&python_err),
            );
        } else {
            let traceback = python_err.contains(TRACEBACK);
            tally.check(
                &format!("{name}: stderr"),
                told(&stderr, traceback),
                told(&python_err, traceback),
            );
        }
        let python: Vec<String> = items(field(case, "written"))
            .iter()
            .map(|file| {
                format!(
                    "{}\t{}",
                    string(field(file, "path")),
                    string(field(file, "text"))
                )
            })
            .collect();
        tally.check(
            &format!("{name}: the files left"),
            left_behind(&root, &given),
            python,
        );

        let printed = *exit == 0 && !python_out.starts_with("usage: ");
        if printed && argv.first().is_some_and(|first| first == "train") {
            trained += 1;
        }
        if printed && argv.first().is_some_and(|first| first == "score") {
            scored += 1;
        }
    }
    println!("{trained} models trained, {scored} records scored");
    assert!(trained >= 15, "{trained} trains");
    assert!(scored >= 80, "{scored} scores");
    tally.finish();
}
