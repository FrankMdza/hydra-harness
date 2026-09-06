//! Deriving contract 8's nine outcomes, and contract 17's returned JSON.
//!
//! Same two kinds of test as the normalizer's suite. The synthetic ones reach
//! frame shapes this machine has never produced, which is most of the enum: no
//! run here has ever been rate limited, crashed, or refused at launch. The corpus
//! ones walk `~/hydra/runs/` and are what would catch a rule written against a
//! shape that does not occur.
//!
//! **Nothing here reads an exit status to expect an outcome.** Where a test sets
//! one it is checking that the derivation ignored it.

use hydra::outcome::{Outcome, Source, derive};
use hydra::record::{Refusal, RunEnd};
use serde_json::json;

mod common;
use common::{Record, Sandbox, corpus, policies, usage_frame};

/// The derivation, off a synthetic record.
fn outcome_of(sandbox: &Sandbox, text: &str) -> Option<(Outcome, Source)> {
    let run = hydra::normalize::read_with(text, &policies(), &sandbox.layout()).unwrap();
    derive(&run).map(|derived| (derived.outcome, derived.source))
}

// --- the enum, and the rung-one column -----------------------------------

#[test]
fn the_rung_one_column_is_contract_8s_table() {
    // The right-hand column of the table, transcribed. It says what rung one
    // learns from and never what Hydra keeps, and every run in the table is
    // recorded in full whatever this says.
    for outcome in [
        Outcome::Completed,
        Outcome::Failed,
        Outcome::TimedOut,
        Outcome::Crashed,
    ] {
        assert!(outcome.rung_one_learns(), "{}", outcome.as_str());
    }
    for outcome in [
        Outcome::Gated,
        Outcome::AuthFailed,
        Outcome::SkillCollision,
        Outcome::McpUnavailable,
        Outcome::RateLimited,
    ] {
        assert!(!outcome.rung_one_learns(), "{}", outcome.as_str());
    }
}

// --- from the stream -----------------------------------------------------

#[test]
fn a_run_that_finished_is_completed() {
    let sandbox = Sandbox::new("completed");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "Done."}]))
    .result(usage_frame())
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Completed, Source::Derived))
    );
}

#[test]
fn a_rate_limited_run_is_not_the_success_its_subtype_claims() {
    // The single nastiest shape in the contract, because it is the one that looks
    // correct: `subtype: "success"` with `is_error: true` and a 429 beside them.
    // A recorder keying off `subtype` files an exhausted run as a clean one.
    let sandbox = Sandbox::new("429");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(json!({
        "is_error": true,
        "api_error_status": 429,
        "result": "Claude AI usage limit reached",
    }))
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::RateLimited, Source::Derived)),
        "the 429 was read as an ordinary failure, which rung one would learn from"
    );
}

#[test]
fn a_login_that_lapsed_mid_run_is_auth_failed() {
    let sandbox = Sandbox::new("401");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(json!({"is_error": true, "api_error_status": 401}))
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::AuthFailed, Source::Derived))
    );
}

#[test]
fn a_provider_that_says_it_failed_is_failed() {
    let sandbox = Sandbox::new("failed");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(json!({"subtype": "error_during_execution", "is_error": true}))
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Failed, Source::Derived))
    );
}

#[test]
fn a_gate_that_refused_everything_is_gated_over_a_clean_terminator() {
    // Exit 0, `subtype: success`, `is_error: false`, and the refusal is the only
    // thing in the file that says the run did no work. This is the shape all
    // three providers produce and the reason the outcome is derived at all.
    let sandbox = Sandbox::new("gated");
    let path = sandbox.worktree().join("probe.txt");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .tool_use(
        "toolu_01",
        "Write",
        json!({"file_path": path, "content": "HELLO\n"}),
    )
    .hook_response(
        "Write",
        "deny",
        "hydra/capability: policy read-only does not allow file_write (Write).",
    )
    .tool_result(
        "toolu_01",
        "hydra/capability: policy read-only does not allow file_write (Write).",
        true,
    )
    .assistant(json!([{"type": "text", "text": "I cannot write under this policy."}]))
    .result(usage_frame())
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Gated, Source::Derived))
    );
}

#[test]
fn one_refused_call_beside_work_that_got_through_is_not_a_gated_run() {
    // Contract 8.1 dropped `denied` partly because it read as a per-tool-call
    // fact. A run that had one call refused and did its work is a trajectory
    // rung one should learn from, and the refusal is still in the record per
    // call, where 7.7 puts it.
    let sandbox = Sandbox::new("one-deny");
    let path = sandbox.worktree().join("probe.txt");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
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
    .tool_result("toolu_01", "hydra/scope: Write writes outside", true)
    .tool_use(
        "toolu_02",
        "Write",
        json!({"file_path": path, "content": "HELLO\n"}),
    )
    .hook_response(
        "Write",
        "allow",
        "hydra/allow: policy standard allows file_write.",
    )
    .tool_result("toolu_02", "File created successfully.", false)
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.permission_decisions().count(), 2, "{:?}", run.findings);
    assert_eq!(
        derive(&run).map(|d| d.outcome),
        Some(Outcome::Completed),
        "a single refused call was filed as a run outcome"
    );
}

#[test]
fn a_gate_leak_is_not_a_gated_run() {
    // The policy denies the call and nothing refused it, so the call went
    // through and the run did its work. Filing that as `gated` would report the
    // run as blocked by the hole that let it through.
    let sandbox = Sandbox::new("leaked");
    let path = sandbox.worktree().join("probe.txt");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .tool_use(
        "toolu_01",
        "Write",
        json!({"file_path": path, "content": "HELLO\n"}),
    )
    .tool_result("toolu_01", "File created successfully.", false)
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings
            .iter()
            .any(|f| matches!(f, hydra::normalize::Finding::GateLeak { .. })),
        "the leak this test is about did not happen: {:?}",
        run.findings
    );
    assert_eq!(derive(&run).map(|d| d.outcome), Some(Outcome::Completed));
}

#[test]
fn a_run_the_users_own_hooks_blocked_is_not_gated() {
    // `decider: external` is the user's own `PreToolUse` guards, which contract
    // 7.7 spends a section separating from Hydra's gate. `gated` is about Hydra's
    // gate, so this comes out `completed`. It is the known weak spot in the rule
    // rather than a reading: nothing in the record says what the run was after.
    let sandbox = Sandbox::new("external");
    let path = sandbox.worktree().join("probe.txt");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .tool_use(
        "toolu_01",
        "Write",
        json!({"file_path": path, "content": "HELLO\n"}),
    )
    .hook_response("Write", "deny", "no writing on my machine")
    .tool_result("toolu_01", "no writing on my machine", true)
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decider, hydra::normalize::Decider::External);
    assert_eq!(derive(&run).map(|d| d.outcome), Some(Outcome::Completed));
}

#[test]
fn a_child_that_died_before_its_terminator_crashed() {
    // `claude` writes exactly one `result` frame on every path, so a `run_end`
    // with no terminator behind it is a process that stopped mid-run.
    let sandbox = Sandbox::new("crashed");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "Starting."}]))
    .closed_with(json!({"exit_code": 137}));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Crashed, Source::Derived))
    );
}

#[test]
fn a_spawn_that_never_started_a_process_is_failed_rather_than_crashed() {
    // Contract 6: a null `exit_code` means no process ran. Nothing died, so
    // `crashed` would be wrong, and `error` carries the reason.
    let sandbox = Sandbox::new("nospawn");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .closed_with(json!({"exit_code": null, "error": "spawning claude: No such file"}));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Failed, Source::Derived))
    );
}

#[test]
fn a_note_about_the_records_fidelity_does_not_make_a_finished_run_a_failure() {
    // `run_end.error` here is the lossy-line count: the record has every line and
    // one of them was stored lossily. Reading that as `failed` would file a run
    // whose task succeeded as a lesson about a UTF-8 byte, which is contract
    // 8.4's mistake pointed the other way. The caller gets `error` beside the
    // outcome and can see both. A tee that actually broke leaves no terminator,
    // so it lands on `crashed` instead.
    let sandbox = Sandbox::new("tee");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(usage_frame())
    .closed_with(json!({
        "exit_code": 0,
        "error": "2 line(s) the child wrote were not valid UTF-8",
    }));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Completed, Source::Derived))
    );
}

#[test]
fn a_run_with_no_ending_has_no_outcome() {
    // Contract 6: `run_end` is not guaranteed to exist, and 8.6 gives the case to
    // `hydra status` rather than to a reader. Either the child is still going or
    // the run is ticket 14's abandoned class, and telling those apart needs the
    // pid liveness test rather than the file.
    let sandbox = Sandbox::new("unclosed");
    let mut record = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    );
    record.assistant(json!([{"type": "text", "text": "still going"}]));
    let text = record.unclosed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        None,
        "a live run was given an outcome, which would file it in the corpus as a dead one"
    );
}

// --- outcome_source ------------------------------------------------------

#[test]
fn a_tool_result_the_adapter_cannot_classify_takes_the_confidence_off_completed() {
    // Measured on `claude` 2.1.261: a successful `Write` came back with no
    // `is_error` field at all. 7.6 forbids reading that as success, so the run
    // holds a call whose fate is unknown and 8.2 says the outcome is `unknown`.
    let sandbox = Sandbox::new("unclassified");
    let path = sandbox.worktree().join("probe.txt");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .tool_use(
        "toolu_01",
        "Write",
        json!({"file_path": path, "content": "HELLO\n"}),
    )
    .hook_response(
        "Write",
        "allow",
        "hydra/allow: policy standard allows file_write.",
    )
    // The measured shape: a result with no `is_error` key.
    .raw(json!({
        "type": "user",
        "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_01", "content": "File created"}
        ]},
    }))
    .result(usage_frame())
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Completed, Source::Unknown))
    );
}

#[test]
fn an_unclassified_result_does_not_soften_an_outcome_something_said_out_loud() {
    // `unknown` is for a `completed` reached by nothing saying otherwise. A 429
    // is a source speaking, and an unreadable tool result does not weaken it.
    let sandbox = Sandbox::new("unclassified-429");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .raw(json!({
        "type": "user",
        "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_01", "content": "?"}
        ]},
    }))
    .result(json!({"is_error": true, "api_error_status": 429}))
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::RateLimited, Source::Derived))
    );
}

// --- from a refusal Hydra wrote ------------------------------------------

#[test]
fn each_refusal_code_derives_to_its_outcome() {
    // Contract 6's four Hydra-side checks. Nothing in this build writes one, so
    // these are the only records that reach the branch.
    //
    // **The code comes off the writer's own enum and not off a literal**, which is
    // the point of this test rather than a nicety. `RunEnd::refused` and
    // `Refusal::outcome_of` are the two halves of one vocabulary and they live in
    // one file so they cannot drift; typing the strings again here would test the
    // match arms against a second copy of themselves.
    for (refusal, expected) in [
        (Refusal::PolicyNarrowing, Outcome::Gated),
        (Refusal::SkillCollision, Outcome::SkillCollision),
        (Refusal::McpUnavailable, Outcome::McpUnavailable),
        (Refusal::AuthPreflight, Outcome::AuthFailed),
    ] {
        // 8.3's shape: `run_start`, then `run_end` with the refusal, nothing
        // between them and no child process behind it. The closing line is the
        // writer's own, serialized, so the record carries whatever
        // `RunEnd::refused` would have written.
        let written = serde_json::to_value(RunEnd::refused(refusal, "fix the thing")).unwrap();
        assert_eq!(written["exit_code"], serde_json::Value::Null);
        assert!(
            written["body_source"].is_null(),
            "a refused run has no body to source"
        );

        let sandbox = Sandbox::new(&format!("refusal-{}", written["refusal"].as_str().unwrap()));
        let text = Record::opened(
            &sandbox.worktree(),
            Some("read-only"),
            Some(policies().sha()),
        )
        .closed_with(written.clone());

        assert_eq!(
            outcome_of(&sandbox, &text),
            Some((expected, Source::Refusal)),
            "refusal {written:?}"
        );
    }
}

#[test]
fn a_refusal_this_build_does_not_know_still_lands_outside_the_corpus() {
    // A code from a version this binary predates. Every refusal is an outcome
    // rung one never learns from, so the class is right even when the check is
    // not one of the four. Falling through to `completed` would put a run that
    // never started in the corpus as a finished one.
    let sandbox = Sandbox::new("refusal-unknown");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .closed_with(json!({"exit_code": null, "refusal": "some_future_check"}));

    let (outcome, source) = outcome_of(&sandbox, &text).unwrap();
    assert_eq!(source, Source::Refusal);
    assert!(!outcome.rung_one_learns(), "{}", outcome.as_str());
}

#[test]
fn a_refusal_beats_a_terminator_that_says_otherwise() {
    // Contract 6 calls `refusal` the strongest input the derivation has. Nothing
    // writes both today, and a file that carried both would be one Hydra refused
    // after a child had already reported success.
    let sandbox = Sandbox::new("refusal-first");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .result(usage_frame())
    .closed_with(json!({"exit_code": 0, "refusal": "skill_collision"}));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::SkillCollision, Source::Refusal))
    );
}

// --- the shapes the review found ------------------------------------------

#[test]
fn the_final_text_is_the_whole_turn_and_not_its_last_block() {
    // Contract 7.3 gives one `message` event per content block, so a frame that
    // interleaves thinking with prose arrives as `[text, thinking, text]`.
    // Taking the last event would return the second half of what the model said
    // and drop the first.
    let sandbox = Sandbox::new("turn");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "an earlier turn"}]))
    .assistant(json!([
        {"type": "text", "text": "First, what I found."},
        {"type": "thinking", "thinking": "weighing it", "signature": "sig"},
        {"type": "text", "text": "So here is the answer."},
    ]))
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(
        hydra::outcome::final_assistant_text(&run).as_deref(),
        Some("First, what I found.\n\nSo here is the answer."),
        "the first block of the final turn was dropped"
    );
}

#[test]
fn a_leak_beside_a_real_refusal_keeps_a_run_out_of_gated() {
    // Both `tool_use` blocks arrive with no `tool_use_id`, which the normalizer
    // tolerates, so both answer to `""`. This pins the outcome across the shape
    // that made matching leaks by `call_id` hard to reason about; counting them
    // needs no identity and reaches the same answer, which is why the change was
    // a simplification rather than a fix.
    let sandbox = Sandbox::new("collide");
    let path = sandbox.worktree().join("probe.txt");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    // One `Write` the policy denies, which went through: a successful result and
    // no refusal, so the gate leaked.
    .assistant(json!([{"type": "tool_use", "name": "Write",
                       "input": {"file_path": path, "content": "one\n"}}]))
    .raw(json!({
        "type": "user",
        "message": {"role": "user", "content": [
            {"type": "tool_result", "content": "File created successfully.", "is_error": false}
        ]},
    }))
    // A second `Write` the gate really refused.
    .assistant(json!([{"type": "tool_use", "name": "Write",
                       "input": {"file_path": path, "content": "two\n"}}]))
    .hook_response(
        "Write",
        "deny",
        "hydra/capability: policy read-only does not allow file_write (Write).",
    )
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let leaks = run
        .findings
        .iter()
        .filter(|f| matches!(f, hydra::normalize::Finding::GateLeak { .. }))
        .count();
    assert_eq!(leaks, 1, "the leak this test is about: {:?}", run.findings);
    assert_eq!(
        derive(&run).map(|d| d.outcome),
        Some(Outcome::Completed),
        "a run with a leaked call did not refuse everything"
    );
}

#[test]
fn a_tee_that_broke_is_not_a_process_that_died() {
    // No terminator, exit 0, and `error` set: Hydra stopped reading and the child
    // may have gone on to finish perfectly. Contract 8 defines `crashed` as "the
    // process died", so asserting that here would witness a death nothing saw.
    // This is the one branch that reads `run_end.error`.
    let sandbox = Sandbox::new("tee-broke");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "working"}]))
    .closed_with(json!({"exit_code": 0, "error": "writing the record: No space left"}));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Failed, Source::Derived))
    );
}

#[test]
fn a_child_that_stopped_with_nothing_from_hydra_still_crashed() {
    // The same shape with `error` null. Hydra has nothing to say about why the
    // terminator is missing, so the process is what stopped.
    let sandbox = Sandbox::new("really-crashed");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "working"}]))
    .closed_with(json!({"exit_code": 137}));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Crashed, Source::Derived))
    );
}

#[test]
fn a_record_with_no_run_id_writes_no_shared_scratch_index() {
    // A torn line 0 normalizes to an empty `run_id` with a `Finding::Limit`
    // rather than failing the read, and `Layout::spawn("")` is the spawn
    // directory itself, so `stat_index` would be one file shared by every such
    // record. The worktree here is a real repo, because without one git refuses
    // before the index is written and the hazard hides.
    let sandbox = Sandbox::new("no-run-id");
    let worktree = sandbox.worktree();
    for args in [
        &["init", "-q", "-b", "main"][..],
        &["config", "user.email", "test@example.invalid"][..],
        &["config", "user.name", "hydra tests"][..],
    ] {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(&worktree)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    }
    std::fs::write(worktree.join("a.txt"), "one\n").unwrap();

    let text = "{\"seq\":0,\"raw\":{\"type\":\"result\",\"is_error\":false}}\n\
                {\"seq\":1,\"hydra\":{\"type\":\"run_end\",\"exit_code\":0,\"error\":null,\
                \"refusal\":null,\"body_source\":\"stream\",\"ended_at\":\"2026-09-05T20:52:01.905Z\"}}\n";

    let run = hydra::normalize::read_with(text, &policies(), &sandbox.layout()).unwrap();
    assert!(run.run_id.is_empty(), "run id: {:?}", run.run_id);

    let result = hydra::run::result(
        &run,
        &worktree.join("record.jsonl"),
        Some(&hydra::record::WorktreeRef {
            path: worktree.display().to_string(),
            branch: "hydra/probe".into(),
        }),
        // The empty tree, which every repo has, so git gets as far as the index.
        Some("4b825dc642cb6eb9a060e54bf8d69288fbee4904"),
        &sandbox.layout(),
    );
    let json: serde_json::Value = serde_json::to_value(&result).unwrap();
    assert!(
        json.get("diff_stat").is_none(),
        "a record with no run id was given a diff stat: {json}"
    );
    assert!(
        !sandbox.layout().spawns().join("stat-index").exists(),
        "the scratch index every empty-run-id record would share was written"
    );
}

// --- what `hydra run` returns, contract 17 -------------------------------

#[test]
fn the_returned_json_is_the_contracts_field_list_and_nothing_else() {
    let sandbox = Sandbox::new("json");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "first"}]))
    .assistant(json!([{"type": "text", "text": "the last thing it said"}]))
    .result(usage_frame())
    .closed();

    let run_file = sandbox.worktree().join("record.jsonl");
    std::fs::write(&run_file, &text).unwrap();
    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let result = hydra::run::result(
        &run,
        &run_file,
        Some(&hydra::record::WorktreeRef {
            path: sandbox.worktree().display().to_string(),
            branch: "hydra/probe".into(),
        }),
        // No seed tree, so the diff stat is absent rather than zeroed.
        None,
        &sandbox.layout(),
    );
    let json: serde_json::Value = serde_json::to_value(&result).unwrap();

    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "branch",
            "error",
            "exit_code",
            "outcome",
            "outcome_source",
            "result_text",
            "run_file",
            "run_id",
            "usage",
        ],
        "contract 17 gives the list and then says \"Nothing else\""
    );

    assert_eq!(json["outcome"], "completed");
    assert_eq!(json["outcome_source"], "derived");
    assert_eq!(json["exit_code"], 0);
    assert!(json["error"].is_null());
    assert_eq!(json["result_text"], "the last thing it said");
    assert_eq!(json["branch"], "hydra/probe");
    assert_eq!(json["run_id"], common::RUN_ID);
    assert_eq!(json["run_file"], run_file.display().to_string());

    // 9.1's reconstruction, off the frame the normalizer's suite shares.
    assert_eq!(json["usage"]["usage_source"], "provider");
    assert_eq!(json["usage"]["counts"]["input_tokens"], 4);
    assert_eq!(json["usage"]["counts"]["cache_write_tokens"], 15110);
    assert_eq!(json["usage"]["counts"]["raw_input"], 4 + 15110 + 30266);
    assert_eq!(
        json["usage"]["counts"]["total_tokens"],
        4 + 15110 + 30266 + 1270
    );
}

#[test]
fn a_provider_that_reported_no_tokens_gets_no_counts_rather_than_zeroes() {
    let sandbox = Sandbox::new("no-usage");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(json!({}))
    .closed();

    let run_file = sandbox.worktree().join("record.jsonl");
    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let result = hydra::run::result(&run, &run_file, None, None, &sandbox.layout());
    let json: serde_json::Value = serde_json::to_value(&result).unwrap();

    assert_eq!(json["usage"]["usage_source"], "unavailable");
    assert!(
        json["usage"].get("counts").is_none(),
        "zeroes cannot be told from a provider reporting zero"
    );
    // No checkout, so neither field applies. 5.5's rule: absent, not null.
    assert!(json.get("branch").is_none());
    assert!(json.get("diff_stat").is_none());
}

#[test]
fn a_run_that_said_nothing_returns_no_text_rather_than_an_empty_string() {
    let sandbox = Sandbox::new("silent");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "   "}]))
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(hydra::outcome::final_assistant_text(&run), None);
}

// --- the corpus ----------------------------------------------------------

#[test]
fn every_record_on_this_machine_derives_an_outcome() {
    let policies = policies();
    let mut seen: Vec<(String, &'static str, &'static str)> = Vec::new();
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies)
            .unwrap_or_else(|err| panic!("{}: {err:#}", path.display()));
        let derived = derive(&run).unwrap_or_else(|| {
            panic!(
                "{} has a run_end and no outcome came out of it",
                path.display()
            )
        });
        seen.push((
            run.run_id.clone(),
            derived.outcome.as_str(),
            derived.source.as_str(),
        ));
    }
    assert!(
        !seen.is_empty(),
        "no records on this machine to derive from"
    );
    eprintln!("{seen:#?}");
}

#[test]
fn the_recorded_write_denials_are_the_only_gated_runs_on_this_machine() {
    // Every record here whose provider got as far as answering reports
    // `subtype: success` and `terminal_reason: completed`, and one of them exits
    // 1 on a 429. What separates them is the gate's answer per call and the
    // status on the terminator, never the exit code.
    let policies = policies();
    let mut gated = 0;
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies).unwrap();
        let derived = derive(&run).unwrap();
        let outcome = derived.outcome;

        // A launch Hydra refused has no stream at all, so the per-call rule below
        // is about a different question. Contract 8.2 calls that outcome a fact
        // Hydra wrote rather than a reading, and this loop only checks readings.
        if derived.source == Source::Refusal {
            assert!(
                !outcome.rung_one_learns(),
                "{}: a refused launch came out {}",
                path.display(),
                outcome.as_str()
            );
            continue;
        }

        // A provider that refused its own service never reached the gate, so the
        // per-call rule has nothing to say about it.
        let status = run
            .result
            .as_ref()
            .and_then(|result| result["api_error_status"].as_u64());
        if status.is_some() {
            assert!(
                !outcome.rung_one_learns(),
                "{}: api_error_status {status:?} came out {}",
                path.display(),
                outcome.as_str()
            );
            continue;
        }

        let refused = run
            .permission_decisions()
            .filter(|d| d.decision == hydra::normalize::Decision::Deny)
            .count();
        let calls = run.permission_decisions().count();
        let expected = if calls > 0 && refused == calls {
            gated += 1;
            Outcome::Gated
        } else {
            Outcome::Completed
        };

        assert_eq!(
            outcome,
            expected,
            "{}: {calls} call(s), {refused} refused",
            path.display()
        );
    }
    assert!(
        gated >= 3,
        "the three read-only Write denials should still be gated, found {gated}"
    );
}

#[test]
fn the_one_unclassified_result_on_this_machine_is_the_only_unknown() {
    let policies = policies();
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies).unwrap();
        let source = derive(&run).unwrap().source;
        let unclassified = run
            .tool_results()
            .any(|r| r.status == hydra::normalize::ResultStatus::Unclassified);
        let completed = derive(&run).unwrap().outcome == Outcome::Completed;

        assert_eq!(
            source == Source::Unknown,
            unclassified && completed,
            "{}",
            path.display()
        );
    }
}
