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
        cwd: "/tmp".into(),
        worktree: None,
        depth: 0,
        pid: Some(pid),
        process_start_time: start,
        skills: None,
        mcp: None,
        auth: None,
        env_allowlist: Some(vec!["PATH".into()]),
        subscription: Some("none".into()),
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
