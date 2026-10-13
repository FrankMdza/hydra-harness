//! Deriving contract 8's nine outcomes, and contract 17's returned JSON.
//!
//! Same two kinds of test as the normalizer's suite. The synthetic ones reach
//! frame shapes this machine has never produced, which is most of the enum: no
//! run here has ever been rate limited, crashed, or refused at launch. The corpus
//! ones are the fixture corpus walk; the live walk over `~/hydra/runs/` is
//! `hydra doctor --walk` (build ticket 126).
//!
//! **Nothing here reads an exit status to expect an outcome.** Where a test sets
//! one it is checking that the derivation ignored it.

#[path = "common/git.rs"]
mod fixture_git;

use std::path::Path;

use hydra::outcome::{Outcome, Source, derive};
use hydra::record::{Refusal, RunEnd};
use serde_json::{Value, json};

mod common;
use common::{Record, Sandbox, Shape, cursor_read_blocked, cursor_usage, policies, usage_frame};

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
fn a_recorded_claude_run_with_a_limit_sentence_still_reads_429_not_text() {
    // The harvested text match must not apply to a streamed body that carries a
    // `result` frame: contract 8 reads `rate_limited` off `api_error_status`
    // there, and the assistant sentence beside it is not consulted.
    let sandbox = Sandbox::new("429-with-limit-text");
    let limit_text = "You've hit your session limit · resets 1:50am (America/Bogota)";
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": limit_text}]))
    .result(json!({
        "is_error": true,
        "api_error_status": 429,
        "result": limit_text,
    }))
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::RateLimited, Source::Derived))
    );

    // Without the 429 the same sentence on a streamed run is not a limit refusal.
    let sandbox = Sandbox::new("stream-limit-text");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": limit_text}]))
    .result(json!({
        "is_error": true,
        "result": limit_text,
    }))
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Failed, Source::Derived))
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
    let text = Record::gated(&sandbox, "claude", "read-only", policies().sha())
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
    let text = Record::gated(&sandbox, "claude", "read-only", policies().sha())
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
fn a_codex_skill_copy_that_changed_does_not_change_the_outcome() {
    // Build ticket 87 puts the copy check on `run_end.error`. A `codex` run
    // that reached `turn.completed` has a terminator, so the note is a fidelity
    // note and the run stays `completed`. One that never reached a turn is
    // `failed` with or without the note, because that is this provider's
    // reading of no terminator.
    let sandbox = Sandbox::new("codex-copy");
    let worktree = sandbox.worktree();
    let codex = |turn: bool, error: Value| {
        let mut record = Record::shaped(Shape {
            provider: Some("codex"),
            policy: Some("standard"),
            policy_sha: Some(policies().sha()),
            worktree: Some(&worktree),
            ..Shape::default()
        });
        record.raw(json!({"type": "thread.started", "thread_id": "t"}));
        if turn {
            record.raw(json!({
                "type": "turn.completed",
                "usage": {"input_tokens": 10, "cached_input_tokens": 0, "output_tokens": 4},
            }));
        }
        record.closed_with(json!({"exit_code": if turn { 0 } else { 1 }, "error": error}))
    };
    let note = format!(
        "{}: alpha/SKILL.md differs",
        hydra::skills::COPY_MISMATCH_NOTE
    );

    assert_eq!(
        outcome_of(&sandbox, &codex(true, json!(note))),
        Some((Outcome::Completed, Source::Derived))
    );
    for error in [json!(note), Value::Null] {
        assert_eq!(
            outcome_of(&sandbox, &codex(false, error)),
            Some((Outcome::Failed, Source::Derived))
        );
    }
}

#[test]
fn a_codex_turn_that_hit_its_usage_limit_is_rate_limited_and_not_failed() {
    // Record 01a0cfdb-e5c3's shape, 0.154.0, 2026-09-23: a tool item started,
    // then a top-level `error` frame and `turn.failed` with the same sentence,
    // then exit 1. Before build ticket 100 it derived `failed`, which rung one
    // learns from. 01a0cc9d-8f15 carries the sentence behind a prefix, and
    // 01a0ca7d-f703 is a `turn.failed` that is not a quota, which stays
    // `failed`.
    let sandbox = Sandbox::new("codex-limit");
    let worktree = sandbox.worktree();
    let codex = |message: &str| {
        let mut record = Record::shaped(Shape {
            provider: Some("codex"),
            policy: Some("standard"),
            policy_sha: Some(policies().sha()),
            worktree: Some(&worktree),
            ..Shape::default()
        });
        record.raw(json!({"type": "thread.started", "thread_id": "t"}));
        record.raw(json!({"type": "turn.started"}));
        record.raw(json!({"type": "item.started", "item": {
            "id": "item_4", "type": "command_execution",
            "command": "/bin/zsh -lc \"sed -n '382,700p' src/run.rs\"",
            "aggregated_output": "", "exit_code": null, "status": "in_progress",
        }}));
        record.raw(json!({"type": "error", "message": message}));
        record.raw(json!({"type": "turn.failed", "error": {"message": message}}));
        record.closed_with(json!({"exit_code": 1, "error": null}))
    };
    let limit = "You've hit your usage limit. Upgrade to Pro \
                 (https://chatgpt.com/explore/pro), visit \
                 https://chatgpt.com/codex/settings/usage to purchase more credits or try \
                 again at 6:31 PM.";

    // Records 01a0f653, 01a0f650 and 01a0f654 on 2026-10-01: same sentence with U+2019.
    let limit_curly = "You\u{2019}ve hit your usage limit. Upgrade to Pro \
                       (https://chatgpt.com/explore/pro), visit \
                       https://chatgpt.com/codex/settings/usage to purchase more credits or try \
                       again at 4:48 AM.";

    assert_eq!(
        outcome_of(&sandbox, &codex(limit)),
        Some((Outcome::RateLimited, Source::Derived))
    );
    assert_eq!(
        outcome_of(&sandbox, &codex(limit_curly)),
        Some((Outcome::RateLimited, Source::Derived))
    );
    assert_eq!(
        outcome_of(
            &sandbox,
            &codex(&format!("Error running remote compact task: {limit}"))
        ),
        Some((Outcome::RateLimited, Source::Derived))
    );
    assert_eq!(
        outcome_of(
            &sandbox,
            &codex("This content was flagged for possible cybersecurity risk.")
        ),
        Some((Outcome::Failed, Source::Derived))
    );

    // Every frame in that record is one the reader knows, so it names no
    // limit; a frame type it does not know is named once.
    let run = hydra::normalize::read_with(&codex(limit), &policies(), &sandbox.layout()).unwrap();
    assert!(
        !run.findings.iter().any(|finding| matches!(
            finding,
            hydra::normalize::Finding::Limit(text) if text.contains("frame of type")
        )),
        "{:?}",
        run.findings
    );
    let mut record = Record::shaped(Shape {
        provider: Some("codex"),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        worktree: Some(&worktree),
        ..Shape::default()
    });
    record.raw(json!({"type": "turn.paused"}));
    record.raw(json!({"type": "turn.paused"}));
    let text = record.closed_with(json!({"exit_code": 0, "error": null}));
    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let named: Vec<_> = run
        .findings
        .iter()
        .filter(|finding| {
            matches!(
                finding,
                hydra::normalize::Finding::Limit(text) if text.contains("frame of type turn.paused")
            )
        })
        .collect();
    assert_eq!(named.len(), 1, "{:?}", run.findings);
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
    let text = Record::gated(&sandbox, "claude", "read-only", policies().sha())
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
fn a_pty_still_held_by_a_descendant_failed_rather_than_crashed() {
    // The child exited and a grandchild kept the slave open. Hydra stopped
    // reading after the deadline; without the note on `run_end.error` the same
    // shape would read as `crashed` because there is no terminator.
    let sandbox = Sandbox::new("pty-held");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "text", "text": "working"}]))
    .closed_with(json!({
        "exit_code": 0,
        "error": hydra::watch::PTY_HELD_NOTE,
    }));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Failed, Source::Derived))
    );
}

#[test]
fn a_pty_held_note_with_a_terminator_is_a_fidelity_note() {
    let sandbox = Sandbox::new("pty-held-note");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(usage_frame())
    .closed_with(json!({
        "exit_code": 0,
        "error": hydra::watch::PTY_HELD_NOTE,
    }));

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Completed, Source::Derived))
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
        let out = fixture_git::command()
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
            base_tree: None,
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

/// A real linked checkout with an empty base tree and one untracked file.
/// Trusted parent metadata lives outside the checkout, as it does at launch.
fn git_repo_with_untracked_file(worktree: &std::path::Path) {
    let parent = worktree
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("parent-repo");
    std::fs::create_dir_all(&parent).unwrap();
    for args in [
        &["init", "-q", "-b", "main"][..],
        &["config", "user.email", "test@example.invalid"][..],
        &["config", "user.name", "hydra tests"][..],
        &["commit", "--allow-empty", "-qm", "empty base"][..],
    ] {
        let out = fixture_git::command()
            .args(args)
            .current_dir(&parent)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    }
    std::fs::remove_dir(worktree).unwrap();
    hydra::worktree::create(&parent, worktree, "hydra/probe", &parent.join("seed-index")).unwrap();
    std::fs::write(worktree.join("a.txt"), "one\n").unwrap();
}

#[test]
fn a_recorded_base_tree_derives_the_diff_stat_with_no_spawn_directory() {
    // Map ticket 49: `worktree.base_tree` is on the record from build ticket 23
    // on, and a reader with it needs no seed index in the spawn directory to
    // answer contract 17's diff stat. The empty tree is what every repository
    // has already, so it stands in for a real seed and the file above counts as
    // the whole of what the run added.
    let sandbox = Sandbox::new("recorded-base-tree");
    let worktree = sandbox.worktree();
    git_repo_with_untracked_file(&worktree);
    let base = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

    let text = Record::shaped(Shape {
        worktree: Some(&worktree),
        base_tree: Some(base),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        ..Shape::default()
    })
    .assistant(json!([{"type": "text", "text": "done"}]))
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        !sandbox.layout().spawn(&run.run_id).exists(),
        "a spawn directory exists before the derivation touched anything"
    );

    let result = hydra::run::result(
        &run,
        &worktree.join("record.jsonl"),
        Some(&hydra::record::WorktreeRef {
            path: worktree.display().to_string(),
            branch: "hydra/probe".into(),
            base_tree: Some(base.to_string()),
        }),
        // No fallback offered, so a correct answer can only have come from the
        // record.
        None,
        &sandbox.layout(),
    );
    let json: serde_json::Value = serde_json::to_value(&result).unwrap();
    assert_eq!(json["diff_stat"]["base_tree"], base);
    assert_eq!(json["diff_stat"]["files_changed"], 1);
}

#[test]
fn no_base_tree_and_no_spawn_directory_derives_no_diff_stat() {
    // The other half of the same seam: a record with neither the field nor a
    // spawn directory to recompute one from has no base to diff against, and
    // the field is absent rather than a guess.
    let sandbox = Sandbox::new("no-base-tree");
    let worktree = sandbox.worktree();
    git_repo_with_untracked_file(&worktree);

    let text = Record::shaped(Shape {
        worktree: Some(&worktree),
        policy: Some("standard"),
        policy_sha: Some(policies().sha()),
        ..Shape::default()
    })
    .assistant(json!([{"type": "text", "text": "done"}]))
    .result(usage_frame())
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(!sandbox.layout().spawn(&run.run_id).exists());

    let result = hydra::run::result(
        &run,
        &worktree.join("record.jsonl"),
        Some(&hydra::record::WorktreeRef {
            path: worktree.display().to_string(),
            branch: "hydra/probe".into(),
            base_tree: None,
        }),
        None,
        &sandbox.layout(),
    );
    let json: serde_json::Value = serde_json::to_value(&result).unwrap();
    assert!(json.get("diff_stat").is_none(), "{json}");
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
            base_tree: None,
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
    let text = Record::gated(&sandbox, "claude", "read-only", policies().sha())
        .assistant(json!([{"type": "text", "text": "   "}]))
        .result(usage_frame())
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(hydra::outcome::final_assistant_text(&run), None);
}

// --- cursor-agent: no terminator on the failure path ----------------------

#[test]
fn a_cursor_run_that_exited_with_no_terminator_failed_rather_than_crashed() {
    // Harness spec 10: `cursor-agent` writes its `result` frame only on
    // success, so its every hard failure is one stderr sentence, no frame and
    // exit 1. That is the provider's own failure path and not a death.
    let sandbox = Sandbox::new("cursor-failed");
    let text = Record::cursor(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .cursor_init()
    .cursor_user("x")
    .noise("Error: something the provider caught")
    .closed_with(json!({"exit_code": 1}));
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Failed, Source::Derived))
    );
}

#[test]
fn a_cursor_usage_limit_refusal_is_rate_limited_off_stderr() {
    // Research 33's shape, 2026-09-04: `system/init` and `user` written, then
    // the sentence on stderr, exit 1, no frame and no usage. Rung one must not
    // learn a skill lesson from an exhausted quota.
    let sandbox = Sandbox::new("cursor-limit");
    let text = Record::cursor(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .cursor_init()
    .cursor_user("x")
    .noise(
        "ActionRequiredError: You've hit your usage limit You've saved $49 on API model \
             usage this month with Pro. Switch to a different model or set a Spend Limit to \
             continue with Sonnet.",
    )
    .closed_with(json!({"exit_code": 1}));
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::RateLimited, Source::Derived))
    );

    let sandbox = Sandbox::new("cursor-auth");
    let text = Record::cursor(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .noise("Error: Authentication required. Please run 'cursor-agent login' first.")
    .closed_with(json!({"exit_code": 1}));
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::AuthFailed, Source::Derived))
    );
}

#[test]
fn a_cursor_child_that_died_on_a_signal_crashed() {
    // The pty reports a signal death as exit code 1, which on this provider is
    // also an orderly failure. The detached half writes the signal into
    // `run_end.error`, and that note is what separates the two.
    let sandbox = Sandbox::new("cursor-signal");
    let text = Record::cursor(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .cursor_init()
    .closed_with(json!({
        "exit_code": 1,
        "error": format!("{}SIGKILL", hydra::watch::SIGNAL_NOTE),
    }));
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Crashed, Source::Derived))
    );
}

#[test]
fn a_cursor_run_whose_only_call_the_gate_refused_is_gated() {
    let sandbox = Sandbox::new("cursor-gated");
    let text = Record::cursor(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .cursor_init()
    .cursor_call(
        "c1",
        "readToolCall",
        json!({"path": "/etc/hosts"}),
        cursor_read_blocked(
            "hydra/out-of-scope: policy read-only allows file_read inside worktree \
                 and this call is outside it.",
        ),
    )
    .cursor_assistant("BLOCKED")
    .cursor_result("BLOCKED", Some(cursor_usage()))
    .closed();
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Gated, Source::Derived))
    );
}

// --- the corpus ----------------------------------------------------------

fn fixture_read(
    policies: &hydra::policy::PolicySet,
) -> impl Fn(&Path) -> anyhow::Result<hydra::normalize::Run> + '_ {
    move |path: &Path| {
        let text = std::fs::read_to_string(path)?;
        hydra::normalize::read_with(&text, policies, &common::fixture_layout())
    }
}

fn assert_no_walk_findings(findings: &[hydra::walk::Finding]) {
    assert!(
        findings.is_empty(),
        "walk findings:\n{}",
        findings
            .iter()
            .map(|finding| format!("  {}: {}", finding.path.display(), finding.assertion))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_fixture_derives_an_outcome() {
    let policies = policies();
    let read = fixture_read(&policies);
    let (findings, seen) = hydra::walk::check_outcomes(&common::fixture_corpus(), &read);
    assert_no_walk_findings(&findings);
    assert!(!seen.is_empty(), "no fixture records to derive from");
    // One of each outcome the fixtures were built to carry.
    for outcome in ["completed", "gated", "rate_limited"] {
        assert!(
            seen.iter().any(|derived| derived.outcome == outcome),
            "no fixture derives {outcome}: {seen:?}"
        );
    }
    assert!(
        seen.iter().any(|derived| derived.source == "refusal"),
        "no fixture derives from a refusal: {seen:?}"
    );
}

#[test]
fn the_fixture_write_denial_is_the_only_gated_fixture_on_the_stream() {
    let policies = policies();
    let read = fixture_read(&policies);
    let (findings, gated) =
        hydra::walk::check_gated_write_denials(&common::fixture_corpus(), &read);
    assert_no_walk_findings(&findings);
    assert_eq!(gated, 1, "the fixtures hold one read-only Write denial");
}

#[test]
fn a_fixture_is_unknown_only_where_a_result_is_unclassified() {
    let policies = policies();
    let read = fixture_read(&policies);
    let findings = hydra::walk::check_unknown_sources(&common::fixture_corpus(), &read);
    assert_no_walk_findings(&findings);
}

// --- agy: the terminator says how it went in a field of its own -----------

#[test]
fn an_agy_run_that_says_it_failed_is_failed() {
    // This provider has no `is_error`. It puts `status` on the terminator,
    // `SUCCESS` or `ERROR`, with an `error` string beside it; measured on 1.1.26
    // on a slash command it would not run in print mode. Without reading that
    // field a run that failed outright comes out `completed`, because `is_error`
    // is absent rather than false.
    let sandbox = Sandbox::new("agy-failed");
    let text = Record::agy(&sandbox.worktree(), None, None)
        .agy_user()
        .agy_result("ERROR", "", None)
        .closed_with(json!({ "exit_code": 1 }));
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Failed, Source::Derived))
    );
}

#[test]
fn an_agy_run_that_worked_is_completed_and_its_success_word_is_worth_nothing_alone() {
    let sandbox = Sandbox::new("agy-completed");
    let text = Record::agy(&sandbox.worktree(), None, None)
        .agy_user()
        .agy_response(1, &["DONE"], Some(common::agy_usage(10, 2, 0, 0)))
        .agy_result("SUCCESS", "DONE", None)
        .closed();
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Completed, Source::Derived))
    );
}

#[test]
fn an_agy_run_whose_every_call_the_gate_refused_is_gated_over_its_own_success() {
    // The shape ticket 26 measured: every tool call denied, `status: SUCCESS`,
    // exit 0, and the model narrating the block and then guessing the answer.
    // An outcome read off that terminator files a fully blocked run as a
    // finished one, which is what contract 8 exists to prevent.
    let sandbox = Sandbox::new("agy-gated");
    let text = Record::agy(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .agy_user()
    .agy_tool_error(
        2,
        "run_command",
        json!({ "CommandLine": "echo T10MARKER" }),
        "tool call denied by pre-tool hook: hydra/capability: policy read-only does not \
         allow shell (run_command).",
    )
    .agy_response(3, &["It could not be executed."], None)
    .agy_result("SUCCESS", "It could not be executed.", None)
    .closed();

    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Gated, Source::Derived))
    );
}

#[test]
fn an_agy_child_that_died_before_its_terminator_crashed() {
    // Unlike `cursor-agent`, this provider writes a terminator on every path, so
    // a record without one is a process that stopped rather than the ordinary
    // shape of a failure.
    let sandbox = Sandbox::new("agy-crashed");
    let text = Record::agy(&sandbox.worktree(), None, None)
        .agy_user()
        .agy_response(1, &["half a th"], None)
        .closed_with(json!({ "exit_code": 143 }));
    assert_eq!(
        outcome_of(&sandbox, &text),
        Some((Outcome::Crashed, Source::Derived))
    );
}

/// Build ticket 259: a harvested `codex` session ends a turn it could not
/// finish on `task_complete` with `error`, in the shape the one TUI session
/// that hit its limit carries (0.154.0, 2026-09-25): `codex_error_info:
/// "usage_limit_exceeded"` beside the sentence. Every line here is synthetic.
#[test]
fn a_harvested_codex_session_that_hit_its_limit_is_rate_limited() {
    let sandbox = Sandbox::new("codex-session-limit");
    let worktree = sandbox.worktree();
    let session = |errors: &[Option<&str>]| {
        let mut record = Record::shaped(Shape {
            provider: Some("codex"),
            worktree: Some(&worktree),
            ..Shape::default()
        });
        record.raw(json!({"type": "session_meta", "payload": {
            "id": "s", "source": "cli", "originator": "codex-tui",
        }}));
        for error in errors {
            record.raw(json!({"type": "event_msg", "payload": {"type": "task_started"}}));
            record.raw(json!({"type": "response_item", "payload": {
                "type": "message", "role": "user",
                "content": [{"type": "input_text", "text": "look around"}],
            }}));
            record.raw(json!({"type": "event_msg", "payload": {
                "type": "token_count", "info": null,
            }}));
            let error = match error {
                Some(info) => json!({
                    "message": "You've hit your usage limit. Try again later.",
                    "codex_error_info": info,
                }),
                None => Value::Null,
            };
            record.raw(json!({"type": "event_msg", "payload": {
                "type": "task_complete", "last_agent_message": null, "error": error,
            }}));
        }
        record.closed_with(json!({"exit_code": 0, "body_source": "harvest"}))
    };

    assert_eq!(
        outcome_of(&sandbox, &session(&[Some("usage_limit_exceeded")])),
        Some((Outcome::RateLimited, Source::Derived))
    );
    // A session that went on past the limit and finished a turn was not
    // stopped by it.
    assert_ne!(
        outcome_of(&sandbox, &session(&[Some("usage_limit_exceeded"), None]))
            .map(|(outcome, _)| outcome),
        Some(Outcome::RateLimited)
    );
}

/// Build ticket 260: a harvested session whose last turn ended on a non-limit
/// error (`codex_error_info` `other` or `cyber_policy`) is the session file's
/// `turn.failed`. It reads `failed` the way a streamed non-quota `turn.failed`
/// does, even when the TUI exited nonzero. Every line here is synthetic.
#[test]
fn a_harvested_codex_session_that_ended_on_a_non_limit_error_is_failed() {
    let sandbox = Sandbox::new("codex-session-failed");
    let worktree = sandbox.worktree();
    let session = |info: &str, message: &str| {
        let mut record = Record::shaped(Shape {
            provider: Some("codex"),
            launch_mode: "interactive",
            worktree: Some(&worktree),
            ..Shape::default()
        });
        record.raw(json!({"type": "session_meta", "payload": {
            "id": "s", "source": "cli", "originator": "codex-tui",
        }}));
        record.raw(json!({"type": "event_msg", "payload": {"type": "task_started"}}));
        record.raw(json!({"type": "response_item", "payload": {
            "type": "message", "role": "user",
            "content": [{"type": "input_text", "text": "look around"}],
        }}));
        record.raw(json!({"type": "event_msg", "payload": {
            "type": "token_count", "info": null,
        }}));
        record.raw(json!({"type": "event_msg", "payload": {
            "type": "task_complete",
            "last_agent_message": null,
            "error": {"message": message, "codex_error_info": info},
        }}));
        record.closed_with(json!({"exit_code": 1, "body_source": "harvest"}))
    };

    for (info, message) in [
        ("other", "the model could not finish this turn."),
        (
            "cyber_policy",
            "This content was flagged for possible cybersecurity risk.",
        ),
    ] {
        assert_eq!(
            outcome_of(&sandbox, &session(info, message)),
            Some((Outcome::Failed, Source::Derived)),
            "{info}"
        );
    }
}
