//! `hydra status` and `hydra logs`, harness spec 10 and 17.
//!
//! The scenario harness spec 10 works through is the test: a session with two
//! children in flight dies, and the next status pass has to tell the three states
//! apart from nothing but the pids in the files. Everything here builds those
//! files by hand, so the reconciliation is what is under test rather than the
//! launch that produced them.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::live::{self, State};
use hydra::paths::Layout;
use serde_json::{Value, json};

fn sandbox(name: &str) -> Layout {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-status-{}-{name}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    layout
}

/// A run file with the liveness pair set to whatever the caller wants to prove.
fn write_run(layout: &Layout, run_id: &str, pid: Option<u32>, closed: bool) -> PathBuf {
    let start = pid.and_then(|pid| hydra::procinfo::start_time(pid).ok().flatten());
    let mut text = envelope(
        run_id,
        0,
        json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "provider_session_id": null,
            "task": "probe",
            "policy": "standard",
            "policy_sha": null,
            "cwd": "/tmp",
            "worktree": {
                "path": layout.worktree(run_id).display().to_string(),
                "branch": format!("hydra/{run_id}"),
            },
            "depth": 0,
            "pid": pid,
            "process_start_time": start
                .map(|t| Value::from(t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)))
                .unwrap_or(Value::Null),
            "subscription": "none",
        }),
    );
    if closed {
        text.push_str(&envelope(
            run_id,
            1,
            json!({
                "type": "run_end",
                "exit_code": 0,
                "error": null,
                "refusal": null,
                "remediation": null,
                "body_source": "stream",
                "ended_at": "2026-09-05T20:52:01.905Z",
            }),
        ));
    }
    let path = layout.run_file(run_id);
    std::fs::write(&path, text).unwrap();
    path
}

fn envelope(run_id: &str, seq: u64, hydra: Value) -> String {
    let mut line = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": seq,
        "ts": "2026-09-05T20:51:39.370Z",
        "provider": "claude",
        "model": null,
        "secret_flag": null,
        "hydra": hydra,
    })
    .to_string();
    line.push('\n');
    line
}

/// A pid that is certainly gone: one we started and reaped ourselves.
fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("sleep")
        .arg("5")
        .spawn()
        .unwrap();
    let pid = child.id();
    child.kill().unwrap();
    child.wait().unwrap();
    pid
}

#[test]
fn status_names_the_root_it_read_before_any_count() {
    // A newcomer ran `status` in a clone and was shown 197 runs that were not
    // theirs, with nothing saying whose they were. The root goes first.
    let layout = sandbox("root-line");
    write_run(&layout, "aaa", Some(std::process::id()), false);
    let out = hydra::status::all(&layout).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!("root {}", layout.root().display()),
        "{out}"
    );
    let policies = hydra::policy::PolicySet::discover().unwrap();
    assert_eq!(
        lines[1],
        format!("policies {}", policies.dir().display()),
        "{out}"
    );
}

#[test]
fn the_three_states_come_apart_on_the_pid_and_nothing_else() {
    let layout = sandbox("three");
    write_run(&layout, "aaa", Some(std::process::id()), false);
    write_run(&layout, "bbb", Some(dead_pid()), false);
    write_run(&layout, "ccc", Some(std::process::id()), true);

    let views = live::scan(&layout).unwrap();
    let state = |id: &str| {
        views
            .iter()
            .find(|v| v.run_id == id)
            .unwrap_or_else(|| panic!("{id} is missing"))
            .state
    };
    assert_eq!(state("aaa"), State::Live);
    assert_eq!(state("bbb"), State::Abandoned);
    // A closing line ends a run whatever the pid says, and it has to: the pid is
    // recycled long before the record is read.
    assert_eq!(state("ccc"), State::Ended);

    let report = hydra::status::all(&layout).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    assert!(
        report.contains(&format!("policies {}", policies.dir().display())),
        "{report}"
    );
    assert!(report.contains("live 1"), "{report}");
    assert!(report.contains("abandoned 1"), "{report}");
    assert!(report.contains("ended 1"), "{report}");
    assert!(report.contains("aaa") && report.contains("bbb"), "{report}");
}

#[test]
fn a_worktree_whose_run_is_still_going_is_not_waiting_for_a_reap() {
    let layout = sandbox("trees");
    write_run(&layout, "aaa", Some(std::process::id()), false);
    write_run(&layout, "bbb", Some(std::process::id()), true);
    std::fs::create_dir_all(layout.worktree("aaa")).unwrap();
    std::fs::create_dir_all(layout.worktree("bbb")).unwrap();
    // A checkout whose record was pruned. Nothing can explain it, which is
    // exactly why a human wants it on the list.
    std::fs::create_dir_all(layout.worktree("orphan")).unwrap();

    // Zero grace, so the freshly made directories are judged on their records
    // rather than on their age. The next test covers the age rule.
    let views = live::scan(&layout).unwrap();
    let stale =
        hydra::status::abandoned_worktrees(&layout, &views, std::time::Duration::ZERO).unwrap();
    let names: Vec<String> = stale
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert_eq!(
        names,
        vec!["bbb".to_string(), "orphan".to_string()],
        "{names:?}"
    );
}

#[test]
fn a_checkout_being_created_right_now_is_not_waiting_for_a_reap() {
    // `hydra run` creates the worktree and seeds it before the detached half
    // writes line 0, so for a moment the directory exists and no record claims
    // it. Reporting that as reapable tells a human to remove the checkout of a
    // run that is about to start.
    let layout = sandbox("racing");
    std::fs::create_dir_all(layout.worktree("just-born")).unwrap();

    let views = live::scan(&layout).unwrap();
    assert!(
        hydra::status::abandoned_worktrees(&layout, &views, hydra::run::LAUNCH_BUDGET)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn one_unreadable_run_file_does_not_stop_the_scan() {
    // The scan runs before every launch, so propagating this would mean one bad
    // entry under `~/hydra/runs/` blocks `hydra run` until a human finds it.
    let layout = sandbox("unreadable");
    write_run(&layout, "aaa", Some(std::process::id()), true);
    // A directory named like a record: `File::open` succeeds and the read fails.
    std::fs::create_dir_all(layout.run_file("bbb")).unwrap();

    let views = live::scan(&layout).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].run_id, "aaa");
}

#[test]
fn a_record_that_will_not_read_back_still_reports_where_to_look() {
    // `hydra run` degrades the same way, and this module's own scan says why: a
    // status command that refuses to report a damaged record hides the run most
    // worth looking at.
    let layout = sandbox("damaged");
    write_run(&layout, "aaa", Some(dead_pid()), true);
    // Truncate the record to a closing line with nothing before it, so the
    // normalizer has no run to build.
    let path = layout.run_file("aaa");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.lines().next_back().unwrap()).unwrap();

    let report: Value = serde_json::from_str(&hydra::status::one(&layout, "aaa").unwrap()).unwrap();
    assert_eq!(report["state"], "ended");
    assert!(report["run_file"].as_str().unwrap().ends_with("aaa.jsonl"));
}

#[test]
fn a_live_run_reports_its_state_and_no_outcome() {
    // Contract 8.6 gives telling a live child from an abandoned one to this
    // command, because `derive` cannot: a file with no closing line has no ending
    // to read, and a page of nulls beside it would read as measurements.
    let layout = sandbox("live");
    write_run(&layout, "aaa", Some(std::process::id()), false);

    let report: Value = serde_json::from_str(&hydra::status::one(&layout, "aaa").unwrap()).unwrap();
    assert_eq!(report["state"], "live");
    assert_eq!(report["pid"], std::process::id());
    assert_eq!(report["policy"], "standard");
    assert_eq!(report["branch"], "hydra/aaa");
    assert!(report.get("result").is_none(), "{report}");
    // The checkout was never created, so the record points at a path that is gone.
    assert_eq!(report["worktree_present"], false);
}

#[test]
fn an_ended_run_reports_the_same_projection_hydra_run_returns() {
    let layout = sandbox("ended");
    write_run(&layout, "aaa", Some(dead_pid()), true);

    let report: Value = serde_json::from_str(&hydra::status::one(&layout, "aaa").unwrap()).unwrap();
    assert_eq!(report["state"], "ended");
    let result = &report["result"];
    assert_eq!(result["run_id"], "aaa");
    // No `result` frame and no error: the child ended without its terminator,
    // which is the one shape where the process is the thing that stopped.
    assert_eq!(result["outcome"], "crashed");
    assert_eq!(result["outcome_source"], "derived");
    assert_eq!(result["exit_code"], 0);
}

#[test]
fn asking_about_a_run_that_is_not_there_says_so() {
    let layout = sandbox("missing");
    let err = hydra::status::one(&layout, "nope").unwrap_err().to_string();
    assert!(err.contains("no run record"), "{err}");
}

#[test]
fn logs_prints_the_run_file_verbatim() {
    let layout = sandbox("logs");
    let path = write_run(&layout, "aaa", Some(std::process::id()), true);

    let mut out = Vec::new();
    hydra::logs::logs(&layout, "aaa", false, &mut out).unwrap();
    assert_eq!(out, std::fs::read(&path).unwrap());
}

#[test]
fn following_stops_at_the_closing_line() {
    let layout = sandbox("follow");
    write_run(&layout, "aaa", Some(std::process::id()), false);
    let path = layout.run_file("aaa");

    // The record grows under the tail, the way a live run's does.
    let writer = std::thread::spawn(move || {
        for seq in 1..4 {
            std::thread::sleep(std::time::Duration::from_millis(120));
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            file.write_all(envelope("aaa", seq, json!({"type": "note"})).as_bytes())
                .unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(120));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(envelope("aaa", 4, json!({"type": "run_end", "exit_code": 0})).as_bytes())
            .unwrap();
    });

    let mut out = Vec::new();
    hydra::logs::logs(&layout, "aaa", true, &mut out).unwrap();
    writer.join().unwrap();

    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.lines().count(), 5, "{text}");
    assert!(text.contains("run_end"), "{text}");
}

#[test]
fn following_does_not_give_up_in_the_gap_before_the_closing_line() {
    // The writer reaps the child and then writes `run_end`, so every ordinary run
    // passes through a moment where the pid is gone and the record has not ended.
    // A follow that gives up there drops the closing line off the tail and calls a
    // finished run abandoned. The dead pid is written from the start here, which
    // is that window held open.
    let layout = sandbox("gap");
    write_run(&layout, "aaa", Some(dead_pid()), false);
    let path = layout.run_file("aaa");

    let writer = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(envelope("aaa", 1, json!({"type": "run_end", "exit_code": 0})).as_bytes())
            .unwrap();
    });

    let mut out = Vec::new();
    hydra::logs::logs(&layout, "aaa", true, &mut out).unwrap();
    writer.join().unwrap();

    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("run_end"),
        "the closing line was dropped: {text}"
    );
    assert_eq!(text.lines().count(), 2, "{text}");
}

#[test]
fn following_an_abandoned_run_ends_rather_than_waiting_forever() {
    let layout = sandbox("abandoned-follow");
    write_run(&layout, "aaa", Some(dead_pid()), false);

    let mut out = Vec::new();
    hydra::logs::logs(&layout, "aaa", true, &mut out).unwrap();
    assert_eq!(String::from_utf8(out).unwrap().lines().count(), 1);
}

// --- interactive harvest, section 10's "any process can do it" ----------

/// A pid that is certainly gone, with the start time it had while it still
/// lived. Contract 5.2 fills both together at launch, before either can be
/// null, so a run file that means "the process died since" needs a real
/// timestamp rather than the one a query against a dead pid answers with.
fn dead_pid_with_start() -> (u32, chrono::DateTime<chrono::Utc>) {
    let mut child = std::process::Command::new("sleep")
        .arg("5")
        .spawn()
        .unwrap();
    let pid = child.id();
    let start = hydra::procinfo::start_time(pid).unwrap().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    (pid, start)
}

/// A dead interactive `claude` run, no `run_end`: harness spec 10's scenario,
/// a closed terminal on a session that kept running until it did not.
fn write_dead_interactive_run(
    layout: &Layout,
    run_id: &str,
    pid: u32,
    start: chrono::DateTime<chrono::Utc>,
    session: &str,
    cwd: &std::path::Path,
) {
    let text = envelope(
        run_id,
        0,
        json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "interactive",
            "provider_version": "2.1.261 (Claude Code)",
            "provider_session_id": session,
            "task": "probe",
            "policy": null,
            "policy_sha": null,
            "cwd": cwd.display().to_string(),
            "worktree": null,
            "depth": 0,
            "pid": pid,
            "process_start_time": start.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            "subscription": "anthropic",
        }),
    );
    std::fs::write(layout.run_file(run_id), text).unwrap();
}

fn write_transcript_fixture(home: &std::path::Path, cwd: &std::path::Path, session: &str) {
    let path = hydra::provider::claude::transcript_path(home, cwd, session);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let line = json!({
        "type": "assistant",
        "uuid": "u1",
        "parentUuid": null,
        "message": {
            "role": "assistant",
            "content": [{"type": "text", "text": "done"}],
            "usage": {"input_tokens": 1, "output_tokens": 2},
        },
    });
    std::fs::write(&path, format!("{line}\n")).unwrap();
}

#[test]
fn a_dead_interactive_run_is_harvested_by_status_all() {
    let layout = sandbox("interactive-dead-all");
    let root = std::env::temp_dir().join(format!(
        "hydra-status-interactive-{}-all",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let cwd = root.join("cwd");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();

    let run_id = "01a07901-0000-7000-8000-000000000001";
    let session = "01a07901-0000-7000-8000-000000000002";
    let (pid, start) = dead_pid_with_start();
    write_dead_interactive_run(&layout, run_id, pid, start, session, &cwd);
    write_transcript_fixture(&home, &cwd, session);

    // Abandoned before the harvest: no `run_end`, and the pid is gone.
    let before = live::scan(&layout).unwrap();
    assert_eq!(
        before.iter().find(|v| v.run_id == run_id).unwrap().state,
        State::Abandoned
    );

    let report = hydra::status::all_in(&layout, &home).unwrap();
    assert!(report.contains("ended 1"), "{report}");
    assert!(!report.contains("abandoned 1"), "{report}");

    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();
    assert!(text.contains("\"body_source\":\"harvest\""), "{text}");
}

#[test]
fn a_dead_interactive_run_is_harvested_by_status_one() {
    let layout = sandbox("interactive-dead-one");
    let root = std::env::temp_dir().join(format!(
        "hydra-status-interactive-{}-one",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let cwd = root.join("cwd");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();

    let run_id = "01a07902-0000-7000-8000-000000000001";
    let session = "01a07902-0000-7000-8000-000000000002";
    let (pid, start) = dead_pid_with_start();
    write_dead_interactive_run(&layout, run_id, pid, start, session, &cwd);
    write_transcript_fixture(&home, &cwd, session);

    let report: Value =
        serde_json::from_str(&hydra::status::one_in(&layout, run_id, &home).unwrap()).unwrap();
    assert_eq!(report["state"], "ended");
    // Nobody watched this session exit, which is exactly what makes the
    // outcome's confidence, rather than the outcome itself, the honest cost.
    assert_eq!(report["result"]["outcome"], "completed");
    assert_eq!(report["result"]["outcome_source"], "unknown");
}

#[test]
fn concurrent_launchers_reserve_exactly_the_cap_without_records() {
    let layout = sandbox("reservations-race");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(16));
    let threads: Vec<_> = (0..16)
        .map(|i| {
            let layout = Layout::at(layout.root());
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                live::reserve(&layout, &format!("run-{i}"), 3)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 3);
    for err in results.iter().filter_map(|r| r.as_ref().err()) {
        assert!(err.downcast_ref::<live::TooManyLive>().is_some(), "{err:#}");
    }
    assert!(live::scan(&layout).unwrap().is_empty());
    drop(results);
    assert!(live::reserve(&layout, "after-failure", 1).is_ok());
}

#[test]
fn dead_or_reused_launcher_pids_release_reservations() {
    let layout = sandbox("reservations-dead");
    let dir = layout.reservations();
    std::fs::create_dir_all(&dir).unwrap();
    let pid = std::process::id();
    let start = hydra::procinfo::start_time(pid).unwrap().unwrap();
    // An impossible pid needs no child process. The other identity models reuse.
    for (id, pid, start) in [
        ("dead", i32::MAX as u32, start),
        ("reused", pid, start - chrono::Duration::seconds(1)),
    ] {
        std::fs::write(
            dir.join(format!("{id}.json")),
            json!({"pid": pid, "process_start_time": start.to_rfc3339()}).to_string(),
        )
        .unwrap();
    }
    let _reservation = live::reserve(&layout, "new", 1).unwrap();
    assert!(!dir.join("dead.json").exists());
    assert!(!dir.join("reused.json").exists());
}

#[test]
fn reservation_handoff_counts_the_run_once_and_releases_on_end() {
    let layout = sandbox("reservations-handoff");
    let reservation = live::reserve(&layout, "first", 2).unwrap();
    write_run(&layout, "first", Some(std::process::id()), false);
    let second = live::reserve(&layout, "second", 2).unwrap();
    assert!(!layout.reservations().join("first.json").exists());
    let err = live::reserve(&layout, "third", 2).unwrap_err();
    assert!(err.downcast_ref::<live::TooManyLive>().is_some());
    assert!(!layout.run_file("third").exists());
    drop(reservation);
    drop(second);
    write_run(&layout, "first", Some(std::process::id()), true);
    assert!(live::reserve(&layout, "after-end", 1).is_ok());
}

#[test]
fn an_unparseable_reservation_is_stale_and_removed() {
    let layout = sandbox("reservations-malformed");
    let dir = layout.reservations();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("garbage.json"), b"{not json").unwrap();
    std::fs::write(
        dir.join("bad-time.json"),
        json!({"pid": std::process::id(), "process_start_time": "yesterday"}).to_string(),
    )
    .unwrap();
    let _reservation = live::reserve(&layout, "new", 1).unwrap();
    assert!(!dir.join("garbage.json").exists());
    assert!(!dir.join("bad-time.json").exists());
}

#[test]
fn a_refused_duplicate_reservation_leaves_no_temp_file() {
    let layout = sandbox("reservations-duplicate");
    let _first = live::reserve(&layout, "same", 3).unwrap();
    let err = live::reserve(&layout, "same", 3).unwrap_err();
    assert!(err.to_string().contains("already has a reservation"));
    assert!(!layout.reservations().join("same.tmp").exists());
    assert!(layout.reservations().join("same.json").exists());
}
