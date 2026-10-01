//! CLI replay: files and successful output are exact. Usage/help is binary-specific;
//! argparse error sentences and exception classes are compared separately.
use super::*;
use std::collections::BTreeMap;
use std::process::Stdio;

fn map(value: &Value) -> BTreeMap<String, String> {
    let Value::Dict(dict) = value else {
        panic!("dict");
    };
    dict.iter().map(|(k, v)| (k.lossy(), string(v))).collect()
}
fn error_line(text: &str) -> String {
    let last = text.lines().last().unwrap_or("");
    last.split_once(": error: ")
        .map_or_else(|| last.to_string(), |(_, msg)| msg.to_string())
}
fn normalized_error(rust: &str, python: &str) -> (String, String) {
    if python.contains("Traceback (most recent call last):") {
        let class = |s: &str| error_line(s).split(':').next().unwrap_or("").to_string();
        (class(rust), class(python))
    } else {
        (error_line(rust), error_line(python))
    }
}
pub(super) fn replay(file: &str, group: &str) {
    let recorded = fixture(file);
    assert!(string(field(&recorded, "version")).starts_with("3.14.7"));
    let mut tally = Tally::new();
    let scratch = Scratch::new(&format!("queue-jev-{group}"));
    for (i, case) in items(field(&recorded, "cases")).iter().enumerate() {
        let name = string(field(case, "name"));
        let root = scratch.0.join(i.to_string());
        std::fs::create_dir_all(&root).unwrap();
        for (path, text) in map(field(case, "seed")) {
            let target = root.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, text).unwrap();
        }
        let env = map(field(case, "env"));
        for step in items(field(case, "git")) {
            if let Value::Dict(_) = step {
                std::fs::write(
                    root.join(string(field(step, "path"))),
                    string(field(step, "text")),
                )
                .unwrap();
            } else {
                let out = Command::new("/usr/bin/git")
                    .args(items(step).iter().map(string))
                    .current_dir(&root)
                    .env_clear()
                    .envs(&env)
                    .stdin(Stdio::null())
                    .output()
                    .unwrap();
                assert!(
                    out.status.success(),
                    "{name}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
        let ran = Command::new(RUST)
            .arg(group)
            .args(items(field(case, "argv")).iter().map(string))
            .current_dir(&root)
            .env_clear()
            .envs(&env)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let code = match field(case, "code") {
            Value::Int(n) => *n as i32,
            _ => panic!("code"),
        };
        tally.check(&format!("{name}: exit"), ran.status.code(), Some(code));
        let stdout = String::from_utf8(ran.stdout).unwrap();
        let stderr = String::from_utf8(ran.stderr).unwrap();
        let pyout = string(field(case, "stdout"));
        let pyerr = string(field(case, "stderr"));
        if pyout.starts_with("usage:") {
            tally.check(
                &format!("{name}: help"),
                stdout.starts_with(&format!("usage: hydra-grader {group}")),
                true,
            );
        } else {
            tally.check(&format!("{name}: stdout"), stdout, pyout);
        }
        let (rust, python) = normalized_error(&stderr, &pyerr);
        tally.check(&format!("{name}: stderr"), rust, python);
        let mut left = BTreeMap::new();
        fn visit(root: &Path, dir: &Path, left: &mut BTreeMap<String, String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.file_name().unwrap() == ".git" {
                    continue;
                }
                if path.is_dir() {
                    visit(root, &path, left);
                } else {
                    left.insert(
                        path.strip_prefix(root).unwrap().display().to_string(),
                        std::fs::read_to_string(path).unwrap(),
                    );
                }
            }
        }
        visit(&root, &root, &mut left);
        tally.check(&format!("{name}: files"), left, map(field(case, "left")));
    }
    tally.finish();
}

#[test]
fn queue_commands_match_python() {
    replay("queue.json", "baseline");
}
