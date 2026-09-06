//! The detached half of a run, and the view over it. Harness spec 9 and 10.
//!
//! Everything here drives `hydra watch` against a stub provider on `/bin/sh`, so
//! the suite spends no subscription. The launch spec is what makes that possible
//! and is not a test hook: `hydra run` composes the argv, the gate and the whole
//! of line 0 before it forks, so the detached half already takes a binary and an
//! argv rather than a provider.
//!
//! What is being tested is the three claims a detached spawn makes. The child runs
//! in a session of its own, so closing the terminal cannot reach it. The record is
//! written by the half that survives, not by the half the human is talking to. And
//! the call's timeout stops the watching without touching the run.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use hydra::live::{self, State};
use hydra::paths::Layout;
use hydra::record::validate::check;
use hydra::record::{RunStart, SCHEMA_VERSION};
use hydra::watch::{self, Launch, Waited};

const RUN_ID: &str = "01996d3c-8f2a-7c31-9b44-000000000042";

fn sandbox(name: &str) -> Layout {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-detached-{}-{name}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    layout
}

fn line_zero(cwd: &Path) -> serde_json::Value {
    serde_json::to_value(RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: "0.2.0".into(),
        command: "run".into(),
        launch_mode: Some("recorded".into()),
        provider_version: Some("stub".into()),
        provider_session_id: None,
        task: Some("stub".into()),
        policy: Some("standard".into()),
        policy_sha: None,
        cwd: cwd.display().to_string(),
        worktree: None,
        depth: 0,
        // The two fields the detached half fills in. Null here is the point.
        pid: None,
        process_start_time: None,
        skills: None,
        mcp: None,
        auth: None,
        env_allowlist: Some(vec!["PATH".into()]),
        subscription: Some("none".into()),
    })
    .unwrap()
}

/// Writes a launch spec that runs `script` under `/bin/sh` and returns its path.
fn spec_for(layout: &Layout, script: &str) -> PathBuf {
    let path = layout.launch_spec(RUN_ID);
    watch::write_spec(
        &path,
        &Launch {
            run_id: RUN_ID.to_string(),
            provider: "claude".to_string(),
            binary: "/bin/sh".to_string(),
            argv: vec!["-c".to_string(), script.to_string()],
            cwd: PathBuf::from("/tmp"),
            run_file: layout.run_file(RUN_ID),
            model: None,
            set_env: Default::default(),
            run_start: line_zero(Path::new("/tmp")),
        },
    )
    .unwrap();
    path
}

/// The binary under test, which is what `hydra run` re-execs off itself.
fn hydra() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_hydra"))
}

fn start(layout: &Layout, script: &str) -> std::process::Child {
    let spec = spec_for(layout, script);
    watch::detach(&hydra(), &spec, &layout.watcher_log(RUN_ID)).unwrap()
}

/// One frame, a pause, one frame, then exit. Long enough to be caught mid-run.
fn slow_stub(seconds: u32) -> String {
    format!(
        r#"printf '{{"type":"system","subtype":"init","model":"stub-1"}}\n'; sleep {seconds}; printf '{{"type":"result","subtype":"success","is_error":false}}\n'"#
    )
}

#[test]
fn the_detached_half_writes_the_record_its_launcher_never_touches() {
    let layout = sandbox("record");
    let mut watcher = start(&layout, &slow_stub(0));
    let run_file = layout.run_file(RUN_ID);

    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();
    assert_eq!(
        watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap(),
        Waited::Ended
    );

    let text = std::fs::read_to_string(&run_file).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();

    // Contract 5.2: the liveness pair is on line 0, and it is the detached half
    // that put it there. A launcher cannot know a pid that does not exist yet.
    let view = live::read_view(&run_file).unwrap();
    assert_eq!(view.state, State::Ended);
    let pid = view
        .pid
        .expect("the detached half filled in its child's pid");
    assert_ne!(pid, std::process::id());
    assert!(view.process_start_time.is_some());
}

#[test]
fn the_child_runs_in_a_session_of_its_own() {
    // This is the whole of "closing the terminal orphans nothing that matters".
    // A Ghostty window closing sends SIGHUP to the foreground process group of
    // its tty; a process in its own session has no controlling terminal to be
    // hung up on. Asserting the session id is asserting that mechanism rather
    // than a symptom of it.
    let layout = sandbox("session");
    let mut watcher = start(&layout, &slow_stub(2));
    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();

    let ours = unsafe { libc::getsid(0) };
    let theirs = unsafe { libc::getsid(watcher.id() as libc::pid_t) };
    assert!(theirs > 0, "getsid failed for the detached half");
    assert_ne!(
        theirs, ours,
        "the detached half is in the launcher's session, so a hangup reaches it"
    );

    watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap();
}

#[test]
fn giving_up_on_the_call_leaves_the_run_alone() {
    // Harness spec 9: on expiry the call returns a handle rather than killing the
    // child. So after the budget runs out the record still has no closing line,
    // the process is still there by contract 5.2's test, and the run finishes on
    // its own with nobody watching.
    let layout = sandbox("timeout");
    let mut watcher = start(&layout, &slow_stub(3));
    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();

    let gave_up = watch::wait_for_end(&run_file, &mut watcher, Duration::from_millis(300)).unwrap();
    assert_eq!(gave_up, Waited::TimedOut);

    let view = live::read_view(&run_file).unwrap();
    assert_eq!(view.state, State::Live, "the timeout killed the child");
    assert!(!live::has_run_end(&run_file).unwrap());

    // Nobody is watching now. The run still ends and still closes its record.
    assert_eq!(
        watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap(),
        Waited::Ended
    );
    assert_eq!(live::read_view(&run_file).unwrap().state, State::Ended);
}

#[test]
fn a_zero_budget_hands_back_the_handle_without_waiting() {
    // Nothing here kills a child, so a zero timeout is "launch it and hand me the
    // handle" rather than "give it no time to run". A caller that wanted the run
    // not to happen would not have launched it.
    let layout = sandbox("zero");
    let mut watcher = start(&layout, &slow_stub(2));
    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();

    let began = std::time::Instant::now();
    let gave_up = watch::wait_for_end(&run_file, &mut watcher, Duration::ZERO).unwrap();
    assert_eq!(gave_up, Waited::TimedOut);
    assert!(began.elapsed() < Duration::from_millis(500));
    assert_eq!(live::read_view(&run_file).unwrap().state, State::Live);

    watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap();
}

#[test]
fn a_binary_that_is_not_there_still_writes_a_record() {
    // Contract 5.2's null pid: a run that never had a process. The record exists
    // because under D10 anything not in a run record did not happen, and a reader
    // must not take the null for dead-and-unharvested.
    let layout = sandbox("nobinary");
    let spec = layout.launch_spec(RUN_ID);
    watch::write_spec(
        &spec,
        &Launch {
            run_id: RUN_ID.to_string(),
            provider: "claude".to_string(),
            binary: "/nonexistent/hydra-stub".to_string(),
            argv: vec![],
            cwd: PathBuf::from("/tmp"),
            run_file: layout.run_file(RUN_ID),
            model: None,
            set_env: Default::default(),
            run_start: line_zero(Path::new("/tmp")),
        },
    )
    .unwrap();
    let mut watcher = watch::detach(&hydra(), &spec, &layout.watcher_log(RUN_ID)).unwrap();

    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();
    assert_eq!(
        watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap(),
        Waited::Ended
    );

    let view = live::read_view(&run_file).unwrap();
    assert_eq!(view.pid, None);
    assert_eq!(view.state, State::Ended);

    let text = std::fs::read_to_string(&run_file).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();
    assert!(text.contains("run_end"));
}

#[test]
fn a_writer_that_dies_before_closing_is_told_apart_from_a_slow_one() {
    // The one failure the view cannot read off the record, because the record
    // never gains the line that would say so. `wait_for_end` answers from the
    // process rather than from the file, and does it without waiting out the
    // caller's whole budget.
    let layout = sandbox("writergone");
    let spec = spec_for(&layout, &slow_stub(30));
    let mut watcher = watch::detach(&hydra(), &spec, &layout.watcher_log(RUN_ID)).unwrap();
    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();

    let child = live::read_view(&run_file).unwrap().pid.unwrap();
    unsafe {
        libc::kill(watcher.id() as libc::pid_t, libc::SIGKILL);
        // Its own child would otherwise sit there for thirty seconds.
        libc::kill(child as libc::pid_t, libc::SIGKILL);
    }

    assert_eq!(
        watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap(),
        Waited::WriterGone
    );
    assert_eq!(live::read_view(&run_file).unwrap().state, State::Abandoned);
}
