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
        }
    }
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
    pub fn opened(worktree: &Path, policy: Option<&str>, policy_sha: Option<&str>) -> Self {
        Self::shaped(Shape {
            policy,
            policy_sha,
            worktree: Some(worktree),
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

    /// A `PreToolUse` response in the shape `--include-hook-events` writes.
    pub fn hook_response(&mut self, tool: &str, decision: &str, reason: &str) -> &mut Self {
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
    pub fn cursor_tool(&mut self, call_id: &str, kind: &str, args: Value, result: Option<Value>) -> &mut Self {
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
        let subtype = if result.is_some() { "completed" } else { "started" };
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
    pub fn cursor_call(&mut self, call_id: &str, kind: &str, args: Value, result: Value) -> &mut Self {
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

/// A read refused by a hook, as measured on the second probe: an `error`
/// case whose message carries every hook's words and Cursor's own marker.
pub fn cursor_read_blocked(reason: &str) -> Value {
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

/// Every `.jsonl` under `~/hydra/runs/`, or nothing on a machine that has run
/// nothing. Same shape as the contract suite's walk.
pub fn corpus() -> Vec<PathBuf> {
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
