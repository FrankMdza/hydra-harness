//! `hydra export`, harness spec 14.
//!
//! A scratch layout, a two-entry allowlist, four synthetic records, and no read
//! of `~/hydra/runs/`.

use std::path::{Path, PathBuf};

use hydra::export::{Reason, audit, export};
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
