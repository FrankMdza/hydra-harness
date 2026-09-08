//! `hydra show <run-id> [--events]`, harness spec 17's full record.
//!
//! One synthetic `claude` run, built the way `tests/normalize.rs` builds one:
//! a message, a thinking block, and a tool call the policy denies, its result
//! and the gate's own denial riding along with it. What is under test here is
//! the shape `hydra show` prints, not the normalizer, so the fixture is kept
//! to the smallest run that exercises every event kind a real one produces.

use serde_json::{Value, json};

mod common;
use common::{RUN_ID, Record, Sandbox, policies};

/// Writes the fixture record to the sandbox's run file and returns its layout.
///
/// `closed` chooses between a run that ended and one Hydra was killed during,
/// which is the second half of the ticket: a record with no `run_end` still
/// has to print what it has.
fn write_fixture(sandbox: &Sandbox, closed: bool) -> hydra::paths::Layout {
    let layout = sandbox.layout();
    let path = sandbox.worktree().join("secret.txt");
    let mut record =
        Record::opened(&sandbox.worktree(), Some("read-only"), Some(policies().sha()));
    record
        .assistant(json!([
            {"type": "text", "text": "Let me try that."},
            {"type": "thinking", "thinking": "This will get denied.", "signature": "SIGXYZ"},
        ]))
        .tool_use("toolu_01", "Write", json!({"file_path": path, "content": "secret\n"}))
        .hook_response(
            "Write",
            "deny",
            "hydra/capability: policy read-only does not allow file_write (Write).",
        )
        .tool_result(
            "toolu_01",
            "hydra/capability: policy read-only does not allow file_write (Write).",
            true,
        );
    let text = if closed { record.closed() } else { record.unclosed() };
    std::fs::write(layout.run_file(RUN_ID), text).unwrap();
    layout
}

#[test]
fn events_lists_every_event_in_file_order_with_its_type_name() {
    let sandbox = Sandbox::new("show-events");
    let layout = write_fixture(&sandbox, true);

    let mut out = Vec::new();
    hydra::show::show(&layout, RUN_ID, true, &mut out).unwrap();
    let report: Value = serde_json::from_slice(&out).unwrap();

    assert_eq!(report["run_id"], RUN_ID);

    let events = report["events"].as_array().unwrap();
    let type_names: Vec<&str> = events
        .iter()
        .map(|event| event["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        type_names,
        vec![
            "run_start",
            "message",
            "thinking",
            "tool_call",
            "permission_decision",
            "tool_result",
            "usage",
            "run_end",
        ],
        "{events:#?}"
    );

    let message = &events[1];
    assert_eq!(message["role"], "assistant");
    assert_eq!(message["text"], "Let me try that.");
    assert_eq!(message["block"], 0);

    let thinking = &events[2];
    assert_eq!(thinking["text"], "This will get denied.");
    assert_eq!(thinking["signed"], true);
    assert!(thinking.get("signature").is_none(), "{thinking}");

    let call = &events[3];
    assert_eq!(call["call_id"], "toolu_01");
    assert_eq!(call["name"], "Write");
    assert_eq!(call["capability"], "file_write");

    let decision = &events[4];
    assert_eq!(decision["call_id"], "toolu_01");
    assert_eq!(decision["decision"], "deny");
    assert_eq!(decision["decider"], "hydra_policy");
    assert!(
        decision["reason"]
            .as_str()
            .unwrap()
            .starts_with("hydra/capability:"),
        "{decision}"
    );

    let result = &events[5];
    assert_eq!(result["call_id"], "toolu_01");
    assert_eq!(result["status"], "error");
    assert_eq!(
        result["output"],
        "hydra/capability: policy read-only does not allow file_write (Write)."
    );
    assert_eq!(result["truncated"], false);

    let usage = &events[6];
    assert_eq!(usage["source"], "unavailable");
    assert!(usage["counts"].is_null(), "{usage}");

    // The record carries no `scopes`, so the reader says once that it rebuilt
    // the paths the decision above was resolved against. Nothing else about
    // this run is worth a finding.
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert_eq!(findings[0]["type"], "limit");
}

#[test]
fn without_events_it_prints_only_the_projection() {
    let sandbox = Sandbox::new("show-projection");
    let layout = write_fixture(&sandbox, true);

    let mut out = Vec::new();
    hydra::show::show(&layout, RUN_ID, false, &mut out).unwrap();
    let printed: Value = serde_json::from_slice(&out).unwrap();

    // The same object `hydra status <id>` prints under `result`, and nothing
    // beside it.
    let projection = hydra::status::projection(&layout, RUN_ID).unwrap();
    assert_eq!(printed, projection);
    assert_eq!(printed["run_id"], RUN_ID);
    // The gate denied the only call the run made, so nothing got through.
    assert_eq!(printed["outcome"], "gated");
    assert!(printed.get("events").is_none(), "{printed}");
    assert!(printed.get("findings").is_none(), "{printed}");
}

#[test]
fn a_record_with_no_run_end_still_prints_what_it_has() {
    let sandbox = Sandbox::new("show-unclosed");
    let layout = write_fixture(&sandbox, false);

    let mut out = Vec::new();
    hydra::show::show(&layout, RUN_ID, true, &mut out).unwrap();
    let report: Value = serde_json::from_slice(&out).unwrap();

    // No `run_end` means no state this command calls ended, so the projection
    // is the same null a live or abandoned run gets from `hydra status`.
    assert!(report["projection"].is_null(), "{report}");

    let events = report["events"].as_array().unwrap();
    let type_names: Vec<&str> = events
        .iter()
        .map(|event| event["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        type_names,
        vec![
            "run_start",
            "message",
            "thinking",
            "tool_call",
            "permission_decision",
            "tool_result",
            "usage",
        ],
        "a record cut before run_end still normalizes everything before the cut, \
         usage included since contract 7.8 gives it no bare null even here: {events:#?}"
    );
}
