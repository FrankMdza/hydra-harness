//! Build ticket 288: `hydra-grader originals` joins each human verdict with the
//! run branch's diff on a synthetic repository and a synthetic queue.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use hydra_pyjson::json::{self, Value};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-grader");
const KEPT: &str = "aaaaaaaa-bbbb-4000-8000-000000000001";
const GONE: &str = "aaaaaaaa-bbbb-4000-8000-000000000002";

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "hydra-grader-originals-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "hydra")
        .env("GIT_AUTHOR_EMAIL", "hydra@test")
        .env("GIT_COMMITTER_NAME", "hydra")
        .env("GIT_COMMITTER_EMAIL", "hydra@test")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn git_text(repo: &Path, args: &[&str]) -> String {
    String::from_utf8(git(repo, args).stdout)
        .unwrap()
        .trim()
        .to_string()
}

/// A repo seeded on `main`, with `hydra/<KEPT>` holding one file change, and no
/// `hydra/<GONE>` branch. Returns the seed tree sha.
fn seeded_repo(root: &Path) -> String {
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("hello.txt"), "base\n").unwrap();
    git(&repo, &["add", "hello.txt"]);
    git(&repo, &["commit", "-q", "-m", "seed"]);
    let base = git_text(&repo, &["rev-parse", "HEAD^{tree}"]);
    git(&repo, &["checkout", "-q", "-b", &format!("hydra/{KEPT}")]);
    std::fs::write(repo.join("hello.txt"), "changed\n").unwrap();
    git(&repo, &["add", "hello.txt"]);
    git(&repo, &["commit", "-q", "-m", "agent"]);
    git(&repo, &["checkout", "-q", "main"]);
    base
}

fn write_record(runs: &Path, run_id: &str, base_tree: &str) {
    std::fs::create_dir_all(runs).unwrap();
    let body = format!(
        "{{\"hydra\":{{\"type\":\"run_start\",\"worktree\":{{\"base_tree\":\"{base_tree}\",\"branch\":\"hydra/{run_id}\"}}}},\"provider\":\"codex\",\"model\":\"gpt\",\"run_id\":\"{run_id}\"}}\n"
    );
    std::fs::write(runs.join(format!("{run_id}.jsonl")), body).unwrap();
}

fn write_queue(home: &Path, tickets: &Path, rows: &[(&str, &str)]) {
    std::fs::create_dir_all(home).unwrap();
    std::fs::create_dir_all(tickets).unwrap();
    let mut verdicts = String::from("{");
    let mut dataset = String::new();
    for (i, (run_id, verdict)) in rows.iter().enumerate() {
        if i > 0 {
            verdicts.push_str(", ");
        }
        verdicts.push_str(&format!("\"{run_id}\": \"{verdict}\""));
        dataset.push_str(&format!(
            "{{\"run_id\": \"{run_id}\", \"provider\": \"codex\", \"model\": \"gpt\", \"outcome\": \"completed\"}}\n"
        ));
        std::fs::write(
            tickets.join(format!("20{i}-one.md")),
            format!("Status: resolved\n\n## Outcome\n\nRun {run_id} applied as written.\n"),
        )
        .unwrap();
    }
    verdicts.push_str("}\n");
    std::fs::write(home.join("ticket-verdicts.json"), verdicts).unwrap();
    std::fs::write(home.join("dataset.jsonl"), dataset).unwrap();
}

fn originals(root: &Path, out: &Path) -> Output {
    Command::new(RUST)
        .args([
            "originals",
            "--home",
            root.join("home").to_str().unwrap(),
            "--dataset",
            root.join("home/dataset.jsonl").to_str().unwrap(),
            "--tickets",
            root.join("tickets").to_str().unwrap(),
            "--repo",
            root.join("repo").to_str().unwrap(),
            "--runs",
            root.join("runs").to_str().unwrap(),
            out.to_str().unwrap(),
        ])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root.join("no-home"))
        .env("HYDRA_ROOT", root.join("no-hydra"))
        .env("HYDRA_GRADER_HOME", root.join("home"))
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn lines_of(path: &Path) -> Vec<Value> {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| json::loads_str(line).unwrap())
        .collect()
}

fn field<'a>(row: &'a Value, key: &str) -> &'a Value {
    match row {
        Value::Dict(dict) => dict.get(key).unwrap_or(&Value::Null),
        _ => panic!("row is not an object"),
    }
}

fn text(value: &Value) -> String {
    match value {
        Value::Str(s) => s.to_utf8().unwrap(),
        other => panic!("not text: {other:?}"),
    }
}

#[test]
fn a_verdict_whose_branch_exists_gets_its_diff() {
    let scratch = Scratch::new("kept");
    let root = &scratch.0;
    let base = seeded_repo(root);
    write_queue(
        &root.join("home"),
        &root.join("tickets"),
        &[(KEPT, "applied")],
    );
    write_record(&root.join("runs"), KEPT, &base);
    let out = root.join("out.jsonl");
    let ran = originals(root, &out);
    assert_eq!(
        ran.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let rows = lines_of(&out);
    assert_eq!(rows.len(), 1);
    assert_eq!(text(field(&rows[0], "run_id")), KEPT);
    assert_eq!(text(field(&rows[0], "verdict")), "applied");
    assert_eq!(text(field(&rows[0], "provider")), "codex");
    assert_eq!(text(field(&rows[0], "model")), "gpt");
    assert_eq!(text(field(&rows[0], "ticket")), "200");
    let diff = text(field(&rows[0], "diff"));
    assert!(diff.contains("hello.txt"), "{diff}");
    assert!(diff.contains("changed"), "{diff}");
    match field(&rows[0], "files_changed") {
        Value::Int(1) => {}
        other => panic!("files_changed: {other:?}"),
    }
    match rows[0] {
        Value::Dict(ref dict) => assert!(
            dict.get("missing").is_none() || matches!(dict.get("missing"), Some(Value::Null))
        ),
        _ => panic!("row is not an object"),
    }
}

#[test]
fn a_verdict_whose_branch_is_gone_gets_a_missing_line() {
    let scratch = Scratch::new("gone");
    let root = &scratch.0;
    let base = seeded_repo(root);
    write_queue(
        &root.join("home"),
        &root.join("tickets"),
        &[(GONE, "fixed")],
    );
    write_record(&root.join("runs"), GONE, &base);
    let out = root.join("out.jsonl");
    let ran = originals(root, &out);
    assert_eq!(
        ran.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let rows = lines_of(&out);
    assert_eq!(rows.len(), 1);
    assert_eq!(text(field(&rows[0], "run_id")), GONE);
    assert_eq!(text(field(&rows[0], "verdict")), "fixed");
    assert_eq!(text(field(&rows[0], "missing")), "branch");
}

#[test]
fn the_line_count_equals_the_verdict_count() {
    let scratch = Scratch::new("count");
    let root = &scratch.0;
    let base = seeded_repo(root);
    write_queue(
        &root.join("home"),
        &root.join("tickets"),
        &[(KEPT, "applied"), (GONE, "fixed")],
    );
    write_record(&root.join("runs"), KEPT, &base);
    write_record(&root.join("runs"), GONE, &base);
    let out = root.join("out.jsonl");
    let ran = originals(root, &out);
    assert_eq!(
        ran.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let rows = lines_of(&out);
    assert_eq!(rows.len(), 2);
    assert_eq!(text(field(&rows[0], "run_id")), KEPT);
    assert!(text(field(&rows[0], "diff")).contains("hello.txt"));
    assert_eq!(text(field(&rows[1], "run_id")), GONE);
    assert_eq!(text(field(&rows[1], "missing")), "branch");
}
