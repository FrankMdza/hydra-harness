//! `hydra doctor` and `hydra doctor --walk`, build ticket 126.

mod common;

use std::process::Command;

use serde_json::json;

use common::{Record, Sandbox, Shape, policies};

#[test]
fn every_provider_skips_replay_when_any_host_leaf_differs() {
    let sandbox = Sandbox::new("doctor-host-leaves");
    let layout = sandbox.layout();
    let agy = Record::shaped(Shape {
        provider: Some("agy"),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        ..Shape::default()
    })
    .agy_init(&["view_file"])
    .agy_tool_ok(
        1,
        "view_file",
        json!({"AbsolutePath": "/foreign/file"}),
        "ok",
    )
    .agy_result("SUCCESS", "ok", None)
    .closed();
    let templates = [
        include_str!("fixtures/records/claude@2.1.261/00000000-0000-7000-8000-000000000001.jsonl"),
        include_str!(
            "fixtures/records/cursor-agent@2026.09.02-c22c1a3/00000000-0000-7000-8000-000000000007.jsonl"
        ),
        agy.as_str(),
        include_str!("fixtures/records/codex@0.154.0/00000000-0000-7000-8000-000000000009.jsonl"),
    ];
    for template in templates {
        let (first, rest) = template.split_once('\n').unwrap();
        let envelope: serde_json::Value = serde_json::from_str(first).unwrap();
        for leaf in ["os", "arch", "machine"] {
            let mut stamped = envelope.clone();
            let mut host = serde_json::to_value(hydra::record::Host::here().unwrap()).unwrap();
            host[leaf] = json!(format!("{}-foreign", host[leaf].as_str().unwrap()));
            stamped["hydra"]["host"] = host;
            let text = format!("{stamped}\n{rest}");
            let run = hydra::normalize::read_with(&text, &policies(), &layout).unwrap();
            assert!(run.tool_calls().count() > 0);
            assert_eq!(
                run.permission_decisions().count(),
                0,
                "{:?} {leaf}",
                run.provider
            );
            assert!(
                !run.findings.iter().any(|finding| matches!(
                    finding,
                    hydra::normalize::Finding::GateLeak { .. }
                        | hydra::normalize::Finding::Disagreement { .. }
                        | hydra::normalize::Finding::Ungated { .. }
                )),
                "{:?} {leaf}: {:?}",
                run.provider,
                run.findings
            );
        }
        let mut local = envelope;
        local["hydra"]["host"] =
            serde_json::to_value(hydra::record::Host::here().unwrap()).unwrap();
        let run =
            hydra::normalize::read_with(&format!("{local}\n{rest}"), &policies(), &layout).unwrap();
        assert!(run.permission_decisions().count() > 0, "{:?}", run.provider);
        if run.provider.as_deref() == Some("claude") {
            assert!(
                run.permission_decisions()
                    .all(|decision| decision.decision == hydra::normalize::Decision::Deny)
            );
            assert_eq!(
                hydra::outcome::derive(&run).unwrap().outcome,
                hydra::outcome::Outcome::Gated
            );
        }
    }
}

#[test]
fn a_local_host_has_zero_foreign_counts_and_foreign_records_still_need_the_contract() {
    let sandbox = Sandbox::new("doctor-host-contract");
    let layout = sandbox.layout();
    let id = "00000000-0000-7000-8000-000000000337";
    let host = hydra::record::Host::here().unwrap();
    let mut record = Record::shaped(Shape {
        run_id: id,
        ..Shape::default()
    });
    let text = record
        .host(&host)
        .result(json!({"subtype": "success", "is_error": false}))
        .closed();
    let path = layout.run_file(id);
    std::fs::write(&path, &text).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("foreign hosts: 0"), "{stdout}");
    assert!(
        stdout.contains(&format!("host {}: 1 record(s)", host.label())),
        "{stdout}"
    );

    let (first, rest) = text.split_once('\n').unwrap();
    let mut first: serde_json::Value = serde_json::from_str(first).unwrap();
    first["hydra"]["host"]["machine"] = json!(format!("{}-foreign", host.machine));
    first["seq"] = json!(99);
    std::fs::write(&path, format!("{first}\n{rest}")).unwrap();
    let report = hydra::walk::walk_paths(&[path], &|_| panic!("foreign replay"));
    assert_eq!(report.foreign_hosts.values().sum::<usize>(), 1);
    assert!(!report.findings.is_empty());
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.assertion.contains("contract"))
    );
}

#[test]
fn walk_counts_foreign_hosts_without_replaying_their_calls() {
    let sandbox = Sandbox::new("doctor-other-host");
    let layout = sandbox.layout();
    let mut foreign = hydra::record::Host::here().unwrap();
    foreign.machine.push_str("-foreign");
    let id = "00000000-0000-7000-8000-000000000336";
    let mut record = Record::shaped(Shape {
        run_id: id,
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        ..Shape::default()
    });
    let text = record
        .host(&foreign)
        .tool_use(
            "foreign-call",
            "Write",
            json!({"file_path": "/foreign/file", "content": "x"}),
        )
        .tool_result("foreign-call", "ok", false)
        .result(json!({"subtype": "success", "is_error": false}))
        .closed();
    std::fs::write(layout.run_file(id), &text).unwrap();
    let run = hydra::normalize::read_with(&text, &policies(), &layout).unwrap();
    assert_eq!(run.tool_calls().count(), 1);
    assert_eq!(run.permission_decisions().count(), 0);
    assert!(run.findings.iter().any(|finding| matches!(
        finding, hydra::normalize::Finding::Limit(said) if said.contains(&foreign.label())
    )));
    // Foreign records must not even reach the walk's replay reader.
    let report = hydra::walk::walk_paths(&[layout.run_file(id)], &|_| {
        panic!("a foreign record reached the replay reader")
    });
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    assert_eq!(report.foreign_hosts[&foreign.label()], 1);
    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(
        stdout.contains(&format!("foreign host {}: 1 record(s)", foreign.label())),
        "{stdout}"
    );
    assert!(stdout.contains("foreign hosts: 1"), "{stdout}");
    assert!(
        stdout.contains("RejectedBeforeHook: 0; Ungated: 0"),
        "{stdout}"
    );
    assert_eq!(std::fs::read_to_string(layout.run_file(id)).unwrap(), text);
}

#[test]
fn walk_counts_an_edit_rejected_before_the_hook_separately() {
    let sandbox = Sandbox::new("doctor-rejected-before-hook");
    let layout = sandbox.layout();
    let id = "00000000-0000-7000-8000-000000000334";
    let text =
        include_str!("fixtures/records/claude@2.1.261/00000000-0000-7000-8000-000000000334.jsonl");
    std::fs::write(layout.run_file(id), text).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    println!("{stdout}");
    assert!(out.status.success(), "{stdout}");
    assert!(
        stdout.contains("RejectedBeforeHook: 1; Ungated: 0"),
        "{stdout}"
    );
    assert!(stdout.contains("checked 1 record(s)"), "{stdout}");
}

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
fn walk_passes_a_harvested_codex_session_that_hit_its_limit() {
    // Build ticket 260: a harvested `codex` limit is an `event_msg` of type
    // `task_complete` with `codex_error_info: "usage_limit_exceeded"`, not
    // `turn.failed`. Without the session shape the walk expected `completed`.
    // Every line here is synthetic.
    let sandbox = Sandbox::new("doctor-walk-harvested-codex-limit");
    let layout = sandbox.layout();
    let run_id = "00000000-0000-7000-8000-000000000096";
    std::fs::create_dir_all(layout.worktree(run_id)).unwrap();
    let mut record = Record::shaped(Shape {
        run_id,
        provider: Some("codex"),
        launch_mode: "interactive",
        worktree: Some(&layout.worktree(run_id)),
        ..Shape::default()
    });
    record.raw(json!({"type": "session_meta", "payload": {
        "id": "s", "source": "cli", "originator": "codex-tui",
    }}));
    record.raw(json!({"type": "event_msg", "payload": {"type": "task_started"}}));
    record.raw(json!({"type": "response_item", "payload": {
        "type": "message", "role": "user",
        "content": [{"type": "input_text", "text": "look around"}],
    }}));
    record.raw(json!({"type": "event_msg", "payload": {
        "type": "token_count", "info": null,
    }}));
    record.raw(json!({"type": "event_msg", "payload": {
        "type": "task_complete",
        "last_agent_message": null,
        "error": {
            "message": "You've hit your usage limit. Try again later.",
            "codex_error_info": "usage_limit_exceeded",
        },
    }}));
    let text = record.closed_with(json!({"exit_code": 0, "body_source": "harvest"}));
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
fn walk_corroborates_each_gate_refusal_row_against_the_call_it_names() {
    // Build ticket 332. Four rows on one synthetic record: one names a call
    // that failed, one names no call, one names a call that succeeded and one
    // has no call id. The middle two are reported as limits of the record and
    // the walk still passes, since the run's shell can write the sidecar.
    let sandbox = Sandbox::new("doctor-walk-refusals");
    let layout = sandbox.layout();
    let run_id = "00000000-0000-7000-8000-000000000332";
    std::fs::create_dir_all(layout.worktree(run_id)).unwrap();
    let row = |call_id: serde_json::Value, command: &str| {
        json!({
            "type": "gate_refusal",
            "tool": "Bash",
            "call_id": call_id,
            "command": command,
            "command_truncated": false,
            "reason": "hydra/capability",
        })
    };
    let mut record = Record::shaped(Shape {
        run_id,
        worktree: Some(&layout.worktree(run_id)),
        ..Shape::default()
    });
    record
        .tool_use("toolu_refused", "Bash", json!({"command": "rm -rf build"}))
        .tool_result("toolu_refused", "refused", true)
        .tool_use("toolu_ran", "Bash", json!({"command": "ls"}))
        .tool_result("toolu_ran", "Cargo.toml", false)
        .result(json!({"subtype": "success", "is_error": false}));
    record.line("hydra", row(json!("toolu_refused"), "rm -rf build"));
    record.line("hydra", row(json!("toolu_ghost"), "curl forged.invalid"));
    record.line("hydra", row(json!("toolu_ran"), "ls"));
    record.line("hydra", row(json!(null), "echo no id"));
    let path = layout.run_file(run_id);
    std::fs::write(&path, record.closed()).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["doctor", "--walk"])
        .env("HYDRA_ROOT", layout.root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(
        stdout.contains(
            "gate_refusal rows: 1 corroborated, 2 uncorroborated, 1 with no frame, 1 with no call_id"
        ),
        "{stdout}"
    );
    let notes: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("Uncorroborated"))
        .collect();
    assert_eq!(notes.len(), 2, "{stdout}");
    assert!(
        notes
            .iter()
            .all(|note| note.starts_with(path.to_str().unwrap()))
    );
    assert!(
        notes[0].contains("\"toolu_ghost\" matches no tool call")
            && notes[0].contains("\"curl forged.invalid\""),
        "{stdout}"
    );
    assert!(
        notes[1].contains("\"toolu_ran\" matches a call whose result is a success")
            && notes[1].contains("command \"ls\""),
        "{stdout}"
    );
    assert!(!stdout.contains("toolu_refused"), "{stdout}");
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

#[test]
fn doctor_prints_unmeasured_when_the_platform_has_no_row() {
    // Build ticket 317. `HYDRA_DRIFT_OS` and `HYDRA_DRIFT_ARCH` force the
    // table lookup off the four macos aarch64 rows so doctor cannot treat a
    // Linux install as drifted from a Mac measurement.
    let sandbox = Sandbox::new("doctor-unmeasured");
    let layout = sandbox.layout();
    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .arg("doctor")
        .env("HYDRA_ROOT", layout.root())
        .env("HYDRA_DRIFT_OS", "linux")
        .env("HYDRA_DRIFT_ARCH", "x86_64")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    for binary in ["claude", "cursor-agent", "agy", "codex"] {
        let line = format!("{binary} unmeasured on linux x86_64; run tests/drift.rs there");
        assert!(stdout.contains(&line), "missing {line}: {stdout}");
        assert!(
            !stdout.contains(&format!("{binary} missing measured")),
            "unmeasured is not drift: {stdout}"
        );
        assert!(
            !stdout
                .lines()
                .any(|line| { line.starts_with(binary) && line.contains(" measured ") }),
            "unmeasured must not compare against a row: {stdout}"
        );
    }
}
