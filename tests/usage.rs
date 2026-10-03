//! `hydra usage recount [<run-id>]`, harness spec 17.
//!
//! Two synthetic records, one with a provider usage frame and one with none,
//! tallied the way the command walks a corpus the test owns rather than the
//! machine's.

mod common;
use common::{Record, Sandbox, policies, usage_frame};

const WITH_USAGE: &str = "01a07357-58a5-79e2-a25a-3df578a750b4";
const WITHOUT_USAGE: &str = "01a07358-58a5-79e2-a25a-3df578a750b5";

fn write_fixture(layout: &hydra::paths::Layout) {
    for run_id in [WITH_USAGE, WITHOUT_USAGE] {
        std::fs::create_dir_all(layout.worktree(run_id)).unwrap();
    }

    let with = Record::opened(
        &layout.worktree(WITH_USAGE),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(usage_frame())
    .closed();
    std::fs::write(layout.run_file(WITH_USAGE), with).unwrap();

    let without = Record::opened(
        &layout.worktree(WITHOUT_USAGE),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(serde_json::json!({}))
    .closed();
    std::fs::write(layout.run_file(WITHOUT_USAGE), without).unwrap();
}

#[test]
fn recount_tallies_provider_and_unavailable_across_the_corpus() {
    let sandbox = Sandbox::new("usage-recount");
    let layout = sandbox.layout();
    write_fixture(&layout);

    let mut out = Vec::new();
    hydra::usage::recount(&layout, None, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();

    assert!(text.contains("provider: 1\n"), "{text}");
    assert!(text.contains("estimated: 0\n"), "{text}");
    assert!(text.contains("unavailable: 1\n"), "{text}");
    assert!(
        text.contains("nothing to recount: no run carries usage_source estimated\n"),
        "{text}"
    );
}

#[test]
fn recount_one_run_prints_the_same_tally_for_that_record() {
    let sandbox = Sandbox::new("usage-one");
    let layout = sandbox.layout();
    write_fixture(&layout);

    let mut out = Vec::new();
    hydra::usage::recount(&layout, Some(WITH_USAGE), &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();

    assert!(text.contains("provider: 1\n"), "{text}");
    assert!(text.contains("estimated: 0\n"), "{text}");
    assert!(text.contains("unavailable: 0\n"), "{text}");
    assert!(
        text.contains("nothing to recount: no run carries usage_source estimated\n"),
        "{text}"
    );
}

#[test]
fn recount_one_missing_run_fails_with_the_path() {
    let sandbox = Sandbox::new("usage-missing");
    let layout = sandbox.layout();
    let path = layout.run_file("does-not-exist");

    let mut out = Vec::new();
    let err = hydra::usage::recount(&layout, Some("does-not-exist"), &mut out).unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("no run record at"), "{msg}");
    assert!(msg.contains(&path.display().to_string()), "{msg}");
}

/// Build ticket 259: `codex` counts the cache inside `input_tokens`, since its
/// `total_tokens` is input plus output, and contract 9.1 counts input net of
/// it. The same mapping reads the stream's `turn.completed` and a session
/// file's `token_count`.
#[test]
fn a_codex_input_count_is_net_of_its_cache_on_both_paths() {
    use common::Shape;
    use serde_json::json;

    let sandbox = Sandbox::new("usage-codex-cache");
    let worktree = sandbox.worktree();
    let usage = json!({
        "input_tokens": 1000, "cached_input_tokens": 800,
        "cache_write_input_tokens": 0, "output_tokens": 50,
        "reasoning_output_tokens": 10, "total_tokens": 1050,
    });
    let frames = [
        json!({"type": "turn.completed", "usage": usage}),
        json!({"type": "event_msg", "payload": {
            "type": "token_count", "info": {"total_token_usage": usage},
        }}),
    ];
    for frame in frames {
        let mut record = Record::shaped(Shape {
            provider: Some("codex"),
            policy: Some("standard"),
            policy_sha: Some(policies().sha()),
            worktree: Some(&worktree),
            ..Shape::default()
        });
        record.raw(frame);
        let text = record.closed();
        let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
        let counts = run.usage().and_then(|usage| usage.counts).unwrap();
        assert_eq!(counts.input_tokens, 200);
        assert_eq!(counts.cache_read_tokens, 800);
        assert_eq!(counts.output_tokens, 50);
    }
}
