//! `hydra resume` without launching a provider.
//!
//! Builds the argv each provider would get and composes `run_start` the way
//! `tests/spawn.rs` does for gates: synthetic records, no binary on the wire.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::{Provider, agy, claude, cursor};
use hydra::record::SCHEMA_VERSION;
use hydra::run::{Lineage, Request, append_resume_args};
use serde_json::{Value, json};

/// A layout per test, because the suite runs in parallel.
fn layout() -> (Layout, PathBuf) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-resume-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    (layout, root)
}

fn envelope(run_id: &str, seq: u64, provider: &str, model: Option<&str>, hydra: Value) -> String {
    let mut line = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": seq,
        "ts": "2026-09-08T09:12:04.100Z",
        "provider": provider,
        "model": model,
        "secret_flag": null,
        "hydra": hydra,
    })
    .to_string();
    line.push('\n');
    line
}

fn closed_record(
    layout: &Layout,
    run_id: &str,
    provider: &str,
    launch_mode: &str,
    session_id: Option<&str>,
    pid: Option<u32>,
    ended: bool,
) -> PathBuf {
    let mut text = envelope(
        run_id,
        0,
        provider,
        Some("haiku"),
        json!({
            "type": "run_start",
            "schema_version": SCHEMA_VERSION,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": launch_mode,
            "provider_version": "stub",
            "provider_session_id": session_id,
            "task": "first turn",
            "policy": "standard",
            "policy_sha": "0000000000000000000000000000000000000000000000000000000000000000",
            "cwd": "/tmp",
            "worktree": null,
            "depth": 0,
            "pid": pid,
            "process_start_time": pid.map(|_| "2026-09-08T09:12:04.100Z"),
            "subscription": "anthropic",
        }),
    );
    if ended {
        text.push_str(&envelope(
            run_id,
            1,
            provider,
            Some("haiku"),
            json!({"type": "run_end", "exit_code": 0, "body_source": "stream"}),
        ));
    }
    let path = layout.run_file(run_id);
    std::fs::write(&path, text).unwrap();
    path
}

fn resume_request(resumed_from: &str, session_id: &str) -> Request {
    Request {
        provider: Provider::Claude,
        task: hydra::resume::DEFAULT_TASK.to_string(),
        model: Some("haiku".to_string()),
        policy: "standard".to_string(),
        floor_lifted: false,
        timeout: Duration::from_secs(600),
        parent: None,
        run_id: None,
        command: "resume".to_string(),
        resumed_from: Some(resumed_from.to_string()),
        resume_session_id: Some(session_id.to_string()),
        working_dir: Some(PathBuf::from("/tmp")),
    }
}

#[test]
fn claude_argv_carries_resume_and_the_session_id() {
    let settings = PathBuf::from("/tmp/spawn/r1/settings.json");
    let mut argv = claude::argv(&claude::Launch {
        task: "continue",
        model: Some("haiku"),
        settings: &settings,
        policy: "standard",
        mcp: None,
    });
    append_resume_args(&mut argv, Provider::Claude, "sess-claude");
    let pos = argv
        .windows(2)
        .position(|pair| pair == ["--resume", "sess-claude"])
        .expect("resume flag");
    assert!(pos < argv.len() - 1);
    assert_eq!(argv.last().unwrap(), "continue");
}

#[test]
fn cursor_argv_carries_resume_and_the_chat_id() {
    let mut argv = cursor::argv(&cursor::Launch {
        task: "continue",
        model: Some("composer-2.5"),
    });
    append_resume_args(&mut argv, Provider::CursorAgent, "chat-9");
    assert!(
        argv.windows(2).any(|pair| pair == ["--resume", "chat-9"]),
        "{argv:?}"
    );
    assert_eq!(argv.last().unwrap(), "continue");
}

#[test]
fn agy_argv_carries_conversation_and_the_id() {
    let worktree = PathBuf::from("/tmp/wt");
    let mut argv = agy::argv(&agy::Launch {
        task: "continue",
        model: None,
        worktree: &worktree,
    });
    append_resume_args(&mut argv, Provider::Agy, "conv-agy");
    assert_eq!(argv[0], "--conversation");
    assert_eq!(argv[1], "conv-agy");
}

#[test]
fn run_start_on_a_resume_carries_command_and_resumed_from() {
    let policies = PolicySet::discover().unwrap();
    let request = resume_request("old-run", "sess-1");
    let (layout, _root) = layout();
    let lineage = Lineage::of(&layout, None).unwrap();
    let start = hydra::run::run_start(
        &request,
        hydra::resume::DEFAULT_TASK,
        policies.sha(),
        Some("stub"),
        std::path::Path::new("/tmp"),
        None,
        None,
        None,
        None,
        &lineage,
        None,
    );
    assert_eq!(start.command, "resume");
    assert_eq!(start.resumed_from.as_deref(), Some("old-run"));
}

#[test]
fn a_record_with_no_session_id_is_refused() {
    let (layout, _root) = layout();
    let run_id = "01a07357-58a5-79e2-a25a-3df578a750b4";
    closed_record(&layout, run_id, "claude", "recorded", None, None, true);
    match hydra::resume::resume_in(
        &layout,
        hydra::resume::Request {
            run_id: run_id.to_string(),
            task: None,
            timeout: Duration::from_secs(600),
        },
    ) {
        Err(err) => assert!(
            err.to_string().contains("no provider_session_id"),
            "{err:#}"
        ),
        Ok(_) => panic!("expected resume to refuse a record with no session id"),
    }
}

#[test]
fn a_live_record_is_refused() {
    let (layout, _root) = layout();
    let run_id = "01a07357-58a5-79e2-a25a-3df578a750c";
    let pid = std::process::id();
    let start = hydra::procinfo::start_time(pid)
        .unwrap()
        .expect("this process has a start time");
    let start_text = start.to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let text = envelope(
        run_id,
        0,
        "claude",
        Some("haiku"),
        json!({
            "type": "run_start",
            "schema_version": SCHEMA_VERSION,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "provider_session_id": "sess-live",
            "task": "first turn",
            "policy": "standard",
            "policy_sha": "0000000000000000000000000000000000000000000000000000000000000000",
            "cwd": "/tmp",
            "worktree": null,
            "depth": 0,
            "pid": pid,
            "process_start_time": start_text,
            "subscription": "anthropic",
        }),
    );
    std::fs::write(layout.run_file(run_id), text).unwrap();
    assert!(
        hydra::live::read_view(&layout.run_file(run_id))
            .unwrap()
            .is_live()
    );
    match hydra::resume::resume_in(
        &layout,
        hydra::resume::Request {
            run_id: run_id.to_string(),
            task: None,
            timeout: Duration::from_secs(600),
        },
    ) {
        Err(err) => assert!(err.to_string().contains("still live"), "{err:#}"),
        Ok(_) => panic!("expected resume to refuse a live record"),
    }
}

#[test]
fn an_interactive_record_is_refused() {
    let (layout, _root) = layout();
    let run_id = "01a07357-58a5-79e2-a25a-3df578a750d";
    closed_record(
        &layout,
        run_id,
        "claude",
        "interactive",
        Some("sess-int"),
        None,
        true,
    );
    match hydra::resume::resume_in(
        &layout,
        hydra::resume::Request {
            run_id: run_id.to_string(),
            task: None,
            timeout: Duration::from_secs(600),
        },
    ) {
        Err(err) => assert!(err.to_string().contains("interactive"), "{err:#}"),
        Ok(_) => panic!("expected resume to refuse an interactive record"),
    }
}

#[test]
fn session_id_is_read_off_the_stream_when_line_zero_is_null() {
    let (layout, _root) = layout();
    let run_id = "01a07357-58a5-79e2-a25a-3df578a750e";
    let path = closed_record(&layout, run_id, "claude", "recorded", None, None, true);
    let mut text = std::fs::read_to_string(&path).unwrap();
    let init = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": 1,
        "ts": "2026-09-08T09:12:05.100Z",
        "provider": "claude",
        "model": "haiku",
        "secret_flag": null,
        "raw": {
            "type": "system",
            "subtype": "init",
            "session_id": "from-stream",
            "model": "haiku"
        },
    })
    .to_string()
        + "\n";
    let end = text.lines().last().unwrap().to_string() + "\n";
    text = text
        .lines()
        .take(2)
        .chain(std::iter::once(init.trim_end()))
        .chain(std::iter::once(end.trim_end()))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&path, text).unwrap();

    let id = hydra::resume::provider_session_id(&path, Provider::Claude, &opening(&path)).unwrap();
    assert_eq!(id, "from-stream");
}

fn opening(path: &std::path::Path) -> Value {
    hydra::live::opening_line(path).unwrap().unwrap()
}
