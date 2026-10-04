//! `hydra export`, harness spec 14.
//!
//! A scratch layout, a two-entry allowlist, four synthetic records, and no read
//! of `~/hydra/runs/`.

use std::path::{Path, PathBuf};

use hydra::export::{Reason, audit, export, export_with_signoff};
use hydra::export_preference::{JudgeDisposition, SignoffManifest, read_judge_pairs};
use hydra::paths::Layout;
use serde_json::json;

mod common;
use common::{Record, Shape, policies, scopes_of};

struct Fixture {
    root: PathBuf,
    repo_a: PathBuf,
    repo_b: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("hydra-export-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo_a = root.join("repo-a");
        let repo_b = root.join("repo-b");
        std::fs::create_dir_all(&repo_a).unwrap();
        std::fs::create_dir_all(&repo_b).unwrap();
        let fixture = Self {
            root,
            repo_a,
            repo_b,
        };
        fixture.layout().ensure().unwrap();
        fixture
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }

    fn allowlist_path(&self) -> PathBuf {
        self.root.join("export-allowlist.json")
    }

    fn write_allowlist(&self) {
        let text = serde_json::to_string(&vec![
            self.repo_a.to_string_lossy().to_string(),
            self.repo_b.to_string_lossy().to_string(),
        ])
        .unwrap();
        std::fs::write(self.allowlist_path(), text).unwrap();
    }

    fn worktree(&self, run_id: &str) -> PathBuf {
        let path = self.layout().worktree(run_id);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write(&self, run_id: &str, text: &str) {
        std::fs::write(self.layout().run_file(run_id), text).unwrap();
    }

    fn allowlist(&self) -> hydra::export::Allowlist {
        hydra::export::Allowlist::load(&self.allowlist_path()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const ELIGIBLE: &str = "01a08080-0000-7000-8000-00000000e001";
const OUTSIDE: &str = "01a08080-0000-7000-8000-00000000e002";
const PARENT: &str = "01a08080-0000-7000-8000-00000000e003";
const CHILD: &str = "01a08080-0000-7000-8000-00000000e004";
const SECRET: &str = "01a08080-0000-7000-8000-00000000e005";

fn read_in_repo(fixture: &Fixture, run_id: &str, parent: Option<&str>, file: &Path) -> String {
    let wt = fixture.worktree(run_id);
    let mut record = Record::shaped(Shape {
        run_id,
        parent_run_id: parent,
        provider: Some("claude"),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&wt),
        scopes: Some(scopes_of(&fixture.layout(), run_id, "standard", &wt)),
        ..Shape::default()
    });
    record
        .tool_use(
            "toolu_01",
            "Read",
            json!({"file_path": file.to_string_lossy()}),
        )
        .tool_result("toolu_01", "bytes\n", false)
        .closed()
}

fn seed_corpus(fixture: &Fixture) {
    let in_a = fixture.repo_a.join("in-repo.txt");
    std::fs::write(&in_a, "hello\n").unwrap();
    fixture.write(ELIGIBLE, &read_in_repo(fixture, ELIGIBLE, None, &in_a));

    let outside = fixture.root.join("outside.txt");
    std::fs::write(&outside, "secret\n").unwrap();
    fixture.write(OUTSIDE, &read_in_repo(fixture, OUTSIDE, None, &outside));

    fixture.write(PARENT, &read_in_repo(fixture, PARENT, None, &in_a));
    let child_outside = fixture.root.join("child-outside.txt");
    std::fs::write(&child_outside, "nope\n").unwrap();
    fixture.write(
        CHILD,
        &read_in_repo(fixture, CHILD, Some(PARENT), &child_outside),
    );

    let flagged = read_in_repo(fixture, SECRET, None, &in_a);
    let flagged = flagged.replace(
        "\"secret_flag\":null",
        "\"secret_flag\":[\"anthropic_api_key\"]",
    );
    fixture.write(SECRET, &flagged);
}

#[test]
fn only_the_eligible_run_is_written() {
    let fixture = Fixture::new("written");
    fixture.write_allowlist();
    seed_corpus(&fixture);

    let out = fixture.root.join("out.jsonl");
    export(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
    )
    .unwrap();

    let text = std::fs::read_to_string(&out).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2, "header plus one run");
    assert!(lines[0].starts_with("export count=1 "));
    assert!(lines[0].contains("allowlist_sha="));
    assert!(lines[0].contains("residue=customer_data_in_fixture"));
    assert!(lines[1].contains("tool_call"));
    assert!(!lines[1].contains(OUTSIDE));
    assert!(!lines[1].contains(PARENT));
    assert!(!lines[1].contains(CHILD));
    assert!(!lines[1].contains(SECRET));
}

#[test]
fn dry_run_names_each_reason() {
    let fixture = Fixture::new("dry");
    fixture.write_allowlist();
    seed_corpus(&fixture);

    let verdicts = audit(&fixture.layout(), &fixture.allowlist(), &policies()).unwrap();
    let said = verdicts
        .iter()
        .map(|verdict| {
            if verdict.eligible() {
                format!("{}: eligible", verdict.run_id)
            } else {
                let names = verdict
                    .reasons
                    .iter()
                    .map(Reason::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: {}", verdict.run_id, names)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(said.contains(&format!("{ELIGIBLE}: eligible")));
    assert!(said.contains(&format!("{OUTSIDE}: path_outside_allowlist")));
    assert!(said.contains(&format!("{CHILD}: path_outside_allowlist")));
    assert!(said.contains(&format!("{PARENT}: child_ineligible")));
    assert!(said.contains(&format!("{SECRET}: secret_flag")));
}

#[test]
fn a_missing_allowlist_refuses() {
    let fixture = Fixture::new("missing");
    let err = export(&fixture.layout(), &fixture.allowlist_path(), None, true)
        .unwrap_err()
        .to_string();
    assert!(err.contains("config/export-allowlist.json"), "{err}");
}

#[test]
fn a_planted_secret_aborts_with_nothing_written() {
    let err = hydra::export::verify_export_line(
        r#"{"events":[{"type":"message","text":"sk-ant-planted-secret"}]}"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("verification scan"), "{err}");

    let fixture = Fixture::new("abort");
    fixture.write_allowlist();
    let in_a = fixture.repo_a.join("in-repo.txt");
    std::fs::write(&in_a, "hello\n").unwrap();
    fixture.write(ELIGIBLE, &read_in_repo(&fixture, ELIGIBLE, None, &in_a));

    let out = fixture.root.join("abort.jsonl");
    export(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
    )
    .unwrap();
    assert!(out.exists());
}

/// `read_in_repo` with its task replaced, map 113.
fn tasked(fixture: &Fixture, run_id: &str, file: &Path, task: &str) -> String {
    read_in_repo(fixture, run_id, None, file)
        .replace(r#""task":"probe""#, &format!(r#""task":{}"#, json!(task)))
}

const TASK_INSIDE: &str = "01a08080-0000-7000-8000-00000000t001";
const TASK_OUTSIDE: &str = "01a08080-0000-7000-8000-00000000t002";
const TASK_HOME: &str = "01a08080-0000-7000-8000-00000000t003";

#[test]
fn a_path_the_task_names_counts_toward_eligibility() {
    let fixture = Fixture::new("task-path");
    fixture.write_allowlist();
    let in_a = fixture.repo_a.join("in-repo.txt");
    std::fs::write(&in_a, "hello\n").unwrap();
    let outside = fixture.root.join("outside.txt");

    fixture.write(
        TASK_INSIDE,
        &tasked(
            &fixture,
            TASK_INSIDE,
            &in_a,
            &format!(
                "Don't rename `{}`. Keep <code>x</code>, // and / alone, and 2>/dev/null.",
                in_a.display()
            ),
        ),
    );
    fixture.write(
        TASK_OUTSIDE,
        &tasked(
            &fixture,
            TASK_OUTSIDE,
            &in_a,
            &format!("Don't touch `{}`, it's private.", outside.display()),
        ),
    );
    fixture.write(
        TASK_HOME,
        &tasked(&fixture, TASK_HOME, &in_a, "Read ~/notes.txt first."),
    );

    let verdicts = audit(&fixture.layout(), &fixture.allowlist(), &policies()).unwrap();
    let reasons = |id: &str| {
        verdicts
            .iter()
            .find(|v| v.run_id == id)
            .unwrap()
            .reasons
            .clone()
    };
    assert_eq!(reasons(TASK_INSIDE), vec![]);
    assert_eq!(reasons(TASK_OUTSIDE), vec![Reason::PathOutsideAllowlist]);
    assert_eq!(reasons(TASK_HOME), vec![Reason::PathOutsideAllowlist]);
}

#[test]
fn the_task_leads_the_line_redacted() {
    let fixture = Fixture::new("task-line");
    fixture.write_allowlist();
    let in_a = fixture.repo_a.join("in-repo.txt");
    std::fs::write(&in_a, "hello\n").unwrap();
    let task = "Port the parser from repo-b, key sk-ant-planted-task-secret.";
    fixture.write(ELIGIBLE, &tasked(&fixture, ELIGIBLE, &in_a, task));

    let out = fixture.root.join("out.jsonl");
    export(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
    )
    .unwrap();

    let text = std::fs::read_to_string(&out).unwrap();
    let line: serde_json::Value = serde_json::from_str(text.lines().nth(1).unwrap()).unwrap();
    let first = &line["events"][0];
    assert_eq!(first["type"], "message");
    assert_eq!(first["role"], "user");
    assert_eq!(
        first["text"],
        "Port the parser from [REDACTED:repo_name], key [REDACTED:anthropic_api_key]"
    );
    assert!(!text.contains("repo-b"), "{text}");
    assert!(!text.contains("sk-ant-"), "{text}");

    // The scan reads the task slot too: the same line with the secret left in
    // aborts.
    let mut planted = line.clone();
    planted["events"][0]["text"] = json!(task);
    let err = hydra::export::verify_export_line(&planted.to_string())
        .unwrap_err()
        .to_string();
    assert!(err.contains("verification scan"), "{err}");
}

const PASS: &str = "01a0c5dd-0000-7000-8000-00000000p001";
const PROPOSER: &str = "01a0c5dd-0000-7000-8000-00000000p002";
const JUDGE: &str = "01a0c5dd-0000-7000-8000-00000000p003";

fn git(repo: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_init(repo: &Path) {
    git(repo, &["init", "-q", "-b", "main"]);
    git(repo, &["config", "user.email", "hydra@test"]);
    git(repo, &["config", "user.name", "hydra"]);
}

fn git_commit_all(repo: &Path, message: &str) {
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", message, "--allow-empty"]);
}

fn head(repo: &Path) -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn write_pass_corpus(
    fixture: &Fixture,
    _repo: &Path,
    base_tree: &str,
    proposer_task: &str,
    judge_answer: &str,
    judge_provider: &str,
) {
    let pass = Record::shaped(Shape {
        run_id: PASS,
        provider: None,
        command: "evolve",
        launch_mode: "recorded",
        policy: None,
        policy_sha: None,
        ..Shape::default()
    })
    .closed();
    fixture.write(PASS, &pass);

    let wt = fixture.worktree(PROPOSER);
    let mut proposer = Record::shaped(Shape {
        run_id: PROPOSER,
        parent_run_id: Some(PASS),
        provider: Some("claude"),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&wt),
        base_tree: Some(base_tree),
        ..Shape::default()
    });
    proposer.assistant(json!([{"type": "text", "text": "done"}]));
    let proposer = proposer.closed().replace(
        r#""task":"probe""#,
        &format!(r#""task":{}"#, json!(proposer_task)),
    );
    fixture.write(PROPOSER, &proposer);

    let judge = Record::shaped(Shape {
        run_id: JUDGE,
        parent_run_id: Some(PASS),
        provider: Some(judge_provider),
        policy: Some("judge"),
        policy_sha: Some(policies().sha()),
        ..Shape::default()
    })
    .assistant(json!([{"type": "text", "text": judge_answer}]))
    .closed();
    fixture.write(JUDGE, &judge);
}

fn seed_blocked_pass(fixture: &Fixture, repo: &Path, with_chosen_on_main: bool) {
    git_init(repo);
    std::fs::write(repo.join("src.txt"), "base\n").unwrap();
    git_commit_all(repo, "base");
    let base = head(repo);
    git(
        repo,
        &["checkout", "-q", "-b", &format!("rungs/blocked/{PASS}")],
    );
    std::fs::write(repo.join("src.txt"), "rejected\n").unwrap();
    git_commit_all(
        repo,
        &format!("hydra evolve pass {PASS}\n\nverdict: blocked"),
    );
    git(repo, &["checkout", "-q", "main"]);
    if with_chosen_on_main {
        std::fs::write(repo.join("src.txt"), "chosen\n").unwrap();
        git_commit_all(repo, &format!("hydra evolve pass {PASS}\n\nhuman fix"));
    }
    let answer = r#"{"verdict":"block","findings":[{"item":"self_consistency","severity":"major","note":"gap"}]}"#;
    write_pass_corpus(fixture, repo, &base, "fix src.txt", answer, "claude");
}

fn signoff_at(fixture: &Fixture, entries: &[serde_json::Value]) {
    let config = fixture.root.join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("export-signoff.json"),
        serde_json::to_string(entries).unwrap(),
    )
    .unwrap();
}

#[test]
fn a_blocked_pass_with_no_chosen_half_is_withheld() {
    let fixture = Fixture::new("withheld");
    seed_blocked_pass(&fixture, &fixture.repo_a, false);
    fixture.write_allowlist();
    let signoff = SignoffManifest::load(&fixture.root.join("config/export-signoff.json")).unwrap();
    let dispositions =
        read_judge_pairs(&fixture.layout(), &policies(), &fixture.repo_a, &signoff).unwrap();
    assert_eq!(
        dispositions,
        vec![JudgeDisposition::Withheld {
            pass_id: PASS.to_string()
        }]
    );
}

#[test]
fn a_blocked_pass_with_a_merged_fix_and_a_manifest_entry_exports() {
    let fixture = Fixture::new("exports");
    seed_blocked_pass(&fixture, &fixture.repo_a, true);
    fixture.write_allowlist();
    signoff_at(
        &fixture,
        &[json!({
            "pass_id": PASS,
            "judge_run_id": JUDGE,
            "finding_item": "self_consistency"
        })],
    );
    let signoff = SignoffManifest::load(&fixture.root.join("config/export-signoff.json")).unwrap();
    let dispositions =
        read_judge_pairs(&fixture.layout(), &policies(), &fixture.repo_a, &signoff).unwrap();
    let exported = dispositions
        .iter()
        .filter_map(|item| match item {
            JudgeDisposition::Exported(pair) => Some(pair),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].source, "judge_verdict");
    assert!(
        exported[0].rejected[0]["content"]
            .as_str()
            .unwrap()
            .contains("rejected")
    );
    assert!(
        exported[0].chosen[0]["content"]
            .as_str()
            .unwrap()
            .contains("chosen")
    );

    let out = fixture.root.join("events.jsonl");
    export_with_signoff(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
        Some(&fixture.root.join("config/export-signoff.json")),
    )
    .unwrap();
    let preference = fixture.root.join("preference.jsonl");
    assert!(preference.is_file());
    let text = std::fs::read_to_string(&preference).unwrap();
    assert!(text.contains("\"source\":\"judge_verdict\""));
}

#[test]
fn a_blocked_pass_with_a_merged_fix_and_no_manifest_entry_is_refused() {
    let fixture = Fixture::new("refused");
    seed_blocked_pass(&fixture, &fixture.repo_a, true);
    fixture.write_allowlist();
    let signoff = SignoffManifest::load(&fixture.root.join("config/export-signoff.json")).unwrap();
    let dispositions =
        read_judge_pairs(&fixture.layout(), &policies(), &fixture.repo_a, &signoff).unwrap();
    assert_eq!(
        dispositions,
        vec![JudgeDisposition::Refused {
            pass_id: PASS.to_string()
        }]
    );
}

#[test]
fn a_pass_whose_base_commit_is_gone_is_unreadable() {
    let fixture = Fixture::new("unreadable");
    let repo = &fixture.repo_a;
    git_init(repo);
    std::fs::write(repo.join("src.txt"), "base\n").unwrap();
    git_commit_all(repo, "base");
    git(
        repo,
        &["checkout", "-q", "-b", &format!("rungs/blocked/{PASS}")],
    );
    std::fs::write(repo.join("src.txt"), "rejected\n").unwrap();
    git_commit_all(
        repo,
        &format!("hydra evolve pass {PASS}\n\nverdict: blocked"),
    );
    git(repo, &["checkout", "-q", "main"]);
    std::fs::write(repo.join("src.txt"), "chosen\n").unwrap();
    git_commit_all(repo, &format!("hydra evolve pass {PASS}\n\nhuman fix"));
    let answer = r#"{"verdict":"block","findings":[{"item":"self_consistency","severity":"major","note":"gap"}]}"#;
    // The sha found on 2026-10-03: a pass branch whose base is no longer in
    // the object store. git diff then used to abort the whole export.
    write_pass_corpus(
        &fixture,
        repo,
        "02a6c80784ba72b845a3c3c02da2b82d329d2dd6",
        "fix src.txt",
        answer,
        "claude",
    );
    fixture.write_allowlist();
    let in_a = fixture.repo_a.join("src.txt");
    fixture.write(ELIGIBLE, &read_in_repo(&fixture, ELIGIBLE, None, &in_a));
    signoff_at(
        &fixture,
        &[json!({
            "pass_id": PASS,
            "judge_run_id": JUDGE,
            "finding_item": "self_consistency"
        })],
    );
    let signoff = SignoffManifest::load(&fixture.root.join("config/export-signoff.json")).unwrap();
    let dispositions =
        read_judge_pairs(&fixture.layout(), &policies(), &fixture.repo_a, &signoff).unwrap();
    assert_eq!(
        dispositions,
        vec![JudgeDisposition::Unreadable {
            pass_id: PASS.to_string()
        }]
    );
    assert_eq!(
        dispositions[0].dry_run_reason(),
        Some(("pass_unreadable", PASS))
    );

    let out = fixture.root.join("events.jsonl");
    export_with_signoff(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
        Some(&fixture.root.join("config/export-signoff.json")),
    )
    .unwrap();
    let events = std::fs::read_to_string(&out).unwrap();
    assert!(
        events.starts_with("export count=3 "),
        "the rest of the corpus should still leave: {events}"
    );
    assert!(events.contains("tool_call"), "{events}");
    let preference = std::fs::read_to_string(fixture.root.join("preference.jsonl")).unwrap();
    assert!(
        !preference.contains("judge_verdict"),
        "unreadable pass left: {preference}"
    );
}

#[test]
fn a_gated_judge_is_read() {
    let fixture = Fixture::new("gated");
    seed_blocked_pass(&fixture, &fixture.repo_a, true);
    fixture.write_allowlist();
    signoff_at(
        &fixture,
        &[json!({
            "pass_id": PASS,
            "judge_run_id": JUDGE,
            "finding_item": "self_consistency"
        })],
    );
    let answer = r#"{"verdict":"block","findings":[{"item":"self_consistency","severity":"major","note":"gap"}]}"#;
    let judge = Record::shaped(Shape {
        run_id: JUDGE,
        parent_run_id: Some(PASS),
        provider: Some("cursor-agent"),
        policy: Some("judge"),
        policy_sha: Some(policies().sha()),
        ..Shape::default()
    })
    .tool_use("toolu_01", "Read", json!({"file_path": "/etc/passwd"}))
    .tool_result("toolu_01", "refused by the gate", true)
    .assistant(json!([{"type": "text", "text": answer}]))
    .closed();
    fixture.write(JUDGE, &judge);
    let signoff = SignoffManifest::load(&fixture.root.join("config/export-signoff.json")).unwrap();
    let dispositions =
        read_judge_pairs(&fixture.layout(), &policies(), &fixture.repo_a, &signoff).unwrap();
    assert!(
        dispositions
            .iter()
            .any(|item| matches!(item, JudgeDisposition::Exported(_))),
        "{dispositions:?}"
    );
    let run = hydra::normalize::read_file(&fixture.layout().run_file(JUDGE), &policies()).unwrap();
    assert_ne!(
        hydra::outcome::derive(&run).map(|d| d.outcome),
        Some(hydra::outcome::Outcome::Completed),
        "a judge that only reads through the gate is not completed"
    );
}

#[test]
fn a_pem_block_is_redacted_to_its_footer_and_not_to_its_first_space() {
    let fixture = Fixture::new("pem");
    fixture.write_allowlist();
    let in_a = fixture.repo_a.join("key.pem");
    std::fs::write(&in_a, "x\n").unwrap();
    let wt = fixture.worktree(ELIGIBLE);
    let mut record = Record::shaped(Shape {
        run_id: ELIGIBLE,
        parent_run_id: None,
        provider: Some("claude"),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&wt),
        scopes: Some(scopes_of(&fixture.layout(), ELIGIBLE, "standard", &wt)),
        ..Shape::default()
    });
    // The record itself is written by a test, so the write-time sweep that
    // would flag this run ineligible never ran; that is the preference path's
    // shape too, where a diff is redacted and exported without a flag.
    let body = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEAplanted\nbodyline2\n-----END RSA PRIVATE KEY-----\nafter";
    let text = record
        .tool_use(
            "toolu_01",
            "Read",
            json!({"file_path": in_a.to_string_lossy()}),
        )
        .tool_result("toolu_01", body, false)
        .closed();
    fixture.write(ELIGIBLE, &text);

    let out = fixture.root.join("pem.jsonl");
    export(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
    )
    .unwrap();
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(written.contains("[REDACTED:pem_private_key]"), "{written}");
    assert!(
        !written.contains("MIIEowIBAAKCAQEAplanted"),
        "the key body survived: {written}"
    );
    assert!(
        !written.contains("bodyline2"),
        "the key body survived: {written}"
    );
    assert!(
        !written.contains("PRIVATE KEY"),
        "the header tail survived: {written}"
    );
    assert!(
        written.contains("after"),
        "text after the footer was eaten: {written}"
    );
}

#[test]
fn out_named_preference_jsonl_is_refused_before_anything_is_written() {
    let fixture = Fixture::new("collision");
    fixture.write_allowlist();
    seed_corpus(&fixture);
    let out = fixture.root.join("preference.jsonl");
    let err = export(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("preference.jsonl"), "{err}");
    assert!(!out.exists());
}

#[test]
fn out_naming_the_preference_file_by_another_spelling_is_refused() {
    // Third panel, H41. The name check compares one spelling. On a
    // case-insensitive volume `Preference.jsonl` is the preference file, and a
    // symbolic or hard link to it by any name is too. Each is refused before a
    // byte is written, and the preference file already there survives.
    let fixture = Fixture::new("collision-alias");
    fixture.write_allowlist();
    seed_corpus(&fixture);
    let preference = fixture.root.join("preference.jsonl");
    let kept = "an earlier export's preference file\n";
    std::fs::write(&preference, kept).unwrap();

    let symbolic = fixture.root.join("events-link.jsonl");
    std::os::unix::fs::symlink(&preference, &symbolic).unwrap();
    let hard = fixture.root.join("events-hard.jsonl");
    std::fs::hard_link(&preference, &hard).unwrap();
    let mut aliases = vec![symbolic, hard];

    let folded = fixture.root.join("Preference.jsonl");
    let case_insensitive = folded.exists();
    if case_insensitive {
        aliases.push(folded.clone());
    } else {
        eprintln!("this volume is case sensitive, so Preference.jsonl is its own file here");
    }

    for out in &aliases {
        let err = export(
            &fixture.layout(),
            &fixture.allowlist_path(),
            Some(out),
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("same file"), "{}: {err}", out.display());
        assert_eq!(
            std::fs::read_to_string(&preference).unwrap(),
            kept,
            "{} truncated the preference file",
            out.display()
        );
    }

    // With no preference file yet, the folded spelling is still one file with
    // the sibling the export would write, and the refusal leaves nothing behind.
    if case_insensitive {
        for alias in &aliases {
            std::fs::remove_file(alias).unwrap();
        }
        std::fs::remove_file(&preference).unwrap_or(());
        let err = export(
            &fixture.layout(),
            &fixture.allowlist_path(),
            Some(&folded),
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("same file"), "{err}");
        assert!(!preference.exists() && !folded.exists());
    }
}

const OPAQUE_SECRET: &str = "q7Kx2Vm9Za4Nc8Rt1By6Hw3Df0Lp5JsUeQiWoAsEdFgHjKlZxCvBnM987654321";

/// The entropy warnings the verifier raises on `text` as one exported value.
fn warnings_on(text: &str) -> Vec<hydra::export::EntropyWarning> {
    hydra::export::verify_export_line(&json!({"output": text}).to_string()).unwrap()
}

#[test]
fn an_unknown_opaque_token_exports_and_is_named_in_the_summary_without_its_value() {
    let fixture = Fixture::new("opaque-secret");
    fixture.write_allowlist();
    assert!(hydra::record::secrets::scan(OPAQUE_SECRET).is_none());
    let file = fixture.repo_a.join("in-repo.txt");
    std::fs::write(&file, "hello\n").unwrap();
    let record = read_in_repo(&fixture, ELIGIBLE, None, &file);
    assert_eq!(record.matches("bytes").count(), 1);
    let record = record.replace("bytes", OPAQUE_SECRET);
    fixture.write(ELIGIBLE, &record);
    let out = fixture.root.join("events.jsonl");
    let warnings = export_with_signoff(
        &fixture.layout(),
        &fixture.allowlist_path(),
        Some(&out),
        false,
        Some(&fixture.root.join("absent-signoff.json")),
    )
    .unwrap();
    assert!(out.is_file());
    // Only the planted token is under test. The fixture's own path, the
    // temp directory plus this process's pid, crosses the entropy line on
    // some pids and warns on `arguments.file_path`, which made this flaky.
    let warnings: Vec<_> = warnings
        .into_iter()
        .filter(|w| w.warning.field.ends_with(".output"))
        .collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].source, ELIGIBLE);
    assert_eq!(warnings[0].warning.class, "alphanumeric");
    assert_eq!(warnings[0].warning.len, OPAQUE_SECRET.len());
    let summary = hydra::export::warning_summary(&warnings);
    assert!(summary.starts_with("entropy warnings: 1 "), "{summary}");
    assert!(
        summary.contains(&format!("{ELIGIBLE}: events[")),
        "{summary}"
    );
    assert!(summary.contains(" alphanumeric len=63"), "{summary}");
    assert!(!summary.contains(&OPAQUE_SECRET[..8]), "{summary}");
}

#[test]
fn openai_keys_are_redacted_and_short_sk_in_prose_is_not() {
    let planted_proj = "sk-proj-planted_openai-key-value-0123";
    let planted_legacy = "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD";
    for text in [planted_proj, planted_legacy] {
        let line = json!({"events": [{"output": text}]}).to_string();
        assert!(hydra::export::verify_export_line(&line).is_err(), "{text}");
    }
    for prose in [
        "the sk- prefix in prose is not a key",
        "task-management-system-v2",
        "disk-utilization-report",
    ] {
        assert!(
            hydra::export::verify_export_line(&json!({ "output": prose }).to_string()).is_ok(),
            "{prose}"
        );
    }
}

#[test]
fn widened_secret_classes_abort_the_verifier() {
    let hook = concat!("https://", "hooks.slack.com/services/T00/B00/planted");
    for text in [
        "AKIAPlantedSecretKey",
        "aws_secret_access_key=planted-aws-secret-value",
        "AWS_SECRET_ACCESS_KEY=planted-aws-env-secret",
        "xoxb-planted-slack-bot-token",
        hook,
        "github_pat_plantedtokenvalue",
        "gho_plantedoauthtokenvalue",
    ] {
        let line = json!({"events": [{"output": text}]}).to_string();
        assert!(hydra::export::verify_export_line(&line).is_err(), "{text}");
        let key = json!({"arguments": {text: "value"}}).to_string();
        assert!(hydra::export::verify_export_line(&key).is_err(), "{text}");
    }
}

#[test]
fn a_prefix_secret_and_a_pem_header_still_abort_in_values_and_keys() {
    for text in [
        "-----BEGIN RSA PRIVATE KEY-----\r\nshort-body\r\n-----END RSA PRIVATE KEY-----",
        "-----BEGIN PRIVATE KEY-----\nunfinished",
        "token sk-ant-planted end",
        "xAIzaSyPlanted",
    ] {
        let line = json!({"events": [{"output": text}]}).to_string();
        assert!(hydra::export::verify_export_line(&line).is_err(), "{text}");
        let key = json!({"arguments": {text: "value"}}).to_string();
        assert!(hydra::export::verify_export_line(&key).is_err(), "{text}");
    }
    let escaped = json!({"output": "-----BEGIN PRIVATE KEY-----"})
        .to_string()
        .replace('B', "\\u0042");
    assert!(hydra::export::verify_export_line(&escaped).is_err());
}

#[test]
fn an_opaque_key_warns_as_key_and_does_not_print_itself() {
    let line = json!({"events": [{"arguments": {OPAQUE_SECRET: "value"}}]}).to_string();
    let warnings = hydra::export::verify_export_line(&line).unwrap();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].field, "events[0].arguments.<key>");
}

#[test]
fn verifier_is_quiet_on_prose_uuids_hex_digests_and_low_entropy_values() {
    for text in [
        "Ordinary prose with several words and a useful explanation.",
        "01a08080-0000-7000-8000-00000000e001",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "[REDACTED:pem_private_key]",
        "q7Kx2Vm9Za4Nc8Rt1By6Hw3Df0Lp5Js",
    ] {
        assert!(warnings_on(text).is_empty(), "{text}");
    }
}

#[test]
fn a_preference_pair_with_an_opaque_token_renders_and_warns() {
    let pair = hydra::export_preference::PreferencePair {
        prompt: vec![json!({"content": "fix this"})],
        rejected: vec![json!({"content": OPAQUE_SECRET})],
        chosen: vec![json!({"content": "fixed"})],
        source: "judge_verdict",
    };
    let (lines, warnings) = hydra::export_preference::render_lines_with_warnings(&[pair]).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].source, "preference pair 1 (judge_verdict)");
    assert_eq!(warnings[0].warning.field, "rejected[0].content");
}

const RANDOM_BASE64_40: &str = "ebpzwnY/+hGzXYYWUImHlrvgzXeYPd1aOeZo5KCg";

#[test]
fn a_forty_byte_random_base64_token_with_no_known_prefix_warns() {
    assert!(hydra::record::secrets::scan(RANDOM_BASE64_40).is_none());
    let warnings = warnings_on(&format!("token {RANDOM_BASE64_40} end"));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].field, "output");
    assert_eq!(warnings[0].class, "base64");
    assert_eq!(warnings[0].len, 40);
}

#[test]
fn a_base64_token_whose_slashes_leave_no_24_character_run_is_not_warned() {
    // The cost of sparing paths: this is random base64, but `/` cuts it into
    // runs of at most 20, so the verifier reads it as a path. About one
    // 40-byte base64 token in seven looks like this.
    assert!(warnings_on("itt/yab+2VYVLstNtWx/baECAfoVLc9TbGqRCdXt").is_empty());
}

#[test]
fn a_hydra_worktree_path_raises_no_warning() {
    let path = "/Users/USER/hydra/worktrees/01a0cc32-3831-7d42-b307-0248751bb76c/IMPLEMENTATION.md";
    assert!(warnings_on(path).is_empty());
}

#[test]
fn a_lockfile_integrity_string_raises_no_warning() {
    let digest =
        "WIC3zgtWJbq9sMTDPqdOd4NJYUaMcVysvf2VZfQ5VHbBXgMkYByLeWTvIATBRHjjvbFecNwsykoIjdsFU53qgA==";
    // The bare digest warns, so the prefix is what spares it.
    assert_eq!(warnings_on(digest).len(), 1);
    assert!(warnings_on(&format!("\"integrity\": \"sha512-{digest}\"")).is_empty());
}

#[test]
fn a_data_uri_raises_no_warning() {
    let payload = "ZXilbzs6Wr1gyrqeYUGkfYK2d+sLDJV1fG62jVCN2QzMu4dTzsPOuei28RkVfYbB";
    assert_eq!(warnings_on(payload).len(), 1);
    assert!(warnings_on(&format!("<img src=\"data:image/png;base64,{payload}\">")).is_empty());
}
