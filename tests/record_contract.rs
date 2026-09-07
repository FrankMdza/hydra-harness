//! Does the harness write what the contract says it writes?
//!
//! The validator in `hydra::record::validate` is the instrument. These tests point
//! it at records the writer produced, and at deliberately broken ones, so that a
//! passing report means something.

use std::collections::HashMap;
use std::path::PathBuf;

use hydra::record::validate::check;
use hydra::record::{Auth, RunEnd, RunStart, SCHEMA_VERSION, WorktreeRef, Writer};
use serde_json::value::RawValue;

const RUN_ID: &str = "01996d3c-8f2a-7c31-9b44-2b0f1d8a5e6c";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hydra-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// The bytes of one envelope field, as they sit in the file.
///
/// Reading a line into `serde_json::Value` sorts the keys and re-spells the
/// numbers, which would hide exactly the rewrite these tests look for.
fn field<'a>(line: &'a str, name: &str) -> &'a str {
    let map: HashMap<String, &RawValue> = serde_json::from_str(line).unwrap();
    map.get(name).unwrap().get()
}

fn a_run_start() -> RunStart {
    RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: "0.1.0".into(),
        command: "run".into(),
        launch_mode: Some("recorded".into()),
        provider_version: Some("2.1.261 (Claude Code)".into()),
        provider_session_id: None,
        task: Some("say pong".into()),
        policy: None,
        policy_sha: None,
        cwd: "/Users/USER/hydra/worktrees/x".into(),
        worktree: Some(WorktreeRef {
            path: "/Users/USER/hydra/worktrees/x".into(),
            branch: "hydra/x".into(),
        }),
        depth: 0,
        parent_policy: None,
        pid: Some(4242),
        process_start_time: Some("2026-09-05T14:53:52.239191Z".into()),
        skills: None,
        mcp: None,
        auth: Some(Auth {
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(vec!["HOME".into(), "PATH".into(), "USER".into()]),
        subscription: Some("anthropic".into()),
    }
}

fn write_a_run(path: &PathBuf, frames: &[&str]) {
    let mut writer = Writer::create(path, RUN_ID, None, Some("claude"), None).unwrap();
    writer.hydra_line(&a_run_start()).unwrap();
    for frame in frames {
        writer.provider_line(frame).unwrap();
    }
    writer.hydra_line(&RunEnd::streamed(Some(0), None)).unwrap();
}

#[test]
fn a_record_the_writer_produced_meets_the_contract() {
    let path = scratch("well-formed.jsonl");
    let _ = std::fs::remove_file(&path);
    write_a_run(
        &path,
        &[
            r#"{"type":"system","subtype":"init","model":"claude-sonnet-5","session_id":"s"}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"pong"}]}}"#,
            r#"{"type":"result","subtype":"success","is_error":false}"#,
        ],
    );

    let text = std::fs::read_to_string(&path).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();
    assert_eq!(text.lines().count(), 5);
}

#[test]
fn a_frame_survives_verbatim() {
    let path = scratch("verbatim.jsonl");
    let _ = std::fs::remove_file(&path);

    // Key order that no struct would reproduce, a float that would re-spell, and an
    // integer wider than f64 carries exactly.
    let frame = r#"{"zebra":1,"usage":{"cache_read_input_tokens":8137,"input_tokens":2},"cost":0.0030,"id":12345678901234567890,"nested":[{"b":2,"a":1}]}"#;
    write_a_run(&path, &[frame]);

    let text = std::fs::read_to_string(&path).unwrap();
    let stored = field(text.lines().nth(1).unwrap(), "raw");
    assert_eq!(
        stored, frame,
        "the raw frame was re-serialized rather than stored"
    );
}

#[test]
fn a_line_that_is_not_json_is_kept_rather_than_dropped() {
    let path = scratch("not-json.jsonl");
    let _ = std::fs::remove_file(&path);

    // What `claude` 2.1.261 writes after its last frame on a pty: show-cursor, no
    // newline. Never-discard has no exception for noise.
    write_a_run(&path, &["\u{1b}[?25h"]);

    let text = std::fs::read_to_string(&path).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();
    assert_eq!(
        field(text.lines().nth(1).unwrap(), "raw"),
        "\"\\u001b[?25h\""
    );
}

#[test]
fn a_credential_in_a_frame_is_flagged_and_left_alone() {
    let path = scratch("secret.jsonl");
    let _ = std::fs::remove_file(&path);

    let frame = r#"{"type":"user","text":"GEMINI_API_KEY=AIzaSyNotARealKeyAtAll"}"#;
    write_a_run(&path, &[frame]);

    let text = std::fs::read_to_string(&path).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();
    let stored = text.lines().nth(1).unwrap();
    assert_eq!(field(stored, "secret_flag"), r#"["google_api_key"]"#);
    assert_eq!(field(stored, "raw"), frame, "the frame was rewritten");
}

#[test]
fn seq_is_gapless_and_starts_at_zero() {
    let path = scratch("gapless.jsonl");
    let _ = std::fs::remove_file(&path);
    write_a_run(&path, &[r#"{"a":1}"#, r#"{"b":2}"#]);

    let text = std::fs::read_to_string(&path).unwrap();
    let seqs: Vec<u64> = text
        .lines()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["seq"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(seqs, vec![0, 1, 2, 3]);
}

// The validator has to fail on things that are wrong, or a pass proves nothing.

fn problems(text: &str) -> Vec<String> {
    check(text, None).problems
}

fn line(seq: u64, body: &str) -> String {
    format!(
        r#"{{"run_id":"{RUN_ID}","parent_run_id":null,"seq":{seq},"ts":"2026-09-05T14:53:52.100Z","provider":"claude","model":null,"secret_flag":null,{body}}}"#
    )
}

fn start_line(seq: u64) -> String {
    line(
        seq,
        r#""hydra":{"type":"run_start","schema_version":1,"hydra_version":"0.1.0","command":"run","launch_mode":"recorded","provider_version":"2.1.261","provider_session_id":null,"task":"t","policy":null,"policy_sha":null,"cwd":"/tmp","worktree":null,"depth":0,"pid":1,"process_start_time":"2026-09-05T14:53:52.100000Z","subscription":"anthropic"}"#,
    )
}

fn end_line(seq: u64) -> String {
    line(
        seq,
        r#""hydra":{"type":"run_end","exit_code":0,"error":null,"refusal":null,"remediation":null,"body_source":"stream","ended_at":"2026-09-05T14:53:53.100Z"}"#,
    )
}

fn frame_line(seq: u64) -> String {
    line(seq, r#""raw":{"type":"assistant"}"#)
}

#[test]
fn the_validator_catches_a_lost_line() {
    let text = format!("{}\n{}\n", start_line(0), end_line(2));
    assert!(
        problems(&text)
            .iter()
            .any(|p| p.contains("a line was lost"))
    );
}

#[test]
fn the_validator_catches_a_run_start_that_is_not_first() {
    let text = format!("{}\n{}\n", frame_line(0), start_line(1));
    let found = problems(&text);
    assert!(found.iter().any(|p| p.contains("not the first line")));
    assert!(found.iter().any(|p| p.contains("not a Hydra run_start")));
}

#[test]
fn the_validator_catches_a_second_run_end() {
    let text = format!("{}\n{}\n{}\n", start_line(0), end_line(1), end_line(2));
    let found = problems(&text);
    assert!(found.iter().any(|p| p.contains("a second run_end")));
    assert!(found.iter().any(|p| p.contains("followed by")));
}

#[test]
fn the_validator_catches_a_line_carrying_both_bodies() {
    let text = format!(
        "{}\n{}\n",
        start_line(0),
        line(
            1,
            r#""raw":{"type":"assistant"},"hydra":{"type":"run_end"}"#
        )
    );
    assert!(
        problems(&text)
            .iter()
            .any(|p| p.contains("both raw and hydra"))
    );
}

#[test]
fn the_validator_catches_a_null_group_that_should_be_absent() {
    let text = start_line(0).replace(r#""worktree":null"#, r#""worktree":null,"skills":null"#);
    assert!(
        problems(&format!("{text}\n"))
            .iter()
            .any(|p| p.contains("skills is null"))
    );
}

/// Line 0 with contract 5.3's `mcp` group on it, as a launch writes it.
fn with_mcp(group: &str) -> String {
    format!(
        "{}\n",
        start_line(0).replace(
            r#""worktree":null"#,
            &format!(r#""worktree":null,"mcp":{group}"#)
        )
    )
}

#[test]
fn the_validator_takes_the_mcp_group_a_launch_writes() {
    // Both shapes a recorded `claude` run produces: the servers resolved, and
    // the provider not asked. 5.3 requires a reader to tell those apart, so the
    // instrument has to accept both.
    let resolved = with_mcp(r#"{"doc_sha":"abc","injected":["fs"],"resolved":["fs"]}"#);
    check(&resolved, None).assert_ok();

    let unanswered = with_mcp(r#"{"doc_sha":"abc","injected":["fs"],"resolved":null}"#);
    check(&unanswered, None).assert_ok();

    let nothing_admitted = with_mcp(r#"{"doc_sha":"abc","injected":[],"resolved":[]}"#);
    check(&nothing_admitted, None).assert_ok();
}

#[test]
fn the_validator_catches_an_mcp_group_that_answers_half_the_question() {
    // 5.3: a sha without names does not say what was selected, and names
    // without a sha do not say what bytes they resolved to.
    let no_sha = with_mcp(r#"{"injected":["fs"],"resolved":["fs"]}"#);
    assert!(
        problems(&no_sha)
            .iter()
            .any(|p| p.contains("mcp is missing doc_sha"))
    );

    let no_names = with_mcp(r#"{"doc_sha":"abc","resolved":null}"#);
    assert!(
        problems(&no_names)
            .iter()
            .any(|p| p.contains("mcp is missing injected"))
    );

    let null_sha = with_mcp(r#"{"doc_sha":null,"injected":[],"resolved":null}"#);
    assert!(
        problems(&null_sha)
            .iter()
            .any(|p| p.contains("mcp.doc_sha is null"))
    );

    let injected_is_not_a_list = with_mcp(r#"{"doc_sha":"abc","injected":"fs","resolved":null}"#);
    assert!(
        problems(&injected_is_not_a_list)
            .iter()
            .any(|p| p.contains("mcp.injected is not a list"))
    );
}

#[test]
fn the_validator_catches_a_pid_without_a_start_time() {
    let text = start_line(0).replace(
        r#""process_start_time":"2026-09-05T14:53:52.100000Z""#,
        r#""process_start_time":null"#,
    );
    assert!(
        problems(&format!("{text}\n"))
            .iter()
            .any(|p| p.contains("disagree about whether a process ran"))
    );
}

#[test]
fn the_validator_catches_time_running_backwards() {
    let text = format!(
        "{}\n{}\n",
        start_line(0).replace(
            r#""ts":"2026-09-05T14:53:52.100Z""#,
            r#""ts":"2026-09-05T14:53:59.100Z""#
        ),
        end_line(1)
    );
    assert!(
        problems(&text)
            .iter()
            .any(|p| p.contains("ts goes backwards"))
    );
}

#[test]
fn the_validator_catches_a_run_id_that_does_not_match_the_filename() {
    let text = format!("{}\n", start_line(0));
    assert!(
        check(&text, Some("some-other-id"))
            .problems
            .iter()
            .any(|p| p.contains("does not match the filename"))
    );
}

#[test]
fn every_record_on_this_machine_meets_the_contract() {
    // Multi-machine use is a non-goal, so the corpus this checks is the one on the
    // laptop Hydra runs on. Nothing here fails on a machine that has run nothing.
    let Ok(home) = hydra::env::real_home() else {
        return;
    };
    let runs = home.join("hydra").join("runs");
    let Ok(entries) = std::fs::read_dir(&runs) else {
        return;
    };

    let mut checked = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let report = hydra::record::validate::check_file(&path).unwrap();
        assert!(
            report.ok(),
            "{} does not meet the contract:\n  {}",
            path.display(),
            report.problems.join("\n  ")
        );
        checked += 1;
    }
    eprintln!("checked {checked} record(s) under {}", runs.display());
}
