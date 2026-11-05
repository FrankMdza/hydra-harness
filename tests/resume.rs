//! `hydra resume` without launching a provider.
//!
//! Builds the argv each provider would get and composes `run_start` the way
//! `tests/spawn.rs` does for gates: synthetic records, no binary on the wire.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::{Provider, agy, claude, codex, cursor};
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
        quiet: false,
        handed_off: None,
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
    append_resume_args(&mut argv, Provider::Claude, "sess-claude", None);
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
    append_resume_args(&mut argv, Provider::CursorAgent, "chat-9", None);
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
    append_resume_args(&mut argv, Provider::Agy, "conv-agy", None);
    assert_eq!(argv[0], "--conversation");
    assert_eq!(argv[1], "conv-agy");
}

#[test]
fn codex_argv_carries_exec_resume_and_the_thread_id() {
    let worktree = PathBuf::from("/tmp/wt");
    let hook_file = PathBuf::from("/tmp/spawn/r1/hook.py");
    let mut argv = codex::argv(&codex::Launch {
        task: "continue",
        model: Some("gpt-5.4"),
        worktree: &worktree,
        hook_file: &hook_file,
    });
    append_resume_args(&mut argv, Provider::Codex, "thr-codex", Some(&hook_file));
    assert_eq!(argv[0], "exec");
    assert_eq!(argv[1], "resume");
    assert_eq!(argv[2], "thr-codex");
    assert!(argv.contains(&"--json".to_string()));
    assert!(
        !argv.iter().any(|arg| arg == "-C"),
        "resume takes no worktree pin: {argv:?}"
    );
    assert!(
        !argv.iter().any(|arg| arg == "--sandbox"),
        "resume takes no sandbox flag: {argv:?}"
    );
    assert!(
        argv.iter()
            .any(|arg| arg == "--dangerously-bypass-hook-trust")
    );
    assert!(
        argv.windows(2).any(|pair| pair == ["-m", "gpt-5.4"]),
        "{argv:?}"
    );
    assert_eq!(argv.last().unwrap(), "continue");
}

#[test]
fn run_start_on_a_resume_carries_command_and_resumed_from() {
    let policies = PolicySet::discover().unwrap();
    let request = resume_request("old-run", "sess-1");
    let (layout, _root) = layout();
    let lineage = Lineage::of(&layout, None).unwrap();
    let start = hydra::run::run_start(
        &request,
        hydra::run::RunStartRequest {
            task: hydra::resume::DEFAULT_TASK,
            policy_sha: policies.sha(),
            provider_version: Some("stub"),
            cwd: std::path::Path::new("/tmp"),
            worktree: None,
            env_allowlist: None,
            skills: None,
            mcp: None,
            lineage: &lineage,
            scopes: None,
            gate_sha: None,
            sandbox: None,
        },
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

/// A record for the lineage tests, build ticket 389: line 0 with the parent edge
/// on the envelope (contract 4) and the depth and policy a child reads off it.
fn lineage_record(
    layout: &Layout,
    run_id: &str,
    parent: Option<&str>,
    policy: &str,
    depth: u32,
) -> PathBuf {
    let mut text = String::new();
    for (seq, hydra) in [
        json!({
            "type": "run_start",
            "schema_version": SCHEMA_VERSION,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "provider_session_id": "sess-lineage",
            "task": "first turn",
            "policy": policy,
            "policy_sha": "0000000000000000000000000000000000000000000000000000000000000000",
            "cwd": "/tmp",
            "worktree": null,
            "depth": depth,
            "pid": null,
            "process_start_time": null,
            "subscription": "anthropic",
        }),
        json!({"type": "run_end", "exit_code": 0, "body_source": "stream"}),
    ]
    .into_iter()
    .enumerate()
    {
        let mut line = serde_json::from_str::<Value>(&envelope(
            run_id,
            seq as u64,
            "claude",
            Some("haiku"),
            hydra,
        ))
        .unwrap();
        line["parent_run_id"] = json!(parent);
        text.push_str(&line.to_string());
        text.push('\n');
    }
    let path = layout.run_file(run_id);
    std::fs::write(&path, text).unwrap();
    path
}

const ROOT: &str = "01a07600-0000-7000-8000-000000038901";
const MIDDLE: &str = "01a07600-0000-7000-8000-000000038902";
const DEEPEST: &str = "01a07600-0000-7000-8000-000000038903";
const RESUMED: &str = "01a07600-0000-7000-8000-000000038904";
const RESUMED_AGAIN: &str = "01a07600-0000-7000-8000-000000038905";

#[test]
fn a_resumed_depth_two_child_keeps_its_depth_and_cannot_delegate() {
    // Build ticket 389. `hydra resume` launches with no parent, so before the fix
    // a depth 2 child came back at depth 0 and could delegate two more levels.
    let (layout, _root) = layout();
    let policies = PolicySet::discover().unwrap();
    lineage_record(&layout, ROOT, None, "standard", 0);
    lineage_record(&layout, MIDDLE, Some(ROOT), "standard", 1);
    lineage_record(&layout, DEEPEST, Some(MIDDLE), "standard", 2);

    let mut request = resume_request(DEEPEST, "sess-lineage");
    request.run_id = Some(RESUMED.to_string());
    let lineage = Lineage::for_request(&layout, &request).unwrap();
    assert_eq!(lineage.depth, 2, "a resume continues the run, at its depth");
    // Contract 4: the edge is what a parent writes at spawn, and no parent
    // spawned this one, so the resumed run's own record carries none.
    assert_eq!(lineage.parent_run_id(), None);

    // Line 0 of the resumed run, written by the path a launch that stopped early
    // takes, so the depth on disk is the one the launch computed.
    assert!(
        hydra::run::record_unlaunched(&layout, &request, policies.sha(), "task", "stub").unwrap()
    );
    let start = hydra::live::opening_line(&layout.run_file(RESUMED))
        .unwrap()
        .unwrap();
    assert_eq!(start["depth"], 2, "{start}");
    assert_eq!(start["resumed_from"], DEEPEST, "{start}");

    // The resumed run's own `delegate` reaches `Lineage::of` with it as parent.
    let err = Lineage::of(&layout, Some(RESUMED)).unwrap_err();
    let said = err.to_string();
    assert!(said.contains(RESUMED), "{said}");
    assert!(said.contains("the cap is 2"), "{said}");

    // A resume of a resume reads the same depth off the chain.
    let mut again = resume_request(RESUMED, "sess-lineage");
    again.run_id = Some(RESUMED_AGAIN.to_string());
    assert_eq!(Lineage::for_request(&layout, &again).unwrap().depth, 2);
}

#[test]
fn a_resumed_child_of_a_read_only_parent_asking_for_permissive_is_refused() {
    // Build ticket 389, harness spec 7.3. The resume carries the source's own
    // policy, so a child recorded under `permissive` beneath a `read-only` parent
    // is checked against that parent as if the parent had spawned it.
    let (layout, _root) = layout();
    let policies = PolicySet::discover().unwrap();
    lineage_record(&layout, ROOT, None, "read-only", 0);
    lineage_record(&layout, MIDDLE, Some(ROOT), "permissive", 1);

    let mut request = resume_request(MIDDLE, "sess-lineage");
    request.policy = "permissive".to_string();
    let lineage = Lineage::for_request(&layout, &request).unwrap();
    let remediation = lineage
        .narrowing(&policies, &request.policy)
        .unwrap()
        .expect("permissive is wider than read-only, resumed or spawned");
    assert_eq!(lineage.depth, 1);
    assert!(remediation.contains("read-only"), "{remediation}");
    assert!(remediation.contains("permissive"), "{remediation}");
    assert!(remediation.contains(ROOT), "{remediation}");

    // The same child resumed under a policy its parent contains passes.
    request.policy = "read-only".to_string();
    assert!(
        lineage
            .narrowing(&policies, &request.policy)
            .unwrap()
            .is_none()
    );
}
