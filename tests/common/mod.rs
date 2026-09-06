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

/// One synthetic run file, built a frame at a time.
pub struct Record {
    seq: u64,
    lines: Vec<String>,
}

impl Record {
    /// Opens with a `run_start` under the named policy, `None` for a pre-gate run.
    pub fn opened(worktree: &Path, policy: Option<&str>, policy_sha: Option<&str>) -> Self {
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

    pub fn line(&mut self, field: &str, body: Value) {
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
