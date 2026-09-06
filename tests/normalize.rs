//! Reading a stored `claude` run back into the ten event types.
//!
//! Two kinds of test. The synthetic ones build a record line by line and ask what
//! the normalizer makes of one frame shape, which is the only way to reach a
//! disagreement or a gate leak, because neither has ever happened on this
//! machine. The corpus ones walk `~/hydra/runs/` and are the ones that would
//! catch a frame shape this file guessed wrong.
//!
//! Every test that touches a policy compiles the real gate and runs it, so these
//! cost a python process per tool call and launch no provider.

use std::path::PathBuf;

use hydra::normalize::{Decider, Decision, Event, Finding, ResultStatus, Role, UsageSource};
use serde_json::json;

mod common;
use common::{
    RUN_ID, Record, Sandbox, corpus, cursor_read_blocked, cursor_read_blocked_late, cursor_read_ok,
    cursor_shell_rejected, cursor_usage, policies, usage_frame,
};

// --- one frame at a time -------------------------------------------------

#[test]
fn a_text_block_becomes_a_message() {
    let sandbox = Sandbox::new("text");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .assistant(json!([{"type": "text", "text": "Blocked. Stopping as instructed."}]))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let messages: Vec<_> = run.messages().collect();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, Role::Assistant);
    assert_eq!(messages[0].text, "Blocked. Stopping as instructed.");
    assert_eq!(messages[0].block, 0);
    assert_eq!(messages[0].seq, 1);
}

#[test]
fn a_thinking_block_carries_its_signature_and_where_it_came_from() {
    let sandbox = Sandbox::new("thinking");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .assistant(json!([
            {"type": "text", "text": "first"},
            {"type": "thinking", "thinking": "weighing it", "signature": "CAISqBMK"},
        ]))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let thinking: Vec<_> = run
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Thinking(block) => Some(block),
            _ => None,
        })
        .collect();
    assert_eq!(thinking.len(), 1);
    assert_eq!(thinking[0].text, "weighing it");
    assert_eq!(thinking[0].signature.as_deref(), Some("CAISqBMK"));
    // The second block of the frame, which is what makes the derivation
    // reproducible from `raw` rather than trusted.
    assert_eq!(thinking[0].block, 1);
}

#[test]
fn a_tool_use_carries_what_the_table_maps_its_name_to() {
    let sandbox = Sandbox::new("call");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .tool_use(
            "toolu_01",
            "Write",
            json!({"file_path": "/tmp/x", "content": "HELLO\n"}),
        )
        .tool_use("toolu_02", "mcp__hydra__delegate", json!({"task": "x"}))
        .tool_use("toolu_03", "Telepathy", json!({}))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let calls: Vec<_> = run.tool_calls().collect();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].name, "Write");
    assert_eq!(calls[0].call_id, "toolu_01");
    assert_eq!(calls[0].arguments["content"], "HELLO\n");
    assert_eq!(calls[0].capability.map(|c| c.as_str()), Some("file_write"));
    // A prefix rather than a name, matched before the table.
    assert_eq!(calls[1].capability.map(|c| c.as_str()), Some("mcp_call"));
    // 7.1: a name the table does not know is a finding rather than a shrug.
    assert_eq!(calls[2].capability, None);
    assert!(run.findings.contains(&Finding::UnknownTool {
        name: "Telepathy".into()
    }));
}

#[test]
fn a_result_joins_its_call_and_nothing_upgrades_what_the_adapter_cannot_tell() {
    let sandbox = Sandbox::new("result");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .tool_use("toolu_01", "Read", json!({"file_path": "/tmp/x"}))
        .tool_result("toolu_01", "HELLO", false)
        .tool_use("toolu_02", "Read", json!({"file_path": "/tmp/y"}))
        .tool_result("toolu_02", "no such file", true)
        .tool_use("toolu_03", "Read", json!({"file_path": "/tmp/z"}))
        .raw(json!({
            "type": "user",
            "message": {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_03", "content": "who knows"}
            ]},
        }))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let results: Vec<_> = run.tool_results().collect();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].call_id, "toolu_01");
    assert_eq!(results[0].status, ResultStatus::Ok);
    assert_eq!(results[0].output, "HELLO");
    assert_eq!(results[1].status, ResultStatus::Error);
    // No `is_error` field at all, so the adapter says it cannot tell.
    assert_eq!(results[2].status, ResultStatus::Unclassified);
    assert!(!results[0].truncated);
}

#[test]
fn a_line_that_is_not_json_normalizes_to_nothing_and_is_not_an_error() {
    let sandbox = Sandbox::new("noise");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .assistant(json!([{"type": "text", "text": "done"}]))
        .noise("\u{1b}[?25h")
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.messages().count(), 1);
    assert!(run.findings.is_empty(), "{:?}", run.findings);
}

// --- usage ---------------------------------------------------------------

#[test]
fn usage_carries_the_parts_and_derives_the_total() {
    let sandbox = Sandbox::new("usage");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .result(usage_frame())
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let usage = run.usage().expect("a result frame carries usage");
    assert_eq!(usage.source, UsageSource::Provider);
    assert_eq!(usage.estimator, None);
    let counts = usage.counts.expect("counts");
    assert_eq!(counts.input_tokens, 4);
    assert_eq!(counts.cache_read_tokens, 30266);
    assert_eq!(counts.cache_write_tokens, Some(15110));
    assert_eq!(counts.thinking_tokens, Some(872));
    // 9.1's reconstruction. Reading `input_tokens` as the prompt size would say 4.
    assert_eq!(counts.raw_input(), 45380);
    assert_eq!(counts.total_tokens(), 46650);
}

#[test]
fn a_run_with_no_result_frame_reports_usage_unavailable_rather_than_zero() {
    let sandbox = Sandbox::new("no-usage");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .assistant(json!([{"type": "text", "text": "killed before the result frame"}]))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let usage = run.usage().expect("never a bare null, 7.8");
    assert_eq!(usage.source, UsageSource::Unavailable);
    assert_eq!(usage.counts, None);
    assert_eq!(usage.seq, None);
}

// --- the decision --------------------------------------------------------

#[test]
fn a_run_with_no_policy_has_no_permission_decision() {
    let sandbox = Sandbox::new("pre-gate");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .tool_use(
            "toolu_01",
            "Write",
            json!({"file_path": "/tmp/x", "content": "HELLO"}),
        )
        .raw(json!({
            "type": "system",
            "subtype": "permission_denied",
            "tool_name": "Write",
            "tool_use_id": "toolu_01",
            "message": "Claude requested permissions to write, but you haven't granted it yet.",
        }))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.policy, None);
    // Absent, never empty. 7.7: `external` presumes a policy exists to have allowed.
    assert_eq!(run.permission_decisions().count(), 0);
    assert!(run.findings.is_empty(), "{:?}", run.findings);
}

#[test]
fn the_runs_own_policy_is_re_evaluated_into_a_decision() {
    let sandbox = Sandbox::new("deny");
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
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].call_id, "toolu_01");
    assert_eq!(decisions[0].decision, Decision::Deny);
    assert_eq!(decisions[0].decider, Decider::HydraPolicy);
    assert!(
        decisions[0].reason.starts_with("hydra/capability:"),
        "{}",
        decisions[0].reason
    );
    // The gate and the derivation agree, so nothing is reported.
    assert!(run.findings.is_empty(), "{:?}", run.findings);
}

#[test]
fn a_call_the_policy_allows_gets_an_allow_decision() {
    let sandbox = Sandbox::new("allow");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .tool_use(
        "toolu_01",
        "Bash",
        json!({"command": "echo HELLO > probe.txt", "description": "write"}),
    )
    .hook_response(
        "Bash",
        "allow",
        "hydra/allow: policy standard allows shell.",
    )
    .tool_result("toolu_01", "exit=0", false)
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, Decision::Allow);
    assert_eq!(decisions[0].decider, Decider::HydraPolicy);
    assert!(run.findings.is_empty(), "{:?}", run.findings);
}

#[test]
fn a_call_the_policy_denies_that_nothing_refused_is_a_gate_leak() {
    let sandbox = Sandbox::new("leak");
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
    // The call went through: a successful result and no refusal anywhere.
    .tool_result("toolu_01", "File created successfully.", false)
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings.iter().any(|finding| matches!(
            finding,
            Finding::GateLeak { call_id, tool, .. } if call_id == "toolu_01" && tool == "Write"
        )),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_refusal_the_policy_would_have_allowed_is_external() {
    let sandbox = Sandbox::new("external");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .tool_use(
        "toolu_01",
        "Bash",
        json!({"command": "echo HELLO", "description": "say hello"}),
    )
    // The user's own guards write prose in their own voice and carry no
    // `hydra/` prefix, which is how the derivation tells them apart.
    .hook_response("Bash", "deny", "Rejected: t30 CUR-PROJ says no bare echo")
    .tool_result("toolu_01", "Rejected: t30 CUR-PROJ says no bare echo", true)
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, Decision::Deny);
    assert_eq!(decisions[0].decider, Decider::External);
    assert_eq!(
        decisions[0].reason,
        "Rejected: t30 CUR-PROJ says no bare echo"
    );
    // Something outside Hydra refused a call the policy allowed. That is not a leak.
    assert!(
        !run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::GateLeak { .. })),
        "{:?}",
        run.findings
    );
}

#[test]
fn the_gates_own_answer_disagreeing_with_the_derivation_is_a_finding() {
    let sandbox = Sandbox::new("disagreement");
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
    // The gate that ran said allow. Re-evaluating the same policy says deny.
    .hook_response(
        "Write",
        "allow",
        "hydra/allow: policy read-only allows file_write.",
    )
    .tool_result("toolu_01", "File created successfully.", false)
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings.iter().any(|finding| matches!(
            finding,
            Finding::Disagreement { call_id, derived, gate_said, .. }
                if call_id == "toolu_01"
                    && *derived == Decision::Deny
                    && *gate_said == Decision::Allow
        )),
        "{:?}",
        run.findings
    );
}

#[test]
fn subagents_spawned_under_a_policy_that_denies_them_is_a_finding() {
    let sandbox = Sandbox::new("subagents");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .result(json!({"subagent_stats": {"spawned": 3, "max_depth": 1}}))
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings.contains(&Finding::SubagentLeak { spawned: 3 }),
        "{:?}",
        run.findings
    );
}

#[test]
fn subagents_spawned_under_a_policy_that_grants_them_is_not() {
    let sandbox = Sandbox::new("subagents-ok");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("permissive"),
        Some(policies().sha()),
    )
    .result(json!({"subagent_stats": {"spawned": 3, "max_depth": 1}}))
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(run.findings.is_empty(), "{:?}", run.findings);
}

#[test]
fn a_policy_sha_that_is_not_todays_is_a_finding() {
    let sandbox = Sandbox::new("stale");
    let stale = "0000000000000000000000000000000000000000000000000000000000000000";
    let text = Record::opened(&sandbox.worktree(), Some("read-only"), Some(stale))
        .assistant(json!([{"type": "text", "text": "done"}]))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings.iter().any(|finding| matches!(
            finding,
            Finding::StalePolicy { recorded, .. } if recorded == stale
        )),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_tool_the_provider_offered_that_the_table_does_not_know_is_a_finding() {
    let sandbox = Sandbox::new("table");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .init(&["Bash", "Read", "Telekinesis"])
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings.contains(&Finding::UnknownTool {
            name: "Telekinesis".into()
        }),
        "{:?}",
        run.findings
    );
    assert_eq!(
        run.findings
            .iter()
            .filter(|finding| matches!(finding, Finding::UnknownTool { .. }))
            .count(),
        1
    );
}

// --- cursor-agent, the second column of 7.2 -------------------------------

#[test]
fn a_cursor_record_is_read_by_its_own_adapter() {
    // The envelope's provider picks the adapter (contract 4). The frames are
    // the ones the 2026-09-06 probe streamed: init, the echoed prompt, thinking,
    // the answer, and a result carrying usage.
    let sandbox = Sandbox::new("cursor-read");
    let text = Record::cursor(&sandbox.worktree(), None, None)
        .cursor_init()
        .cursor_user("Reply with the single word DONE.")
        .cursor_thinking(&["The user wants", " DONE."])
        .cursor_assistant("DONE")
        .cursor_result("DONE", Some(cursor_usage()))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.provider.as_deref(), Some("cursor-agent"));

    let messages: Vec<_> = run.messages().collect();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[1].role, Role::Assistant);
    assert_eq!(messages[1].text, "DONE");

    // 7.5: text alone, one event per block, concatenated over its deltas and
    // reproducible from `raw` by the seq of the first delta.
    let thinking: Vec<_> = run
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Thinking(block) => Some(block),
            _ => None,
        })
        .collect();
    assert_eq!(thinking.len(), 1);
    assert_eq!(thinking[0].text, "The user wants DONE.");
    assert_eq!(thinking[0].signature, None);
    assert_eq!(thinking[0].block, 0);
    assert_eq!(thinking[0].seq, 3, "the first delta's frame");

    // Section 9: the parts, net of cache, and no thinking count on this
    // provider.
    let usage = run.usage().unwrap();
    assert_eq!(usage.source, UsageSource::Provider);
    let counts = usage.counts.unwrap();
    assert_eq!(counts.input_tokens, 8383);
    assert_eq!(counts.cache_read_tokens, 109307);
    assert_eq!(counts.cache_write_tokens, Some(0));
    assert_eq!(counts.thinking_tokens, None);
    assert_eq!(counts.raw_input(), 8383 + 109307);
    assert!(run.findings.is_empty(), "{:?}", run.findings);
}

#[test]
fn a_cursor_tool_call_is_named_as_the_hook_saw_it() {
    // The stream says `editToolCall` with `path` and `streamContent`; the hook
    // that gated it saw `Write` with `file_path` and `content`, and 7.7
    // re-evaluates the policy against what the gate saw.
    let sandbox = Sandbox::new("cursor-calls");
    let ws = sandbox.worktree();
    let text = Record::cursor(&ws, None, None)
        .cursor_tool("c1", "readToolCall", json!({"path": ws.join("a.txt")}), None)
        .cursor_tool(
            "c2",
            "shellToolCall",
            json!({"command": "echo T09PROBE", "workingDirectory": "", "timeout": 30000}),
            None,
        )
        .cursor_tool(
            "c3",
            "editToolCall",
            json!({"path": ws.join("b.txt"), "streamContent": "hello-b\n"}),
            None,
        )
        .cursor_tool(
            "c4",
            "globToolCall",
            json!({"targetDirectory": ws, "globPattern": "*"}),
            None,
        )
        .cursor_tool("c5", "deleteToolCall", json!({"path": ws.join("b.txt")}), None)
        .cursor_tool("c6", "semSearchToolCall", json!({"query": "x"}), None)
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let calls: Vec<_> = run.tool_calls().collect();
    assert_eq!(calls.len(), 6);
    assert_eq!(calls[0].name, "Read");
    assert_eq!(calls[0].call_id, "c1");
    assert_eq!(
        calls[0].arguments["file_path"].as_str(),
        Some(ws.join("a.txt").to_string_lossy().as_ref())
    );
    assert_eq!(calls[0].capability.map(|c| c.as_str()), Some("file_read"));
    assert_eq!(calls[1].name, "Shell");
    assert_eq!(calls[1].arguments["command"], "echo T09PROBE");
    assert_eq!(calls[1].capability.map(|c| c.as_str()), Some("shell"));
    assert_eq!(calls[2].name, "Write");
    assert_eq!(calls[2].arguments["content"], "hello-b\n");
    assert_eq!(calls[2].capability.map(|c| c.as_str()), Some("file_write"));
    assert_eq!(calls[3].name, "Grep", "the listing tool is gated as Grep");
    assert_eq!(calls[3].capability.map(|c| c.as_str()), Some("file_read"));
    assert_eq!(calls[4].name, "Delete");
    assert_eq!(calls[4].capability.map(|c| c.as_str()), Some("file_write"));
    // A key the adapter does not know keeps its own spelling and 7.1 makes
    // it a finding rather than a shrug.
    assert_eq!(calls[5].name, "semSearchToolCall");
    assert_eq!(calls[5].capability, None);
    assert!(run.findings.contains(&Finding::UnknownTool {
        name: "semSearchToolCall".into()
    }));
}

#[test]
fn a_cursor_result_joins_its_call_by_id_and_names_its_case() {
    let sandbox = Sandbox::new("cursor-results");
    let ws = sandbox.worktree();
    let a = ws.join("a.txt").to_string_lossy().to_string();
    let text = Record::cursor(&ws, None, None)
        .cursor_call("c1", "readToolCall", json!({"path": a}), cursor_read_ok(&a, "hi\n"))
        .cursor_call(
            "c2",
            "readToolCall",
            json!({"path": ws.join("nope.txt")}),
            json!({"error": {"errorMessage": "File not found"}}),
        )
        .cursor_call(
            "c3",
            "shellToolCall",
            json!({"command": "exit 3", "workingDirectory": "", "timeout": 30000}),
            json!({"rejected": {"command": "exit 3", "workingDirectory": "", "reason": "", "isReadonly": false}}),
        )
        .cursor_call("c4", "readToolCall", json!({"path": a}), json!({"somethingNew": {}}))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    // One call per id, whichever frames carried it.
    assert_eq!(run.tool_calls().count(), 4);
    let results: Vec<_> = run.tool_results().collect();
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].call_id, "c1");
    assert_eq!(results[0].status, ResultStatus::Ok);
    assert!(results[0].output.contains("hi\\n"));
    assert_eq!(results[1].status, ResultStatus::Error);
    assert_eq!(results[2].status, ResultStatus::Error, "a rejection is not ok");
    assert_eq!(results[3].status, ResultStatus::Unclassified);
    assert!(!results[0].truncated);
}

#[test]
fn a_cursor_run_with_no_result_frame_is_unavailable_never_estimated() {
    // Contract 7.8 and 18.1: the failure path writes no frame, and Cursor
    // counts about twice what the transcript keeps, so nothing estimates.
    let sandbox = Sandbox::new("cursor-nousage");
    let text = Record::cursor(&sandbox.worktree(), None, None)
        .cursor_init()
        .cursor_user("x")
        .noise("ActionRequiredError: You've hit your usage limit ...")
        .closed_with(json!({"exit_code": 1}));

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let usage = run.usage().unwrap();
    assert_eq!(usage.source, UsageSource::Unavailable);
    assert!(usage.counts.is_none());
    assert!(usage.estimator.is_none());
    assert_eq!(run.noise.len(), 1, "the stderr sentence is kept for the outcome");

    // A `result` frame with no usage block, which the emitter permits on the
    // success path too, is the same answer.
    let text = Record::cursor(&sandbox.worktree(), None, None)
        .cursor_result("DONE", None)
        .closed();
    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.usage().unwrap().source, UsageSource::Unavailable);
}

#[test]
fn a_cursor_read_the_policy_denies_and_the_hook_refused_is_hydras_deny() {
    // Both shapes a refused read arrives in, measured on 2026-09-06: the real
    // run's, refused at `preToolUse` with the gate's words in front, and the
    // probe's, refused at `beforeReadFile` behind Cursor's own prefix.
    let reason = "hydra/out-of-scope: policy read-only allows file_read inside worktree \
                  and this call is outside it.";
    for (name, blocked) in [
        ("cursor-deny-early", cursor_read_blocked(reason)),
        ("cursor-deny-late", cursor_read_blocked_late(reason)),
    ] {
        let sandbox = Sandbox::new(name);
        let ws = sandbox.worktree();
        let text = Record::cursor(&ws, Some("read-only"), Some(policies().sha()))
            .cursor_call("c1", "readToolCall", json!({"path": "/etc/hosts"}), blocked)
            .cursor_assistant("BLOCKED")
            .cursor_result("BLOCKED", Some(cursor_usage()))
            .closed();

        let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
        let decisions: Vec<_> = run.permission_decisions().collect();
        assert_eq!(decisions.len(), 1, "{name}");
        assert_eq!(decisions[0].call_id, "c1");
        assert_eq!(decisions[0].decision, Decision::Deny);
        assert_eq!(decisions[0].decider, Decider::HydraPolicy);
        assert!(decisions[0].reason.starts_with("hydra/out-of-scope:"), "{}", decisions[0].reason);
        assert!(run.findings.is_empty(), "{name}: {:?}", run.findings);
    }
}

#[test]
fn a_cursor_refusal_carrying_hydras_allow_under_a_denying_policy_is_a_disagreement() {
    // The gate allowed at run time, the user's guard denied, and the policy
    // denies now. Two sources disagree, and the record says so rather than
    // filing the call as one the gate never answered.
    let sandbox = Sandbox::new("cursor-disagree");
    let ws = sandbox.worktree();
    let text = Record::cursor(&ws, Some("read-only"), Some(policies().sha()))
        .cursor_call(
            "c1",
            "shellToolCall",
            json!({"command": "git t30probe", "workingDirectory": "", "timeout": 30000}),
            cursor_shell_rejected(
                "git t30probe",
                "hydra/allow: policy standard allows shell.\n\n---\n\nHook blocked with \
                 message: 'git t30probe' is not a known subcommand.",
            ),
        )
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions[0].decision, Decision::Deny);
    assert_eq!(decisions[0].decider, Decider::HydraPolicy);
    assert!(
        run.findings.iter().any(|finding| matches!(
            finding,
            Finding::Disagreement { call_id, gate_said: Decision::Allow, .. } if call_id == "c1"
        )),
        "{:?}",
        run.findings
    );
    assert!(
        !run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::Ungated { .. })),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_cursor_frame_with_no_call_id_still_yields_a_call_and_says_what_it_lost() {
    let sandbox = Sandbox::new("cursor-noid");
    let ws = sandbox.worktree();
    let mut record = Record::cursor(&ws, None, None);
    for _ in 0..2 {
        record.raw(json!({
            "type": "tool_call", "subtype": "started",
            "tool_call": {"readToolCall": {"args": {"path": ws.join("a.txt")}}},
        }));
    }
    let text = record.closed();
    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.tool_calls().count(), 2, "two starts are two calls, id or no id");
}

#[test]
fn a_cursor_call_the_policy_denies_that_went_through_is_a_gate_leak() {
    let sandbox = Sandbox::new("cursor-leak");
    let ws = sandbox.worktree();
    let b = ws.join("b.txt").to_string_lossy().to_string();
    let text = Record::cursor(&ws, Some("read-only"), Some(policies().sha()))
        .cursor_call(
            "c1",
            "editToolCall",
            json!({"path": b, "streamContent": "x\n"}),
            json!({"success": {"path": b, "linesAdded": 1, "linesRemoved": 0}}),
        )
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings.iter().any(|finding| matches!(
            finding,
            Finding::GateLeak { call_id, tool, .. } if call_id == "c1" && tool == "Write"
        )),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_cursor_refusal_carrying_hydras_allow_is_external() {
    // Research 30's U7 shape: the project hook allowed, the user's guard
    // denied by exit code, and Cursor concatenated both messages. The policy
    // allows the call, so the decider is the guard and nothing leaked.
    let sandbox = Sandbox::new("cursor-external");
    let ws = sandbox.worktree();
    let text = Record::cursor(&ws, Some("standard"), Some(policies().sha()))
        .cursor_call(
            "c1",
            "shellToolCall",
            json!({"command": "git t30probe", "workingDirectory": "", "timeout": 30000}),
            cursor_shell_rejected(
                "git t30probe",
                "hydra/allow: policy standard allows shell.\n\n---\n\nHook blocked with \
                 message: 'git t30probe' is not a known subcommand: it could be an alias \
                 wrapping a destructive operation.",
            ),
        )
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, Decision::Deny);
    assert_eq!(decisions[0].decider, Decider::External);
    assert!(
        !run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::GateLeak { .. })),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_cursor_call_the_policy_allows_that_went_through_raises_no_finding() {
    // No hook frames on this provider, so an allowed call carries no gate
    // answer and its absence is not evidence: the config fails closed.
    let sandbox = Sandbox::new("cursor-allow");
    let ws = sandbox.worktree();
    let text = Record::cursor(&ws, Some("standard"), Some(policies().sha()))
        .cursor_call(
            "c1",
            "shellToolCall",
            json!({"command": "echo T09PROBE", "workingDirectory": "", "timeout": 30000}),
            json!({"success": {"command": "echo T09PROBE", "exitCode": 0, "stdout": "T09PROBE\n", "stderr": ""}}),
        )
        .cursor_result("DONE", Some(cursor_usage()))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, Decision::Allow);
    assert_eq!(decisions[0].decider, Decider::HydraPolicy);
    assert!(run.findings.is_empty(), "{:?}", run.findings);
}

#[test]
fn a_cursor_refusal_without_hydras_voice_under_a_denying_policy_is_ungated() {
    // The policy denies, something refused, and Hydra's message is not in the
    // concatenation. On this provider that is a config the loader did not run,
    // which it does silently for a symlinked one.
    let sandbox = Sandbox::new("cursor-ungated");
    let ws = sandbox.worktree();
    let text = Record::cursor(&ws, Some("read-only"), Some(policies().sha()))
        .cursor_call(
            "c1",
            "shellToolCall",
            json!({"command": "echo hi", "workingDirectory": "", "timeout": 30000}),
            cursor_shell_rejected("echo hi", "Hook blocked with message: no echo today"),
        )
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions[0].decision, Decision::Deny);
    assert_eq!(decisions[0].decider, Decider::HydraPolicy);
    assert!(
        run.findings.iter().any(|finding| matches!(
            finding,
            Finding::Ungated { call_id, .. } if call_id == "c1"
        )),
        "{:?}",
        run.findings
    );
}

// --- the corpus ----------------------------------------------------------

#[test]
fn every_record_on_this_machine_normalizes() {
    let policies = policies();
    let mut gated = 0;
    let mut pre_gate = 0;
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies)
            .unwrap_or_else(|err| panic!("{} did not normalize: {err:#}", path.display()));
        assert!(
            matches!(run.provider.as_deref(), Some("claude") | Some("cursor-agent")),
            "{}: {:?}",
            path.display(),
            run.provider
        );
        assert!(run.run_start().is_some(), "{}", path.display());

        match run.policy {
            // 7.2.1 and 5.6: a run with no policy has nothing to re-evaluate.
            None => {
                assert_eq!(
                    run.permission_decisions().count(),
                    0,
                    "{} has no policy and still produced a decision",
                    path.display()
                );
                pre_gate += 1;
            }
            Some(_) => {
                assert_eq!(
                    run.permission_decisions().count(),
                    run.tool_calls().count(),
                    "{} owes one decision per call",
                    path.display()
                );
                gated += 1;
            }
        }
        // 7.8: never a bare null, on every record.
        assert!(run.usage().is_some(), "{}", path.display());
    }
    eprintln!("normalized {gated} gated and {pre_gate} pre-gate record(s)");
}

#[test]
fn the_recorded_write_denial_derives_to_a_deny_under_capability() {
    let policies = policies();
    let mut seen = 0;
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies).unwrap();
        if run.policy.as_deref() != Some("read-only") {
            continue;
        }
        for call in run.tool_calls().filter(|call| call.name == "Write") {
            let decision = run
                .permission_decisions()
                .find(|decision| decision.call_id == call.call_id)
                .expect("one per call");
            assert_eq!(decision.decision, Decision::Deny);
            assert_eq!(decision.decider, Decider::HydraPolicy);
            assert!(
                decision.reason.starts_with("hydra/capability:"),
                "{}",
                decision.reason
            );
            seen += 1;
        }
    }
    // The three read-only runs ticket 02 measured each asked for one Write.
    assert!(seen >= 1, "no recorded Write denial to check against");
}

#[test]
fn no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent() {
    let policies = policies();
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies).unwrap();
        let leaks: Vec<_> = run
            .findings
            .iter()
            .filter(|finding| {
                matches!(
                    finding,
                    Finding::GateLeak { .. }
                        | Finding::SubagentLeak { .. }
                        // Every gated call on this machine has a gate answer in
                        // the stream, which is what makes the fail-open check
                        // worth having rather than noise.
                        | Finding::Ungated { .. }
                )
            })
            .collect();
        assert!(leaks.is_empty(), "{}: {leaks:?}", path.display());
    }
}

#[test]
fn normalizing_writes_nothing() {
    // Contract 11 forbids a normalized field on disk, and the cheapest check that
    // it stays that way is that reading the corpus changes no byte of it.
    let before: Vec<(PathBuf, Vec<u8>)> = corpus()
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    if before.is_empty() {
        return;
    }

    let policies = policies();
    for (path, _) in &before {
        hydra::normalize::read_file(path, &policies).unwrap();
    }

    assert_eq!(
        corpus().len(),
        before.len(),
        "normalizing added or removed a file under runs/"
    );
    for (path, bytes) in &before {
        assert_eq!(&std::fs::read(path).unwrap(), bytes, "{}", path.display());
    }
}

#[test]
fn the_recorded_shell_denial_and_the_call_beside_it_derive_apart() {
    // One `standard` run asked for two `Bash` calls: one naming `~/hydra/runs`,
    // one writing inside its own worktree. 7.5 refuses the first under every
    // policy and the second is ordinary shell, so a re-evaluation that collapsed
    // them would be caught here and nowhere else in this file.
    let policies = policies();
    let mut pairs = 0;
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies).unwrap();
        if run.policy.as_deref() != Some("standard") {
            continue;
        }
        let shell: Vec<_> = run
            .tool_calls()
            .filter(|call| call.name == "Bash")
            .collect();
        if shell.len() != 2 {
            continue;
        }
        let decision_for = |call_id: &str| {
            run.permission_decisions()
                .find(|decision| decision.call_id == call_id)
                .map(|decision| (decision.decision, decision.decider, decision.reason.clone()))
                .expect("one per call")
        };

        let (decision, decider, reason) = decision_for(&shell[0].call_id);
        assert_eq!(decision, Decision::Deny);
        assert_eq!(decider, Decider::HydraPolicy);
        assert!(reason.starts_with("hydra/protected-path:"), "{reason}");

        let (decision, decider, reason) = decision_for(&shell[1].call_id);
        assert_eq!(decision, Decision::Allow);
        assert_eq!(decider, Decider::HydraPolicy);
        assert!(reason.starts_with("hydra/allow:"), "{reason}");
        pairs += 1;
    }
    assert_eq!(
        pairs, 1,
        "the ticket 06 measurement is the only run of this shape"
    );
}

#[test]
fn a_file_with_no_run_start_says_so_rather_than_reading_as_pre_gate() {
    // A truncated file and a pre-gate run both have a null policy and no
    // decisions, and they mean opposite things, so the derivation says which one
    // it is looking at.
    let sandbox = Sandbox::new("headless");
    let frames = json!({
        "run_id": RUN_ID,
        "parent_run_id": null,
        "seq": 0,
        "ts": "2026-09-05T20:51:39.370Z",
        "provider": "claude",
        "model": null,
        "secret_flag": null,
        "raw": {"type": "assistant", "message": {"role": "assistant", "content": [
            {"type": "text", "text": "the opening line is gone"}
        ]}},
    })
    .to_string();

    let run = hydra::normalize::read_with(&frames, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.policy, None);
    assert!(run.run_start().is_none());
    assert!(
        run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::Limit(said) if said.contains("run_start"))),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_call_the_record_stops_after_is_unknown_rather_than_a_leak() {
    // Ticket 14's abandoned-run class arrives here: Hydra is killed between a
    // `tool_use` block and its result, so the policy denies a call whose fate the
    // file never records. Calling that a gate leak would invent evidence.
    let sandbox = Sandbox::new("dangling");
    let path = sandbox.worktree().join("probe.txt");
    let mut record = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    );
    record.tool_use(
        "toolu_01",
        "Write",
        json!({"file_path": path, "content": "HELLO\n"}),
    );
    // No result, no hook frame, no `run_end`.
    let text = record.unclosed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        !run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::GateLeak { .. })),
        "{:?}",
        run.findings
    );
    assert!(
        run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::Limit(said) if said.contains("toolu_01"))),
        "{:?}",
        run.findings
    );
    // The decision itself still follows the policy.
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, Decision::Deny);
    assert_eq!(decisions[0].decider, Decider::HydraPolicy);
}

#[test]
fn the_terminator_frame_is_kept_verbatim_for_the_readers_that_need_it() {
    // 8.5 keeps `permission_denials` and `subagent_stats` in `raw` rather than
    // normalizing them, and contract 8's outcome derivation reads `subtype`,
    // `is_error` and `api_error_status` off the same frame. None of that becomes
    // an event, so the frame itself is what a reader gets.
    let sandbox = Sandbox::new("terminator");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .result(json!({
            "is_error": true,
            "api_error_status": 429,
            "permission_denials": [{"tool_name": "Write", "tool_use_id": "toolu_01"}],
        }))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let result = run.result.as_ref().expect("the run reached a result frame");
    // The shape ticket 04 has to meet: success on the label, an error underneath.
    assert_eq!(result["subtype"], "success");
    assert_eq!(result["is_error"], true);
    assert_eq!(result["api_error_status"], 429);
    assert_eq!(result["permission_denials"][0]["tool_use_id"], "toolu_01");
}

#[test]
fn an_external_deny_belongs_to_the_call_it_answered() {
    // The join has no `tool_use_id` to work with, so it runs on tool name and
    // order. Without an upper bound the first of two `Bash` calls takes the deny
    // the guard wrote about the second, and both decisions come out wrong: call
    // one reads `deny`/`external` carrying call two's reason, and call two reads
    // `allow`. Conditional guards that fire on one command and not another are
    // the normal case, so this is the shape that bound exists for.
    let sandbox = Sandbox::new("two-calls");
    let mut record = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    );
    record.tool_use("toolu_01", "Bash", json!({"command": "echo one"}));
    record.hook_response(
        "Bash",
        "allow",
        "hydra/allow: policy standard allows shell.",
    );
    record.tool_result("toolu_01", "one", false);
    record.tool_use("toolu_02", "Bash", json!({"command": "echo two"}));
    record.hook_response(
        "Bash",
        "allow",
        "hydra/allow: policy standard allows shell.",
    );
    record.hook_response("Bash", "deny", "Rejected: t30 CUR-PROJ says no second echo");
    record.tool_result(
        "toolu_02",
        "Rejected: t30 CUR-PROJ says no second echo",
        true,
    );
    let text = record.closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 2);

    assert_eq!(decisions[0].call_id, "toolu_01");
    assert_eq!(decisions[0].decision, Decision::Allow);
    assert_eq!(decisions[0].decider, Decider::HydraPolicy);

    assert_eq!(decisions[1].call_id, "toolu_02");
    assert_eq!(decisions[1].decision, Decision::Deny);
    assert_eq!(decisions[1].decider, Decider::External);
    assert_eq!(
        decisions[1].reason,
        "Rejected: t30 CUR-PROJ says no second echo"
    );
}

#[test]
fn a_gated_call_the_gate_never_answered_is_a_finding() {
    // 6.6 measured a hook that fails to start failing open on this provider, and
    // `--include-hook-events` is on for every recorded run, so a call with no
    // gate answer and no refusal in Hydra's own voice is a call that reached the
    // model with nothing in front of it.
    let sandbox = Sandbox::new("ungated");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("standard"),
        Some(policies().sha()),
    )
    .tool_use("toolu_01", "Bash", json!({"command": "echo HELLO"}))
    .tool_result("toolu_01", "HELLO", false)
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings.contains(&Finding::Ungated {
            call_id: "toolu_01".into(),
            tool: "Bash".into()
        }),
        "{:?}",
        run.findings
    );
    // The policy still answers for the call.
    let decisions: Vec<_> = run.permission_decisions().collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, Decision::Allow);
}

#[test]
fn a_torn_line_costs_its_own_frames_and_nothing_else() {
    // The writer syncs per line, so a killed run can leave half a line at the
    // tail. Contract 6 calls that an abandoned run rather than a corrupt file.
    let sandbox = Sandbox::new("torn");
    let mut record = Record::opened(&sandbox.worktree(), None, None);
    record.assistant(json!([{"type": "text", "text": "this line is whole"}]));
    let mut text = record.unclosed();
    text.push_str("{\"run_id\":\"x\",\"seq\":9,\"raw\":{\"type\":\"assis");

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert_eq!(run.messages().count(), 1);
    assert!(
        run.findings.iter().any(
            |finding| matches!(finding, Finding::Limit(said) if said.contains("did not parse"))
        ),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_usage_object_with_no_counts_in_it_is_unavailable_rather_than_zero() {
    let sandbox = Sandbox::new("empty-usage");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .result(json!({"usage": {"service_tier": "standard"}}))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let usage = run.usage().expect("7.8, never a bare null");
    // Counts of zero would be indistinguishable from a provider reporting zero.
    assert_eq!(usage.source, UsageSource::Unavailable);
    assert_eq!(usage.counts, None);
}

#[test]
fn a_policy_with_no_sha_is_a_record_nothing_can_vouch_for() {
    let sandbox = Sandbox::new("no-sha");
    let text = Record::opened(&sandbox.worktree(), Some("read-only"), None)
        .assistant(json!([{"type": "text", "text": "done"}]))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    assert!(
        run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::Limit(said) if said.contains("policy_sha"))),
        "{:?}",
        run.findings
    );
}

#[test]
fn two_text_blocks_in_one_result_stay_two_blocks() {
    let sandbox = Sandbox::new("blocks");
    let text = Record::opened(&sandbox.worktree(), None, None)
        .raw(json!({
            "type": "user",
            "message": {"role": "user", "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_01",
                "is_error": false,
                "content": [
                    {"type": "text", "text": "first block"},
                    {"type": "text", "text": "second block"},
                ],
            }]},
        }))
        .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    let results: Vec<_> = run.tool_results().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].output, "first block\nsecond block");
}

#[test]
fn a_call_with_no_id_still_gets_a_decision_and_says_what_it_lost() {
    // `call_id` is the provider's to supply. Two blocks without one used to key
    // the same empty string, so the second decision overwrote the first and one
    // call ended up with none.
    let sandbox = Sandbox::new("no-id");
    let text = Record::opened(
        &sandbox.worktree(),
        Some("read-only"),
        Some(policies().sha()),
    )
    .assistant(json!([{"type": "tool_use", "name": "Read", "input": {"file_path": "/etc/hosts"}}]))
    .assistant(json!([{"type": "tool_use", "name": "Read", "input": {"file_path": "/etc/passwd"}}]))
    .closed();

    let run = hydra::normalize::read_with(&text, &policies(), &sandbox.layout()).unwrap();
    // One decision per call, which is 7.7's invariant, positionally rather than
    // by a key two calls share.
    assert_eq!(run.permission_decisions().count(), 2);
    assert_eq!(
        run.findings
            .iter()
            .filter(
                |finding| matches!(finding, Finding::Limit(said) if said.contains("tool_use_id"))
            )
            .count(),
        2
    );
}
