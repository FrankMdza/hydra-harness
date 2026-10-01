//! `hydra-grader monitor features`, `train` and `score` against `monitor.py` (build ticket
//! 215).
//!
//! `record_monitor.py` ran the Python once per case, the way `record_baseline.py` does: as
//! a child with nothing in its environment but what the case names, in a scratch directory
//! holding the case's files under relative paths. `monitor.json` keeps each case's command
//! line, files, exit code, stdout, stderr and every file the command wrote. This test
//! builds the same directory, runs the binary there with the same arguments and
//! environment, and compares the four.
//!
//! Equal to the byte: the exit code, stdout (the feature view, the report, the score
//! line), the model file with its `version` digest, any other file left behind, and stderr
//! when the Python refused with a sentence or skipped a record.
//!
//! Normalized, and only what `differential/baseline.rs` normalizes:
//! - Where the Python ended in a traceback, the Rust's last line of stderr must start with
//!   the same exception class. What stderr held before the traceback is compared whole.
//! - On a wrong command line, exit code 2, only the last line of stderr is compared, and
//!   the Python's `monitor.py` at its start is read as `hydra-grader monitor`.
//! - Where the Python printed its help, the Rust must exit 0 and print a usage line.
//!
//! `HYDRA_PUPPETS_DATA` is added to the environment so the binary reads the frozen
//! `aliases.json`, which is the file the recorder made the Python read.

use std::collections::BTreeMap;
use std::process::Stdio;

use super::baseline::{TRACEBACK, bytes_of, corpus, has, left_behind, told};
use super::*;

/// The files of one spec of a case: a directory of records, or one file from whichever of
/// the places its bytes are kept.
fn files_of(
    spec: &Value,
    corpus: &BTreeMap<String, Vec<u8>>,
    models: &BTreeMap<String, String>,
) -> Vec<(String, Vec<u8>)> {
    if has(spec, "bundle") {
        let dir = string(field(spec, "dir"));
        let Value::Dict(files) =
            fixture(&format!("monitor/{}.json", string(field(spec, "bundle"))))
        else {
            panic!("a bundle of records");
        };
        return files
            .iter()
            .map(|(name, text)| (format!("{dir}/{}", name.lossy()), string(text).into_bytes()))
            .collect();
    }
    let data = if has(spec, "dataset") || has(spec, "baseline") {
        let data = if has(spec, "dataset") {
            bytes_of(&format!("monitor/{}.jsonl", string(field(spec, "dataset"))))
        } else {
            bytes_of(&format!(
                "baseline/{}.jsonl",
                string(field(spec, "baseline"))
            ))
        };
        if has(spec, "line") {
            // The dataset with one line replaced.
            let Value::Int(at) = field(spec, "line") else {
                panic!("a line number");
            };
            let mut lines: Vec<&[u8]> = data.split_inclusive(|byte| *byte == b'\n').collect();
            let with = unhex(&string(field(spec, "with")));
            lines[*at as usize] = &with;
            lines.concat()
        } else {
            data
        }
    } else if has(spec, "scan") {
        corpus[&string(field(spec, "scan"))].clone()
    } else if has(spec, "model") {
        models[&string(field(spec, "model"))].clone().into_bytes()
    } else if has(spec, "text") {
        // A lone surrogate is kept escaped inside the JSON, so the text itself is UTF-8.
        string(field(spec, "text")).into_bytes()
    } else {
        unhex(&string(field(spec, "hex")))
    };
    vec![(string(field(spec, "path")), data)]
}

fn last_line(stderr: &str) -> String {
    let line = stderr
        .trim_end_matches('\n')
        .rsplit('\n')
        .next()
        .unwrap_or("");
    match line.strip_prefix("monitor.py") {
        Some(rest) => format!("hydra-grader monitor{rest}"),
        None => line.to_string(),
    }
}

#[test]
fn the_monitor_reads_trains_and_scores_as_the_python_did() {
    let recorded = fixture("monitor.json");
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

    let scratch = Scratch::new("monitor");
    let mut tally = Tally::new();
    let (mut viewed, mut trained, mut scored) = (0, 0, 0);
    for (n, case) in items(&recorded).iter().enumerate() {
        let name = string(field(case, "name"));
        let root = scratch.0.join(format!("c{n}"));
        let mut given = BTreeMap::new();
        for spec in items(field(case, "files")) {
            for (path, data) in files_of(spec, &corpus, &models) {
                std::fs::create_dir_all(root.join(&path).parent().unwrap()).unwrap();
                std::fs::write(root.join(&path), &data).unwrap();
                given.insert(path, data);
            }
        }
        std::fs::create_dir_all(&root).unwrap();

        let argv: Vec<String> = items(field(case, "argv")).iter().map(string).collect();
        let Value::Dict(env) = field(case, "env") else {
            panic!("an environment");
        };
        let ran = Command::new(RUST)
            .arg("monitor")
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
        let help = *exit == 0 && python_out.starts_with("usage: ");
        if help {
            tally.check(
                &format!("{name}: prints a usage line"),
                stdout.starts_with("usage: hydra-grader monitor"),
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
        // A case's directory can hold a few hundred records, so it goes once compared.
        let _ = std::fs::remove_dir_all(&root);

        if *exit == 0 && !help {
            match argv
                .iter()
                .find(|arg| !arg.starts_with('-'))
                .map(String::as_str)
            {
                Some("features") => viewed += 1,
                Some("train") => trained += 1,
                _ => scored += 1,
            }
        }
    }
    println!("{viewed} feature views, {trained} models trained, {scored} records scored");
    assert!(viewed >= 150, "{viewed} feature views");
    assert!(trained >= 20, "{trained} trains");
    assert!(scored >= 80, "{scored} scores");
    tally.finish();
}
