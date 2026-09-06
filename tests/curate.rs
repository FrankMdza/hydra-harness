//! Ouroboros rung one, `docs/ouroboros-spec.md` section 3.
//!
//! Two halves, and neither launches a provider. The first is which runs the
//! curator is told about, which is contract 8's outcome column and 8.6's kind
//! check over a corpus this file builds a record at a time. The second is where
//! its proposal goes, which is a git repository standing in for the skill store.
//!
//! **The corpus tests build their own runs and never read `~/hydra/runs/`.** The
//! machine's own corpus grows while the suite runs, and a test that asserted a
//! count over it would be a test about the day it was written.

use std::path::{Path, PathBuf};
use std::process::Command;

use hydra::normalize::Run;
use hydra::ouroboros::corpus::{self, Selection, Why};
use hydra::ouroboros::curate;
use hydra::outcome::{Outcome, RunResult, Usage};
use hydra::paths::Layout;
use serde_json::json;

mod common;
use common::{Record, Shape, policies, usage_frame};

// --- the fixture ---------------------------------------------------------

/// A `~/hydra` the test owns, with a corpus in it and a skill store beside it.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("hydra-curate-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let fixture = Self { root };
        fixture.layout().ensure().unwrap();
        fixture
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }

    /// A worktree directory for one run, because the gate resolves the scope
    /// against a real path when it re-evaluates a call.
    fn worktree(&self, run_id: &str) -> PathBuf {
        let path = self.layout().worktree(run_id);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write(&self, run_id: &str, text: &str) {
        std::fs::write(self.layout().run_file(run_id), text).unwrap();
    }

    /// The skill store as ticket 07 will leave it: a git repository with a
    /// commit in it. Nothing in this build creates one.
    fn store(&self) -> PathBuf {
        let store = self.layout().skills();
        std::fs::create_dir_all(&store).unwrap();
        git(&store, &["init", "-q", "-b", "main"]);
        git(&store, &["config", "user.email", "test@example.invalid"]);
        git(&store, &["config", "user.name", "hydra tests"]);
        std::fs::write(store.join("README.md"), "the skill store\n").unwrap();
        git(&store, &["add", "-A"]);
        git(&store, &["commit", "-qm", "the store"]);
        store
    }

    fn select(&self) -> Selection {
        corpus::select(&self.layout(), &policies()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
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

/// The run ids rung one may learn from, in the order the brief lists them.
fn learned(selection: &Selection) -> Vec<&str> {
    selection
        .learn
        .iter()
        .map(|run| run.run_id.as_str())
        .collect()
}

fn why<'a>(selection: &'a Selection, run_id: &str) -> &'a Why {
    &selection
        .skipped
        .iter()
        .find(|skipped| skipped.run_id == run_id)
        .unwrap_or_else(|| panic!("{run_id} was not skipped"))
        .why
}

fn brief_for(fixture: &Fixture, selection: &Selection) -> String {
    curate::brief(
        "01a07500-0000-7000-8000-00000000cafe",
        &fixture
            .layout()
            .skill_worktree("01a07500-0000-7000-8000-00000000cafe"),
        &fixture.layout(),
        selection,
    )
}

/// Contract 17's projection, as the commit message reads it.
fn result_of(run_id: &str, said: &str) -> RunResult {
    RunResult {
        run_id: run_id.to_string(),
        outcome: Some("completed"),
        outcome_source: Some("derived"),
        exit_code: Some(0),
        error: None,
        result_text: Some(said.to_string()),
        branch: None,
        diff_stat: None,
        usage: Usage::unavailable(),
        run_file: format!("/tmp/{run_id}.jsonl"),
    }
}

const A: &str = "01a07217-6007-7733-9ce2-7bb31830eab8";
const B: &str = "01a07218-8817-7581-9ddb-47565e5d5ba7";
const C: &str = "01a0721b-2883-7c30-ab56-f06ba0027cb8";
const D: &str = "01a07307-2693-7ef0-b572-a565ffcc3190";
const E: &str = "01a07308-39e3-79c2-8901-ac052723b79a";
const F: &str = "01a07340-67a3-7c41-8c69-8d1e28bb58a7";

// --- which runs the curator is told about --------------------------------

#[test]
fn a_harness_run_is_skipped_on_its_kind_rather_than_on_its_outcome() {
    // Contract 8.6, and the case that decides it: a pass killed with Ctrl-C is a
    // dead pid with no `run_end`, which is exactly rung one's abandoned class.
    // Keying on the outcome would turn every interrupted `hydra evolve` into a
    // failure signal about a run that tried nothing.
    let fixture = Fixture::new("harness");
    let pass = Shape {
        run_id: A,
        provider: None,
        command: "evolve",
        ..Shape::default()
    };
    fixture.write(A, &Record::shaped(pass).closed());

    let interrupted = Shape {
        run_id: B,
        provider: None,
        command: "evolve",
        ..Shape::default()
    };
    fixture.write(B, &Record::shaped(interrupted).unclosed());

    let selection = fixture.select();
    assert!(learned(&selection).is_empty(), "a pass reached the curator");
    assert!(matches!(why(&selection, A), Why::HarnessRun));
    assert!(
        matches!(why(&selection, B), Why::HarnessRun),
        "an interrupted pass read as an abandoned run"
    );
}

#[test]
fn the_five_outcomes_rung_one_never_learns_from_are_left_out() {
    // Section 3.10 and contract 8's right-hand column. Two of the five derive
    // from the stream and three arrive as a refusal Hydra wrote, and all five are
    // keyed on the outcome rather than on a guess about what the run was doing.
    let fixture = Fixture::new("never-learns");
    let sha = policies().sha().to_string();

    // `gated`: the gate refused everything the run tried.
    let worktree = fixture.worktree(A);
    let mut gated = Record::shaped(Shape {
        run_id: A,
        policy: Some("read-only"),
        policy_sha: Some(&sha),
        worktree: Some(&worktree),
        ..Shape::default()
    });
    let text = gated
        .tool_use(
            "toolu_01",
            "Write",
            json!({"file_path": worktree.join("x.txt"), "content": "no\n"}),
        )
        .hook_response(
            "Write",
            "deny",
            "hydra/capability: policy read-only does not allow file_write (Write).",
        )
        .tool_result("toolu_01", "hydra/capability: refused", true)
        .result(usage_frame())
        .closed();
    fixture.write(A, &text);

    // `rate_limited`: the nastiest shape in the contract, a 429 reporting
    // `subtype: success`.
    let mut limited = Record::shaped(Shape {
        run_id: B,
        policy: Some("standard"),
        policy_sha: Some(&sha),
        worktree: Some(&fixture.worktree(B)),
        ..Shape::default()
    });
    let text = limited
        .result(json!({"is_error": true, "api_error_status": 429}))
        .closed_with(json!({"exit_code": 1}));
    fixture.write(B, &text);

    // The three that arrive as a refusal Hydra wrote: a wider policy, a skill
    // collision, an MCP shortfall, and the auth pre-flight beside them.
    for (run_id, refusal, outcome) in [
        (C, "policy_narrowing", Outcome::Gated),
        (D, "skill_collision", Outcome::SkillCollision),
        (E, "mcp_unavailable", Outcome::McpUnavailable),
        (F, "auth_preflight", Outcome::AuthFailed),
    ] {
        let mut refused = Record::shaped(Shape {
            run_id,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            pid: None,
            process_start_time: None,
            ..Shape::default()
        });
        let text = refused.closed_with(json!({
            "exit_code": null,
            "refusal": refusal,
            "remediation": "name a policy no wider than the parent's",
        }));
        fixture.write(run_id, &text);

        let selection = fixture.select();
        assert!(
            matches!(why(&selection, run_id), Why::NeverLearns(seen) if *seen == outcome),
            "{refusal} reached the curator"
        );
    }

    let selection = fixture.select();
    assert!(learned(&selection).is_empty(), "{:?}", learned(&selection));
    assert!(matches!(
        why(&selection, A),
        Why::NeverLearns(Outcome::Gated)
    ));
    assert!(matches!(
        why(&selection, B),
        Why::NeverLearns(Outcome::RateLimited)
    ));

    // The other side of the same rule: every one of them is still on disk, in
    // full. Skipped is not thrown away.
    for run_id in [A, B, C, D, E, F] {
        assert!(fixture.layout().run_file(run_id).is_file());
    }
}

#[test]
fn one_refused_call_is_a_policy_bug_and_the_run_is_still_learned_from() {
    // `gated` fires only when the gate refused everything, so a run that had one
    // call denied and did its work stays in the corpus. Ticket 27: that denial is
    // a policy bug rather than a skill bug, and the brief has to say so or the
    // curator rewrites a skill because a policy was too narrow.
    let fixture = Fixture::new("one-deny");
    let sha = policies().sha().to_string();
    let worktree = fixture.worktree(A);
    let mut record = Record::shaped(Shape {
        run_id: A,
        policy: Some("standard"),
        policy_sha: Some(&sha),
        worktree: Some(&worktree),
        ..Shape::default()
    });
    let text = record
        .tool_use(
            "toolu_01",
            "Write",
            json!({"file_path": "/etc/hosts", "content": "no\n"}),
        )
        .hook_response(
            "Write",
            "deny",
            "hydra/scope: Write writes outside the worktree.",
        )
        .tool_result("toolu_01", "hydra/scope: refused", true)
        .tool_use(
            "toolu_02",
            "Write",
            json!({"file_path": worktree.join("ok.txt"), "content": "yes\n"}),
        )
        .hook_response(
            "Write",
            "allow",
            "hydra/allow: policy standard allows file_write.",
        )
        .tool_result("toolu_02", "File created successfully.", false)
        .result(usage_frame())
        .closed();
    fixture.write(A, &text);

    let selection = fixture.select();
    assert_eq!(learned(&selection), vec![A]);
    let run = &selection.learn[0];
    assert_eq!(run.signals.denied_by_hydra, 1);
    assert_eq!(run.signals.tool_calls, 2);

    let brief = brief_for(&fixture, &selection);
    assert!(brief.contains("denied=1"), "{brief}");
    assert!(
        brief.contains("policy bug rather than a skill bug"),
        "the brief lets a policy bug reach the curator as a skill failure"
    );
}

#[test]
fn an_abandoned_run_is_learned_from_and_a_live_one_is_not_read_at_all() {
    // The first of the five classes is a dead pid with no `run_end`, and the
    // outcome derivation says `None` to exactly that. A live run says the same
    // thing and means the opposite, so the liveness test is what tells them apart
    // and it runs before the file is opened.
    let fixture = Fixture::new("abandoned");
    let sha = policies().sha().to_string();

    let mut child = Command::new("sleep").arg("5").spawn().unwrap();
    let dead = child.id();
    let started = hydra::procinfo::start_time(dead).unwrap().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();

    let stamp =
        |t: chrono::DateTime<chrono::Utc>| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    fixture.write(
        A,
        &Record::shaped(Shape {
            run_id: A,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            pid: Some(dead),
            process_start_time: Some(stamp(started)),
            worktree: Some(&fixture.worktree(A)),
            ..Shape::default()
        })
        .unclosed(),
    );

    let mine = std::process::id();
    fixture.write(
        B,
        &Record::shaped(Shape {
            run_id: B,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            pid: Some(mine),
            process_start_time: Some(stamp(hydra::procinfo::start_time(mine).unwrap().unwrap())),
            worktree: Some(&fixture.worktree(B)),
            ..Shape::default()
        })
        .unclosed(),
    );

    let selection = fixture.select();
    assert_eq!(learned(&selection), vec![A]);
    assert!(selection.learn[0].signals.abandoned);
    assert_eq!(selection.learn[0].outcome_name(), "abandoned");
    assert_eq!(selection.classes().abandoned, 1);
    assert!(matches!(why(&selection, B), Why::StillRunning));
}

#[test]
fn a_pre_gate_record_goes_and_an_interactive_one_stays() {
    // Both carry a null policy and they mean opposite things. A recorded run with
    // no policy predates the gate, so contract 7.7 derives no permission decision
    // and a refusal inside it is invisible: one such record on this machine had a
    // `Write` refused and derives `completed`. An interactive run has no policy
    // because none was compiled, and it is the only population a human correction
    // can come from, so a rule that keyed on the null alone would take the whole
    // of section 3.2's second class with it.
    let fixture = Fixture::new("pre-gate");

    fixture.write(
        A,
        &Record::shaped(Shape {
            run_id: A,
            policy: None,
            worktree: Some(&fixture.worktree(A)),
            ..Shape::default()
        })
        .result(usage_frame())
        .closed(),
    );
    fixture.write(
        B,
        &Record::shaped(Shape {
            run_id: B,
            policy: None,
            launch_mode: "interactive",
            ..Shape::default()
        })
        .result(usage_frame())
        .closed(),
    );

    let selection = fixture.select();
    assert_eq!(learned(&selection), vec![B]);
    assert!(matches!(why(&selection, A), Why::PreGate));
}

#[test]
fn a_run_a_curator_pass_spawned_is_not_read_back() {
    // Section 3.5. Otherwise the curator learns from itself. The pass's own run
    // stays, because section 7 says a curator's failures are learnable by the
    // next pass; it is the children it spawned that come out.
    let fixture = Fixture::new("own-children");
    let sha = policies().sha().to_string();

    fixture.write(
        A,
        &Record::shaped(Shape {
            run_id: A,
            policy: Some("curate"),
            policy_sha: Some(&sha),
            worktree: Some(&fixture.worktree(A)),
            ..Shape::default()
        })
        .result(usage_frame())
        .closed(),
    );
    fixture.write(
        B,
        &Record::shaped(Shape {
            run_id: B,
            parent_run_id: Some(A),
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&fixture.worktree(B)),
            ..Shape::default()
        })
        .result(usage_frame())
        .closed(),
    );

    let selection = fixture.select();
    assert_eq!(learned(&selection), vec![A]);
    assert!(matches!(why(&selection, B), Why::CuratorsChild));
}

#[test]
fn the_fifth_class_is_a_run_that_ended_clean_with_a_failure_inside_it() {
    // The class that makes the other four trustworthy. All three providers close
    // a run reporting success with a failed tool in it, so the derivation saying
    // `completed` is not evidence that the task worked.
    let fixture = Fixture::new("clean-looking");
    let sha = policies().sha().to_string();
    let worktree = fixture.worktree(A);
    let mut record = Record::shaped(Shape {
        run_id: A,
        policy: Some("standard"),
        policy_sha: Some(&sha),
        worktree: Some(&worktree),
        ..Shape::default()
    });
    let text = record
        .tool_use(
            "toolu_01",
            "Read",
            json!({"file_path": worktree.join("missing.txt")}),
        )
        .hook_response(
            "Read",
            "allow",
            "hydra/allow: policy standard allows file_read.",
        )
        .tool_result("toolu_01", "File does not exist.", true)
        .result(usage_frame())
        .closed();
    fixture.write(A, &text);

    let selection = fixture.select();
    let run = &selection.learn[0];
    assert_eq!(run.outcome_name(), "completed");
    assert_eq!(run.signals.failed_tools, 1);
    assert!(run.signals.clean_looking_failure(run.outcome.as_ref()));
    assert_eq!(selection.classes().clean_looking, 1);
    assert_eq!(selection.classes().tool_errors, 1);

    let brief = brief_for(&fixture, &selection);
    assert!(
        brief.contains("[ended clean with a failure inside it]"),
        "{brief}"
    );
}

#[test]
fn the_brief_says_what_it_left_out_and_never_pretends_the_corpus_is_smaller() {
    // Section 3.4's honesty rule, one level up: a curator that is told only what
    // it may learn from cannot tell an absent signal from an absent producer.
    let fixture = Fixture::new("brief");
    let sha = policies().sha().to_string();
    fixture.write(
        A,
        &Record::shaped(Shape {
            run_id: A,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&fixture.worktree(A)),
            ..Shape::default()
        })
        .result(usage_frame())
        .closed(),
    );
    fixture.write(
        B,
        &Record::shaped(Shape {
            run_id: B,
            provider: None,
            command: "evolve",
            ..Shape::default()
        })
        .closed(),
    );

    let selection = fixture.select();
    let brief = brief_for(&fixture, &selection);
    assert!(brief.contains(A), "the brief left out a run to learn from");
    assert!(
        !brief.contains(B),
        "a harness run reached the curator by name"
    );
    assert!(
        brief.contains("1: a harness run"),
        "the brief hid what it skipped: {brief}"
    );
    // The counts are the ranking, and each class with no producer says so rather
    // than reading as a zero somebody measured.
    assert!(brief.contains("no producer yet"), "{brief}");
    assert!(brief.contains("ouroboros/curate/"), "{brief}");
}

#[test]
fn the_walk_reads_through_the_normalizer_and_writes_nothing() {
    // Contract 11: normalization is a read-time library function. A curator that
    // wrote a derived field back into the corpus would be editing the evidence it
    // learns from, which is the one thing neither rung may do.
    let fixture = Fixture::new("read-only");
    let sha = policies().sha().to_string();
    fixture.write(
        A,
        &Record::shaped(Shape {
            run_id: A,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&fixture.worktree(A)),
            ..Shape::default()
        })
        .result(usage_frame())
        .closed(),
    );
    let before = std::fs::read(fixture.layout().run_file(A)).unwrap();
    fixture.select();
    assert_eq!(
        before,
        std::fs::read(fixture.layout().run_file(A)).unwrap(),
        "the walk changed a record"
    );
}

// --- where the proposal goes ---------------------------------------------

#[test]
fn the_curators_writes_are_scoped_to_its_checkout_and_not_to_the_store() {
    // Section 3.6. A run pins the store at a sha and a dirty store is committed
    // before that pinning, so a curator writing into the store's working tree
    // would collide with the next launch's commit.
    let fixture = Fixture::new("scope");
    let store = fixture.store();
    let run_id = A;
    let checkout = fixture.layout().skill_worktree(run_id);
    curate::check_out(&store, &checkout, &format!("ouroboros/curate/{run_id}")).unwrap();

    let spawn = hydra::run::write_spawn_config(
        &fixture.layout(),
        run_id,
        &policies(),
        "curate",
        None,
        false,
    )
    .unwrap();

    let script = std::fs::read_to_string(&spawn.gate).unwrap();
    let resolved = |path: &Path| {
        std::fs::canonicalize(path)
            .unwrap()
            .to_string_lossy()
            .to_lowercase()
    };
    assert!(
        script.contains(&format!("skill_store\\\":\\\"{}", resolved(&checkout))),
        "the gate scoped the store somewhere other than the checkout"
    );
    assert!(
        !script.contains(&resolved(&store)),
        "the store's own working tree is inside the write scope"
    );

    // Every other policy keeps the store itself, which nothing but `curate`
    // grants anything on.
    let ordinary = hydra::run::write_spawn_config(
        &fixture.layout(),
        "r2",
        &policies(),
        "standard",
        None,
        false,
    )
    .unwrap();
    assert!(
        std::fs::read_to_string(&ordinary.gate)
            .unwrap()
            .contains(&format!("skill_store\\\":\\\"{}", resolved(&store))),
        "an ordinary run's store scope moved"
    );
}

#[test]
fn a_curate_runs_own_record_does_not_read_back_as_a_gate_that_leaked() {
    // Contract 7.7 re-evaluates the run's policy against every recorded call, and
    // it rebuilds the scopes rather than reading them off the record. A curate
    // run's store scope is its own checkout, so a reader that used
    // `~/hydra/skills/` instead would report every write the curator made as a
    // call the gate should have refused. Ten of those landed in one real record
    // before `Layout::skill_store_scope` existed.
    let fixture = Fixture::new("replay");
    let store = fixture.store();
    let run_id = A;
    let checkout = fixture.layout().skill_worktree(run_id);
    curate::check_out(&store, &checkout, &format!("ouroboros/curate/{run_id}")).unwrap();

    let mut record = Record::shaped(Shape {
        run_id,
        policy: Some("curate"),
        policy_sha: Some(policies().sha()),
        ..Shape::default()
    });
    let text = record
        .tool_use(
            "toolu_01",
            "Write",
            json!({"file_path": checkout.join("proposal.md"), "content": "a class\n"}),
        )
        .hook_response(
            "Write",
            "allow",
            "hydra/allow: policy curate allows file_write.",
        )
        .tool_result("toolu_01", "File created successfully.", false)
        .result(usage_frame())
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &fixture.layout()).unwrap();
    assert!(
        run.findings.is_empty(),
        "a curator's own writes read back as findings: {:?}",
        run.findings
    );
    let decision = run.permission_decisions().next().unwrap();
    assert_eq!(decision.decision.as_str(), "allow");
}

#[test]
fn the_commit_is_the_proposal_and_the_checkout_goes_after_it() {
    // The pass's only post-run action, contract 5.5. The order is git's
    // guarantee rather than ours: `git worktree remove` refuses a checkout with
    // changes in it, so a commit that did not happen stops the removal.
    let fixture = Fixture::new("commit");
    let store = fixture.store();
    let run_id = A;
    let branch = format!("ouroboros/curate/{run_id}");
    let checkout = fixture.layout().skill_worktree(run_id);
    curate::check_out(&store, &checkout, &branch).unwrap();

    std::fs::write(
        checkout.join("writing-tests.md"),
        "the class: repeated tool errors\n",
    )
    .unwrap();

    let proposal = curate::propose(
        &store,
        &checkout,
        &branch,
        run_id,
        &result_of(run_id, "I proposed one edit for the repeated tool errors."),
    )
    .unwrap();

    assert!(proposal.commit.is_some(), "nothing was committed");
    assert!(proposal.checkout_removed);
    assert!(
        !proposal.branch_removed,
        "the proposal's branch was deleted"
    );
    assert!(!checkout.exists());

    // The bytes, read back off the branch. This is what the pass exists to leave.
    assert_eq!(
        git(&store, &["show", &format!("{branch}:writing-tests.md")]),
        "the class: repeated tool errors\n"
    );

    // What the commit message keeps apart: Hydra's facts, then the model's words
    // labelled as words.
    let message = git(&store, &["log", "-1", "--format=%B", &branch]);
    assert!(message.contains(run_id), "{message}");
    assert!(message.contains("policy: curate"), "{message}");
    assert!(
        message.contains("What the curator said it did. The diff is what it did."),
        "{message}"
    );

    // Nothing landed. The store's own branch is where it was, and its working
    // tree never saw the file.
    assert_eq!(
        git(&store, &["log", "--oneline", "main"]).lines().count(),
        1,
        "the proposal reached the store's own branch"
    );
    assert!(!store.join("writing-tests.md").exists());
    assert!(git(&store, &["status", "--porcelain"]).trim().is_empty());
}

#[test]
fn a_pass_that_proposed_nothing_leaves_no_branch() {
    // An empty pass is a real answer. `git branch -d` is what decides this: it
    // refuses a branch that is not fully merged, so a branch only goes when
    // deleting it loses nothing.
    let fixture = Fixture::new("empty");
    let store = fixture.store();
    let run_id = B;
    let branch = format!("ouroboros/curate/{run_id}");
    let checkout = fixture.layout().skill_worktree(run_id);
    curate::check_out(&store, &checkout, &branch).unwrap();

    let proposal = curate::propose(
        &store,
        &checkout,
        &branch,
        run_id,
        &result_of(run_id, "The corpus does not support a change."),
    )
    .unwrap();

    assert!(proposal.commit.is_none());
    assert!(proposal.checkout_removed);
    assert!(proposal.branch_removed);
    assert!(proposal.left.is_empty(), "{:?}", proposal.left);
    assert!(
        !git(&store, &["branch", "--list", "--format=%(refname:short)"]).contains("ouroboros"),
        "an empty branch was left behind"
    );
}

#[test]
fn a_pass_refuses_before_it_launches_when_there_is_no_store() {
    // A curator with nowhere to propose into is a run that spends a subscription
    // to find that out.
    let fixture = Fixture::new("no-store");
    let err = curate::store(&fixture.layout()).unwrap_err();
    let said = format!("{err:#}");
    assert!(said.contains("no skill store"), "{said}");
    assert!(said.contains("ticket 07"), "{said}");

    // A directory that is not a repository is the second half of the same check.
    std::fs::create_dir_all(fixture.layout().skills()).unwrap();
    let err = curate::store(&fixture.layout()).unwrap_err();
    assert!(
        format!("{err:#}").contains("not a git repository"),
        "{err:#}"
    );
}

// --- the mechanism that keeps the evidence read-only ---------------------

#[test]
fn no_ouroboros_code_path_reaches_the_cleanup_commands() {
    // Section 2.2, and it is the only mechanism there is: `hydra reap`, `hydra
    // prune` and `hydra export` are a binary reached through `shell`, so a gate
    // rule against them is an argv match with the pattern floor's failure mode.
    // What holds them is that no code path under `src/ouroboros/` calls one, and
    // this is that sentence as a test rather than as a comment.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ouroboros");
    let mut read = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        read += 1;
        let source = std::fs::read_to_string(&path).unwrap();
        for (number, line) in source.lines().enumerate() {
            // Prose about the rule is not a call to it, and this file is full of
            // prose about the rule.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for forbidden in ["reap::", "prune::", "export::", "\"hydra\""] {
                assert!(
                    !line.contains(forbidden),
                    "{}:{} reaches {forbidden}: {line}",
                    path.display(),
                    number + 1
                );
            }
        }
    }
    assert!(
        read >= 3,
        "the source of rung one moved out from under this"
    );
}

#[test]
fn a_run_carries_the_parent_the_self_exclusion_reads() {
    // `parent_run_id` is on the envelope rather than in the body, so the
    // normalizer has to lift it or section 3.5 has nothing to key on.
    let sandbox = common::Sandbox::new("lineage");
    let text = Record::shaped(Shape {
        run_id: common::RUN_ID,
        parent_run_id: Some(A),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&sandbox.worktree()),
        ..Shape::default()
    })
    .result(usage_frame())
    .closed();
    let run: Run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.parent_run_id.as_deref(), Some(A));
}
