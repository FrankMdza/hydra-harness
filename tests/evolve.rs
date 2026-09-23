//! Rung two, `docs/rungs-spec.md` section 4.
//!
//! Nothing here launches a provider. Every test builds a scratch [`Layout`] under
//! a temporary directory and calls the library directly.

use std::path::{Path, PathBuf};
use std::process::Command;

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::record::validate::check;
use hydra::run::write_spawn_config;
use hydra::rungs::evolve::{
    HUMAN_EVIDENCE_HEADING, PATCH_LINE_CAP, PROTECTED_CODE_PATHS, ProposerPatch,
    TASK_DOES_NOT_WAIVE_CHECKLIST, admission_gate, ensure_reviewable, judge_task,
    pass_run_id_from_branch, patch_over_cap, protected_path_hit, rate_limit_abort,
    review_commit_message, write_harness_end, write_harness_start,
};
use hydra::rungs::review::{
    ApprovedPatch, CHECKLIST_ENTRIES, CHECKLIST_IDS, PanelAnswers, PanelVerdict, VerdictField,
    checklist_prompt, panel_verdict, parse_judge_answer,
};
use hydra::worktree::Stat;
use serde_json::{Value, json};

mod common;
use common::{Record, Shape, usage_frame};

#[test]
fn admission_write_is_not_carried_by_the_offered_branch() {
    let fixture = Fixture::new("admission-tree");
    let repo = scratch_repo("admission-tree");
    std::fs::create_dir_all(repo.join("src/policy")).unwrap();
    std::fs::write(repo.join("src/policy/guard.rs"), "// original\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "protected baseline"]);
    std::fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"tree-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(repo.join("src/lib.rs"), "pub fn ok() {}\n").unwrap();
    std::fs::write(repo.join(".gitignore"), "/target\n/Cargo.lock\n").unwrap();
    std::fs::write(repo.join("build.rs"), "fn main() { std::fs::write(\"src/policy/guard.rs\", \"// admission edit\\n\").unwrap(); }\n").unwrap();
    let base_tree = git(&repo, &["rev-parse", "HEAD^{tree}"]);
    let run_id = "01a08000-0000-7000-8000-00000000ad02";
    let record = Record::shaped(Shape {
        run_id,
        worktree: Some(&repo),
        base_tree: Some(base_tree.trim()),
        ..Shape::default()
    })
    .result(usage_frame())
    .closed();
    let run_file = fixture.layout().run_file(run_id);
    std::fs::write(&run_file, record).unwrap();
    let result = hydra::outcome::RunResult {
        run_id: run_id.into(),
        outcome: Some("completed"),
        outcome_source: Some("derived"),
        exit_code: Some(0),
        error: None,
        result_text: None,
        branch: None,
        diff_stat: None,
        usage: hydra::outcome::Usage {
            usage_source: "unknown",
            estimator: None,
            counts: None,
        },
        run_file: run_file.to_string_lossy().into_owned(),
    };
    let patch = hydra::rungs::evolve::read_proposer_patch(
        &fixture.layout(),
        &PolicySet::discover().unwrap(),
        &result,
    )
    .unwrap();
    let read_tree = patch.tree.clone();
    assert_eq!(patch.stat.files_changed, 4);
    assert_eq!(
        patch.paths,
        [".gitignore", "Cargo.toml", "build.rs", "src/lib.rs"]
    );
    assert!(patch.diff.contains("diff --git a/build.rs b/build.rs"));
    assert!(protected_path_hit(&patch.paths, &repo).is_none());
    assert!(patch_over_cap(&patch.stat).is_none());
    let clean = pass_answer("pass", "[]");
    let answers = PanelAnswers::new(
        parse_judge_answer("claude", &clean),
        parse_judge_answer("cursor-agent", &clean),
        parse_judge_answer("agy", &clean),
        parse_judge_answer("codex", &clean),
    );
    let PanelVerdict::Offerable(approved) = panel_verdict("test", &read_tree, &answers) else {
        panic!("clean panel must offer the patch");
    };
    assert_eq!(approved.tree(), read_tree);
    // The same commit path also preserves a worktree that has not changed.
    hydra::rungs::evolve::commit_pass_branch(
        &repo,
        &patch,
        "rungs/unchanged",
        "verdict: offerable",
    )
    .unwrap();
    assert_eq!(
        git(&repo, &["rev-parse", "rungs/unchanged^{tree}"]).trim(),
        read_tree
    );
    // Run admission's build step with a real writing build script, without a provider.
    let build = Command::new("cargo")
        .args(["build", "--offline"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("src/policy/guard.rs")).unwrap(),
        "// admission edit\n"
    );
    // Admission's code can also move the checkout's `HEAD`; the verdict commit
    // keeps the parent read with the tree.
    let read_head = patch.head.clone();
    git(
        &repo,
        &["commit", "-q", "--allow-empty", "-m", "moved by admission"],
    );
    hydra::rungs::evolve::commit_pass_branch(&repo, &patch, "rungs/test", "verdict: offerable")
        .unwrap();
    assert_eq!(
        git(&repo, &["show", "rungs/test:src/policy/guard.rs"]),
        "// original\n"
    );
    assert_eq!(
        git(&repo, &["rev-parse", "rungs/test^{tree}"]).trim(),
        read_tree
    );
    assert!(
        git(&repo, &["log", "-1", "--format=%B", "rungs/test"])
            .contains(&format!("read-tree: {read_tree}"))
    );
    assert_eq!(git(&repo, &["rev-parse", "rungs/test^"]).trim(), read_head);
    std::fs::remove_dir_all(repo).unwrap();
}

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

/// A `rate_limit_event` frame the way `claude` writes it, contract 8.5: the status
/// nested under `rate_limit_info`, beside the windows nothing keys on.
fn rate_limit_frame(status: &str) -> Value {
    json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "rateLimitType": "seven_day",
            "status": status,
            "resetsAt": 1_758_000_000,
            "overageStatus": "rejected",
            "overageDisabledReason": null,
            "isUsingOverage": false,
            "unifiedWindows": { "seven_day": { "utilization": 0.24, "resetsAt": 1_758_000_000 } }
        }
    })
}

/// A recorded `claude` run under `standard`, the shape the proposer leaves, with
/// the given frames in it, written where `rate_limit_abort` looks.
fn proposer_record(fixture: &Fixture, run_id: &str, frames: &[Value]) {
    let mut record = Record::shaped(Shape {
        run_id,
        policy: Some("standard"),
        ..Shape::default()
    });
    for frame in frames {
        record.raw(frame.clone());
    }
    let text = record.result(usage_frame()).closed();
    std::fs::write(fixture.layout().run_file(run_id), text).unwrap();
}

#[test]
fn a_harness_run_ends_with_a_null_body_source() {
    // Contract 6: a pass owns no pty and tees no frames. The eight harness
    // records written before 2026-09-14 say `stream`, which is the only place
    // the corpus was measured contradicting a contract table.
    let dir = std::env::temp_dir().join(format!("hydra-harness-end-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("01a00000-0000-7000-8000-000000000000.jsonl");
    std::fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::json!({
                "run_id": "01a00000-0000-7000-8000-000000000000",
                "seq": 0,
                "hydra": {"type": "run_start", "command": "evolve"}
            })
        ),
    )
    .unwrap();

    hydra::rungs::evolve::write_harness_end(&path, Some(0), None, None).unwrap();

    let last: serde_json::Value = serde_json::from_str(
        std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(last["hydra"]["type"], "run_end");
    assert!(last["hydra"]["body_source"].is_null(), "{}", last["hydra"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_proposer_whose_last_rate_limit_frame_is_rejected_aborts_before_any_judge() {
    // Spec 4.7. The status is under `rate_limit_info`, and until build ticket 41
    // the abort read the top of the frame and never fired.
    let fixture = Fixture::new("rate-limit-rejected");
    proposer_record(
        &fixture,
        "01a08000-0000-7000-8000-00000000rl01",
        &[rate_limit_frame("allowed"), rate_limit_frame("rejected")],
    );
    let abort = rate_limit_abort(&fixture.layout()).unwrap();
    let reason = abort.expect("a rejected proposer aborts the pass");
    assert!(reason.contains("rejected"), "{reason}");
    assert!(reason.contains("proposer"), "{reason}");
}

#[test]
fn a_proposer_whose_last_rate_limit_frame_is_allowed_does_not_abort() {
    // A run warned early and allowed later is not rate-limited: the last frame
    // is the one that counts.
    let fixture = Fixture::new("rate-limit-allowed");
    proposer_record(
        &fixture,
        "01a08000-0000-7000-8000-00000000rl02",
        &[
            rate_limit_frame("allowed_warning"),
            rate_limit_frame("allowed"),
        ],
    );
    assert_eq!(rate_limit_abort(&fixture.layout()).unwrap(), None);
}

#[test]
fn a_frame_carrying_status_at_the_top_still_counts() {
    // The shape contract 8.5 described before the frame was measured. Nothing
    // on this machine wrote it, and reading it costs one fallback.
    let fixture = Fixture::new("rate-limit-flat");
    proposer_record(
        &fixture,
        "01a08000-0000-7000-8000-00000000rl03",
        &[json!({ "type": "rate_limit_event", "status": "rejected" })],
    );
    let reason = rate_limit_abort(&fixture.layout()).unwrap().unwrap();
    assert!(reason.contains("rejected"), "{reason}");
}

#[test]
fn a_newer_run_from_another_provider_is_not_the_one_read() {
    // The abort reads the newest `claude` record, not the newest record. A
    // `cursor-agent` run that came after the proposer is skipped over.
    let fixture = Fixture::new("rate-limit-other");
    proposer_record(
        &fixture,
        "01a08000-0000-7000-8000-00000000rl04",
        &[rate_limit_frame("rejected")],
    );
    let text = Record::shaped(Shape {
        run_id: "01a08000-0000-7000-8000-00000000rl05",
        provider: Some("cursor-agent"),
        policy: Some("standard"),
        ..Shape::default()
    })
    .closed();
    std::fs::write(
        fixture
            .layout()
            .run_file("01a08000-0000-7000-8000-00000000rl05"),
        text,
    )
    .unwrap();
    let reason = rate_limit_abort(&fixture.layout()).unwrap().unwrap();
    assert!(reason.contains("rejected"), "{reason}");
}

#[test]
fn a_patch_over_the_line_cap_is_rejected_for_size() {
    let stat = Stat {
        files_changed: 50,
        insertions: PATCH_LINE_CAP as u64,
        deletions: 1,
    };
    assert_eq!(patch_over_cap(&stat), Some(PATCH_LINE_CAP + 1));
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
        "src/rungs/review.rs",
        "src/rungs/evolve.rs",
        "src/spawn_config.rs",
        "src/policy/mod.rs",
        "src/policy/table.rs",
        "src/policy/gate.py",
        "src/record/mod.rs",
        "src/record/lines.rs",
        "src/env.rs",
        "src/sandbox.rs",
        "src/containment.rs",
        "tests/corpus/mod.rs",
        "tests/corpus/README.md",
    ];
    for row in rows {
        let hit = protected_path_hit(&[row.to_string()], &repo);
        assert_eq!(hit.as_deref(), Some(row), "{row} reached the panel");
    }
    for row in PROTECTED_CODE_PATHS {
        let sample = repo
            .join(row)
            .join(if row.ends_with(".rs") { "" } else { "probe.rs" });
        let path = if row.ends_with(".rs") {
            repo.join(row)
        } else {
            sample
        };
        let hit = protected_path_hit(&[path.display().to_string()], &repo);
        assert_eq!(
            hit.as_deref(),
            Some(path.display().to_string().as_str()),
            "PROTECTED_CODE_PATHS row {row} reached the panel"
        );
    }
    // Case is not a way past it: `write_file(path="Review.rs")` on macOS lands
    // on the same file.
    let cased = protected_path_hit(&["src/Rungs/REVIEW.rs".into()], &repo);
    assert_eq!(cased.as_deref(), Some("src/Rungs/REVIEW.rs"));
    // An absolute path under the repo is the same patch spelled longer.
    let absolute = repo.join("src/record/validate.rs").display().to_string();
    let hit = protected_path_hit(std::slice::from_ref(&absolute), &repo);
    assert_eq!(hit.as_deref(), Some(absolute.as_str()));
    // Every row is checked against every path, so a clean patch with one bad
    // path in the middle is refused on that path.
    let mixed = protected_path_hit(
        &[
            "src/lib.rs".into(),
            "src/rungs/curate.rs".into(),
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
            "src/rungs/corpus.rs".into(),
            "src/rungs/curate.rs".into(),
            "src/policyholder.rs".into(),
            "src/recorder.rs".into(),
            "src/environment.rs".into(),
            "tests/evolve.rs".into(),
            "docs/rungs-spec.md".into(),
        ],
        &repo,
    );
    assert!(ok.is_none(), "{ok:?} was refused");
}

#[test]
fn four_judge_answers_in_the_closed_shape_produce_the_expected_verdict() {
    let pass = pass_answer("pass", "[]");
    let answers = PanelAnswers::new(
        parse_judge_answer("claude", &pass),
        parse_judge_answer("cursor-agent", &pass),
        parse_judge_answer("agy", &pass),
        parse_judge_answer("codex", &pass),
    );
    assert!(matches!(
        panel_verdict("01a08000-0000-7000-8000-00000000ev01", "tree", &answers),
        PanelVerdict::Offerable(_)
    ));
}

#[test]
fn one_critical_finding_blocks_whatever_the_verdict_field_says() {
    let answer = pass_answer(
        "pass",
        r#"[{"item":"secrets_check","severity":"critical","note":"token in src"}]"#,
    );
    let clean = pass_answer("pass", "[]");
    let claude = parse_judge_answer("claude", &answer);
    assert_eq!(claude.verdict, VerdictField::Pass);
    let answers = PanelAnswers::new(
        claude,
        parse_judge_answer("cursor-agent", &clean),
        parse_judge_answer("agy", &clean),
        parse_judge_answer("codex", &clean),
    );
    let blocked = panel_verdict("01a08000-0000-7000-8000-00000000ev02", "tree", &answers);
    assert_eq!(
        blocked,
        PanelVerdict::Blocked {
            reasons: vec!["claude: critical secrets_check — token in src".into()],
        }
    );
}

#[test]
fn prose_outside_the_closed_shape_is_a_block_with_a_parse_error() {
    let answer = format!("looks good {}\n", pass_answer("pass", "[]"));
    let pass_answer_text = pass_answer("pass", "[]");
    let claude = parse_judge_answer("claude", &answer);
    assert!(claude.parse_error.is_some());
    let answers = PanelAnswers::new(
        claude,
        parse_judge_answer("cursor-agent", &pass_answer_text),
        parse_judge_answer("agy", &pass_answer_text),
        parse_judge_answer("codex", &pass_answer_text),
    );
    let verdict = panel_verdict("01a08000-0000-7000-8000-00000000ev03", "tree", &answers);
    assert!(matches!(verdict, PanelVerdict::Blocked { .. }));
}

#[test]
fn the_harness_record_is_two_lines_with_only_contract_fields() {
    let fixture = Fixture::new("harness");
    let pass_id = "01a08000-0000-7000-8000-00000000ev04";
    let path = fixture.layout().run_file(pass_id);
    write_harness_start(&fixture.layout(), pass_id, &path).unwrap();
    write_harness_end(&path, Some(0), None, None).unwrap();
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
    let answers = PanelAnswers::new(
        parse_judge_answer("claude", &pass),
        parse_judge_answer("cursor-agent", &pass),
        parse_judge_answer("agy", &pass),
        parse_judge_answer("codex", &pass),
    );
    let verdict = panel_verdict("01a08000-0000-7000-8000-00000000ev05", "tree", &answers);
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

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn scratch_repo(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("hydra-judge-prompt-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "test@example.invalid"]);
    git(&dir, &["config", "user.name", "hydra tests"]);
    std::fs::write(dir.join("README"), "scratch\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "empty"]);
    dir
}

fn patch_fixture(
    repo: &Path,
    paths: &[&str],
    diff: &str,
    seed_test: Option<(&str, &str)>,
) -> ProposerPatch {
    let base_tree = if let Some((path, content)) = seed_test {
        let file = repo.join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(file, content).unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-qm", "baseline"]);
        git(repo, &["rev-parse", "HEAD^{tree}"]).trim().to_string()
    } else {
        git(repo, &["rev-parse", "HEAD^{tree}"]).trim().to_string()
    };
    ProposerPatch {
        worktree: repo.to_path_buf(),
        head: git(repo, &["rev-parse", "HEAD"]).trim().to_string(),
        tree: base_tree.clone(),
        base_tree,
        stat: Stat {
            files_changed: paths.len(),
            insertions: 1,
            deletions: 1,
        },
        paths: paths.iter().map(|path| (*path).to_string()).collect(),
        diff: diff.to_string(),
    }
}

#[test]
fn the_judge_prompt_carries_the_task_and_the_fixed_sentence() {
    let repo = scratch_repo("task");
    let patch = patch_fixture(&repo, &["src/lib.rs"], "diff", None);
    let task = "make reap report nothing to do for an unknown id";
    let prompt = judge_task(&patch, Some(task), None);
    assert!(prompt.contains(task), "{prompt}");
    assert!(prompt.contains(TASK_DOES_NOT_WAIVE_CHECKLIST), "{prompt}");
    assert!(
        prompt.contains("## Intent the patch was written against"),
        "{prompt}"
    );
}

#[test]
fn a_diff_touching_tests_puts_the_baseline_in_the_prompt() {
    let repo = scratch_repo("baseline-test");
    let baseline = "assert_eq!(answer(), false);\n";
    let patch = patch_fixture(
        &repo,
        &["tests/x.rs"],
        "diff --git a/tests/x.rs",
        Some(("tests/x.rs", baseline)),
    );
    let prompt = judge_task(&patch, None, None);
    assert!(prompt.contains("## Baseline tests"), "{prompt}");
    assert!(prompt.contains("### tests/x.rs"), "{prompt}");
    assert!(prompt.contains(baseline.trim()), "{prompt}");
    assert!(prompt.contains("baseline_test_provenance"), "{prompt}");
}

#[test]
fn a_diff_touching_only_src_does_not_put_a_baseline_in_the_prompt() {
    let repo = scratch_repo("src-only");
    let patch = patch_fixture(&repo, &["src/lib.rs"], "diff --git a/src/lib.rs", None);
    let prompt = judge_task(&patch, None, None);
    assert!(!prompt.contains("## Baseline tests"), "{prompt}");
}

#[test]
fn checklist_names_and_definitions_match_the_spec() {
    assert_eq!(CHECKLIST_ENTRIES.len(), CHECKLIST_IDS.len());
    for (entry, id) in CHECKLIST_ENTRIES.iter().zip(CHECKLIST_IDS.iter()) {
        assert_eq!(entry.id, *id);
    }
    let prompt = checklist_prompt();
    for entry in CHECKLIST_ENTRIES {
        assert!(
            prompt.contains(entry.definition),
            "{} missing definition",
            entry.id
        );
        let ceiling = if entry.advisory {
            "advisory"
        } else {
            "critical"
        };
        assert!(
            prompt.contains(&format!("- {} ({})", entry.id, ceiling)),
            "{} missing from prompt",
            entry.id
        );
    }
}

#[test]
fn a_legacy_pass_branch_prefix_is_still_recognised() {
    let id = "01a08000-0000-7000-8000-00000000ev99";
    assert_eq!(
        pass_run_id_from_branch(&format!("ouroboros/{id}")),
        Some(id)
    );
    assert_eq!(
        pass_run_id_from_branch(&format!("ouroboros/blocked/{id}")),
        Some(id)
    );
    assert_eq!(pass_run_id_from_branch(&format!("rungs/{id}")), Some(id));
    assert_eq!(
        pass_run_id_from_branch(&format!("rungs/blocked/{id}")),
        Some(id)
    );
    assert_eq!(pass_run_id_from_branch("rungs/curate/foo"), None);
}

#[test]
fn a_review_pass_commit_message_names_the_pass_it_reviews() {
    let reviewed = "01a08000-0000-7000-8000-00000000rv01";
    let pass = "01a08000-0000-7000-8000-00000000rv02";
    let proposer = "01a08000-0000-7000-8000-00000000rv03";
    let judges = vec![
        "01a08000-0000-7000-8000-00000000rv04".into(),
        "01a08000-0000-7000-8000-00000000rv05".into(),
        "01a08000-0000-7000-8000-00000000rv06".into(),
        "01a08000-0000-7000-8000-00000000rv07".into(),
    ];
    let message = review_commit_message(pass, reviewed, proposer, &judges, "verdict: blocked\n\n");
    assert!(
        message.contains(&format!("reviews: {reviewed}")),
        "{message}"
    );
    assert!(
        message.contains(&format!("hydra evolve pass {pass}")),
        "{message}"
    );
}

#[test]
fn human_evidence_reaches_the_judge_prompt() {
    let repo = scratch_repo("review-evidence");
    let patch = patch_fixture(&repo, &["src/lib.rs"], "diff", None);
    let evidence = "self_consistency means the commit message, not show and reap agreeing";
    let prompt = judge_task(&patch, Some("fix reap for unknown ids"), Some(evidence));
    assert!(prompt.contains(HUMAN_EVIDENCE_HEADING), "{prompt}");
    assert!(prompt.contains(evidence), "{prompt}");
}

#[test]
fn a_pass_that_is_not_blocked_is_refused() {
    let repo = scratch_repo("not-blocked");
    let pass_id = "01a08000-0000-7000-8000-00000000rv08";
    git(&repo, &["branch", &format!("rungs/{pass_id}")]);
    let err = ensure_reviewable(&repo, pass_id).unwrap_err();
    assert!(err.to_string().contains("not blocked"), "{}", err);
}

#[test]
fn an_offerable_pass_is_refused_even_when_a_blocked_branch_exists() {
    let repo = scratch_repo("offerable");
    let pass_id = "01a08000-0000-7000-8000-00000000rv09";
    git(&repo, &["branch", &format!("rungs/blocked/{pass_id}")]);
    git(&repo, &["branch", &format!("rungs/{pass_id}")]);
    let err = ensure_reviewable(&repo, pass_id).unwrap_err();
    assert!(err.to_string().contains("offerable"), "{}", err);
}

fn minimal_cargo_worktree(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "hydra-admission-worktree-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"admission-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/lib.rs"), "pub fn ok() {}\n").unwrap();
    dir
}

#[test]
fn admission_runs_under_the_proposers_seatbelt_and_refuses_a_hostile_test() {
    if !hydra::sandbox::exec_available() || hydra::sandbox::inside_seatbelt() {
        return;
    }
    let fixture = Fixture::new("admission-seatbelt");
    let layout = fixture.layout();
    let worktree = minimal_cargo_worktree("hostile");
    let proposer_id = "01a08000-0000-7000-8000-00000000ad01";
    let set = PolicySet::discover().unwrap();
    let spawn = write_spawn_config(
        &layout,
        proposer_id,
        &set,
        Provider::Claude,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();
    let profile = spawn
        .sandbox_profile
        .expect("macOS spawn writes a profile beside the gate");
    let runs_target = layout.runs().join("hostile");
    let hostile = format!(
        "#[test]\nfn writes_into_hydra_runs() {{\n    std::fs::write(r\"{}\", b\"x\").unwrap();\n}}\n",
        runs_target.display()
    );
    std::fs::write(worktree.join("tests/zz_hostile.rs"), hostile).unwrap();
    let admission_root = layout.spawn("pass-admission").join("admission-root");
    std::fs::create_dir_all(admission_root.join("runs")).unwrap();
    let result = admission_gate(&worktree, &profile, &admission_root);
    let output = result.output.expect("a hostile test should fail admission");
    assert!(output.contains("cargo test"), "{output}");
    assert!(
        !runs_target.exists(),
        "the seatbelt should refuse the write"
    );
    assert!(result.run.profile_sha.is_some(), "profile sha is recorded");
    assert_eq!(result.run.hydra_root, admission_root.to_string_lossy());
}
