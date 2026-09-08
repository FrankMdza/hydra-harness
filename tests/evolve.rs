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
