//! What the two record-reading suites share.
//!
//! `normalize.rs` asks what the adapter makes of a frame shape; `outcome.rs` asks
//! what an outcome derivation makes of a whole run. Both need to build a record a
//! frame at a time, and both need a layout whose scopes are directories the test
//! owns, so the builder lives here rather than twice.
//!
//! Cargo compiles this module once per test binary, and neither suite uses all of
//! it, so the whole file is exempt from the dead-code lint rather than each half
//! carrying an attribute for the other's benefit.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use serde_json::{Value, json};

pub const RUN_ID: &str = "01a07357-58a5-79e2-a25a-3df578a750b4";

/// A layout whose three scopes are directories this test owns.
///
/// The gate resolves `worktree`, `skill_store` and `corpus` against real paths,
/// so a synthetic run that points them at `~/hydra` would be asking about the
/// developer's own tree rather than about the record.
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("hydra-record-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let layout = Layout::at(&root);
        layout.ensure().unwrap();
        std::fs::create_dir_all(layout.worktree(RUN_ID)).unwrap();
        std::fs::create_dir_all(layout.skills()).unwrap();
        Self { root }
    }

    pub fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }

    pub fn worktree(&self) -> PathBuf {
        self.layout().worktree(RUN_ID)
    }

    /// One skill in a snapshot, linked into the checkout the way
    /// `hydra::skills::inject` links it. Answers with the link path, which is
    /// what the provider names to the model as the skill's base directory and
    /// therefore what a `Read` on a sibling carries (ticket 39).
    pub fn link_skill(&self, store_sha: &str, name: &str) -> PathBuf {
        let source = self.layout().snapshot(store_sha).join(name);
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "---\nname: x\n---\nbody\n").unwrap();
        std::fs::write(source.join("SIBLING.md"), "The second sentinel is\n").unwrap();
        let root = self.worktree().join(".claude/skills");
        std::fs::create_dir_all(&root).unwrap();
        let link = root.join(name);
        std::os::unix::fs::symlink(&source, &link).unwrap();
        link
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The envelope and line-0 fields a record varies, defaulted to the recorded
/// `claude` run the two record-reading suites ask about.
///
/// It exists for the corpus suite, which needs a set of records that differ in
/// exactly the fields rung one's filters key on: the kind (`provider`), the
/// launch mode beside the policy, the lineage, and the liveness pair.
pub struct Shape<'a> {
    pub run_id: &'a str,
    pub parent_run_id: Option<&'a str>,
    /// `None` is a harness run, contract 5.5.
    pub provider: Option<&'a str>,
    pub launch_mode: &'a str,
    pub command: &'a str,
    pub policy: Option<&'a str>,
    pub policy_sha: Option<&'a str>,
    /// The liveness pair of contract 5.2. The default is a pid nothing on this
    /// machine matches, so an unclosed record reads as abandoned.
    pub pid: Option<u32>,
    pub process_start_time: Option<String>,
    pub worktree: Option<&'a Path>,
    /// Contract 5.3's `skills.store_sha`, the sha the run pinned. `None` leaves
    /// the whole `skills` object off, which is what a run with no checkout
    /// writes. Contract 7.7's re-evaluation recomputes the `skill_snapshot`
    /// scope from this field and from nothing else.
    pub store_sha: Option<&'a str>,
    /// Contract 5.1's `scopes`, the paths the gate was compiled against.
    /// `None` leaves the object off, which is what every record written before
    /// build ticket 15 looks like and what most fixtures here want.
    pub scopes: Option<Value>,
    /// Contract 5.1's `floor_lifted`. `None` leaves the field off, the shape a
    /// record written before build ticket 15 has, and the reader reads that as
    /// false.
    pub floor_lifted: Option<bool>,
    /// Contract 5.1's `gate_sha`, build ticket 49. `None` leaves the field off,
    /// which is what every record written before that ticket carries and what
    /// the reader treats as a gate it cannot name. A fixture that wants a leak
    /// or a disagreement reported has to carry [`gate_sha_of`].
    pub gate_sha: Option<String>,
    /// Map ticket 49's `worktree.base_tree`, the sha `git write-tree` answered
    /// against the seed index at launch. `None` is what every record written
    /// before build ticket 23 looks like, and what a run with no worktree
    /// writes.
    pub base_tree: Option<&'a str>,
}

impl Default for Shape<'_> {
    fn default() -> Self {
        Self {
            run_id: RUN_ID,
            parent_run_id: None,
            provider: Some("claude"),
            launch_mode: "recorded",
            command: "run",
            policy: None,
            policy_sha: None,
            pid: Some(4242),
            process_start_time: Some("2026-09-05T20:51:39.369329Z".to_string()),
            worktree: None,
            store_sha: None,
            scopes: None,
            floor_lifted: None,
            gate_sha: None,
            base_tree: None,
        }
    }
}

/// The `scopes` object a launch writes, for a record a test builds by hand.
///
/// The five keys a run under this layout has a path for. `parent_git_dir` is
/// left to the caller, since a fixture rarely has a repository, and
/// `skill_checkout` rides on the policy the way `write_spawn_config` decides it.
pub fn scopes_of(layout: &Layout, run_id: &str, policy: &str, worktree: &Path) -> Value {
    let mut scopes = json!({
        "worktree": worktree.to_string_lossy(),
        "skill_store": layout.skills().to_string_lossy(),
        "corpus": layout.runs().to_string_lossy(),
        "gate_script": layout.gate_script(run_id).to_string_lossy(),
    });
    if policy == "curate" {
        scopes["skill_checkout"] = json!(layout.skill_checkout(run_id).to_string_lossy());
    }
    scopes
}

/// The `gate_sha` a launch under this layout would write, build ticket 49.
///
/// Contract 7.7's re-evaluation compiles the gate for the spawn it rebuilds off
/// the record and compares. A fixture that wants a leak or a disagreement
/// reported has to carry the sha of the gate this build compiles, so the paths
/// here are the ones [`scopes_of`] writes, read back the same way the replay
/// reads them. Pass a different string to build a record from a foreign gate.
pub fn gate_sha_of(
    layout: &Layout,
    run_id: &str,
    policy: &str,
    provider: hydra::provider::Provider,
    worktree: &Path,
) -> String {
    let checkout = (policy == "curate").then(|| layout.skill_checkout(run_id));
    hydra::policy::gate_sha(
        &hydra::policy::PolicySet::discover().expect("the shipped policy set loads"),
        &hydra::policy::Spawn {
            provider,
            policy,
            layout,
            worktree,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            // `scopes_of` writes no `skills` object, so the replay recomputes
            // this scope from a store sha that is not there.
            skill_snapshot: &layout.skill_snapshot_scope(run_id, None),
            skill_checkout: checkout.as_deref(),
            // `scopes_of` carries no `parent_git_dir`, and a record that carries
            // a `scopes` object answers the key by leaving it out.
            parent_git_dir: None,
            gate_script: &layout.gate_script(run_id),
            floor_lifted: false,
        },
    )
    .expect("the gate compiles for a fixture's spawn")
}

/// Every finding but the one a record with no `scopes` on it earns.
///
/// Build ticket 15 made the reader say out loud that it rebuilt the paths a
/// decision resolves against, and no fixture here carries `scopes` unless it
/// says so, which is what every record written before that ticket looks like.
/// A test asking whether a call raised anything wants the rest of the list.
pub fn other_findings(run: &hydra::normalize::Run) -> Vec<&hydra::normalize::Finding> {
    run.findings
        .iter()
        .filter(|finding| !rebuilt_the_scopes(finding) && !named_its_bar(finding))
        .collect()
}

/// Whether this finding is the reader saying which bar its counts are held to.
///
/// Build ticket 55 and contract 7.7's table. Every `cursor-agent` and `agy`
/// record carries one, because on those two a leak count is a floor, and a test
/// asking whether a call raised anything wants the rest of the list.
pub fn named_its_bar(finding: &hydra::normalize::Finding) -> bool {
    matches!(finding, hydra::normalize::Finding::Limit(said) if said.contains("floor rather than")
        || said.contains("floors rather than totals"))
}

/// Whether this finding is the reader saying it rebuilt the run's scopes.
pub fn rebuilt_the_scopes(finding: &hydra::normalize::Finding) -> bool {
    matches!(finding, hydra::normalize::Finding::Limit(said) if said.contains("carries no scopes"))
}

/// One synthetic run file, built a frame at a time.
pub struct Record {
    seq: u64,
    lines: Vec<String>,
    run_id: String,
    parent_run_id: Option<String>,
    provider: Option<String>,
}

impl Record {
    /// Opens with a `run_start` under the named policy, `None` for a pre-gate run.
    ///
    /// No `scopes`, which is what every record on this machine looks like.
    pub fn opened(worktree: &Path, policy: Option<&str>, policy_sha: Option<&str>) -> Self {
        Self::shaped(Shape {
            policy,
            policy_sha,
            worktree: Some(worktree),
            ..Shape::default()
        })
    }

    /// A record this build would have written: the `scopes` object beside the
    /// policy, so a replay resolves against the paths the gate really used.
    ///
    /// **A leak and a disagreement are only reported on a record like this
    /// one.** Rebuild a path and the derivation answered about another gate, so
    /// build ticket 15 downgrades both to a limit. A test that wants either has
    /// to say which paths the gate had.
    pub fn gated(sandbox: &Sandbox, provider: &str, policy: &str, policy_sha: &str) -> Self {
        let worktree = sandbox.worktree();
        let layout = sandbox.layout();
        Self::shaped(Shape {
            provider: Some(provider),
            policy: Some(policy),
            policy_sha: Some(policy_sha),
            worktree: Some(&worktree),
            scopes: Some(scopes_of(&layout, RUN_ID, policy, &worktree)),
            // Build ticket 49: a leak and a disagreement are reported only on a
            // record that names the gate this build compiles, so the helper
            // whose whole point is a reportable finding names it.
            gate_sha: Some(gate_sha_of(
                &layout,
                RUN_ID,
                policy,
                provider.parse().expect("a fixture names a routed provider"),
                &worktree,
            )),
            ..Shape::default()
        })
    }

    /// The same, with every envelope field the caller's.
    pub fn shaped(shape: Shape<'_>) -> Self {
        let mut record = Self {
            seq: 0,
            lines: Vec::new(),
            run_id: shape.run_id.to_string(),
            parent_run_id: shape.parent_run_id.map(str::to_string),
            provider: shape.provider.map(str::to_string),
        };
        let worktree = match shape.worktree {
            Some(path) => json!({
                "path": path.to_string_lossy(),
                "branch": format!("hydra/{}", shape.run_id),
                "base_tree": shape.base_tree,
            }),
            None => Value::Null,
        };
        let start = json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": shape.command,
            "launch_mode": shape.launch_mode,
            "provider_version": "2.1.261 (Claude Code)",
            "provider_session_id": null,
            "task": "probe",
            "policy": shape.policy,
            "policy_sha": shape.policy_sha,
            "cwd": shape
                .worktree
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| "/tmp".to_string()),
            "worktree": worktree,
            "depth": 0,
            "pid": shape.pid,
            "process_start_time": shape.process_start_time,
            "subscription": "anthropic",
        });
        // Contract 5.3 and 5.5: absent when the question does not apply, never
        // null. `check_run_start` flags a null group, so this key exists only
        // when a sha was pinned, which is what `src/run.rs` writes.
        let mut start = start;
        if let Some(sha) = shape.store_sha {
            start["skills"] = json!({ "store_sha": sha, "linked": [], "resolved": null });
        }
        // Contract 5.1's two fields from build ticket 15. Both are absent by
        // default, because that is what every record written before it carries
        // and what the reader has to keep reading.
        if let Some(scopes) = shape.scopes {
            start["scopes"] = scopes;
        }
        if let Some(lifted) = shape.floor_lifted {
            start["floor_lifted"] = json!(lifted);
        }
        if let Some(sha) = shape.gate_sha {
            start["gate_sha"] = json!(sha);
        }
        record.line("hydra", start);
        record
    }

    pub fn line(&mut self, field: &str, body: Value) {
        let mut envelope = json!({
            "run_id": self.run_id,
            "parent_run_id": self.parent_run_id,
            "seq": self.seq,
            "ts": "2026-09-05T20:51:39.370Z",
            "provider": self.provider,
            "model": null,
            "secret_flag": null,
        });
        envelope[field] = body;
        self.lines.push(envelope.to_string());
        self.seq += 1;
    }

    pub fn raw(&mut self, frame: Value) -> &mut Self {
        self.line("raw", frame);
        self
    }

    /// A line the provider wrote that is not JSON, kept as a string (contract 3).
    pub fn noise(&mut self, text: &str) -> &mut Self {
        self.line("raw", Value::String(text.into()));
        self
    }

    pub fn assistant(&mut self, blocks: Value) -> &mut Self {
        self.raw(json!({
            "type": "assistant",
            "message": {"id": "msg_01", "role": "assistant", "content": blocks},
        }))
    }

    pub fn tool_use(&mut self, id: &str, name: &str, input: Value) -> &mut Self {
        self.assistant(json!([{"type": "tool_use", "id": id, "name": name, "input": input}]))
    }

    pub fn tool_result(&mut self, id: &str, content: &str, is_error: bool) -> &mut Self {
        self.raw(json!({
            "type": "user",
            "message": {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": id, "content": content, "is_error": is_error}
            ]},
        }))
    }

    /// A `PreToolUse` response in the shape `--include-hook-events` writes,
    /// under the fixed `hook_id` every fixture but a ticket 22 test shares.
    pub fn hook_response(&mut self, tool: &str, decision: &str, reason: &str) -> &mut Self {
        self.hook_response_id(
            tool,
            decision,
            reason,
            "4f041c03-db30-4ae0-96dd-be3022a64b1e",
        )
    }

    /// The same, under the `hook_id` the caller names, for a test that joins a
    /// response to the `hook_started` frame it answers.
    pub fn hook_response_id(
        &mut self,
        tool: &str,
        decision: &str,
        reason: &str,
        hook_id: &str,
    ) -> &mut Self {
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
            "hook_id": hook_id,
            "hook_name": format!("PreToolUse:{tool}"),
            "hook_event": "PreToolUse",
            "output": output,
            "exit_code": 0,
            "outcome": "success",
        }))
    }

    /// A `PreToolUse` hook starting, the frame `--include-hook-events` writes
    /// before the matching `hook_response`, sharing its `hook_id`.
    pub fn hook_started(&mut self, tool: &str, hook_id: &str) -> &mut Self {
        self.raw(json!({
            "type": "system",
            "subtype": "hook_started",
            "hook_id": hook_id,
            "hook_name": format!("PreToolUse:{tool}"),
            "hook_event": "PreToolUse",
        }))
    }

    pub fn init(&mut self, tools: &[&str]) -> &mut Self {
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

    pub fn result(&mut self, extra: Value) -> &mut Self {
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

    // --- cursor-agent frames, measured on 2026.09.02-c22c1a3, 2026-09-06 ----

    /// A record whose envelope names `cursor-agent`, opened under a policy.
    pub fn cursor(worktree: &Path, policy: Option<&str>, policy_sha: Option<&str>) -> Self {
        Self::shaped(Shape {
            provider: Some("cursor-agent"),
            policy,
            policy_sha,
            worktree: Some(worktree),
            ..Shape::default()
        })
    }

    /// `system/init` as this provider writes it: a display name, no tool list.
    pub fn cursor_init(&mut self) -> &mut Self {
        self.raw(json!({
            "type": "system",
            "subtype": "init",
            "apiKeySource": "login",
            "cwd": "/tmp/ws",
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "model": "Composer 2.5",
            "permissionMode": "default",
        }))
    }

    pub fn cursor_user(&mut self, text: &str) -> &mut Self {
        self.raw(json!({
            "type": "user",
            "message": {"role": "user", "content": [{"type": "text", "text": text}]},
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
        }))
    }

    pub fn cursor_assistant(&mut self, text: &str) -> &mut Self {
        self.raw(json!({
            "type": "assistant",
            "message": {"role": "assistant", "content": [{"type": "text", "text": text}]},
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
        }))
    }

    /// One thinking block: its deltas, then the `completed` marker.
    pub fn cursor_thinking(&mut self, deltas: &[&str]) -> &mut Self {
        for delta in deltas {
            self.raw(json!({
                "type": "thinking", "subtype": "delta", "text": delta,
                "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
                "timestamp_ms": 1788720518000u64,
            }));
        }
        self.raw(json!({
            "type": "thinking", "subtype": "completed",
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "timestamp_ms": 1788720518001u64,
        }))
    }

    /// A `tool_call` frame. `kind` is the oneof key, `readToolCall` and the
    /// like; `result` is `None` on the `started` frame and the oneof result
    /// object on the `completed` one.
    pub fn cursor_tool(
        &mut self,
        call_id: &str,
        kind: &str,
        args: Value,
        result: Option<Value>,
    ) -> &mut Self {
        let mut inner = json!({ "args": args });
        if let Some(result) = &result {
            inner["result"] = result.clone();
        }
        let mut object = json!({
            "hookAdditionalContexts": [],
            "toolCallId": call_id,
            "startedAtMs": "1788720518123",
        });
        object[kind] = inner;
        let subtype = if result.is_some() {
            "completed"
        } else {
            "started"
        };
        if result.is_some() {
            object["completedAtMs"] = json!("1788720518321");
        }
        self.raw(json!({
            "type": "tool_call",
            "subtype": subtype,
            "call_id": call_id,
            "tool_call": object,
            "model_call_id": "fc_1",
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "timestamp_ms": 1788720518123u64,
        }))
    }

    /// `started` then `completed` for one call, which is how every call the
    /// probe made arrived.
    pub fn cursor_call(
        &mut self,
        call_id: &str,
        kind: &str,
        args: Value,
        result: Value,
    ) -> &mut Self {
        self.cursor_tool(call_id, kind, args.clone(), None);
        self.cursor_tool(call_id, kind, args, Some(result))
    }

    /// The terminator, with or without a usage block.
    pub fn cursor_result(&mut self, text: &str, usage: Option<Value>) -> &mut Self {
        let mut frame = json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 12015,
            "duration_api_ms": 12015,
            "is_error": false,
            "result": text,
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "request_id": "19bf8e59-0aff-41c9-bef0-1e82a660d286",
        });
        if let Some(usage) = usage {
            frame["usage"] = usage;
        }
        self.raw(frame)
    }

    // --- agy frames, measured on 1.1.26, 2026-09-06 -------------------------

    /// A record whose envelope names `agy`, opened under a policy.
    pub fn agy(worktree: &Path, policy: Option<&str>, policy_sha: Option<&str>) -> Self {
        Self::shaped(Shape {
            provider: Some("agy"),
            policy,
            policy_sha,
            worktree: Some(worktree),
            ..Shape::default()
        })
    }

    /// The `init` frame: the model, the cwd and the whole tool list, which is
    /// what 7.1's table check reads on this provider.
    pub fn agy_init(&mut self, tools: &[&str]) -> &mut Self {
        self.raw(json!({
            "event": "init",
            "conversation_id": AGY_CONVERSATION,
            "init": {
                "model": "gemini-3.8-flash-low",
                "cwd": "/tmp/ws",
                "tools": tools,
                "permission_mode": "request-review",
            },
        }))
    }

    /// One `step_update`, the frame that carries nearly everything.
    pub fn agy_step(&mut self, step: u64, state: &str, step_type: &str, extra: Value) -> &mut Self {
        let mut update = json!({
            "conversation_id": AGY_CONVERSATION,
            "step_index": step,
            "state": state,
            "step_type": step_type,
        });
        for (key, value) in extra.as_object().unwrap() {
            update[key] = value.clone();
        }
        self.raw(json!({ "event": "step_update", "step_update": update }))
    }

    /// The echoed prompt, which arrives as one `DONE` step with no text.
    pub fn agy_user(&mut self) -> &mut Self {
        self.agy_step(0, "DONE", "user_input", json!({}))
    }

    /// One `agent_response` step: its `text_delta` frames, then the `DONE`
    /// frame that carries the step's usage. Contract 7.3's concatenation runs
    /// over exactly this.
    pub fn agy_response(&mut self, step: u64, deltas: &[&str], usage: Option<Value>) -> &mut Self {
        let (last, rest) = match deltas.split_last() {
            Some(split) => split,
            None => (&"", &[] as &[&str]),
        };
        for delta in rest {
            self.agy_step(
                step,
                "ACTIVE",
                "agent_response",
                json!({ "text_delta": delta }),
            );
        }
        let mut done = json!({ "duration_seconds": 2.74 });
        if !last.is_empty() {
            done["text_delta"] = json!(last);
        }
        if let Some(usage) = usage {
            done["usage"] = usage;
        }
        self.agy_step(step, "DONE", "agent_response", done)
    }

    /// One tool step: the `ACTIVE` frame is the call and the terminal frame is
    /// the result, joined on the step index because there is no id.
    pub fn agy_tool(
        &mut self,
        step: u64,
        name: &str,
        parameters: Value,
        state: &str,
        outcome: Value,
    ) -> &mut Self {
        self.agy_step(
            step,
            "ACTIVE",
            "tool",
            json!({
                "tool_name": name,
                "tool_info": { "name": name, "parameters": parameters.clone() },
            }),
        );
        let mut info = json!({ "name": name, "parameters": parameters });
        for (key, value) in outcome.as_object().unwrap() {
            info[key] = value.clone();
        }
        self.agy_step(
            step,
            state,
            "tool",
            json!({
                "tool_name": name,
                "duration_seconds": 0.49,
                "tool_info": info,
            }),
        )
    }

    /// A tool step that succeeded, with the output the provider reported.
    pub fn agy_tool_ok(
        &mut self,
        step: u64,
        name: &str,
        parameters: Value,
        output: &str,
    ) -> &mut Self {
        self.agy_tool(step, name, parameters, "DONE", json!({ "output": output }))
    }

    /// A tool step that failed, with the `TOOL_ERROR` this provider gives a
    /// refusal and a missing file alike.
    pub fn agy_tool_error(
        &mut self,
        step: u64,
        name: &str,
        parameters: Value,
        message: &str,
    ) -> &mut Self {
        self.agy_tool(
            step,
            name,
            parameters,
            "ERROR",
            json!({ "error": { "type": "TOOL_ERROR", "message": message } }),
        )
    }

    /// The terminator, which this provider writes on every path.
    ///
    /// `status` is `SUCCESS` even on a run whose every tool call was denied, and
    /// `usage` here is the conversation's rather than the run's (9.2), so the
    /// adapter records it and sums the steps instead.
    pub fn agy_result(&mut self, status: &str, response: &str, usage: Option<Value>) -> &mut Self {
        let mut result = json!({
            "conversation_id": AGY_CONVERSATION,
            "status": status,
            "response": response,
            "duration_seconds": 8.57,
            "num_turns": 1,
        });
        if let Some(usage) = usage {
            result["usage"] = usage;
        }
        self.raw(json!({ "event": "result", "result": result }))
    }

    pub fn closed(&mut self) -> String {
        self.closed_with(json!({"exit_code": 0}))
    }

    /// The same closing line with fields overridden, for a run that did not end
    /// cleanly. `exit_code`, `error` and `refusal` are the three an outcome
    /// derivation reads.
    pub fn closed_with(&mut self, extra: Value) -> String {
        let mut end = json!({
            "type": "run_end",
            "exit_code": 0,
            "error": null,
            "refusal": null,
            "remediation": null,
            "body_source": "stream",
            "ended_at": "2026-09-05T20:52:01.905Z",
        });
        for (key, value) in extra.as_object().unwrap() {
            end[key] = value.clone();
        }
        self.line("hydra", end);
        self.text()
    }

    /// The file so far, with no closing line. A run whose `run_end` nothing
    /// wrote, which is contract 6's "not guaranteed to exist" made real.
    pub fn unclosed(&mut self) -> String {
        self.text()
    }

    fn text(&self) -> String {
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }
}

pub fn policies() -> PolicySet {
    PolicySet::discover().unwrap()
}

pub fn usage_frame() -> Value {
    json!({"usage": {
        "input_tokens": 4,
        "cache_creation_input_tokens": 15110,
        "cache_read_input_tokens": 30266,
        "output_tokens": 1270,
        "output_tokens_details": {"thinking_tokens": 872},
    }})
}

/// The usage block the probe's `result` frame carried on 2026-09-06.
/// The conversation id every `agy` frame carries, off the measured run.
pub const AGY_CONVERSATION: &str = "b631e11b-4a99-43db-9f37-11a373cdd98a";

/// One `agent_response` step's usage, in this provider's five fields.
///
/// `cache_write_tokens` is not among them: section 9's table marks it absent on
/// this provider, and the adapter leaves it absent rather than zeroing it.
pub fn agy_usage(input: u64, output: u64, cache_read: u64, thinking: u64) -> Value {
    json!({
        "input_tokens": input,
        "output_tokens": output,
        "thinking_tokens": thinking,
        "cache_read_tokens": cache_read,
        "total_tokens": input + output,
    })
}

pub fn cursor_usage() -> Value {
    json!({
        "inputTokens": 8383,
        "outputTokens": 731,
        "cacheReadTokens": 109307,
        "cacheWriteTokens": 0,
    })
}

/// A read result body, success case, as measured.
pub fn cursor_read_ok(path: &str, content: &str) -> Value {
    json!({ "success": {
        "content": content, "isEmpty": false, "exceededLimit": false,
        "totalLines": 2, "fileSize": content.len(), "path": path,
        "readRange": {"startLine": 1, "endLine": 2},
        "relatedCursorRulePaths": [], "relatedCursorRules": [],
    }})
}

/// A read refused at `preToolUse`, as run `01a07892-459d-7420-904e-91d6a6986159`
/// recorded it on 2026-09-06: an `error` case whose message is the hook's own
/// words with Cursor's marker after them and nothing in front.
pub fn cursor_read_blocked(reason: &str) -> Value {
    json!({ "error": { "errorMessage": format!(
        "{reason}\n\nAgent note: Do not suggest workarounds to the blocked tool."
    )}})
}

/// The same read refused at `beforeReadFile`, as the second probe measured on
/// the same build and day: Cursor puts `File read was blocked by a hook: ` in
/// front and its settings hint between.
pub fn cursor_read_blocked_late(reason: &str) -> Value {
    json!({ "error": { "errorMessage": format!(
        "File read was blocked by a hook: {reason}\n\nTo view or modify configured \
         hooks, go to Cursor Settings > Hooks.\n\nAgent note: Do not suggest \
         workarounds to the blocked tool."
    )}})
}

/// A shell command refused by a hook, research 30's shape.
pub fn cursor_shell_rejected(command: &str, reason: &str) -> Value {
    json!({ "rejected": {
        "command": command, "workingDirectory": "",
        "reason": format!(
            "Command execution was blocked by a hook: {reason}\n\nTo view or modify \
             configured hooks, go to Cursor Settings > Hooks.\n\nAgent note: Do not \
             suggest workarounds to the blocked tool."
        ),
        "isReadonly": false,
    }})
}

/// The `runs/` directory corpus walks read.
///
/// When `HYDRA_ROOT` is set, walks `$HYDRA_ROOT/runs`. When it is unset, walks
/// `~/hydra/runs`, where `~` is the real home from the passwd entry, not `HOME`.
/// Returns `None` only when `HYDRA_ROOT` is unset and the real home cannot be
/// located.
pub fn corpus_runs_dir() -> Option<PathBuf> {
    let root = if let Some(root) = std::env::var_os("HYDRA_ROOT") {
        PathBuf::from(root)
    } else {
        match hydra::env::real_home() {
            Ok(home) => home.join("hydra"),
            Err(_) => return None,
        }
    };
    Some(root.join("runs"))
}

/// The corpus, or `None` when this machine has no records.
///
/// A walk that checks every record holds on an empty corpus and says nothing.
/// A test that measures one particular record does not: it asserts the record
/// is there, and on a machine that has run nothing that assertion is about the
/// machine rather than about the code. CI is exactly that machine, because
/// `.github/workflows/ci.yml` points `HYDRA_ROOT` at an empty scratch root, so
/// four tests went red the day the walks started honouring the variable (build
/// ticket 51). Returning early is what the walks already do.
pub fn corpus_or_skip() -> Option<Vec<PathBuf>> {
    let paths = corpus();
    (!paths.is_empty()).then_some(paths)
}

/// Every `.jsonl` under the corpus `runs/` directory, or nothing when the
/// directory is missing or empty. Same shape as the contract suite's walk.
pub fn corpus() -> Vec<PathBuf> {
    let Some(runs) = corpus_runs_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(runs) else {
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
