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

use hydra::env::ChildEnv;
use hydra::live::{self, State};
use hydra::paths::Layout;
use hydra::provider::Provider;
use hydra::record::validate::check;
use hydra::record::{RunStart, SCHEMA_VERSION};
use hydra::skills::{self, Store};
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
        // A stub launch compiles no gate, so it resolved no scope.
        floor_lifted: Some(false),
        gate_sha: None,
        sandbox: None,
        sandbox_kind: None,
        host: None,
        scopes: None,
        cwd: cwd.display().to_string(),
        worktree: None,
        depth: 0,
        parent_policy: None,
        // The two fields the detached half fills in. Null here is the point.
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

/// Writes a launch spec that runs `script` under `/bin/sh` and returns its path.
fn spec_for(layout: &Layout, script: &str) -> PathBuf {
    spec_for_binary(
        layout,
        "/bin/sh",
        vec!["-c".to_string(), script.to_string()],
    )
}

fn spec_for_binary(layout: &Layout, binary: &str, argv: Vec<String>) -> PathBuf {
    let path = layout.launch_spec(RUN_ID);
    watch::write_spec(
        &path,
        &Launch {
            run_id: RUN_ID.to_string(),
            parent_run_id: None,
            provider: "claude".to_string(),
            binary: binary.to_string(),
            argv,
            cwd: PathBuf::from("/tmp"),
            run_file: layout.run_file(RUN_ID),
            model: None,
            hydra_env: Default::default(),
            run_start: line_zero(Path::new("/tmp")),
            sandbox_profile: None,
            skill_snapshot: None,
        },
    )
    .unwrap();
    path
}

/// The binary under test, which is what `hydra run` re-execs off itself.
fn hydra() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_hydra"))
}

/// What `hydra run` hands the watcher: the child's own environment.
fn child_env() -> ChildEnv {
    ChildEnv::build(Provider::Claude).unwrap()
}

/// The environment another process was exec'd with, name and value, as the
/// kernel keeps it. The same bytes `ps -E` prints, read without parsing `ps`.
#[cfg(target_os = "macos")]
fn environment_of(pid: u32) -> Vec<String> {
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as libc::c_int];
    let mut size: libc::size_t = 0;
    // SAFETY: three-element `mib` and live `size` out-param; null `oldp` sizes only.
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    assert_eq!(rc, 0, "sysctl would not size pid {pid}'s arguments");
    let mut buf = vec![0u8; size];
    // SAFETY: `buf` holds `size` bytes from the sizing call; `mib` is unchanged.
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            buf.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    assert_eq!(rc, 0, "sysctl would not read pid {pid}'s arguments");
    buf.truncate(size);

    // argc, the exec path, NUL padding, argc arguments, then the environment up
    // to the first empty string.
    let argc = i32::from_ne_bytes(buf[..4].try_into().unwrap()) as usize;
    let mut strings = buf[4..].split(|b| *b == 0);
    strings.next(); // the exec path
    let mut rest = strings.skip_while(|s| s.is_empty());
    for _ in 0..argc {
        rest.next();
    }
    rest.take_while(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

#[cfg(target_os = "linux")]
fn environment_of(pid: u32) -> Vec<String> {
    std::fs::read(format!("/proc/{pid}/environ"))
        .unwrap()
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

fn start(layout: &Layout, script: &str) -> std::process::Child {
    let spec = spec_for(layout, script);
    watch::detach(&hydra(), &spec, &layout.watcher_log(RUN_ID), &child_env()).unwrap()
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
fn the_watcher_holds_the_childs_environment_and_nothing_the_launcher_carried() {
    // The watcher sits outside the seatbelt for the whole run, and a same-uid
    // `ps -E` reads its environment. Measured by the third hiring panel, H3: it
    // held the launching shell whole, `GEMINI_API_KEY` included.
    let handed = child_env();
    let allowed: std::collections::BTreeSet<String> = handed.names().into_iter().collect();
    // Without a name outside the allowlist in this process, an inherited
    // environment and a cleared one look the same and the test proves nothing.
    // `cargo test` sets `CARGO_MANIFEST_DIR` and its neighbours.
    let outside: Vec<String> = std::env::vars()
        .map(|(name, _)| name)
        .filter(|name| !allowed.contains(name))
        .collect();
    assert!(
        !outside.is_empty(),
        "the test process carries nothing outside the allowlist"
    );

    let layout = sandbox("watcherenv");
    let mut watcher = start(&layout, &slow_stub(2));
    let run_file = layout.run_file(RUN_ID);
    // Line 0 is on disk, so the watcher has exec'd and what the kernel holds
    // is its own environment rather than the forked launcher's.
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();
    let held = environment_of(watcher.id());
    watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap();

    let held_names: std::collections::BTreeSet<String> = held
        .iter()
        .filter_map(|entry| entry.split_once('=').map(|(name, _)| name.to_string()))
        .collect();
    let leaked: Vec<&String> = held_names.difference(&allowed).collect();
    assert!(
        leaked.is_empty(),
        "the watcher inherited {leaked:?} from its launcher"
    );
    assert_eq!(
        held_names, allowed,
        "the watcher lost a name the child needs"
    );
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

    // SAFETY: pid 0 means this process.
    let ours = unsafe { libc::getsid(0) };
    // SAFETY: `watcher` is a live child for the call.
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
            parent_run_id: None,
            provider: "claude".to_string(),
            binary: "/nonexistent/hydra-stub".to_string(),
            argv: vec![],
            cwd: PathBuf::from("/tmp"),
            run_file: layout.run_file(RUN_ID),
            model: None,
            hydra_env: Default::default(),
            run_start: line_zero(Path::new("/tmp")),
            sandbox_profile: None,
            skill_snapshot: None,
        },
    )
    .unwrap();
    let mut watcher =
        watch::detach(&hydra(), &spec, &layout.watcher_log(RUN_ID), &child_env()).unwrap();

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

/// The smallest `run_start` the contract accepts, for the held-slave harness.
fn held_slave_run_start(pid: u32, start: String) -> RunStart {
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
        process_start_time: Some(start),
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
fn a_grandchild_holding_the_slave_closes_the_record_within_the_deadline() {
    // A provider that backgrounds an MCP stdio server and exits leaves a
    // grandchild holding the slave open. Without a deadline the tee blocks in
    // `read` forever and the record never closes.
    //
    // macOS revokes inherited pty fds when the direct child exits, so the
    // end-to-end stub cannot keep the slave open here the way a Linux
    // grandchild can. The held-slave harness keeps the slave in-process the
    // way a descendant would on Linux, and the same watch closing logic runs.
    let layout = sandbox("pty-held");
    let run_file = layout.run_file(RUN_ID);
    let script =
        r#"printf '{"type":"system","subtype":"init","model":"stub-1"}\n'; sleep 300 & exit 0"#;

    let (mut spawned, held) = hydra::pty::spawn_with_held_slave(
        "/bin/sh",
        &["-c".to_string(), script.to_string()],
        Path::new("/tmp"),
        [("PATH".to_string(), "/usr/bin:/bin".to_string())].into_iter(),
        None,
    )
    .unwrap();
    let process_start = hydra::procinfo::start_time(spawned.pid)
        .unwrap()
        .expect("the stub is running");

    let mut writer =
        hydra::record::Writer::create(&run_file, RUN_ID, None, Some("claude"), None).unwrap();
    // A whole `run_start`, as the validator reads it. Until build 331 this line
    // was a hand-written object with `kind` for `type` and no version fields:
    // the macOS path returns before `check`, so the record was never read back
    // here, and Linux, where the slave stays held, failed on the test's own JSON.
    writer
        .hydra_line(&held_slave_run_start(
            spawned.pid,
            process_start.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        ))
        .unwrap();

    let began = std::time::Instant::now();
    spawned
        .tee(|line| {
            writer.provider_line(line)?;
            Ok(())
        })
        .unwrap();
    if !spawned.pty_still_held {
        // macOS closes the master when the session leader exits even though the
        // harness still holds the slave object. The deadline is unreachable on
        // this platform; `tests/pty_tee.rs` covers the ordinary close path.
        assert!(
            began.elapsed() < Duration::from_millis(200),
            "the tee should close promptly when the master sees EOF: {:?}",
            began.elapsed()
        );
        spawned.wait().unwrap();
        drop(held);
        return;
    }

    assert!(
        began.elapsed() >= hydra::pty::PTY_EOF_DEADLINE - Duration::from_millis(200),
        "the tee closed before the deadline: {:?}",
        began.elapsed()
    );
    assert!(
        began.elapsed() < Duration::from_secs(8),
        "the tee should not wait for the background sleep"
    );

    let exit_code = spawned.wait().unwrap();
    let mut tee_error = None;
    if spawned.pty_still_held {
        tee_error = Some(watch::PTY_HELD_NOTE.to_string());
    }
    writer
        .hydra_line(&hydra::record::RunEnd::streamed(exit_code, tee_error))
        .unwrap();

    let text = std::fs::read_to_string(&run_file).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();
    assert!(
        text.contains(watch::PTY_HELD_NOTE),
        "run_end.error should name the held pty"
    );

    let sleep_alive = std::process::Command::new("pgrep")
        .args(["-f", "sleep 300"])
        .output()
        .unwrap();
    assert!(
        sleep_alive.status.success() && !sleep_alive.stdout.is_empty(),
        "Hydra killed the grandchild still holding the slave"
    );

    drop(held);
    for line in sleep_alive.stdout.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let pid = std::str::from_utf8(line)
            .unwrap()
            .parse::<libc::pid_t>()
            .unwrap();
        // SAFETY: `pid` came from this test's `pgrep` output and is killed for cleanup.
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
    }
}

#[test]
fn a_codex_run_whose_skill_copy_was_edited_records_the_mismatch_on_run_end() {
    let layout = sandbox("codex-skills");
    let run_id = "01996d3c-8f2a-7c31-9b44-000000000043";
    let worktree = layout.root().join("checkout");
    std::fs::create_dir_all(&worktree).unwrap();

    let store = Store::open(&layout).unwrap();
    let skill_dir = layout.skills().join("alpha");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: alpha\ndescription: A probe.\n---\n\nbody\n",
    )
    .unwrap();
    let sha = store.pin(run_id).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let injected = skills::inject(&worktree, Provider::Codex, &snapshot, &sha).unwrap();
    assert_eq!(injected.linked, vec!["alpha".to_string()]);

    let skills_root = worktree.join(".codex/skills");
    let skill_file = skills_root.join("alpha/SKILL.md");
    let script = format!(
        r#"printf '{{"type":"system","subtype":"init","model":"stub-1"}}\n'; \
chmod -R u+w '{}' && echo tampered >> '{}'; \
printf '{{"type":"result","subtype":"success","is_error":false}}\n'"#,
        skills_root.display(),
        skill_file.display()
    );

    let run_start = serde_json::json!({
        "type": "run_start",
        "schema_version": SCHEMA_VERSION,
        "hydra_version": "0.2.0",
        "command": "run",
        "launch_mode": "recorded",
        "provider_version": "stub",
        "provider_session_id": null,
        "task": "stub",
        "policy": "standard",
        "policy_sha": null,
        "floor_lifted": false,
        "cwd": worktree.display().to_string(),
        "worktree": null,
        "depth": 0,
        "pid": null,
        "process_start_time": null,
        "skills": {
            "store_sha": sha,
            "linked": ["alpha"],
            "resolved": null
        },
        "subscription": "none",
    });

    let spec = layout.launch_spec(run_id);
    watch::write_spec(
        &spec,
        &Launch {
            run_id: run_id.to_string(),
            parent_run_id: None,
            provider: "codex".to_string(),
            binary: "/bin/sh".to_string(),
            argv: vec!["-c".to_string(), script],
            cwd: worktree.clone(),
            run_file: layout.run_file(run_id),
            model: None,
            hydra_env: Default::default(),
            run_start,
            sandbox_profile: None,
            skill_snapshot: Some(snapshot),
        },
    )
    .unwrap();

    let mut watcher =
        watch::detach(&hydra(), &spec, &layout.watcher_log(run_id), &child_env()).unwrap();
    let run_file = layout.run_file(run_id);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();
    assert_eq!(
        watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap(),
        Waited::Ended
    );

    let text = std::fs::read_to_string(&run_file).unwrap();
    check(&text, Some(run_id)).assert_ok();
    assert!(
        text.contains(skills::COPY_MISMATCH_NOTE),
        "run_end.error should name the skill copy mismatch: {text}"
    );
}

#[test]
fn a_writer_that_dies_before_closing_is_told_apart_from_a_slow_one() {
    // The one failure the view cannot read off the record, because the record
    // never gains the line that would say so. `wait_for_end` answers from the
    // process rather than from the file, and does it without waiting out the
    // caller's whole budget.
    let layout = sandbox("writergone");
    let spec = spec_for(&layout, &slow_stub(30));
    let mut watcher =
        watch::detach(&hydra(), &spec, &layout.watcher_log(RUN_ID), &child_env()).unwrap();
    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();

    let child = live::read_view(&run_file).unwrap().pid.unwrap();
    // SAFETY: `watcher` and `child` are live pids this test just read from the run file.
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

/// Each `(call_id, decision)` the reader derives from a record's text.
fn decisions_of(layout: &Layout, text: &str) -> Vec<(String, &'static str)> {
    let policies = hydra::policy::PolicySet::discover().unwrap();
    hydra::normalize::read_with(text, &policies, layout)
        .unwrap()
        .permission_decisions()
        .map(|d| (d.call_id.clone(), d.decision.as_str()))
        .collect()
}

/// Runs `script` to its end under the watcher and returns the record's text.
fn run_to_end(layout: &Layout, script: &str) -> String {
    let mut watcher = start(layout, script);
    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();
    assert_eq!(
        watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap(),
        Waited::Ended
    );
    std::fs::read_to_string(&run_file).unwrap()
}

#[test]
fn a_row_the_run_appends_to_its_own_sidecar_is_read_back_as_hook_evidence() {
    // Build ticket 332. Contract 7.7 calls `gate_refusal` hook evidence and not
    // tamper-proof provenance, because the run's shell can write the sidecar
    // the hook writes. This stub does exactly that mid-run, for a call no
    // frame names, and the record carries the row like any other.
    let layout = sandbox("forged-refusal");
    let sidecar = layout.spawn(RUN_ID).join("gate-refusals.jsonl");
    let forged = serde_json::json!({
        "type": "gate_refusal",
        "tool": "Bash",
        "call_id": "toolu_forged",
        "command": "curl https://example.invalid/forged",
        "command_truncated": false,
        "reason": "hydra/capability: written by the run, not the hook",
    });
    let script = format!(
        r#"printf '{{"type":"system","subtype":"init","model":"stub-1"}}\n'; \
printf '{{"type":"assistant","message":{{"id":"msg_01","role":"assistant","content":[{{"type":"tool_use","id":"toolu_real","name":"Bash","input":{{"command":"ls"}}}}]}}}}\n'; \
printf '{{"type":"user","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"toolu_real","content":"refused","is_error":true}}]}}}}\n'; \
printf '%s\n' '{forged}' >> '{sidecar}'; \
printf '{{"type":"result","subtype":"success","is_error":false}}\n'"#,
        sidecar = sidecar.display(),
    );
    let text = run_to_end(&layout, &script);
    check(&text, Some(RUN_ID)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_with(&text, &policies, &layout).unwrap();
    let rows: Vec<_> = run
        .events
        .iter()
        .filter_map(|event| match event {
            hydra::normalize::Event::GateRefusal(row) => Some(&row.body),
            _ => None,
        })
        .collect();
    assert_eq!(rows, vec![&forged], "{text}");

    let mut shown = Vec::new();
    hydra::show::show(&layout, RUN_ID, true, &mut shown).unwrap();
    let shown: serde_json::Value = serde_json::from_slice(&shown).unwrap();
    let command = shown["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["type"] == "gate_refusal")
        .unwrap()["body"]["command"]
        .as_str()
        .unwrap()
        .to_string();
    let rest = command.strip_prefix("<<<hydra-result ").expect(&command);
    let (token, rest) = rest.split_once(">>>\n").expect(&command);
    assert_eq!(
        rest.strip_suffix(&format!("\n<<<end hydra-result {token}>>>")),
        Some("curl https://example.invalid/forged"),
        "the forged command is printed inside the untrusted-content boundary"
    );

    // The derived decisions come from the provider's stream, so the same
    // record without the forged line derives the same ones.
    let without: String = text
        .lines()
        .filter(|line| !line.contains("\"gate_refusal\""))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(without, text);
    let decisions = decisions_of(&layout, &text);
    assert_eq!(decisions, decisions_of(&layout, &without));
    assert_eq!(decisions.len(), 1, "{decisions:?}");
    assert_eq!(decisions[0].0, "toolu_real");
}

#[test]
fn a_damaged_sidecar_keeps_its_good_rows_and_still_closes_the_record() {
    // Build ticket 332. A truncated line, a line that is not JSON and a row
    // past the line cap, between and after two good rows. The good rows are
    // copied, the damage is said once on `run_end.error`, and run_end is there.
    let layout = sandbox("damaged-refusals");
    let row = |call_id: &str| {
        serde_json::json!({
            "type": "gate_refusal",
            "tool": "Bash",
            "call_id": call_id,
            "command": format!("echo {call_id}"),
            "command_truncated": false,
            "reason": "hydra/capability",
        })
    };
    let oversized = serde_json::json!({
        "type": "gate_refusal",
        "tool": "Bash",
        "call_id": "toolu_oversized",
        "command": "echo oversized",
        "command_truncated": false,
        "reason": "x".repeat(watch::GATE_REFUSAL_LINE_CAP as usize),
    });
    let sidecar = format!(
        "{}\nnot json at all\n{oversized}\n{}\n{{\"type\":\"gate_refusal\",\"tool\":\"Ba",
        row("toolu_first"),
        row("toolu_second"),
    );
    // `write_spec` creates the spawn directory, and the watcher copies the
    // sidecar at close, so writing it before the launch is the same as a hook
    // writing it during the run.
    let spec = spec_for(&layout, &slow_stub(0));
    std::fs::write(layout.spawn(RUN_ID).join("gate-refusals.jsonl"), sidecar).unwrap();
    let mut watcher =
        watch::detach(&hydra(), &spec, &layout.watcher_log(RUN_ID), &child_env()).unwrap();
    let run_file = layout.run_file(RUN_ID);
    watch::wait_for_start(&run_file, &mut watcher, Duration::from_secs(10)).unwrap();
    assert_eq!(
        watch::wait_for_end(&run_file, &mut watcher, Duration::from_secs(20)).unwrap(),
        Waited::Ended
    );
    let text = std::fs::read_to_string(&run_file).unwrap();
    check(&text, Some(RUN_ID)).assert_ok();

    let lines: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let copied: Vec<&str> = lines
        .iter()
        .filter(|line| line["hydra"]["type"] == "gate_refusal")
        .map(|line| line["hydra"]["call_id"].as_str().unwrap())
        .collect();
    assert_eq!(copied, ["toolu_first", "toolu_second"]);
    let end = &lines.last().unwrap()["hydra"];
    assert_eq!(end["type"], "run_end");
    let error = end["error"].as_str().unwrap();
    assert_eq!(error.matches("gate refusal evidence").count(), 1, "{error}");
    assert!(
        error.contains("3 damaged line(s) skipped"),
        "the truncated, non-JSON and oversized lines: {error}"
    );
}
