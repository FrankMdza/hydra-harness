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

use std::path::{Path, PathBuf};

use hydra::normalize::{Decider, Decision, Event, Finding, ResultStatus, Role, UsageSource};
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use serde_json::{Value, json};

const RUN_ID: &str = "01a07357-58a5-79e2-a25a-3df578a750b4";

/// A layout whose three scopes are directories this test owns.
///
/// The gate resolves `worktree`, `skill_store` and `corpus` against real paths,
/// so a synthetic run that points them at `~/hydra` would be asking about the
/// developer's own tree rather than about the record.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("hydra-normalize-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let layout = Layout::at(&root);
        layout.ensure().unwrap();
        std::fs::create_dir_all(layout.worktree(RUN_ID)).unwrap();
        std::fs::create_dir_all(layout.skills()).unwrap();
        Self { root }
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }

    fn worktree(&self) -> PathBuf {
        self.layout().worktree(RUN_ID)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// One synthetic run file, built a frame at a time.
struct Record {
    seq: u64,
    lines: Vec<String>,
}

impl Record {
    /// Opens with a `run_start` under the named policy, `None` for a pre-gate run.
    fn opened(worktree: &Path, policy: Option<&str>, policy_sha: Option<&str>) -> Self {
        let mut record = Self {
            seq: 0,
            lines: Vec::new(),
        };
        let start = json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "2.1.261 (Claude Code)",
            "provider_session_id": null,
            "task": "probe",
            "policy": policy,
            "policy_sha": policy_sha,
            "cwd": worktree.to_string_lossy(),
            "worktree": {
                "path": worktree.to_string_lossy(),
                "branch": format!("hydra/{RUN_ID}"),
            },
            "depth": 0,
            "pid": 4242,
            "process_start_time": "2026-09-05T20:51:39.369329Z",
            "subscription": "anthropic",
        });
        record.line("hydra", start);
        record
    }

    fn line(&mut self, field: &str, body: Value) {
        let mut envelope = json!({
            "run_id": RUN_ID,
            "parent_run_id": null,
            "seq": self.seq,
            "ts": "2026-09-05T20:51:39.370Z",
            "provider": "claude",
            "model": null,
            "secret_flag": null,
        });
        envelope[field] = body;
        self.lines.push(envelope.to_string());
        self.seq += 1;
    }

    fn raw(&mut self, frame: Value) -> &mut Self {
        self.line("raw", frame);
        self
    }

    /// A line the provider wrote that is not JSON, kept as a string (contract 3).
    fn noise(&mut self, text: &str) -> &mut Self {
        self.line("raw", Value::String(text.into()));
        self
    }

    fn assistant(&mut self, blocks: Value) -> &mut Self {
        self.raw(json!({
            "type": "assistant",
            "message": {"id": "msg_01", "role": "assistant", "content": blocks},
        }))
    }

    fn tool_use(&mut self, id: &str, name: &str, input: Value) -> &mut Self {
        self.assistant(json!([{"type": "tool_use", "id": id, "name": name, "input": input}]))
    }

    fn tool_result(&mut self, id: &str, content: &str, is_error: bool) -> &mut Self {
        self.raw(json!({
            "type": "user",
            "message": {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": id, "content": content, "is_error": is_error}
            ]},
        }))
    }

    /// A `PreToolUse` response in the shape `--include-hook-events` writes.
    fn hook_response(&mut self, tool: &str, decision: &str, reason: &str) -> &mut Self {
        let output = json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": decision,
                "permissionDecisionReason": reason,
            }
        })
        .to_string();
        self.raw(json!({
            "type": "system",
            "subtype": "hook_response",
            "hook_id": "4f041c03-db30-4ae0-96dd-be3022a64b1e",
            "hook_name": format!("PreToolUse:{tool}"),
            "hook_event": "PreToolUse",
            "output": output,
            "exit_code": 0,
            "outcome": "success",
        }))
    }

    fn init(&mut self, tools: &[&str]) -> &mut Self {
        self.raw(json!({
            "type": "system",
            "subtype": "init",
            "session_id": "3c9b2699-9e74-4bf1-8dda-8c7ddc66ff09",
            "tools": tools,
            "mcp_servers": [],
            "model": "claude-opus-5[1m]",
            "permissionMode": "bypassPermissions",
        }))
    }

    fn result(&mut self, extra: Value) -> &mut Self {
        let mut frame = json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "num_turns": 2,
            "session_id": "3c9b2699-9e74-4bf1-8dda-8c7ddc66ff09",
        });
        for (key, value) in extra.as_object().unwrap() {
            frame[key] = value.clone();
        }
        self.raw(frame)
    }

    fn closed(&mut self) -> String {
        self.line(
            "hydra",
            json!({
                "type": "run_end",
                "exit_code": 0,
                "error": null,
                "refusal": null,
                "remediation": null,
                "body_source": "stream",
                "ended_at": "2026-09-05T20:52:01.905Z",
            }),
        );
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }
}

fn policies() -> PolicySet {
    PolicySet::discover().unwrap()
}

fn usage_frame() -> Value {
    json!({"usage": {
        "input_tokens": 4,
        "cache_creation_input_tokens": 15110,
        "cache_read_input_tokens": 30266,
        "output_tokens": 1270,
        "output_tokens_details": {"thinking_tokens": 872},
    }})
}

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

// --- the corpus ----------------------------------------------------------

/// Every `.jsonl` under `~/hydra/runs/`, or nothing on a machine that has run
/// nothing. Same shape as the contract suite's walk.
fn corpus() -> Vec<PathBuf> {
    let Ok(home) = hydra::env::real_home() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(home.join("hydra").join("runs")) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn every_record_on_this_machine_normalizes() {
    let policies = policies();
    let mut gated = 0;
    let mut pre_gate = 0;
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies)
            .unwrap_or_else(|err| panic!("{} did not normalize: {err:#}", path.display()));
        assert_eq!(run.provider.as_deref(), Some("claude"));
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
fn no_record_on_this_machine_reports_a_gate_leak() {
    let policies = policies();
    for path in corpus() {
        let run = hydra::normalize::read_file(&path, &policies).unwrap();
        let leaks: Vec<_> = run
            .findings
            .iter()
            .filter(|finding| {
                matches!(
                    finding,
                    Finding::GateLeak { .. } | Finding::SubagentLeak { .. }
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
    let text = record.lines.join("\n");

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
