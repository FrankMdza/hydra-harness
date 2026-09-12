//! Ouroboros rung two, `docs/ouroboros-spec.md` section 4.
//!
//! Nothing here launches a provider. Every test builds a scratch [`Layout`] under
//! a temporary directory and calls the library directly.

use std::path::PathBuf;

use hydra::ouroboros::evolve::{
    PATCH_LINE_CAP, patch_over_cap, protected_path_hit, write_harness_end, write_harness_start,
};
use hydra::ouroboros::review::{
    ApprovedPatch, PanelVerdict, VerdictField, panel_verdict, parse_judge_answer,
};
use hydra::paths::Layout;
use hydra::record::validate::check;
use hydra::worktree::Stat;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("hydra-evolve-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let fixture = Self { root };
        fixture.layout().ensure().unwrap();
        fixture
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn pass_answer(verdict: &str, findings: &str) -> String {
    format!(r#"{{"verdict":"{verdict}","findings":{findings}}}"#)
}

#[test]
fn a_patch_over_the_line_cap_is_rejected_for_size() {
    let stat = Stat {
        files_changed: 50,
        insertions: PATCH_LINE_CAP as u64,
        deletions: 1,
    };
    assert_eq!(patch_over_cap(&stat), Some(PATCH_LINE_CAP as usize + 1));
    let under = Stat {
        files_changed: 1,
        insertions: 10,
        deletions: 10,
    };
    assert!(patch_over_cap(&under).is_none());
}

#[test]
fn a_diff_naming_a_protected_path_is_refused_at_admission() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let hit = protected_path_hit(&["config/preamble.md".into()], &repo);
    assert_eq!(hit.as_deref(), Some("config/preamble.md"));
    let ok = protected_path_hit(&["src/lib.rs".into()], &repo);
    assert!(ok.is_none());
}

#[test]
fn a_diff_naming_a_code_row_of_harness_15_2_is_refused_before_any_judge() {
    // Build ticket 40. Harness spec 15.2's five code rows: the review module, the
    // protected-path list, the hook-config writer, the run-record writer and the
    // admission gate. Each is planted as the one path of a patch, alone, so a
    // refusal names that row and no other. The repo is a scratch path rather than
    // this checkout: the check is on the names, and nothing has to exist.
    let repo = PathBuf::from("/scratch/hydra-evolve-rows");
    let rows = [
        "src/ouroboros/review.rs",
        "src/ouroboros/evolve.rs",
        "src/policy/mod.rs",
        "src/policy/table.rs",
        "src/policy/gate.py",
        "src/record/mod.rs",
        "src/record/lines.rs",
        "src/env.rs",
    ];
    for row in rows {
        let hit = protected_path_hit(&[row.to_string()], &repo);
        assert_eq!(hit.as_deref(), Some(row), "{row} reached the panel");
    }
    // Case is not a way past it: `write_file(path="Review.rs")` on macOS lands
    // on the same file.
    let cased = protected_path_hit(&["src/Ouroboros/REVIEW.rs".into()], &repo);
    assert_eq!(cased.as_deref(), Some("src/Ouroboros/REVIEW.rs"));
    // An absolute path under the repo is the same patch spelled longer.
    let absolute = repo.join("src/record/validate.rs").display().to_string();
    let hit = protected_path_hit(&[absolute.clone()], &repo);
    assert_eq!(hit.as_deref(), Some(absolute.as_str()));
    // Every row is checked against every path, so a clean patch with one bad
    // path in the middle is refused on that path.
    let mixed = protected_path_hit(
        &[
            "src/lib.rs".into(),
            "src/ouroboros/curate.rs".into(),
            "src/record/secrets.rs".into(),
        ],
        &repo,
    );
    assert_eq!(mixed.as_deref(), Some("src/record/secrets.rs"));
}

#[test]
fn a_diff_outside_the_protected_rows_passes_admission() {
    // Rung two's own launch sequence, the corpus reader and the curate pass are
    // not rows, and neither is a file whose name merely starts like one.
    let repo = PathBuf::from("/scratch/hydra-evolve-rows");
    let ok = protected_path_hit(
        &[
            "src/run.rs".into(),
            "src/ouroboros/corpus.rs".into(),
            "src/ouroboros/curate.rs".into(),
            "src/policyholder.rs".into(),
            "src/recorder.rs".into(),
            "src/environment.rs".into(),
            "tests/evolve.rs".into(),
            "docs/ouroboros-spec.md".into(),
        ],
        &repo,
    );
    assert!(ok.is_none(), "{ok:?} was refused");
}

#[test]
fn three_judge_answers_in_the_closed_shape_produce_the_expected_verdict() {
    let pass = pass_answer("pass", "[]");
    let outcomes = [
        parse_judge_answer("claude", &pass),
        parse_judge_answer("cursor-agent", &pass),
        parse_judge_answer("agy", &pass),
    ];
    assert!(matches!(
        panel_verdict("01a08000-0000-7000-8000-00000000ev01", &outcomes),
        PanelVerdict::Offerable(_)
    ));
}

#[test]
fn one_critical_finding_blocks_whatever_the_verdict_field_says() {
    let answer = pass_answer(
        "pass",
        r#"[{"item":"secrets_check","severity":"critical","note":"token in src"}]"#,
    );
    let one = parse_judge_answer("claude", &answer);
    assert_eq!(one.verdict, VerdictField::Pass);
    let blocked = panel_verdict(
        "01a08000-0000-7000-8000-00000000ev02",
        &[one.clone(), one.clone(), one],
    );
    assert!(matches!(blocked, PanelVerdict::Blocked { .. }));
}

#[test]
fn prose_outside_the_closed_shape_is_a_block_with_a_parse_error() {
    let answer = format!("looks good {}\n", pass_answer("pass", "[]"));
    let outcome = parse_judge_answer("claude", &answer);
    assert!(outcome.parse_error.is_some());
    let verdict = panel_verdict("01a08000-0000-7000-8000-00000000ev03", &[outcome]);
    assert!(matches!(verdict, PanelVerdict::Blocked { .. }));
}

#[test]
fn the_harness_record_is_two_lines_with_only_contract_fields() {
    let fixture = Fixture::new("harness");
    let pass_id = "01a08000-0000-7000-8000-00000000ev04";
    let path = fixture.layout().run_file(pass_id);
    write_harness_start(&fixture.layout(), pass_id, &path).unwrap();
    write_harness_end(&path, Some(0), None).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.lines().count(), 2);
    let report = check(&text, Some(pass_id));
    report.assert_ok();
    let start = text.lines().next().unwrap();
    assert!(start.contains(r#""command":"evolve""#));
    assert!(start.contains(r#""provider":null"#));
    assert!(!start.contains(r#""skills""#));
}

#[test]
fn approved_patch_cannot_be_built_outside_the_review_module() {
    let pass = pass_answer("pass", "[]");
    let outcomes = [
        parse_judge_answer("claude", &pass),
        parse_judge_answer("cursor-agent", &pass),
        parse_judge_answer("agy", &pass),
    ];
    let verdict = panel_verdict("01a08000-0000-7000-8000-00000000ev05", &outcomes);
    let patch = match verdict {
        PanelVerdict::Offerable(patch) => patch,
        PanelVerdict::Blocked { .. } => panic!("expected offerable"),
    };
    assert_eq!(patch.pass_run_id(), "01a08000-0000-7000-8000-00000000ev05");
    let _proof: ApprovedPatch = patch;
}

#[test]
fn one_fence_comes_off_and_everything_else_still_blocks() {
    // Map ticket 52's answer, amended 2026-09-08 after the first real panel:
    // the `claude` judge returned the exact object inside a ```json fence and
    // was recorded as a block for a syntax error. One wrapping fence comes off.
    let object = r#"{"verdict": "pass", "findings": []}"#;
    for wrapped in [
        format!("```json\n{object}\n```"),
        format!("```\n{object}\n```"),
        format!("```JSON\n{object}\n```\n"),
    ] {
        let outcome = parse_judge_answer("claude", &wrapped);
        assert!(outcome.parse_error.is_none(), "{wrapped}: {outcome:?}");
        assert_eq!(outcome.verdict, VerdictField::Pass);
    }

    // Everything else the ticket named still blocks.
    for deviant in [
        format!("Here is my review:\n\n```json\n{object}\n```"),
        format!("```json\n{object}\n```\n\nLet me know if you need more."),
        format!("```json\n{object}\n```\n```json\n{object}\n```"),
        format!("```json\n{object}"),
        format!("```rust\n{object}\n```"),
    ] {
        let outcome = parse_judge_answer("claude", &deviant);
        assert!(outcome.parse_error.is_some(), "{deviant} parsed");
        assert_eq!(outcome.verdict, VerdictField::Block);
    }
}
