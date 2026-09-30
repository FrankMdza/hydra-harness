//! `hydra doctor` and `hydra doctor --walk`, build ticket 126.

mod common;

use std::process::Command;

use serde_json::json;

use common::{Record, Sandbox, Shape, policies};

#[test]
fn walk_reports_a_failed_run_with_a_terminator_and_exit_zero() {
    let sandbox = Sandbox::new("doctor-walk-bad");
    let layout = sandbox.layout();
    let run_id = "00000000-0000-7000-8000-000000000099";
    std::fs::create_dir_all(layout.worktree(run_id)).unwrap();
    let text = Record::shaped(Shape {
        run_id,
        provider: Some("agy"),
        worktree: Some(&layout.worktree(run_id)),
        ..Shape::default()
    })
    .agy_init(&["view_file"])
    .agy_user()
    .agy_result("ERROR", "stopped", None)
    .closed_with(json!({ "exit_code": 0 }));
    let path = layout.run_file(run_id);
    std::fs::write(&path, text).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "{stdout}");
    assert!(stdout.contains(path.to_str().unwrap()), "{stdout}");
    assert!(stdout.contains("failed with exit 0"), "{stdout}");
    assert!(stdout.contains("checked 1 record(s)"), "{stdout}");
}

#[test]
fn walk_passes_on_a_clean_scratch_corpus() {
    let sandbox = Sandbox::new("doctor-walk-clean");
    let layout = sandbox.layout();
    let run_id = "00000000-0000-7000-8000-000000000098";
    std::fs::create_dir_all(layout.worktree(run_id)).unwrap();
    let text = Record::shaped(Shape {
        run_id,
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&layout.worktree(run_id)),
        ..Shape::default()
    })
    .assistant(json!([{"type": "text", "text": "ok"}]))
    .result(serde_json::json!({"subtype": "success", "is_error": false}))
    .closed();
    std::fs::write(layout.run_file(run_id), text).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("checked 1 record(s)"), "{stdout}");
}

#[test]
fn walk_passes_a_stopped_run_whose_provider_wrote_a_success_terminator() {
    // Build ticket 173: a provider that catches SIGTERM and writes its own
    // terminator on the way out. The record derives `stopped`, which rung one
    // does not learn from, so the per-call rule has nothing to say about it.
    let sandbox = Sandbox::new("doctor-walk-stopped");
    let layout = sandbox.layout();
    let run_id = "00000000-0000-7000-8000-000000000097";
    std::fs::create_dir_all(layout.worktree(run_id)).unwrap();
    let text = Record::shaped(Shape {
        run_id,
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&layout.worktree(run_id)),
        ..Shape::default()
    })
    .assistant(json!([{"type": "text", "text": "stopping"}]))
    .result(json!({"subtype": "success", "is_error": false}))
    .closed_with(json!({
        "exit_code": 143,
        "error": format!("{}; the child died on signal SIGTERM", hydra::watch::STOP_NOTE),
    }));
    std::fs::write(layout.run_file(run_id), text).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("checked 1 record(s)"), "{stdout}");
}

#[test]
fn walk_passes_a_stopped_codex_run_that_ended_on_turn_failed() {
    // Build ticket 175, from a codex review of 173: a `codex` run stopped as it
    // wrote a non-quota `turn.failed`. It derives `stopped`, and the walk's
    // `turn.failed` rule must not ask it for `failed`.
    let sandbox = Sandbox::new("doctor-walk-stopped-codex");
    let layout = sandbox.layout();
    let run_id = "00000000-0000-7000-8000-000000000098";
    std::fs::create_dir_all(layout.worktree(run_id)).unwrap();
    let text = Record::shaped(Shape {
        run_id,
        provider: Some("codex"),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&layout.worktree(run_id)),
        ..Shape::default()
    })
    .raw(json!({"type": "turn.failed", "error": {"message": "the model stopped"}}))
    .closed_with(json!({
        "exit_code": 143,
        "error": format!("{}; the child died on signal SIGTERM", hydra::watch::STOP_NOTE),
    }));
    std::fs::write(layout.run_file(run_id), text).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("checked 1 record(s)"), "{stdout}");
}

#[test]
fn doctor_lists_the_panel_shapes_beside_the_other_protected_configs() {
    // 15.1 #6, build ticket 138. The home is a scratch one, so the line reads
    // a file this test wrote and never the checkout's own `config/`.
    let sandbox = Sandbox::new("doctor-panels");
    let layout = sandbox.layout();
    let home = layout.root().join("home");
    let panels = home.join("config/panels.json");
    std::fs::create_dir_all(panels.parent().unwrap()).unwrap();

    let doctor = || {
        let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
            .arg("doctor")
            .env("HYDRA_ROOT", layout.root())
            .env("HYDRA_HOME", &home)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let stdout = doctor();
    let line = format!("approved panel shapes {} missing", panels.display());
    assert!(stdout.contains(&line), "{stdout}");
    assert!(
        stdout.contains("canonical MCP document"),
        "the other five are no longer listed: {stdout}"
    );

    std::fs::write(&panels, "{\"shapes\": []}").unwrap();
    let stdout = doctor();
    let line = format!("approved panel shapes {} present", panels.display());
    assert!(stdout.contains(&line), "{stdout}");
}
