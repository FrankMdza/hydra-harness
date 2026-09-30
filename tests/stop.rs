//! `hydra stop`, build ticket 169.
//!
//! Every run here is a real detached watcher over a stub provider on `/bin/sh`,
//! the way `tests/detached.rs` drives one, so the stop goes through the same
//! marker, signal and closing line a real run would and spends no subscription.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use hydra::env::ChildEnv;
use hydra::outcome::Outcome;
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::record::{RunStart, SCHEMA_VERSION};
use hydra::stop::{self, Target};
use hydra::watch::{self, Launch, Waited};

const SESSION: &str = "01996d3c-8f2a-7c31-9b44-000000000169";
const SEAT: &str = "01996d3c-8f2a-7c31-9b44-000000000170";

/// Prints one frame and then waits far longer than any test does.
const LONG_STUB: &str =
    r#"printf '{"type":"system","subtype":"init","model":"stub-1"}\n'; sleep 120"#;

fn sandbox(name: &str) -> Layout {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-stop-{}-{name}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    layout
}

fn line_zero(depth: u32) -> serde_json::Value {
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
        floor_lifted: Some(false),
        gate_sha: None,
        sandbox: None,
        scopes: None,
        cwd: "/tmp".into(),
        worktree: None,
        depth,
        parent_policy: None,
        pid: None,
        process_start_time: None,
        skills: None,
        mcp: None,
        auth: None,
        env_allowlist: Some(vec!["PATH".into()]),
        subscription: Some("none".into()),
        resumed_from: None,
        may_edit: None,
    })
    .unwrap()
}

/// Starts a detached watcher for `run_id` over `script` and waits for line 0.
fn start(layout: &Layout, run_id: &str, parent: Option<&str>, script: &str) -> std::process::Child {
    let spec = layout.launch_spec(run_id);
    watch::write_spec(
        &spec,
        &Launch {
            run_id: run_id.to_string(),
            parent_run_id: parent.map(str::to_string),
            provider: "claude".to_string(),
            binary: "/bin/sh".to_string(),
            argv: vec!["-c".to_string(), script.to_string()],
            cwd: PathBuf::from("/tmp"),
            run_file: layout.run_file(run_id),
            model: None,
            hydra_env: Default::default(),
            run_start: line_zero(u32::from(parent.is_some())),
            sandbox_profile: None,
            skill_snapshot: None,
        },
    )
    .unwrap();
    let mut watcher = watch::detach(
        Path::new(env!("CARGO_BIN_EXE_hydra")),
        &spec,
        &layout.watcher_log(run_id),
        &ChildEnv::build(Provider::Claude).unwrap(),
    )
    .unwrap();
    watch::wait_for_start(
        &layout.run_file(run_id),
        &mut watcher,
        Duration::from_secs(10),
    )
    .unwrap();
    watcher
}

fn policies() -> PolicySet {
    PolicySet::discover().unwrap()
}

fn closing(layout: &Layout, run_id: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    let last = text.lines().last().unwrap();
    serde_json::from_str(last).unwrap()
}

#[test]
fn a_stopped_run_closes_its_record_and_derives_stopped() {
    let layout = sandbox("one");
    let mut watcher = start(&layout, SESSION, None, LONG_STUB);

    let stopped = stop::stop(
        &layout,
        &policies(),
        Target::Run(&SESSION[..20]),
        Duration::from_secs(10),
    )
    .unwrap();
    watcher.wait().unwrap();

    let end = closing(&layout, SESSION);
    assert_eq!(end["hydra"]["type"], "run_end", "{end}");
    let error = end["hydra"]["error"].as_str().unwrap_or_default();
    assert!(error.starts_with(watch::STOP_NOTE), "{error}");

    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0].run_id, SESSION);
    assert!(!stopped[0].killed, "sh exits on SIGTERM");
    // Before map ticket 89 this was `crashed`, off the watcher's signal note.
    assert_eq!(stopped[0].outcome, Some(Outcome::Stopped));
    assert!(!Outcome::Stopped.rung_one_learns());
    assert!(stop::report(&stopped).contains(&format!("stopped {SESSION}: stopped")));
}

#[test]
fn a_session_stop_ends_the_seat_before_the_session() {
    let layout = sandbox("session");
    let mut session = start(&layout, SESSION, None, LONG_STUB);
    let mut seat = start(&layout, SEAT, Some(SESSION), LONG_STUB);

    let stopped = stop::stop(
        &layout,
        &policies(),
        Target::Session(SESSION),
        Duration::from_secs(10),
    )
    .unwrap();
    session.wait().unwrap();
    seat.wait().unwrap();

    let order: Vec<&str> = stopped.iter().map(|run| run.run_id.as_str()).collect();
    assert_eq!(order, [SEAT, SESSION]);
    assert!(
        stopped
            .iter()
            .all(|run| run.outcome == Some(Outcome::Stopped)),
        "{stopped:?}"
    );

    // The records agree with the order: the seat's closing line is no later
    // than the session's, and `stop` waits for each record to close in turn.
    let seat_end = closing(&layout, SEAT)["ts"].as_str().unwrap().to_string();
    let session_end = closing(&layout, SESSION)["ts"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(seat_end <= session_end, "{seat_end} {session_end}");
}

#[test]
fn a_run_that_is_not_live_is_refused() {
    let layout = sandbox("ended");
    let mut watcher = start(&layout, SESSION, None, "true");
    assert_eq!(
        watch::wait_for_end(
            &layout.run_file(SESSION),
            &mut watcher,
            Duration::from_secs(20)
        )
        .unwrap(),
        Waited::Ended
    );
    watcher.wait().unwrap();

    let err = stop::stop(
        &layout,
        &policies(),
        Target::Run(SESSION),
        Duration::from_secs(10),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("not live"), "{err}");
    // Nothing was marked, so the ended record keeps its own reading.
    assert!(!layout.stop_marker(SESSION).exists());
}

#[test]
fn a_prefix_that_names_two_runs_is_refused() {
    let layout = sandbox("ambiguous");
    std::fs::create_dir_all(layout.runs()).unwrap();
    for run_id in [SESSION, SEAT] {
        std::fs::write(layout.run_file(run_id), "").unwrap();
    }

    let err = stop::stop(
        &layout,
        &policies(),
        Target::Run("01996d3c"),
        Duration::from_secs(10),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("matches 2 runs"), "{err}");
    assert!(!layout.stop_marker(SESSION).exists());
    assert!(!layout.stop_marker(SEAT).exists());
}
