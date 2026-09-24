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
    Admission, DEFAULT_MAY_EDIT, HUMAN_EVIDENCE_HEADING, PATCH_LINE_CAP, PROTECTED_CODE_PATHS,
    ProposerPatch, TASK_DOES_NOT_WAIVE_CHECKLIST, admission_gate, admission_refusal,
    admission_toolchain, changed_paths, ensure_reviewable, judge_task, may_edit_paragraph,
    may_edit_set, outside_may_edit, pass_run_id_from_branch, patch_over_cap, prepare_admission,
    protected_path_hit, rate_limit_abort, review_commit_message, write_harness_end,
    write_harness_start,
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
    let run_id = "01a08000-0000-7000-8000-00000000ad02";
    let parent = repo;
    let repo = fixture.layout().worktree(run_id);
    hydra::worktree::create(
        &parent,
        &repo,
        &format!("hydra/{run_id}"),
        &fixture.layout().seed_index(run_id),
    )
    .unwrap();
    std::fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"tree-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(repo.join("src/lib.rs"), "pub fn ok() {}\n").unwrap();
    std::fs::write(repo.join(".gitignore"), "/target\n/Cargo.lock\n").unwrap();
    std::fs::write(repo.join("build.rs"), "fn main() { std::fs::write(\"src/policy/guard.rs\", \"// admission edit\\n\").unwrap(); }\n").unwrap();
    let base_tree = git(&repo, &["rev-parse", "HEAD^{tree}"]);
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
    // Since build ticket 96 the floor refuses this patch at admission, on its
    // first floor path. The rest of the test is about the branch a verdict
    // writes, which carries the tree read before admission whatever the verdict.
    assert_eq!(
        protected_path_hit(&patch.paths, &repo).as_deref(),
        Some("Cargo.toml")
    );
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
        &parent,
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
    hydra::rungs::evolve::commit_pass_branch(&parent, &patch, "rungs/test", "verdict: offerable")
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
    std::fs::remove_dir_all(parent).unwrap();
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
        "src/pty.rs",
        "src/watch.rs",
        "src/paths.rs",
        "Cargo.toml",
        "Cargo.lock",
        ".cargo/config.toml",
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
fn a_path_off_the_floor_is_still_refused_unless_named() {
    // Rung two's own launch sequence, the corpus reader and the curate pass are
    // not floor rows, and neither is a file whose name merely starts like one.
    // Until build ticket 96 that was enough to reach the panel; since map
    // ticket 73 each is refused under the default set, one at a time.
    let repo = PathBuf::from("/scratch/hydra-evolve-rows");
    let paths = [
        "src/run.rs",
        "src/lib.rs",
        "src/rungs/corpus.rs",
        "src/rungs/curate.rs",
        "src/policyholder.rs",
        "src/recorder.rs",
        "src/environment.rs",
        "src/normalizer.rs",
        "tests/evolve.rs",
        "docs/rungs-spec.md",
        "CLAUDE.md",
        "IMPLEMENTATION.md",
    ];
    let owned: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
    assert!(protected_path_hit(&owned, &repo).is_none());
    let defaults = may_edit_set(&[]).unwrap();
    for path in paths {
        let hit = outside_may_edit(&[path.to_string()], &defaults, &repo);
        assert_eq!(hit.as_deref(), Some(path), "{path} passed unnamed");
    }
}

fn named(paths: &[&str]) -> Vec<String> {
    may_edit_set(
        &paths
            .iter()
            .map(|path| path.to_string())
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn an_unnamed_path_is_refused_and_a_named_one_admitted() {
    let repo = scratch_repo("may-edit");
    let patch = patch_fixture(&repo, &["src/lib.rs"], "diff", None);
    let defaults = may_edit_set(&[]).unwrap();
    assert_eq!(defaults, DEFAULT_MAY_EDIT);
    let refused = admission_refusal(&patch, &defaults, &repo).unwrap();
    let reason = refused.expect("an unnamed path reached the panel");
    assert!(reason.contains("src/lib.rs"), "{reason}");
    assert!(reason.contains("--may-edit"), "{reason}");
    assert_eq!(
        admission_refusal(&patch, &named(&["src/lib.rs"]), &repo).unwrap(),
        None
    );
    // A directory prefix names everything under it, and a trailing slash or a
    // leading `./` is the same name.
    let normalizers = patch_fixture(
        &repo,
        &["src/normalize/claude.rs", "src/show.rs"],
        "diff",
        None,
    );
    assert_eq!(
        admission_refusal(&normalizers, &defaults, &repo).unwrap(),
        None
    );
    assert_eq!(named(&["./src/normalize/"]), ["src/normalize"]);
    // One unnamed path among named ones is enough.
    let mixed = patch_fixture(&repo, &["src/show.rs", "src/cli.rs"], "diff", None);
    let reason = admission_refusal(&mixed, &defaults, &repo)
        .unwrap()
        .unwrap();
    assert!(reason.contains("src/cli.rs"), "{reason}");
    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn run_rs_is_refused_unnamed_and_admitted_named() {
    // Harness spec 15.2 and map ticket 60 keep `src/run.rs` editable by rung
    // two, so it is off the floor; map ticket 73 makes it refused unless named.
    let repo = scratch_repo("may-edit-run");
    let patch = patch_fixture(&repo, &["src/run.rs", "src/cli.rs"], "diff", None);
    let reason = admission_refusal(&patch, &may_edit_set(&[]).unwrap(), &repo)
        .unwrap()
        .expect("src/run.rs reached the panel unnamed");
    assert!(reason.contains("src/run.rs"), "{reason}");
    assert_eq!(
        admission_refusal(&patch, &named(&["src/run.rs", "src/cli.rs"]), &repo).unwrap(),
        None
    );
    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn a_floor_path_is_refused_even_when_named() {
    let repo = scratch_repo("may-edit-floor");
    for path in [
        "src/pty.rs",
        "src/policy/gate.py",
        "Cargo.lock",
        ".cargo/config.toml",
    ] {
        let patch = patch_fixture(&repo, &[path], "diff", None);
        // Handed the name directly, past the check that refuses it at launch.
        let reason = admission_refusal(&patch, &[path.to_string()], &repo)
            .unwrap()
            .unwrap_or_else(|| panic!("{path} reached the panel named"));
        assert_eq!(reason, format!("protected path {path}"));
    }
    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn a_may_edit_reaching_the_floor_refuses_the_pass() {
    for value in [
        "src/pty.rs",
        "src/watch.rs",
        "src/paths.rs",
        "Cargo.toml",
        "cargo.LOCK",
        ".cargo",
        "src/policy/gate.py",
        "src/rungs/evolve.rs",
        "config",
        "policies/standard.md",
        // A directory holding a floor path.
        "src",
        "src/rungs",
        "tests",
        "tools/build.rs",
        // Not a path inside the repository, or all of it.
        ".",
        "../hydra",
        "/etc",
    ] {
        let err = may_edit_set(&[value.to_string()])
            .err()
            .unwrap_or_else(|| panic!("--may-edit {value} was accepted"));
        assert!(err.to_string().contains(value), "{err}");
    }
    // One floor name among good ones refuses the whole set.
    assert!(may_edit_set(&["src/show.rs".into(), "src/watch.rs".into()]).is_err());
}

#[test]
fn a_may_edit_naming_a_floor_path_refuses_before_the_proposer_is_launched() {
    // The built binary, pointed at a scratch root that does not exist yet.
    // Refusing before launch means nothing under that root is created: no
    // pass record, no worktree, no spawn directory.
    let root = std::env::temp_dir().join(format!("hydra-evolve-floor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = scratch_repo("may-edit-launch");
    // `PATH` holds `git` and no provider, so a regression that gets past the
    // check fails at the launch and still writes the pass record this test
    // looks for, rather than starting a real `claude`.
    let bin = repo.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let real_git = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join("git"))
        .find(|git| git.is_file())
        .expect("git on PATH");
    std::os::unix::fs::symlink(real_git, bin.join("git")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args(["evolve", "--may-edit", "src/pty.rs", "--task", "anything"])
        .current_dir(&repo)
        .env("HYDRA_ROOT", &root)
        .env("HYDRA_HOME", repo.join("hydra-home"))
        .env("PATH", &bin)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("src/pty.rs"), "{stderr}");
    assert!(stderr.contains("no name lifts"), "{stderr}");
    assert!(
        !root.exists(),
        "the refused pass wrote under {}",
        root.display()
    );
    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn build_rs_anywhere_is_refused() {
    let repo = scratch_repo("build-rs");
    for path in [
        "build.rs",
        "src/normalize/build.rs",
        "src/show.rs/build.rs",
        "tools/x/Build.RS",
    ] {
        let hit = protected_path_hit(&[path.to_string()], &repo);
        assert_eq!(hit.as_deref(), Some(path), "{path} reached the panel");
        // Named or under a named prefix, it is still refused.
        let patch = patch_fixture(&repo, &[path], "diff", None);
        let reason = admission_refusal(&patch, &may_edit_set(&[]).unwrap(), &repo)
            .unwrap()
            .unwrap();
        assert_eq!(reason, format!("protected path {path}"));
    }
    // A file whose name only ends like one is not a build script.
    assert!(protected_path_hit(&["src/rebuild.rs".into()], &repo).is_none());
    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn an_edit_to_an_existing_test_is_refused_and_a_new_named_test_admitted() {
    // Build ticket 80. Pass 7 edited `tests/cleanup.rs` and only a judge
    // caught it; the refusal holds whatever `--may-edit` says.
    let repo = scratch_repo("baseline-refusal");
    let patch = patch_fixture(
        &repo,
        &["tests/cleanup.rs"],
        "diff",
        Some(("tests/cleanup.rs", "#[test]\nfn pins() {}\n")),
    );
    let reason = admission_refusal(&patch, &named(&["tests/cleanup.rs"]), &repo)
        .unwrap()
        .expect("an edit to a baseline test reached the panel");
    assert_eq!(reason, "existing test tests/cleanup.rs");
    // A different case is the same file on macOS.
    let cased = patch_fixture(&repo, &["tests/Cleanup.rs"], "diff", None);
    assert!(
        admission_refusal(&cased, &named(&["tests/Cleanup.rs"]), &repo)
            .unwrap()
            .is_some()
    );
    // A new file under `tests/`, named, is admitted; unnamed, it is refused.
    let new = patch_fixture(&repo, &["tests/resolver_prefix.rs"], "diff", None);
    assert_eq!(
        admission_refusal(&new, &named(&["tests/resolver_prefix.rs"]), &repo).unwrap(),
        None
    );
    let reason = admission_refusal(&new, &may_edit_set(&[]).unwrap(), &repo)
        .unwrap()
        .unwrap();
    assert!(reason.contains("outside --may-edit"), "{reason}");
    std::fs::remove_dir_all(repo).unwrap();
}

fn tree_after(repo: &Path, change: impl FnOnce(&Path)) -> (String, String) {
    let base = git(repo, &["rev-parse", "HEAD^{tree}"]).trim().to_string();
    change(repo);
    git(repo, &["add", "-A"]);
    let tree = git(repo, &["write-tree"]).trim().to_string();
    (base, tree)
}

#[test]
fn a_rename_out_of_a_named_directory_is_refused() {
    let repo = scratch_repo("rename");
    std::fs::create_dir_all(repo.join("src/normalize")).unwrap();
    std::fs::create_dir_all(repo.join("tests")).unwrap();
    std::fs::write(repo.join("src/normalize/codex.rs"), "pub fn read() {}\n").unwrap();
    std::fs::write(repo.join("tests/normalize.rs"), "#[test]\nfn reads() {}\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "baseline"]);

    // Both sides of a rename are changed paths, so the side outside the set is
    // what refuses it.
    let (base, tree) = tree_after(&repo, |repo| {
        git(repo, &["mv", "src/normalize/codex.rs", "src/codex.rs"]);
    });
    let paths = changed_paths(&repo, &base, &tree).unwrap();
    assert_eq!(paths, ["src/codex.rs", "src/normalize/codex.rs"]);
    let mut patch = patch_fixture(&repo, &[], "diff", None);
    patch.base_tree = base.clone();
    patch.paths = paths;
    let reason = admission_refusal(&patch, &may_edit_set(&[]).unwrap(), &repo)
        .unwrap()
        .expect("a rename out of the named directory reached the panel");
    assert!(reason.contains("path src/codex.rs"), "{reason}");
    git(&repo, &["reset", "-q", "--hard"]);

    // Renaming a baseline test, or changing only its mode, is editing it.
    let (base, tree) = tree_after(&repo, |repo| {
        git(
            repo,
            &["mv", "tests/normalize.rs", "tests/normalize_codex.rs"],
        );
    });
    let paths = changed_paths(&repo, &base, &tree).unwrap();
    let reason = admission_refusal(
        &ProposerPatch {
            base_tree: base,
            paths,
            ..patch_fixture(&repo, &[], "diff", None)
        },
        &named(&["tests/normalize_codex.rs"]),
        &repo,
    )
    .unwrap();
    assert_eq!(reason.as_deref(), Some("existing test tests/normalize.rs"));
    git(&repo, &["reset", "-q", "--hard"]);

    let (base, tree) = tree_after(&repo, |repo| {
        use std::os::unix::fs::PermissionsExt;
        let file = repo.join("tests/normalize.rs");
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
    });
    let paths = changed_paths(&repo, &base, &tree).unwrap();
    assert_eq!(paths, ["tests/normalize.rs"]);
    let reason = admission_refusal(
        &ProposerPatch {
            base_tree: base,
            paths,
            ..patch_fixture(&repo, &[], "diff", None)
        },
        &named(&["tests/normalize.rs"]),
        &repo,
    )
    .unwrap();
    assert_eq!(reason.as_deref(), Some("existing test tests/normalize.rs"));
    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn the_proposer_is_told_which_paths_it_may_edit() {
    let repo = PathBuf::from("/scratch/hydra-evolve-brief");
    let text = may_edit_paragraph(&repo, &named(&["src/run.rs", "src/cli.rs"]));
    assert!(text.contains("  - src/run.rs\n  - src/cli.rs\n"), "{text}");
    assert!(text.contains("refused before review"), "{text}");
    assert!(text.contains("tests/"), "{text}");
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
    let may_edit = named(&["src/run.rs", "src/normalize"]);
    write_harness_start(&fixture.layout(), pass_id, &path, Some(&may_edit)).unwrap();
    write_harness_end(&path, Some(0), None, None).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.lines().count(), 2);
    let report = check(&text, Some(pass_id));
    report.assert_ok();
    let start = text.lines().next().unwrap();
    assert!(start.contains(r#""command":"evolve""#));
    // Build ticket 96: the pass record carries the set the pass ran under.
    assert!(
        start.contains(r#""may_edit":["src/run.rs","src/normalize"]"#),
        "{start}"
    );
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
        git_context: None,
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
    // The worktree stands in for the copy here; the copy has its own test below.
    let admission = Admission {
        tree: worktree.clone(),
        cargo_home: worktree.join("cargo-home"),
        profile,
        toolchain: admission_toolchain().unwrap(),
    };
    let result = admission_gate(&admission, &admission_root);
    let output = result.output.expect("a hostile test should fail admission");
    assert!(output.contains("cargo test"), "{output}");
    assert!(
        !runs_target.exists(),
        "the seatbelt should refuse the write"
    );
    assert!(result.run.profile_sha.is_some(), "profile sha is recorded");
    assert_eq!(result.run.hydra_root, admission_root.to_string_lossy());
}

const READ_TREE_FAILS: &str = "#[cfg(test)]
mod tests {
    #[test]
    fn read_tree() {
        panic!(\"the read tree fails\");
    }
}
";

const CHECKOUT_PASSES: &str = "#[cfg(test)]
mod tests {
    #[test]
    fn read_tree() {}
}
";

/// Build ticket 98, H2 and H38 of the third hiring panel. The read tree holds a
/// failing test. After the read, a proposer process that outlived its run makes
/// the test pass in the checkout and plants a cargo `runner` of `/usr/bin/true`
/// in a `.cargo` above everything, where `~/.cargo` sits on this machine. Run in
/// the checkout the old way, admission passes. Run on the copy it fails, with
/// the read tree's own panic.
#[test]
fn admission_runs_on_a_copy_of_the_read_tree_that_nothing_after_the_read_reaches() {
    if !hydra::sandbox::exec_available() || hydra::sandbox::inside_seatbelt() {
        return;
    }
    let fixture = Fixture::new("admission-copy");
    let layout = fixture.layout();
    let parent = scratch_repo("admission-copy");
    let run_id = "01a08000-0000-7000-8000-00000000ad03";
    let checkout = layout.worktree(run_id);
    hydra::worktree::create(
        &parent,
        &checkout,
        &format!("hydra/{run_id}"),
        &layout.seed_index(run_id),
    )
    .unwrap();
    let base_tree = git(&checkout, &["rev-parse", "HEAD^{tree}"]);
    std::fs::create_dir_all(checkout.join("src")).unwrap();
    std::fs::write(
        checkout.join("Cargo.toml"),
        "[package]\nname = \"copy-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(checkout.join(".gitignore"), "/target\n/Cargo.lock\n").unwrap();
    std::fs::write(checkout.join("src/lib.rs"), READ_TREE_FAILS).unwrap();
    let record = Record::shaped(Shape {
        run_id,
        worktree: Some(&checkout),
        base_tree: Some(base_tree.trim()),
        ..Shape::default()
    })
    .result(usage_frame())
    .closed();
    let run_file = layout.run_file(run_id);
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
    let set = PolicySet::discover().unwrap();
    let patch = hydra::rungs::evolve::read_proposer_patch(&layout, &set, &result).unwrap();
    let proposer_profile = write_spawn_config(
        &layout,
        run_id,
        &set,
        Provider::Claude,
        "standard",
        Some(&checkout),
        None,
        None,
        false,
    )
    .unwrap()
    .sandbox_profile
    .expect("macOS spawn writes a profile beside the gate");

    // After the read: the checkout changes and a cargo configuration appears.
    std::fs::write(checkout.join("src/lib.rs"), CHECKOUT_PASSES).unwrap();
    std::fs::create_dir_all(fixture.root.join(".cargo")).unwrap();
    std::fs::write(
        fixture.root.join(".cargo/config.toml"),
        "[target.'cfg(all())']\nrunner = \"/usr/bin/true\"\n",
    )
    .unwrap();
    let toolchain = admission_toolchain().unwrap();
    let hydra_root = layout.spawn("pass-admission-copy").join("admission-root");
    std::fs::create_dir_all(hydra_root.join("runs")).unwrap();

    // The old shape, the steps in the checkout, passes a tree nobody read.
    let old_cargo_home = checkout.join("target/old-cargo-home");
    let old = admission_gate(
        &Admission {
            tree: checkout.clone(),
            cargo_home: old_cargo_home,
            profile: proposer_profile.clone(),
            toolchain: toolchain.clone(),
        },
        &hydra_root,
    );
    assert_eq!(old.output, None, "the checkout should pass the old way");

    let dir = layout.spawn("pass-admission-copy").join("admission");
    let admission = prepare_admission(
        patch.git_context.as_ref().unwrap(),
        &patch.tree,
        &dir,
        &proposer_profile,
        &toolchain,
    )
    .unwrap()
    .expect("the read tree has nothing admission refuses to copy");
    assert_eq!(
        std::fs::read_to_string(admission.tree.join("src/lib.rs")).unwrap(),
        READ_TREE_FAILS
    );
    // A process that kept the proposer's profile cannot write the copy.
    let write = Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(&proposer_profile)
        .args(["/bin/sh", "-c"])
        .arg(format!(
            "echo x > '{}'",
            admission.tree.join("src/lib.rs").display()
        ))
        .output()
        .unwrap();
    assert!(
        !write.status.success(),
        "the proposer's profile wrote the copy"
    );

    // What the admission profile opens is the copy and ancestor metadata; a
    // sibling run's spawn directory stays closed to it.
    let sibling = layout.spawn("sibling-run").join("launch.json");
    std::fs::create_dir_all(sibling.parent().unwrap()).unwrap();
    std::fs::write(&sibling, "{}").unwrap();
    for probe in [&["/bin/cat"][..], &["/usr/bin/stat", "-f", "%z"][..]] {
        let read = Command::new("/usr/bin/sandbox-exec")
            .arg("-f")
            .arg(&admission.profile)
            .args(probe)
            .arg(&sibling)
            .output()
            .unwrap();
        assert!(
            !read.status.success(),
            "{probe:?} read a sibling's spawn file"
        );
    }

    let gate = admission_gate(&admission, &hydra_root);
    admission.discard();
    let output = gate
        .output
        .expect("the read tree's failing test should fail admission");
    assert!(output.contains("cargo test failed"), "{output}");
    assert!(output.contains("the read tree fails"), "{output}");
    assert!(!dir.join("tree").exists(), "the copy is discarded");
}

/// A symbolic link that leaves the tree would have admission compile bytes the
/// judges never saw, so the copy refuses it rather than writing it.
#[test]
fn admission_refuses_to_copy_a_link_that_leaves_the_tree() {
    let repo = scratch_repo("admission-link");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::os::unix::fs::symlink("/etc/hosts", repo.join("src/lib.rs")).unwrap();
    std::os::unix::fs::symlink("../README", repo.join("src/up")).unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "links"]);
    let tree = git(&repo, &["rev-parse", "HEAD^{tree}"]);
    let dir = repo.join("admission");
    let refusal = prepare_admission(
        repo.as_path(),
        tree.trim(),
        &dir,
        &repo.join("missing.sb"),
        Path::new("/usr/bin"),
    )
    .unwrap()
    .err()
    .expect("a link out of the tree is refused");
    assert!(refusal.contains("symbolic link"), "{refusal}");
    assert!(!dir.join("tree").exists(), "a refused copy is discarded");
}

// --- build ticket 101: the pass paths on a live scratch layout -------------

/// Four clean `pass` answers, for a panel closure that should offer.
fn clean_panel() -> PanelAnswers {
    let clean = pass_answer("pass", "[]");
    PanelAnswers::new(
        parse_judge_answer("claude", &clean),
        parse_judge_answer("cursor-agent", &clean),
        parse_judge_answer("agy", &clean),
        parse_judge_answer("codex", &clean),
    )
}

/// A proposer's `RunResult` as `hydra run` hands it back, for a record the
/// test wrote.
fn ended_proposer(run_id: &str, run_file: &Path) -> hydra::outcome::RunResult {
    hydra::outcome::RunResult {
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
    }
}

/// A pass blocked on `path`, the way a pass leaves one: `rungs/blocked/<pass>`
/// on top of the base commit with the patch's tree, a commit message naming
/// the proposer, the proposer's record under the layout with its `base_tree`,
/// and the pass's own harness record carrying `may_edit` when one is given.
fn blocked_pass(
    layout: &Layout,
    repo: &Path,
    pass_id: &str,
    proposer_id: &str,
    path: &str,
    may_edit: Option<&[String]>,
) {
    let base_tree = git(repo, &["rev-parse", "HEAD^{tree}"]).trim().to_string();
    git(
        repo,
        &["checkout", "-q", "-b", &format!("rungs/blocked/{pass_id}")],
    );
    let file = repo.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "// the reviewed patch\n").unwrap();
    git(repo, &["add", "-A"]);
    git(
        repo,
        &[
            "commit",
            "-qm",
            &format!(
                "hydra evolve pass {pass_id}\n\nproposer: {proposer_id}\njudges: (none)\n\nblocked"
            ),
        ],
    );
    git(repo, &["checkout", "-q", "main"]);

    let record = Record::shaped(Shape {
        run_id: proposer_id,
        policy: Some("standard"),
        worktree: Some(&layout.worktree(proposer_id)),
        base_tree: Some(&base_tree),
        ..Shape::default()
    })
    .result(usage_frame())
    .closed();
    std::fs::write(layout.run_file(proposer_id), record).unwrap();
    let pass_record = layout.run_file(pass_id);
    write_harness_start(layout, pass_id, &pass_record, may_edit).unwrap();
    write_harness_end(&pass_record, Some(0), None, None).unwrap();
}

/// The third panel's H36, `review_blocked`'s half. Build ticket 96 wired the
/// refusals into a fresh review and said the wiring was "covered by reading",
/// because the path started from `Layout::discover()` and ended in four judge
/// launches. It runs here on a scratch layout with a panel closure, and the
/// closure fails the test if a refused patch reaches it.
#[test]
fn a_fresh_review_refuses_before_the_panel_what_its_pass_could_not_edit() {
    let policies = PolicySet::discover().unwrap();
    for (name, path, may_edit, refused) in [
        ("floor", "Cargo.toml", None, Some("Cargo.toml")),
        // `--may-edit` refuses to name the floor, so a record naming it is
        // written by hand: the review still refuses what no name lifts.
        (
            "floor-named",
            "Cargo.toml",
            Some(vec!["Cargo.toml".to_string()]),
            Some("Cargo.toml"),
        ),
        ("unnamed", "src/run.rs", None, Some("src/run.rs")),
        ("named", "src/run.rs", Some(named(&["src/run.rs"])), None),
        ("default", "src/show.rs", None, None),
    ] {
        let fixture = Fixture::new(&format!("review-wiring-{name}"));
        let layout = fixture.layout();
        let repo = scratch_repo(&format!("review-wiring-{name}"));
        let pass_id = "01a08000-0000-7000-8000-0000000101a1";
        let proposer_id = "01a08000-0000-7000-8000-0000000101a2";
        blocked_pass(
            &layout,
            &repo,
            pass_id,
            proposer_id,
            path,
            may_edit.as_deref(),
        );

        let mut reached = 0;
        let mut prompt_seen = String::new();
        let done = hydra::rungs::evolve::review_blocked_in(
            &layout,
            &policies,
            &repo,
            pass_id,
            "the human's evidence",
            &mut |_, prompt, _| {
                reached += 1;
                prompt_seen = prompt.to_string();
                Ok((Vec::new(), clean_panel()))
            },
        );

        match refused {
            Some(floor_path) => {
                let err = done.err().unwrap_or_else(|| {
                    panic!("{name}: a review of a patch on {path} was not refused")
                });
                let said = format!("{err:#}");
                assert!(said.contains("cannot be reviewed again"), "{name}: {said}");
                assert!(said.contains(floor_path), "{name}: {said}");
                assert_eq!(reached, 0, "{name}: a refused patch reached the panel");
                // Nothing was opened for a pass that never started.
                let records = std::fs::read_dir(layout.runs()).unwrap().count();
                assert_eq!(records, 2, "{name}: a refused review wrote a record");
            }
            None => {
                let done = done.unwrap_or_else(|err| panic!("{name}: {err:#}"));
                assert_eq!(reached, 1, "{name}");
                assert!(prompt_seen.contains(path), "{name}: {prompt_seen}");
                assert!(prompt_seen.contains("the human's evidence"), "{name}");
                let hydra::rungs::evolve::Done::Finished(finished) = done else {
                    panic!("{name}: the review did not finish");
                };
                assert_eq!(finished.branch, format!("rungs/{}", finished.pass_run_id));
                assert_eq!(
                    git(
                        &repo,
                        &["rev-parse", &format!("{}^{{tree}}", finished.branch)]
                    ),
                    git(
                        &repo,
                        &["rev-parse", &format!("rungs/blocked/{pass_id}^{{tree}}")]
                    ),
                    "{name}: the offered branch is not the reviewed tree"
                );
                let record = std::fs::read_to_string(&finished.pass_record).unwrap();
                check(&record, Some(&finished.pass_run_id)).assert_ok();
            }
        }
        std::fs::remove_dir_all(&repo).unwrap();
    }
}

/// The patch the tree-half test's proposer wrote, formatted the way `cargo
/// fmt --check` wants it.
const SHOW_READ: &str = "pub fn shown() -> u8 {\n    2\n}\n";

/// The third panel's H36, G42's tree half. Build ticket 89 made the verdict
/// commit carry the tree read before admission, and said that the old code
/// would have carried a later edit was "reasoned, not run": the test could not
/// compile against it. This drives the whole path after the proposer, the one
/// `hydra evolve` runs, on a scratch layout: the read, the refusals, admission
/// on its copy, the panel and the commit. The panel closure stands in for a
/// process the proposer left running and edits the checkout after admission,
/// which is the last moment before the commit. The offered branch carries the
/// read tree and none of the edit.
#[test]
fn a_pass_offers_the_tree_it_read_whatever_the_checkout_holds_at_the_verdict() {
    if !hydra::sandbox::exec_available() || hydra::sandbox::inside_seatbelt() {
        println!("skipped: admission needs sandbox-exec outside a seatbelt");
        return;
    }
    let fixture = Fixture::new("land-tree");
    let layout = fixture.layout();
    let parent = scratch_repo("land-tree");
    std::fs::create_dir_all(parent.join("src")).unwrap();
    std::fs::write(
        parent.join("Cargo.toml"),
        "[package]\nname = \"land-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(parent.join(".gitignore"), "/target\n/Cargo.lock\n").unwrap();
    std::fs::write(parent.join("src/lib.rs"), "pub mod show;\n").unwrap();
    std::fs::write(
        parent.join("src/show.rs"),
        "pub fn shown() -> u8 {\n    1\n}\n",
    )
    .unwrap();
    git(&parent, &["add", "-A"]);
    git(&parent, &["commit", "-qm", "a crate admission can build"]);

    let pass_id = "01a08000-0000-7000-8000-0000000101b1";
    let proposer_id = "01a08000-0000-7000-8000-0000000101b2";
    let checkout = layout.worktree(proposer_id);
    hydra::worktree::create(
        &parent,
        &checkout,
        &format!("hydra/{proposer_id}"),
        &layout.seed_index(proposer_id),
    )
    .unwrap();
    let base_tree = git(&checkout, &["rev-parse", "HEAD^{tree}"]);
    std::fs::write(checkout.join("src/show.rs"), SHOW_READ).unwrap();

    let set = PolicySet::discover().unwrap();
    write_spawn_config(
        &layout,
        proposer_id,
        &set,
        Provider::Claude,
        "standard",
        Some(&checkout),
        None,
        None,
        false,
    )
    .unwrap();
    let record = Record::shaped(Shape {
        run_id: proposer_id,
        policy: Some("standard"),
        worktree: Some(&checkout),
        base_tree: Some(base_tree.trim()),
        ..Shape::default()
    })
    .result(usage_frame())
    .closed();
    let run_file = layout.run_file(proposer_id);
    std::fs::write(&run_file, record).unwrap();
    let pass_record = layout.run_file(pass_id);
    let may_edit = may_edit_set(&[]).unwrap();
    write_harness_start(&layout, pass_id, &pass_record, Some(&may_edit)).unwrap();
    let toolchain = admission_toolchain().unwrap();

    let mut prompt_seen = String::new();
    let done = hydra::rungs::evolve::land_proposer_patch(
        &hydra::rungs::evolve::Landing {
            layout: &layout,
            policies: &set,
            repo: &parent,
            pass_run_id: pass_id,
            proposer_run_id: proposer_id,
            may_edit: &may_edit,
            task: Some("change what show prints"),
            toolchain: &toolchain,
            pass_record: &pass_record,
        },
        &ended_proposer(proposer_id, &run_file),
        &mut |_, prompt, _| {
            prompt_seen = prompt.to_string();
            // After admission and before the commit: a leftover process
            // rewrites the reviewed file and plants a protected one.
            std::fs::write(checkout.join("src/show.rs"), "// written after the read\n").unwrap();
            std::fs::create_dir_all(checkout.join("src/policy")).unwrap();
            std::fs::write(checkout.join("src/policy/guard.rs"), "// planted\n").unwrap();
            Ok((Vec::new(), clean_panel()))
        },
    )
    .unwrap();

    let hydra::rungs::evolve::Done::Finished(finished) = done else {
        panic!("the pass did not finish");
    };
    let branch = format!("rungs/{pass_id}");
    assert_eq!(
        finished.branch,
        branch,
        "the pass was not offered: {:?}",
        match &finished.verdict {
            PanelVerdict::Blocked { reasons } => reasons.clone(),
            PanelVerdict::Offerable(_) => Vec::new(),
        }
    );
    assert!(prompt_seen.contains("-    1\n+    2"), "{prompt_seen}");
    let approved = finished
        .approved
        .expect("an offered pass carries its patch");
    let offered_tree = git(&parent, &["rev-parse", &format!("{branch}^{{tree}}")]);
    assert_eq!(offered_tree.trim(), approved.tree());
    assert_eq!(
        git(&parent, &["show", &format!("{branch}:src/show.rs")]),
        SHOW_READ,
        "the offered branch carries an edit made after the read"
    );
    let files = git(&parent, &["ls-tree", "-r", "--name-only", &branch]);
    assert!(
        !files.contains("src/policy/guard.rs"),
        "the offered branch carries a file planted after the read: {files}"
    );
    let record = std::fs::read_to_string(&pass_record).unwrap();
    check(&record, Some(pass_id)).assert_ok();
    std::fs::remove_dir_all(&parent).unwrap();
}
