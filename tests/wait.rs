mod common;
#[path = "../src/wait.rs"]
mod wait;
use common::{Record, Sandbox};
use hydra::paths::Layout;
use serde_json::json;
use std::time::Duration;

fn record(layout: &Layout, id: &str, closed: bool) {
    let mut r = Record::opened(&layout.worktree(id), None, None);
    let text = if closed {
        r.result(json!({})).closed()
    } else {
        r.unclosed()
    };
    let pid = std::process::id();
    let start = hydra::procinfo::start_time(pid).unwrap().unwrap();
    let text = text
        .lines()
        .map(|line| {
            let mut v: serde_json::Value = serde_json::from_str(line).unwrap();
            v["run_id"] = json!(id);
            if v["hydra"]["type"] == "run_start" {
                v["hydra"]["pid"] = json!(pid);
                v["hydra"]["process_start_time"] = json!(start.to_rfc3339());
            }
            v.to_string() + "\n"
        })
        .collect::<String>();
    std::fs::write(layout.run_file(id), text).unwrap();
}

fn scenario(any: bool) {
    let sandbox = Sandbox::new(if any { "wait-any" } else { "wait-all" });
    let layout = sandbox.layout();
    record(&layout, "first", false);
    record(&layout, "second", false);
    std::thread::scope(|scope| {
        let (tx, rx) = std::sync::mpsc::channel();
        let layout_ref = &layout;
        scope.spawn(move || {
            let mut out = Vec::new();
            let code = wait::wait(
                layout_ref,
                &["fir".into(), "sec".into()],
                any,
                Some(Duration::from_secs(8)),
                &mut out,
            )
            .unwrap();
            tx.send((code, String::from_utf8(out).unwrap())).unwrap();
        });
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
        record(&layout, "first", true);
        if !any {
            assert!(rx.recv_timeout(Duration::from_millis(2300)).is_err());
            record(&layout, "second", true);
        }
        let (code, out) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(code, 0);
        assert!(out.contains("first\tclaude\tunknown\tcompleted\tunavailable"));
        assert!(out.contains("second\tclaude\t"));
        assert_eq!(
            hydra::live::read_view(&layout.run_file("second"))
                .unwrap()
                .is_live(),
            any
        );
    });
}
#[test]
fn returns_when_last_run_ends() {
    scenario(false);
}
#[test]
fn any_returns_when_first_run_ends() {
    scenario(true);
}
#[test]
fn timeout_returns_two_and_lists_only_live_runs() {
    use clap::Parser;
    let cli = hydra::cli::Cli::try_parse_from([
        "hydra",
        "wait",
        "ended",
        "running",
        "--any",
        "--timeout",
        "30m",
    ])
    .unwrap();
    assert!(
        matches!(cli.command, hydra::cli::Command::Wait { runs, any: true, timeout: Some(t) } if runs.len() == 2 && t == Duration::from_secs(1800))
    );
    let sandbox = Sandbox::new("wait-timeout");
    let layout = sandbox.layout();
    record(&layout, "ended", true);
    record(&layout, "running", false);
    let mut out = Vec::new();
    assert_eq!(
        wait::wait(
            &layout,
            &["ended".into(), "running".into()],
            false,
            Some(Duration::ZERO),
            &mut out
        )
        .unwrap(),
        2
    );
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("still live: running"));
    assert!(!out.contains("still live: ended"));
}
