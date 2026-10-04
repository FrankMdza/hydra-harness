//! `hydra cost`, build ticket 274.
//!
//! A scratch layout and hand-built records. No orchestrator and no goal is
//! launched. Agents are runs that started after a named orchestrator, in its
//! repository; a later named orchestrator in the same repository closes the
//! window.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::cost::{self, CostRequest};
use hydra::paths::Layout;
use serde_json::{Value, json};

mod common;
use common::{Record, Shape, policies, scopes_of};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "hydra-cost-{}-{}-{}",
            std::process::id(),
            name,
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let fixture = Self { root };
        fixture.layout().ensure().unwrap();
        fixture
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const ORCH_A: &str = "01a02740-0000-7000-8000-00000000aa01";
const AGENT_A1: &str = "01a02740-0000-7000-8000-00000000aa11";
const AGENT_A2: &str = "01a02740-0000-7000-8000-00000000aa12";
const BEFORE: &str = "01a02740-0000-7000-8000-00000000aa00";
const OTHER: &str = "01a02740-0000-7000-8000-00000000bb01";
const ORCH_B: &str = "01a02740-0000-7000-8000-00000000bb02";
const AGENT_B: &str = "01a02740-0000-7000-8000-00000000bb12";
const HARNESS: &str = "01a02740-0000-7000-8000-00000000cc01";
const CHECK: &str = "01a02740-0000-7000-8000-00000000dd01";
const CHECK_AGENT: &str = "01a02740-0000-7000-8000-00000000dd11";

const REPO_A: &str = "/tmp/tally-a";
const REPO_B: &str = "/tmp/tally-b";
const REPO_C: &str = "/tmp/tally-c";

fn usage_tokens(total: u64) -> Value {
    json!({"usage": {
        "input_tokens": total,
        "cache_creation_input_tokens": 0,
        "cache_read_input_tokens": 0,
        "output_tokens": 0,
    }})
}

fn with_ts(text: &str, ts: &str) -> String {
    text.replace("2026-09-05T20:51:39.370Z", ts)
}

fn plant(
    fixture: &Fixture,
    run_id: &str,
    ts: &str,
    repo: &str,
    tokens: u64,
    cwd_only: bool,
    provider: Option<&str>,
) {
    let layout = fixture.layout();
    let worktree = layout.worktree(run_id);
    std::fs::create_dir_all(&worktree).unwrap();
    let mut scopes = scopes_of(&layout, run_id, "standard", &worktree);
    if !cwd_only {
        scopes["parent_git_dir"] = json!(format!("{repo}/.git"));
    }
    let policies = policies();
    let sha = policies.sha();
    let mut record = Record::shaped(Shape {
        run_id,
        provider,
        launch_mode: "recorded",
        policy: Some("standard"),
        policy_sha: Some(sha),
        worktree: if cwd_only { None } else { Some(&worktree) },
        scopes: if cwd_only { None } else { Some(scopes) },
        ..Shape::default()
    });
    let mut text = with_ts(&record.result(usage_tokens(tokens)).closed(), ts);
    if cwd_only {
        text = set_cwd(&text, repo);
    }
    std::fs::write(layout.run_file(run_id), text).unwrap();
}

fn set_cwd(text: &str, cwd: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().expect("a record has line 0");
    let mut envelope: Value = serde_json::from_str(first).unwrap();
    envelope["hydra"]["cwd"] = json!(cwd);
    let mut out = envelope.to_string();
    for line in lines {
        out.push('\n');
        out.push_str(line);
    }
    out.push('\n');
    out
}

fn report(fixture: &Fixture, runs: &[&str], check: &[&str]) -> String {
    let mut out = Vec::new();
    cost::command(
        &fixture.layout(),
        CostRequest {
            runs: runs.iter().map(|id| id.to_string()).collect(),
            check: check.iter().map(|id| id.to_string()).collect(),
        },
        &mut out,
    )
    .unwrap();
    String::from_utf8(out).unwrap()
}

fn plant_base(fixture: &Fixture) {
    plant(
        fixture,
        ORCH_A,
        "2026-10-01T10:00:00Z",
        REPO_A,
        1000,
        true,
        Some("claude"),
    );
    plant(
        fixture,
        AGENT_A1,
        "2026-10-01T10:05:00Z",
        REPO_A,
        400,
        false,
        Some("claude"),
    );
    plant(
        fixture,
        AGENT_A2,
        "2026-10-01T10:10:00Z",
        REPO_A,
        600,
        false,
        Some("claude"),
    );
    plant(
        fixture,
        BEFORE,
        "2026-10-01T09:00:00Z",
        REPO_A,
        9999,
        false,
        Some("claude"),
    );
    plant(
        fixture,
        OTHER,
        "2026-10-01T10:06:00Z",
        REPO_B,
        8888,
        false,
        Some("claude"),
    );
    plant(
        fixture,
        HARNESS,
        "2026-10-01T10:07:00Z",
        REPO_A,
        7777,
        false,
        None,
    );
}

fn plant_window(fixture: &Fixture) {
    plant_base(fixture);
    plant(
        fixture,
        ORCH_B,
        "2026-10-01T11:00:00Z",
        REPO_A,
        2000,
        true,
        Some("claude"),
    );
    plant(
        fixture,
        AGENT_B,
        "2026-10-01T11:05:00Z",
        REPO_A,
        500,
        false,
        Some("claude"),
    );
}

#[test]
fn agents_are_runs_that_started_after_the_orchestrator_in_its_repository() {
    let fixture = Fixture::new("match");
    plant_base(&fixture);
    let text = report(&fixture, &[ORCH_A], &[]);
    assert!(
        text.contains(&format!("bench\t{ORCH_A}\t2\t1000\t1000\t1.000\n")),
        "{text}"
    );
    assert!(
        !text.contains(BEFORE) && !text.contains(OTHER) && !text.contains(AGENT_B),
        "{text}"
    );
}

#[test]
fn a_later_orchestrator_in_the_same_repository_closes_the_window() {
    let fixture = Fixture::new("window");
    plant_window(&fixture);
    let text = report(&fixture, &[ORCH_A, ORCH_B], &[]);
    assert!(
        text.contains(&format!("bench\t{ORCH_A}\t2\t1000\t1000\t1.000\n")),
        "{text}"
    );
    assert!(
        text.contains(&format!("bench\t{ORCH_B}\t1\t2000\t500\t4.000\n")),
        "{text}"
    );
    assert!(text.contains("bench\t2\t2.500\t1.000\t4.000\n"), "{text}");
}

#[test]
fn an_interactive_cwd_matches_an_agent_parent_git_dir() {
    let fixture = Fixture::new("cwd");
    plant(
        &fixture,
        ORCH_A,
        "2026-10-01T10:00:00Z",
        REPO_C,
        300,
        true,
        Some("claude"),
    );
    plant(
        &fixture,
        AGENT_A1,
        "2026-10-01T10:05:00Z",
        REPO_C,
        900,
        false,
        Some("claude"),
    );
    let text = report(&fixture, &[ORCH_A], &[]);
    assert!(
        text.contains(&format!("bench\t{ORCH_A}\t1\t300\t900\t0.333\n")),
        "{text}"
    );
}

#[test]
fn real_sessions_print_beside_the_set() {
    let fixture = Fixture::new("check");
    plant_base(&fixture);
    plant(
        &fixture,
        CHECK,
        "2026-10-02T10:00:00Z",
        REPO_B,
        1500,
        true,
        Some("claude"),
    );
    plant(
        &fixture,
        CHECK_AGENT,
        "2026-10-02T10:05:00Z",
        REPO_B,
        500,
        false,
        Some("claude"),
    );
    let text = report(&fixture, &[ORCH_A], &[CHECK]);
    assert!(
        text.contains(&format!("bench\t{ORCH_A}\t2\t1000\t1000\t1.000\n")),
        "{text}"
    );
    assert!(
        text.contains(&format!("check\t{CHECK}\t1\t1500\t500\t3.000\n")),
        "{text}"
    );
    assert!(text.contains("check\t1\t3.000\t3.000\t3.000\n"), "{text}");
}

#[test]
fn a_run_with_no_agents_is_left_out_of_the_median() {
    let fixture = Fixture::new("none");
    plant(
        &fixture,
        ORCH_A,
        "2026-10-01T10:00:00Z",
        REPO_A,
        1000,
        true,
        Some("claude"),
    );
    let text = report(&fixture, &[ORCH_A], &[]);
    assert!(
        text.contains(&format!("bench\t{ORCH_A}\t0\t1000\t0\t\n")),
        "{text}"
    );
    assert!(text.contains("bench\t0\t\t\t\n"), "{text}");
}

#[test]
fn a_missing_run_is_an_error() {
    let fixture = Fixture::new("missing");
    let mut out = Vec::new();
    let err = cost::command(
        &fixture.layout(),
        CostRequest {
            runs: vec!["01a02740-0000-7000-8000-00000000ffff".into()],
            check: Vec::new(),
        },
        &mut out,
    )
    .unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("no run record"), "{msg}");
}
