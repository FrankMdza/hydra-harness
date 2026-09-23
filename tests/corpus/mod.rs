//! Shared rig for the attack corpus: compile a gate into a scratch layout, feed
//! it a payload, read the answer back, and run a command under a real seatbelt
//! profile. `tests/gate.rs` and `tests/spawn.rs` import this module; future
//! `tests/corpus_*.rs` files will too.
//!
//! A class enters the corpus when it has a primary source, a probe that runs
//! with no provider and no path outside a scratch directory, a row in
//! `README.md`, and either a mechanism with a file and line or a ticket number.

#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::paths::Layout;
use hydra::policy::{PolicySet, Spawn, write_gate};
use hydra::provider::Provider;
use serde_json::{Value, json};

/// What the gate script's shebang names, and what every probe runs it on.
pub const SHEBANG_PYTHON: &str = "/usr/bin/python3";

/// The run every fixture path is keyed by.
pub const RUN: &str = "own";

/// A sibling run, whose worktree, spawn directory and store checkout this run
/// owns nothing of. Half the deny list of 7.5 exists to keep these apart.
pub const OTHER: &str = "other";

pub struct Decision {
    pub allowed: bool,
    pub reason: String,
    pub exit: i32,
}

impl Decision {
    pub fn denied_for(&self, code: &str) -> bool {
        !self.allowed && self.reason.contains(code)
    }
}

/// A compiled gate plus the directories its scopes point at.
///
/// Every path comes off a [`Layout`] rooted at `root`, so a probe naming
/// `worktrees/other` names what a real sibling run would hold and the deny list
/// of 7.5 is checked against the layout it was written for.
pub struct Gate {
    pub root: PathBuf,
    pub script: PathBuf,
    pub worktree: PathBuf,
    pub runs: PathBuf,
    pub skills: PathBuf,
    /// The run's own snapshot of the store, `~/hydra/skills-snapshots/<sha>/` on
    /// a real launch. The links inside the worktree point here.
    pub snapshot: PathBuf,
    /// The `curate` pass's own checkout of the store, and `None` under every
    /// other policy, which is what makes the carve-out of 7.5 per spawn.
    pub checkout: Option<PathBuf>,
    /// The parent repo's `.git`, which a real launch knows because it ran
    /// `git worktree add` from it.
    pub parent_git: PathBuf,
}

impl Gate {
    pub fn compile(policy: &str, floor_lifted: bool) -> Self {
        Self::compile_for(Provider::Claude, policy, floor_lifted)
    }

    pub fn compile_for(provider: Provider, policy: &str, floor_lifted: bool) -> Self {
        Self::build(provider, policy, floor_lifted, true)
    }

    /// The gate a run outside a repo gets. It has no checkout, so `hydra run`
    /// points its `worktree` scope at `spawn/<run-id>/no-worktree`, a path
    /// nothing creates and nothing carves out of the denials.
    pub fn without_a_checkout(policy: &str) -> Self {
        Self::build(Provider::Claude, policy, false, false)
    }

    pub fn build(provider: Provider, policy: &str, floor_lifted: bool, has_checkout: bool) -> Self {
        // A root per Gate, never per policy: the suite runs in parallel and two
        // tests on the same policy would otherwise share a directory, and the
        // first to drop would delete the other's script.
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "hydra-gate-{}-{}-{}",
            std::process::id(),
            policy,
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self::build_at(provider, policy, floor_lifted, has_checkout, root)
    }

    /// A gate rooted at `root` rather than a fresh temp directory. The caller
    /// owns the path and must pick one unique per parallel test.
    pub fn build_at(
        provider: Provider,
        policy: &str,
        floor_lifted: bool,
        has_checkout: bool,
        root: PathBuf,
    ) -> Self {
        let _ = std::fs::remove_dir_all(&root);
        let layout = Layout::at(&root);
        let worktree = if has_checkout {
            layout.worktree(RUN)
        } else {
            layout.spawn(RUN).join("no-worktree")
        };
        let runs = layout.runs();
        let skills = layout.skills();
        let snapshot = layout.snapshot("deadbeef");
        let script = layout.gate_script(RUN);
        // Only a `curate` pass gets one, which is the whole of the difference
        // between the last two rows of the matrix in ticket 40.
        let checkout = (policy == "curate").then(|| layout.skill_checkout(RUN));
        let parent_git = root.join("parent/.git");
        for dir in [&runs, &skills, &snapshot, &layout.spawn(RUN), &parent_git] {
            std::fs::create_dir_all(dir).unwrap();
        }
        // The placeholder is a path nothing creates, so a gate without a
        // checkout gets none made for it either.
        if has_checkout {
            std::fs::create_dir_all(&worktree).unwrap();
        }
        if let Some(checkout) = &checkout {
            std::fs::create_dir_all(checkout).unwrap();
        }

        let set = PolicySet::discover().unwrap();
        write_gate(
            &set,
            &Spawn {
                provider,
                policy,
                layout: &layout,
                worktree: &worktree,
                skill_store: &skills,
                corpus: &runs,
                skill_snapshot: &snapshot,
                skill_checkout: checkout.as_deref(),
                parent_git_dir: Some(&parent_git),
                gate_script: &script,
                floor_lifted,
            },
        )
        .unwrap();

        Self {
            root,
            script,
            worktree,
            runs,
            skills,
            snapshot,
            checkout,
            parent_git,
        }
    }

    /// The layout every path above came off, for a probe that needs one this
    /// struct does not hold.
    pub fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }

    /// One skill in the snapshot, linked into the checkout the way `inject`
    /// links it: `<worktree>/.claude/skills/<name>` pointing at the snapshot
    /// directory. Answers with the link path, which is the path the provider
    /// names to the model as the skill's base directory (ticket 39).
    pub fn link_skill(&self, name: &str) -> PathBuf {
        let source = self.snapshot.join(name);
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "---\nname: x\n---\nbody\n").unwrap();
        std::fs::write(source.join("reference.md"), "a sibling file\n").unwrap();
        let root = self.worktree.join(".claude/skills");
        std::fs::create_dir_all(&root).unwrap();
        let link = root.join(name);
        std::os::unix::fs::symlink(&source, &link).unwrap();
        link
    }

    pub fn new(policy: &str) -> Self {
        Self::compile(policy, false)
    }

    /// A gate compiled for `cursor-agent`, which answers in Cursor's own shape.
    pub fn cursor(policy: &str) -> Self {
        Self::compile_for(Provider::CursorAgent, policy, false)
    }

    /// A gate compiled for `agy`, which answers `decision` plus `reason`.
    pub fn agy(policy: &str) -> Self {
        Self::compile_for(Provider::Agy, policy, false)
    }

    /// One skill linked into `agy`'s own root rather than `claude`'s, so a read
    /// through the link is the same question on this provider's spelling.
    pub fn link_agy_skill(&self, name: &str) -> PathBuf {
        let source = self.snapshot.join(name);
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "---\nname: x\n---\nbody\n").unwrap();
        let root = self.worktree.join(".agents/skills");
        std::fs::create_dir_all(&root).unwrap();
        let link = root.join(name);
        std::os::unix::fs::symlink(&source, &link).unwrap();
        link
    }

    /// Asks in the `agy` dialect and reads its response shape, which the
    /// binary's own embedded reference documents: `decision` plus `reason`.
    pub fn agy_ask(&self, payload: Value) -> Decision {
        let (value, exit) = self.raw_answer(Some("agy"), payload);
        let decision = value["decision"]
            .as_str()
            .unwrap_or_else(|| panic!("no decision field in {value}"))
            .to_string();
        assert!(
            value.get("overwrite").is_none(),
            "6.6 forbids `overwrite`, which would rewrite the call the record shows: {value}"
        );
        Decision {
            allowed: decision == "allow",
            reason: value["reason"].as_str().unwrap_or_default().to_string(),
            exit,
        }
    }

    /// A `PreToolUse` payload in the shape measured on 1.1.26 on 2026-09-06:
    /// camelCase protojson, the call nested under `toolCall`, no `cwd` field,
    /// and `workspacePaths` carrying the directory passed on `--add-dir`.
    pub fn agy_tool(&self, tool: &str, args: Value) -> Decision {
        self.agy_ask(json!({
            "artifactDirectoryPath": "/Users/probe/.gemini/antigravity-cli/brain/b631e11b",
            "conversationId": "b631e11b-4a99-43db-9f37-11a373cdd98a",
            "modelName": "gemini-3.8-flash-low",
            "stepIdx": 2,
            "toolCall": { "name": tool, "args": args },
            "transcriptPath": "/Users/probe/.gemini/antigravity-cli/brain/b631e11b/t.jsonl",
            "workspacePaths": [self.worktree.to_string_lossy()],
        }))
    }

    /// Runs the script in a dialect and hands back the first line it printed.
    pub fn raw_answer(&self, dialect: Option<&str>, payload: Value) -> (Value, i32) {
        self.raw_answer_in(dialect, payload, None)
    }

    /// `raw_answer` with PWD set the way a provider sets it for the live hook,
    /// to the directory the call runs in.
    pub fn raw_answer_in(
        &self,
        dialect: Option<&str>,
        payload: Value,
        pwd: Option<&Path>,
    ) -> (Value, i32) {
        let mut command = Command::new(SHEBANG_PYTHON);
        command.arg(&self.script);
        command.env("HYDRA_PROBE_RUNS", &self.runs);
        if let Some(pwd) = pwd {
            command.env("PWD", pwd);
        }
        if let Some(dialect) = dialect {
            command.arg(dialect);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("python3 is on PATH");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let first = stdout.lines().next().unwrap_or_default();
        let value: Value = serde_json::from_str(first).unwrap_or_else(|e| {
            panic!("the gate did not answer in JSON: {stdout:?} ({e})\nstderr: {stderr}")
        });
        (value, out.status.code().unwrap_or(-1))
    }

    /// Asks in the `cursor` dialect and reads Cursor's own response shape:
    /// `permission` plus `user_message`, measured on 2026.09.02-c22c1a3 by
    /// research 30's rig, which answered in exactly that shape and was obeyed.
    pub fn cursor_ask(&self, payload: Value) -> Decision {
        let (value, exit) = self.raw_answer(Some("cursor"), payload);
        let permission = value["permission"]
            .as_str()
            .unwrap_or_else(|| panic!("no permission field in {value}"))
            .to_string();
        assert!(
            value.get("hookSpecificOutput").is_none(),
            "the cursor dialect is Cursor's own shape, not claude's nested one: {value}"
        );
        Decision {
            allowed: permission == "allow",
            reason: value["user_message"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            exit,
        }
    }

    /// A `preToolUse` payload in the shape measured on 2026.09.02-c22c1a3 on
    /// 2026-09-06: `tool_name`, `tool_input`, `tool_use_id`, the event name in
    /// camel case, and `workspace_roots` where `claude` sends `cwd`.
    pub fn cursor_tool(&self, tool: &str, input: Value) -> Decision {
        self.cursor_ask(json!({
            "conversation_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "model": "composer-2.5",
            "tool_name": tool,
            "tool_input": input,
            "tool_use_id": "tool_1e718bbc-5384-48a2-86a4-e6bcf9c4dbf",
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "hook_event_name": "preToolUse",
            "cursor_version": "2026.09.02-c22c1a3",
            "workspace_roots": [self.worktree.to_string_lossy()],
        }))
    }

    /// A `beforeReadFile` payload, same build and date: no `tool_name`, the path
    /// at the top level as `file_path`, and the content beside it.
    pub fn cursor_read(&self, path: &Path) -> Decision {
        self.cursor_ask(json!({
            "conversation_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "model": "composer-2.5",
            "content": "alpha line one\n",
            "file_path": path.to_string_lossy(),
            "attachments": [],
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "hook_event_name": "beforeReadFile",
            "cursor_version": "2026.09.02-c22c1a3",
            "workspace_roots": [self.worktree.to_string_lossy()],
        }))
    }

    pub fn ask_with_dialect(&self, dialect: Option<&str>, payload: Value) -> Decision {
        // The interpreter the shebang names, never the one on this shell's PATH.
        // `python3` here is Homebrew's 3.14.7 and the gate runs on the 3.9.6
        // macOS ships, so testing the first would leave the second unmeasured.
        let mut command = Command::new(SHEBANG_PYTHON);
        command.arg(&self.script);
        // The gate expands variables out of the environment it inherits, which
        // on a real run is the child's. Naming this run's corpus lets a test
        // write `$HYDRA_PROBE_RUNS/r.jsonl` and mean a path that is really
        // protected, rather than one that happens not to be.
        command.env("HYDRA_PROBE_RUNS", &self.runs);
        if let Some(dialect) = dialect {
            command.arg(dialect);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("python3 is on PATH");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let first = stdout.lines().next().unwrap_or_default();
        let value: Value = serde_json::from_str(first).unwrap_or_else(|e| {
            panic!("the gate did not answer in JSON: {stdout:?} ({e})\nstderr: {stderr}")
        });

        // `claude`'s shape, ticket 26 on 2.1.257. A response the provider does
        // not recognise is one it ignores, so the shape is the assertion.
        let hook = &value["hookSpecificOutput"];
        assert_eq!(hook["hookEventName"], "PreToolUse");
        let decision = hook["permissionDecision"].as_str().unwrap().to_string();
        Decision {
            allowed: decision == "allow",
            reason: hook["permissionDecisionReason"]
                .as_str()
                .unwrap()
                .to_string(),
            exit: out.status.code().unwrap_or(-1),
        }
    }

    pub fn ask(&self, tool: &str, input: Value) -> Decision {
        self.ask_with_dialect(
            Some("claude"),
            json!({
                "session_id": "d9ca2399-0000-0000-0000-000000000000",
                "cwd": self.worktree.to_string_lossy(),
                "permission_mode": "bypassPermissions",
                "hook_event_name": "PreToolUse",
                "tool_name": tool,
                "tool_input": input,
                "tool_use_id": "toolu_01Eaek",
            }),
        )
    }

    pub fn shell(&self, command: &str) -> Decision {
        self.ask(
            "Bash",
            json!({ "command": command, "description": "a probe" }),
        )
    }

    pub fn write_to(&self, path: &Path) -> Decision {
        self.ask(
            "Write",
            json!({ "file_path": path.to_string_lossy(), "content": "x" }),
        )
    }

    pub fn read_from(&self, path: &Path) -> Decision {
        self.ask("Read", json!({ "file_path": path.to_string_lossy() }))
    }

    pub fn apply_patch(&self, command: &str) -> Decision {
        self.ask("apply_patch", json!({ "command": command }))
    }

    pub fn decision_from(&self, dialect: &str, value: &Value) -> Decision {
        match dialect {
            "claude" => {
                let hook = &value["hookSpecificOutput"];
                let decision = hook["permissionDecision"].as_str().unwrap().to_string();
                Decision {
                    allowed: decision == "allow",
                    reason: hook["permissionDecisionReason"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    exit: 0,
                }
            }
            "cursor" => {
                let permission = value["permission"].as_str().unwrap().to_string();
                Decision {
                    allowed: permission == "allow",
                    reason: value["user_message"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    exit: 0,
                }
            }
            "agy" => {
                let decision = value["decision"].as_str().unwrap().to_string();
                Decision {
                    allowed: decision == "allow",
                    reason: value["reason"].as_str().unwrap_or_default().to_string(),
                    exit: 0,
                }
            }
            other => panic!("unknown dialect {other}"),
        }
    }

    /// Runs the batch entry: one interpreter, a JSON array on stdin, one answer
    /// per payload in order.
    pub fn batch_ask(&self, dialect: &str, payloads: &[Value]) -> Vec<Decision> {
        let mut command = Command::new(SHEBANG_PYTHON);
        command.arg(&self.script);
        command.env("HYDRA_PROBE_RUNS", &self.runs);
        command.arg(dialect);
        command.arg("batch");
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("python3 is on PATH");
        let body = serde_json::to_string(payloads).unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(body.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let answers: Vec<Value> = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!(
                "the gate batch entry did not answer in JSON: {stdout:?} ({e})\nstderr: {stderr}"
            )
        });
        answers
            .iter()
            .map(|value| self.decision_from(dialect, value))
            .collect()
    }

    pub fn claude_payload(&self, tool: &str, input: Value, cwd: &Path) -> Value {
        json!({
            "session_id": "d9ca2399-0000-0000-0000-000000000000",
            "cwd": cwd.to_string_lossy(),
            "permission_mode": "bypassPermissions",
            "hook_event_name": "PreToolUse",
            "tool_name": tool,
            "tool_input": input,
            "tool_use_id": "toolu_01Eaek",
        })
    }

    pub fn cursor_payload(&self, tool: &str, input: Value, cwd: &Path) -> Value {
        json!({
            "conversation_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "model": "composer-2.5",
            "tool_name": tool,
            "tool_input": input,
            "tool_use_id": "tool_1e718bbc-5384-48a2-86a4-e6bcf9c4dbf",
            "session_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "hook_event_name": "preToolUse",
            "cursor_version": "2026.09.02-c22c1a3",
            "workspace_roots": [cwd.to_string_lossy()],
        })
    }

    pub fn agy_payload(&self, tool: &str, args: Value, cwd: &Path) -> Value {
        json!({
            "artifactDirectoryPath": "/Users/probe/.gemini/antigravity-cli/brain/b631e11b",
            "conversationId": "b631e11b-4a99-43db-9f37-11a373cdd98a",
            "modelName": "gemini-3.8-flash-low",
            "stepIdx": 2,
            "toolCall": { "name": tool, "args": args },
            "transcriptPath": "/Users/probe/.gemini/antigravity-cli/brain/b631e11b/t.jsonl",
            "workspacePaths": [cwd.to_string_lossy()],
        })
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub fn canonical_subpath(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// One shell command under a spawn's own profile, answering what it printed.
pub fn under_seatbelt(profile: &Path, command: &str) -> String {
    let out = Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(profile)
        .arg("/bin/sh")
        .arg("-c")
        .arg(format!("{command}; echo rc=$?"))
        .output()
        .unwrap();
    // The shell reports a refused redirect on its own stderr, so both streams
    // are read and the status line at the end is what the assertions key on.
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    )
}

/// Whether the command under the seatbelt failed, however the shell worded it.
pub fn refused(out: &str) -> bool {
    !out.contains("rc=0")
}

/// Whether `/usr/bin/sandbox-exec` is present for a live seatbelt probe.
///
/// When it is absent, the probe prints that it skipped and returns false. On CI
/// the absence is a failure, because macOS runners are expected to carry it.
pub fn seatbelt_exec_or_skip(test_name: &str) -> bool {
    // Nested `sandbox-exec` is refused, so every `refused(..)` assertion would
    // pass on a probe that never ran. Not a CI failure: CI is never nested.
    if hydra::sandbox::inside_seatbelt() {
        eprintln!("skipped {test_name}: already inside a seatbelt, which cannot nest");
        return false;
    }
    if hydra::sandbox::exec_available() {
        return true;
    }
    eprintln!("skipped {test_name}: sandbox-exec is not available");
    if std::env::var_os("CI").is_some() {
        panic!("sandbox-exec is required on CI (macOS)");
    }
    false
}
