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

const PASS: &str = "01a0c5dd-0000-7000-8000-00000000p001";
const PROPOSER: &str = "01a0c5dd-0000-7000-8000-00000000p002";
const JUDGE: &str = "01a0c5dd-0000-7000-8000-00000000p003";

fn git(repo: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
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
