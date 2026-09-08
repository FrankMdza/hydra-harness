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
