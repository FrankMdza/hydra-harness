//! The labels the grader reads through `hydra-puppets show` are the labels the Python fold
//! gave.
//!
//! Map 93 let `puppets.py` go on one condition: the grader's labels, computed through the
//! Python fold and through the shell-out, are equal on every goal log there is. Build ticket
//! 196 ran that comparison with both alive, and `record.py` wrote the fold's side of it into
//! each `fixtures/read/*.json`: the goal log (`seed`), what `puppets.py show` exited with,
//! and under `labels` what `labels.py` derived from `puppets.fold` before it was rewritten.
//!
//! This is `integrations/grader/tests/test_labels_recorded.py`, which build ticket 220
//! deleted with the rest of the grader's Python, with `hydra-grader`'s `labels.rs` where
//! `labels.py` was. It lives in this crate because the helper it runs is this crate's
//! binary, the real one and not the stub the grader's own differential test plants. No
//! test here starts `python3`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use hydra_grader::collect::STACK_BYTES;
use hydra_grader::labels::{self, Env, LabelsError};
use hydra_pyjson::json::{self, Value};

const HELPER: &str = env!("CARGO_BIN_EXE_hydra-puppets");

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value {
        Value::Dict(dict) => dict.get(key).unwrap_or_else(|| panic!("no {key}")),
        other => panic!("{key} of {other:?}"),
    }
}

fn has(value: &Value, key: &str) -> bool {
    matches!(value, Value::Dict(dict) if dict.get(key).is_some())
}

fn items(value: &Value) -> &[Value] {
    match value {
        Value::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

fn string(value: &Value) -> String {
    match value {
        Value::Str(text) => text.to_utf8().expect("a surrogate"),
        other => panic!("not a string: {other:?}"),
    }
}

/// The bytes of a seeded file: its text, or its hex where it is not UTF-8.
fn blob(value: &Value) -> Vec<u8> {
    if has(value, "text") {
        return string(field(value, "text")).into_bytes();
    }
    let hex = string(field(value, "hex"));
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn the_shell_out_gives_the_labels_the_python_fold_gave() {
    // One recorded goal log holds an event nested 3,000 deep, and reading it recurses once
    // per level, so this runs on the stack `hydra-grader` gives its command.
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(compare)
        .unwrap()
        .join()
        .unwrap();
}

fn compare() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&fixtures)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension() == Some(OsStr::new("json")))
        .collect();
    paths.sort();
    let scratch = std::env::temp_dir().join(format!("hydra-grader-labels-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);

    let (mut compared, mut refused, mut count, mut verdicts) = (0, 0, 0, 0);
    let mut differing = Vec::new();
    for (n, path) in paths.iter().enumerate() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(path).unwrap();
        let recorded = json::loads_str(&text).unwrap_or_else(|err| panic!("{name}: {err:?}"));
        let goal = string(field(&recorded, "goal"));
        let label = string(field(&recorded, "label"));
        let root = scratch.join(n.to_string());
        let home = root.join("puppets");
        std::fs::create_dir_all(&home).unwrap();
        for file in items(field(&recorded, "seed")) {
            let target = root.join(string(field(file, "path")));
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(&target, blob(field(file, "bytes"))).unwrap();
        }
        // Nothing of this test's shell reaches the helper but where it is.
        let env = Env::from_pairs([
            ("HYDRA_PUPPETS_BIN", OsStr::new(HELPER)),
            ("HYDRA_ROOT", root.as_os_str()),
        ]);

        // The first step is `show <goal>` under `puppets.py`.
        let first = &items(field(&recorded, "steps"))[0];
        let args: Vec<String> = items(field(first, "args")).iter().map(string).collect();
        assert_eq!(args, ["show", goal.as_str()], "{name}");
        let shown = labels::show_goal(OsStr::new(&goal), &home, &env);
        if !matches!(field(first, "code"), Value::Int(0)) {
            // The fold refused the log, so the helper must, and loudly.
            let err = shown.expect_err(&label);
            assert!(
                err.downcast_ref::<LabelsError>().is_some(),
                "{label}: {err}"
            );
            refused += 1;
            let _ = std::fs::remove_dir_all(&root);
            continue;
        }
        let shown = shown.unwrap_or_else(|err| panic!("{label}: {err}"));
        let events = labels::committed_events(&home.join(format!("{goal}.jsonl"))).unwrap();
        let found = labels::labels_for_goal(&shown, &events).unwrap();
        // As `record.py` wrote them: every pair in order, as text, so NaN equals NaN.
        let pairs: Vec<Value> = found
            .iter()
            .map(|(run_id, row)| Value::List(vec![run_id.clone(), row.clone()]))
            .collect();
        let ours = json::dumps_default(&Value::List(pairs));
        let theirs = string(field(field(&recorded, "labels"), "pairs"));
        if ours != theirs {
            differing.push(format!("{label}:\n  rust   {ours}\n  python {theirs}"));
        }
        compared += 1;
        count += found.len();
        verdicts += found
            .iter()
            .filter(|(_, row)| !matches!(field(row, "human_verdict"), Value::Null))
            .count();
        let _ = std::fs::remove_dir_all(&root);
    }
    let _ = std::fs::remove_dir_all(&scratch);
    println!(
        "labels: {compared} goal logs equal with {count} labels, {verdicts} of them with a \
         human verdict; {refused} logs refused by both"
    );
    assert!(
        differing.is_empty(),
        "{} of {compared} goal logs are labelled differently:\n{}",
        differing.len(),
        differing.join("\n")
    );
    // The recording is not allowed to shrink to nothing without this failing.
    assert!(compared > 100, "{compared} compared");
    assert!(count > 200, "{count} labels");
    assert!(verdicts >= 10, "{verdicts} verdicts");
    assert!(refused > 10, "{refused} refused");
}
