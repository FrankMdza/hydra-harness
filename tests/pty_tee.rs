//! The pty and the tee, end to end, without spending a subscription.
//!
//! A stub provider on `/bin/sh` produces the shapes `claude` produces: JSON lines,
//! and a trailing fragment with no newline. What is being tested is that Hydra owns
//! the pty, that ONLCR's carriage returns do not reach the record, and that the
//! file this path writes meets the contract.

use std::path::PathBuf;

use hydra::record::validate::check;
use hydra::record::{RunEnd, RunStart, SCHEMA_VERSION, Writer};

const RUN_ID: &str = "01996d3c-8f2a-7c31-9b44-000000000001";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hydra-pty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let _ = std::fs::remove_file(&path);
    path
}

fn minimal_run_start(pid: u32, start: Option<String>) -> RunStart {
    RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: "0.1.0".into(),
        command: "run".into(),
        launch_mode: Some("recorded".into()),
        provider_version: Some("stub".into()),
        provider_session_id: None,
        task: Some("stub".into()),
        policy: None,
        policy_sha: None,
        floor_lifted: None,
        gate_sha: None,
        sandbox: None,
        sandbox_kind: None,
        host: None,
        scopes: None,
        cwd: "/tmp".into(),
        worktree: None,
        depth: 0,
        parent_policy: None,
        pid: Some(pid),
        process_start_time: start,
        skills: None,
        mcp: None,
        auth: None,
        env_allowlist: Some(vec!["PATH".into()]),
        subscription: Some("none".into()),
        resumed_from: None,
        may_edit: None,
    }
}

#[test]
fn the_child_gets_the_environment_it_is_handed_and_nothing_else() {
    // The allowlist in `src/env.rs` is only a boundary if the pty starts the child
    // from an empty environment. portable-pty's `CommandBuilder::new` copies this
    // process's, found by the hiring panel of 2026-09-23; without `env_clear` every
    // recorded run inherited the shell that launched `hydra run`. Cargo sets
    // `CARGO_MANIFEST_DIR` on this test process, and `HOME` is set on any shell that
    // runs the suite, so either reaching the child is the leak.
    assert!(std::env::var_os("CARGO_MANIFEST_DIR").is_some());
    let mut spawned = hydra::pty::spawn(
        "/bin/sh",
        &["-c".to_string(), "env".to_string()],
        std::path::Path::new("/tmp"),
        [("PATH".to_string(), "/usr/bin:/bin".to_string())].into_iter(),
        None,
    )
    .unwrap();
    let mut lines = Vec::new();
    spawned
        .tee(|line| {
            lines.push(line.to_string());
            Ok(())
        })
        .unwrap();
    spawned.wait().unwrap();

    let names: Vec<&str> = lines
        .iter()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name))
        .collect();
    assert!(names.contains(&"PATH"), "{names:?}");
    for leaked in ["CARGO_MANIFEST_DIR", "HOME", "USER"] {
        assert!(
            !names.contains(&leaked),
            "{leaked} reached the child without being handed to it: {names:?}"
        );
    }
}

#[test]
fn a_teed_run_is_a_valid_record() {
    let path = scratch("teed.jsonl");
    let script = r#"printf '{"type":"system","subtype":"init","model":"stub-1"}\n'; printf '{"type":"assistant","text":"hi"}\n'; printf '\033[?25h'"#;

    let mut spawned = hydra::pty::spawn(
        "/bin/sh",
        &["-c".to_string(), script.to_string()],
        std::path::Path::new("/tmp"),
        [("PATH".to_string(), "/usr/bin:/bin".to_string())].into_iter(),
        None,
    )
    .unwrap();

    let start = hydra::procinfo::start_time(spawned.pid)
        .unwrap()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));

    let mut writer = Writer::create(&path, RUN_ID, None, Some("claude"), None).unwrap();
    writer
        .hydra_line(&minimal_run_start(spawned.pid, start))
        .unwrap();

    let mut lines = Vec::new();
    spawned
        .tee(|line| {
            lines.push(line.to_string());
            writer.provider_line(line)?;
            Ok(())
        })
        .unwrap();
    let exit = spawned.wait().unwrap();
    writer.hydra_line(&RunEnd::streamed(exit, None)).unwrap();

    assert_eq!(exit, Some(0));
    assert_eq!(spawned.lossy_lines, 0);

    // The tty put a carriage return on the end of every line. None of them survived
    // into a frame.
    assert!(
        lines.iter().all(|l| !l.contains('\r')),
        "a carriage return the provider never wrote reached the record: {lines:?}"
    );
    // The trailing fragment has no newline and is delivered anyway.
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[2], "\u{1b}[?25h");

    let text = std::fs::read_to_string(&path).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();
    assert_eq!(text.lines().count(), 5);
}

#[test]
fn a_long_frame_survives_being_split_across_reads() {
    let path = scratch("long.jsonl");
    // Larger than the tee's 64 KB read buffer, so the line arrives in pieces.
    let payload = "x".repeat(200_000);
    let script = format!(r#"printf '{{"type":"assistant","text":"%s"}}\n' '{payload}'"#);

    let mut spawned = hydra::pty::spawn(
        "/bin/sh",
        &["-c".to_string(), script],
        std::path::Path::new("/tmp"),
        [("PATH".to_string(), "/usr/bin:/bin".to_string())].into_iter(),
        None,
    )
    .unwrap();

    let start = hydra::procinfo::start_time(spawned.pid)
        .unwrap()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
    let mut writer = Writer::create(&path, RUN_ID, None, Some("claude"), None).unwrap();
    writer
        .hydra_line(&minimal_run_start(spawned.pid, start))
        .unwrap();

    let mut seen = 0usize;
    spawned
        .tee(|line| {
            seen += 1;
            assert!(
                serde_json::from_str::<serde_json::Value>(line).is_ok(),
                "a frame was cut at a read boundary"
            );
            writer.provider_line(line)?;
            Ok(())
        })
        .unwrap();
    spawned.wait().unwrap();

    assert_eq!(seen, 1);
}

#[test]
fn a_line_past_the_cap_becomes_two_frames_and_is_named_on_run_end() {
    let path = scratch("capped.jsonl");
    let total = 20 * 1024 * 1024;
    let script = format!("dd if=/dev/zero bs={total} count=1 2>/dev/null | tr '\\0' 'x'");

    let mut spawned = hydra::pty::spawn(
        "/bin/sh",
        &["-c".to_string(), script],
        std::path::Path::new("/tmp"),
        [("PATH".to_string(), "/usr/bin:/bin".to_string())].into_iter(),
        None,
    )
    .unwrap();

    let start = hydra::procinfo::start_time(spawned.pid)
        .unwrap()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
    let mut writer = Writer::create(&path, RUN_ID, None, Some("claude"), None).unwrap();
    writer
        .hydra_line(&minimal_run_start(spawned.pid, start))
        .unwrap();

    let mut lines = Vec::new();
    spawned
        .tee(|line| {
            lines.push(line.to_string());
            writer.provider_line(line)?;
            Ok(())
        })
        .unwrap();
    let exit = spawned.wait().unwrap();
    assert_eq!(spawned.capped_lines, 1);
    assert_eq!(spawned.lossy_lines, 0);
    writer
        .hydra_line(&RunEnd::streamed(
            exit,
            Some(hydra::pty::capped_line_note(spawned.capped_lines)),
        ))
        .unwrap();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].len(), hydra::pty::LINE_CAP);
    assert_eq!(lines[1].len(), total - hydra::pty::LINE_CAP);
    assert!(
        lines.iter().all(|line| line.bytes().all(|b| b == b'x')),
        "a byte was dropped or rewritten"
    );

    let text = std::fs::read_to_string(&path).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();
    let last = text.lines().last().unwrap();
    let end: serde_json::Value = serde_json::from_str(last).unwrap();
    assert_eq!(
        end["hydra"]["error"].as_str(),
        Some(hydra::pty::capped_line_note(1).as_str())
    );
}
