//! The gate script, fed synthetic hook payloads on stdin.
//!
//! No provider is launched here. This file asserts what the compiled script
//! answers; `tests/spawn.rs` checks that a run writes it where the run will find
//! it. Ticket 02 measured the rest on a real `claude` run, and
//! `IMPLEMENTATION.md` carries the numbers: a compiled deny held under
//! `--permission-mode bypassPermissions`.
//!
//! Every payload below is the shape ticket 26 measured coming out of `claude`
//! 2.1.257: `tool_name`, `tool_input`, `cwd`, and the rest of the envelope the
//! gate ignores.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::policy::{PolicySet, Spawn, write_gate};
use hydra::provider::Provider;
use serde_json::{Value, json};

/// What the gate script's shebang names, and what every test below runs it on.
const SHEBANG_PYTHON: &str = "/usr/bin/python3";

struct Decision {
    allowed: bool,
    reason: String,
    exit: i32,
}

impl Decision {
    fn denied_for(&self, code: &str) -> bool {
        !self.allowed && self.reason.contains(code)
    }
}

/// A compiled gate plus the directories its scopes point at.
struct Gate {
    root: PathBuf,
    script: PathBuf,
    worktree: PathBuf,
    runs: PathBuf,
    skills: PathBuf,
    /// The run's own snapshot of the store, `~/hydra/skills-snapshots/<sha>/` on
    /// a real launch. The links inside the worktree point here.
    snapshot: PathBuf,
}

impl Gate {
    fn compile(policy: &str, floor_lifted: bool) -> Self {
        Self::compile_for(Provider::Claude, policy, floor_lifted)
    }

    fn compile_for(provider: Provider, policy: &str, floor_lifted: bool) -> Self {
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
        let _ = std::fs::remove_dir_all(&root);
        let worktree = root.join("worktree");
        let runs = root.join("runs");
        let skills = root.join("skills");
        let snapshot = root.join("skills-snapshots/deadbeef");
        let spawn_dir = root.join("spawn");
        for dir in [&worktree, &runs, &skills, &snapshot, &spawn_dir] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let script = spawn_dir.join("gate");

        let set = PolicySet::discover().unwrap();
        write_gate(
            &set,
            &Spawn {
                provider,
                policy,
                worktree: &worktree,
                skill_store: &skills,
                skill_snapshot: &snapshot,
                runs: &runs,
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
        }
    }

    /// One skill in the snapshot, linked into the checkout the way `inject`
    /// links it: `<worktree>/.claude/skills/<name>` pointing at the snapshot
    /// directory. Answers with the link path, which is the path the provider
    /// names to the model as the skill's base directory (ticket 39).
    fn link_skill(&self, name: &str) -> PathBuf {
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

    fn new(policy: &str) -> Self {
        Self::compile(policy, false)
    }

    /// A gate compiled for `cursor-agent`, which answers in Cursor's own shape.
    fn cursor(policy: &str) -> Self {
        Self::compile_for(Provider::CursorAgent, policy, false)
    }

    /// Runs the script in a dialect and hands back the first line it printed.
    fn raw_answer(&self, dialect: Option<&str>, payload: Value) -> (Value, i32) {
        let mut command = Command::new(SHEBANG_PYTHON);
        command.arg(&self.script);
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
        (value, out.status.code().unwrap_or(-1))
    }

    /// Asks in the `cursor` dialect and reads Cursor's own response shape:
    /// `permission` plus `user_message`, measured on 2026.09.02-c22c1a3 by
    /// research 30's rig, which answered in exactly that shape and was obeyed.
    fn cursor_ask(&self, payload: Value) -> Decision {
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
            reason: value["user_message"].as_str().unwrap_or_default().to_string(),
            exit,
        }
    }

    /// A `preToolUse` payload in the shape measured on 2026.09.02-c22c1a3 on
    /// 2026-09-06: `tool_name`, `tool_input`, `tool_use_id`, the event name in
    /// camel case, and `workspace_roots` where `claude` sends `cwd`.
    fn cursor_tool(&self, tool: &str, input: Value) -> Decision {
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
    fn cursor_read(&self, path: &Path) -> Decision {
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

    fn ask_with_dialect(&self, dialect: Option<&str>, payload: Value) -> Decision {
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

    fn ask(&self, tool: &str, input: Value) -> Decision {
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

    fn shell(&self, command: &str) -> Decision {
        self.ask(
            "Bash",
            json!({ "command": command, "description": "a probe" }),
        )
    }

    fn write_to(&self, path: &Path) -> Decision {
        self.ask(
            "Write",
            json!({ "file_path": path.to_string_lossy(), "content": "x" }),
        )
    }

    fn read_from(&self, path: &Path) -> Decision {
        self.ask("Read", json!({ "file_path": path.to_string_lossy() }))
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

// ---------------------------------------------------------------------------
// The dialect
// ---------------------------------------------------------------------------

#[test]
fn the_script_denies_when_the_dialect_argument_is_absent() {
    // 6.6: `claude` ignores a response in a shape it does not recognise and
    // allows the call, so the script never guesses which shape to answer in.
    let gate = Gate::new("permissive");
    let decision = gate.ask_with_dialect(
        None,
        json!({ "tool_name": "Read", "tool_input": { "file_path": "/etc/hosts" } }),
    );
    assert!(!decision.allowed);
    assert_eq!(
        decision.exit, 2,
        "a non-zero exit is what `agy` fails closed on, whatever it makes of the body"
    );
}

#[test]
fn a_payload_that_is_not_json_is_denied() {
    let gate = Gate::new("permissive");
    let mut child = Command::new(SHEBANG_PYTHON)
        .arg(&gate.script)
        .arg("claude")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"not json at all")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"], "deny",
        "a gate that cannot read the question does not answer yes"
    );
}

// ---------------------------------------------------------------------------
// The seven path denials, harness spec 7.5
// ---------------------------------------------------------------------------

#[test]
fn the_path_denials_hold_under_permissive_and_curate() {
    // 7.5: denied under every policy, `permissive` and `curate` included. This
    // is where `permissive` stops being permissive.
    let policies_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("policies");
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("config");

    for policy in ["permissive", "curate"] {
        let gate = Gate::new(policy);
        let targets = [
            (
                "the delegation policy set",
                policies_dir.join("standard.json"),
            ),
            (
                "the environment allowlist",
                config.join("env-allowlist.json"),
            ),
            ("the instruction preamble", config.join("preamble.md")),
            // 15.1 #5. This one is a file now, and it is the one where a write
            // is arbitrary code inside every future run rather than a wrong
            // instruction in one.
            ("the canonical MCP document", config.join("mcp.md")),
            ("the generated gate script", gate.script.clone()),
            ("the run records directory", gate.runs.join("any-run.jsonl")),
        ];
        for (what, path) in targets {
            let decision = gate.write_to(&path);
            assert!(
                decision.denied_for("hydra/protected-path"),
                "{policy} wrote to {what} at {}: {}",
                path.display(),
                decision.reason
            );
        }
    }
}

#[test]
fn the_mcp_document_is_denied_under_all_five_policies() {
    // 15.1 #5 and 7.5. The test above covers the two policies where the denial is
    // the only thing standing there; this one is literal about "every policy",
    // because a server definition is a command Hydra spawns inside every future
    // run, and the answer has to be no from all five rather than no from the two
    // that could otherwise reach it.
    let document = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("mcp.md");
    for policy in ["judge", "curate", "read-only", "standard", "permissive"] {
        let gate = Gate::new(policy);
        let decision = gate.write_to(&document);
        assert!(!decision.allowed, "{policy} wrote to the MCP document");
    }
}

#[test]
fn curate_writes_inside_the_skill_store_and_nowhere_else() {
    let gate = Gate::new("curate");
    assert!(
        gate.write_to(&gate.skills.join("naming/SKILL.md")).allowed,
        "curate's whole write surface is the skill store"
    );
    assert!(
        gate.write_to(&gate.worktree.join("notes.md"))
            .denied_for("hydra/out-of-scope"),
        "curate cannot touch a worktree, which is why it and standard are incomparable"
    );
    assert!(
        gate.ask(
            "Read",
            json!({ "file_path": gate.runs.join("r.jsonl").to_string_lossy() })
        )
        .allowed
    );
}

// ---------------------------------------------------------------------------
// The five policies
// ---------------------------------------------------------------------------

#[test]
fn judge_refuses_everything() {
    let gate = Gate::new("judge");
    for (tool, input) in [
        ("Read", json!({ "file_path": "README.md" })),
        ("Write", json!({ "file_path": "out.txt", "content": "x" })),
        ("Bash", json!({ "command": "echo hi" })),
        ("WebFetch", json!({ "url": "https://example.com" })),
        ("Task", json!({ "prompt": "go" })),
        ("mcp__server__tool", json!({})),
    ] {
        let decision = gate.ask(tool, input);
        assert!(
            !decision.allowed,
            "judge allowed {tool}: {}",
            decision.reason
        );
    }
}

#[test]
fn read_only_reads_the_worktree_and_refuses_a_write() {
    let gate = Gate::new("read-only");
    assert!(
        gate.ask("Read", json!({ "file_path": "src/main.rs" }))
            .allowed
    );
    assert!(
        gate.write_to(&gate.worktree.join("out.txt"))
            .denied_for("hydra/capability"),
        "read-only holds no file_write entry at all"
    );
    assert!(
        gate.shell("echo hi").denied_for("hydra/capability"),
        "no shell under read-only"
    );
}

#[test]
fn standard_writes_inside_the_worktree_and_refuses_outside_it() {
    let gate = Gate::new("standard");
    assert!(
        gate.write_to(&gate.worktree.join("src/new.rs")).allowed,
        "the ordinary coding case"
    );
    assert!(
        gate.write_to(&gate.skills.join("naming/SKILL.md"))
            .denied_for("hydra/out-of-scope"),
        "the skill store is curate's surface, not standard's"
    );
    assert!(
        gate.write_to(Path::new("/tmp/somewhere-else.txt"))
            .denied_for("hydra/out-of-scope")
    );
    // 7.2: standard allows network reads, because denying them breaks ordinary
    // coding work and people then route around the policy.
    assert!(
        gate.ask("WebFetch", json!({ "url": "https://example.com" }))
            .allowed
    );
}

#[test]
fn a_symlink_out_of_the_worktree_does_not_widen_the_scope() {
    let gate = Gate::new("standard");
    let escape = gate.worktree.join("escape");
    std::os::unix::fs::symlink(&gate.runs, &escape).unwrap();
    assert!(
        gate.write_to(&escape.join("stolen.jsonl"))
            .denied_for("hydra/protected-path"),
        "the gate resolves symlinks, so a link is not a way out of a scoped policy"
    );
}

#[test]
fn a_dotdot_path_does_not_widen_the_scope() {
    let gate = Gate::new("standard");
    assert!(
        gate.write_to(&gate.worktree.join("../runs/stolen.jsonl"))
            .denied_for("hydra/protected-path")
    );
}

#[test]
fn every_policy_but_permissive_refuses_task() {
    // 7.6, and the cost is real: a delegated `claude` child cannot use `Task`.
    for policy in ["judge", "curate", "read-only", "standard"] {
        let gate = Gate::new(policy);
        assert!(
            !gate.ask("Task", json!({ "prompt": "go" })).allowed,
            "{policy} allowed an in-process subagent, which gets no run id and \
             writes no run file"
        );
    }
    assert!(
        Gate::new("permissive")
            .ask("Task", json!({ "prompt": "go" }))
            .allowed
    );
}

#[test]
fn a_tool_the_table_does_not_know_is_denied_under_every_policy() {
    // 7.1: a self-update that renames a tool breaks the next run loudly instead
    // of quietly widening what a child may do. `permissive` is included on
    // purpose.
    for policy in ["judge", "curate", "read-only", "standard", "permissive"] {
        let gate = Gate::new(policy);
        assert!(
            gate.ask("BashOutputV2", json!({}))
                .denied_for("hydra/unknown-tool"),
            "{policy} allowed a tool name Hydra's table has never heard of"
        );
    }
}

#[test]
fn the_provider_native_tools_belong_to_permissive_alone() {
    // The last four arrived with 2.1.263. They are in the list so a row added
    // after the gate script was written still reaches the compiled gate.
    //
    // `Skill` used to be here. Map ticket 39 moved it to `skill_invoke` on
    // 2026-09-06, and the test below is where it lives now.
    for tool in [
        "CronCreate",
        "SendMessage",
        "ToolSearch",
        "TaskCreate",
        "TaskGet",
        "TaskList",
        "TaskUpdate",
    ] {
        assert!(
            !Gate::new("standard").ask(tool, json!({})).allowed,
            "{tool} is provider_native and standard does not grant it"
        );
        assert!(Gate::new("permissive").ask(tool, json!({})).allowed);
    }
}

// ---------------------------------------------------------------------------
// `skill_invoke` and the `skill_snapshot` scope, map ticket 39
// ---------------------------------------------------------------------------

#[test]
fn every_policy_that_reads_at_all_may_invoke_a_linked_skill() {
    // 7.2. A run that is linked a skill and cannot invoke it was given a listing
    // entry rather than a skill. `judge` reads nothing and gets neither.
    for policy in ["read-only", "standard", "curate", "permissive"] {
        let decision = Gate::new(policy).ask("Skill", json!({ "skill": "hydra-sentinel" }));
        assert!(
            decision.allowed,
            "{policy} refused a linked skill: {}",
            decision.reason
        );
    }
    let judge = Gate::new("judge").ask("Skill", json!({ "skill": "hydra-sentinel" }));
    assert!(
        judge.denied_for("hydra/capability"),
        "judge allows nothing, and the refusal is a capability one: {}",
        judge.reason
    );
}

#[test]
fn a_read_through_a_skill_link_lands_in_the_snapshot() {
    // Ticket 39 measured `claude` naming the link path inside the worktree as
    // the skill's base directory, so the model reaches for the sibling through
    // the link. The gate resolves the link into the snapshot before comparing,
    // which is what the `skill_snapshot` scope is for.
    for policy in ["read-only", "standard", "curate"] {
        let gate = Gate::new(policy);
        let link = gate.link_skill("hydra-sentinel");
        let decision = gate.read_from(&link.join("reference.md"));
        assert!(
            decision.allowed,
            "{policy} refused a sibling read through the link: {}",
            decision.reason
        );
        assert!(decision.reason.starts_with("hydra/allow:"));
    }
}

#[test]
fn a_write_through_a_skill_link_is_still_out_of_scope() {
    // The snapshot is a read scope and appears on no policy's `file_write`. A
    // run that could edit it could edit its own instructions mid-session.
    let gate = Gate::new("standard");
    let link = gate.link_skill("hydra-sentinel");
    assert!(
        gate.write_to(&link.join("SKILL.md"))
            .denied_for("hydra/out-of-scope")
    );
    assert!(
        gate.write_to(&link.join("new-file.md"))
            .denied_for("hydra/out-of-scope")
    );
}

#[test]
fn the_snapshot_scope_is_this_runs_snapshot_and_not_the_directory_above_it() {
    // Snapshots are keyed by sha and shared, so a scope pointing at
    // `skills-snapshots/` would hand every run every other run's snapshot.
    let gate = Gate::new("standard");
    gate.link_skill("hydra-sentinel");
    let sibling = gate.root.join("skills-snapshots/cafebabe");
    std::fs::create_dir_all(&sibling).unwrap();
    assert!(
        gate.read_from(&sibling.join("other/SKILL.md"))
            .denied_for("hydra/out-of-scope")
    );
}

#[test]
fn a_link_to_anywhere_else_is_still_out_of_scope() {
    // The snapshot scope is one more root, not a hole in the symlink rule. A
    // link inside the injected root that points somewhere neither protected nor
    // pinned resolves the same way it did before ticket 13.
    let gate = Gate::new("standard");
    gate.link_skill("hydra-sentinel");
    let elsewhere = gate.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("secret.md"), "not yours\n").unwrap();
    let link = gate
        .worktree
        .join(".claude/skills")
        .join("pretending-to-be-a-skill");
    std::os::unix::fs::symlink(&elsewhere, &link).unwrap();
    assert!(
        gate.read_from(&link.join("secret.md"))
            .denied_for("hydra/out-of-scope")
    );
}

#[test]
fn judge_reads_no_snapshot() {
    let gate = Gate::new("judge");
    let link = gate.link_skill("hydra-sentinel");
    assert!(
        gate.read_from(&link.join("SKILL.md"))
            .denied_for("hydra/capability")
    );
}

// ---------------------------------------------------------------------------
// The cursor dialect, harness spec 6.6, measured on 2026.09.02-c22c1a3
// ---------------------------------------------------------------------------

#[test]
fn a_cursor_gate_answers_in_cursors_own_shape() {
    // Research 30 measured Cursor obeying `{"permission": ...}` on
    // `preToolUse`, and `beforeReadFile` gets no translation of the claude
    // shape at all, so one shape serves both events on this provider.
    let gate = Gate::cursor("standard");
    let decision = gate.cursor_tool(
        "Shell",
        json!({ "command": "echo T09PROBE", "cwd": "", "timeout": 30000 }),
    );
    assert!(decision.allowed, "{}", decision.reason);
    assert!(decision.reason.starts_with("hydra/allow:"));
    assert_eq!(decision.exit, 0);
}

#[test]
fn a_before_read_file_payload_is_gated_as_a_read() {
    // 6.6: reads are a capability, and this event is why Cursor registers two.
    // The payload carries no `tool_name`, so the gate maps it onto `Read` and
    // the path field it measured, `file_path`.
    let gate = Gate::cursor("read-only");
    let inside = gate.cursor_read(&gate.worktree.join("a.txt"));
    assert!(inside.allowed, "{}", inside.reason);

    let outside = gate.cursor_read(Path::new("/etc/hosts"));
    assert!(
        outside.denied_for("hydra/out-of-scope"),
        "a read outside the worktree walked past beforeReadFile: {}",
        outside.reason
    );

    // `judge` allows nothing, reads included.
    let judge = Gate::cursor("judge").cursor_read(Path::new("/tmp/anything"));
    assert!(judge.denied_for("hydra/capability"));
}

#[test]
fn cursors_tool_names_answer_to_the_capabilities_the_policies_speak_in() {
    // The names measured off the hook on 2026-09-06: `Read`, `Shell`, `Write`,
    // `Grep` and `Delete`, and the list tool arrives as `Grep` with an
    // `output_mode` rather than as a tool of its own.
    let standard = Gate::cursor("standard");
    let inside = standard.worktree.join("probe.txt");
    assert!(
        standard
            .cursor_tool("Read", json!({ "file_path": inside.to_string_lossy() }))
            .allowed
    );
    assert!(
        standard
            .cursor_tool(
                "Write",
                json!({ "file_path": inside.to_string_lossy(), "content": "x\n" })
            )
            .allowed
    );
    assert!(
        standard
            .cursor_tool("Delete", json!({ "file_path": inside.to_string_lossy() }))
            .allowed
    );
    assert!(
        standard
            .cursor_tool(
                "Grep",
                json!({ "pattern": "", "file_path": standard.worktree.to_string_lossy(),
                        "glob": "**/*", "output_mode": "files_with_matches" })
            )
            .allowed
    );

    let read_only = Gate::cursor("read-only");
    let probe = read_only.worktree.join("probe.txt");
    assert!(
        read_only
            .cursor_tool("Delete", json!({ "file_path": probe.to_string_lossy() }))
            .denied_for("hydra/capability"),
        "a delete is a write"
    );
    assert!(
        read_only
            .cursor_tool(
                "Write",
                json!({ "file_path": probe.to_string_lossy(), "content": "x\n" })
            )
            .denied_for("hydra/capability")
    );
    assert!(
        read_only
            .cursor_tool("Shell", json!({ "command": "echo hi", "cwd": "", "timeout": 30000 }))
            .denied_for("hydra/capability")
    );
    assert!(
        read_only
            .cursor_tool("Write", json!({ "file_path": "/etc/hosts", "content": "x" }))
            .denied_for("hydra/capability")
    );
}

#[test]
fn cursor_names_an_mcp_tool_with_a_prefix_of_its_own() {
    // `MCP:<tool>` rather than `mcp__<server>__<tool>`, read off the bundle's
    // `preToolUse` call for MCP tools. `mcp_call` is granted by `permissive`
    // alone (7.2).
    let standard = Gate::cursor("standard");
    assert!(
        standard
            .cursor_tool("MCP:ping", json!({}))
            .denied_for("hydra/capability")
    );
    assert!(
        Gate::cursor("permissive")
            .cursor_tool("MCP:ping", json!({}))
            .allowed
    );
}

#[test]
fn the_seven_path_denials_hold_through_cursors_shell_too() {
    let gate = Gate::cursor("permissive");
    let target = gate.runs.join("r.jsonl");
    let decision = gate.cursor_tool(
        "Shell",
        json!({ "command": format!("echo x >> {}", target.display()), "cwd": "", "timeout": 30000 }),
    );
    assert!(decision.denied_for("hydra/protected-path"), "{}", decision.reason);
}

#[test]
fn a_dialect_that_is_not_the_one_the_gate_was_compiled_for_is_refused() {
    // The dialect is argv[1] and the provider is compiled in. Both name the
    // same binary, and a gate handed to one provider answered in another's
    // shape would be an open door on `claude`, which allows what it cannot
    // parse. So a mismatch denies rather than guesses.
    let gate = Gate::cursor("permissive");
    let (value, _) = gate.raw_answer(
        Some("claude"),
        json!({ "tool_name": "Shell", "tool_input": { "command": "echo hi" } }),
    );
    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("hydra/wrong-dialect"),
        "{value}"
    );

    let claude = Gate::new("permissive");
    let mismatch = claude.cursor_ask(json!({
        "tool_name": "Bash", "tool_input": { "command": "echo hi" },
        "hook_event_name": "preToolUse",
    }));
    assert!(mismatch.denied_for("hydra/wrong-dialect"));
}

#[test]
fn a_claude_tool_name_means_nothing_to_a_cursor_gate() {
    // 7.1's table is per provider. `Bash` is `claude`'s spelling of `Shell`,
    // and a Cursor gate that answered for it would be answering for a tool the
    // provider never calls, so it is denied as unknown.
    let gate = Gate::cursor("permissive");
    assert!(
        gate.cursor_tool("Bash", json!({ "command": "echo hi" }))
            .denied_for("hydra/unknown-tool")
    );
}

// ---------------------------------------------------------------------------
// The pattern floor, harness spec 7.4
// ---------------------------------------------------------------------------

#[test]
fn the_floor_refuses_the_destructive_git_forms() {
    let gate = Gate::new("standard");
    for command in [
        "git push --force",
        "git push -f origin topic",
        "git push --force-with-lease",
        "git push --force-if-includes",
        "git push --mirror",
        "git push --all",
        "git push origin +topic:main",
        "git push origin main",
        "git push origin release/2026-09",
        "git branch -D topic",
        "git update-ref refs/heads/main HEAD",
        "git symbolic-ref HEAD refs/heads/topic",
        "git reflog expire --all",
        "git reset --hard HEAD~3",
        "git clean -fd",
        "git clean -f -x",
        "git clean --force -d",
    ] {
        assert!(
            gate.shell(command).denied_for("hydra/floor"),
            "the floor let through: {command}"
        );
    }
}

#[test]
fn the_floor_leaves_ordinary_git_alone() {
    let gate = Gate::new("standard");
    for command in [
        "git status",
        "git push origin topic",
        "git commit -m 'work'",
        "git clean -n",
        "git clean -f",
        "git symbolic-ref HEAD",
        "git log --oneline -5",
    ] {
        let decision = gate.shell(command);
        assert!(
            decision.allowed,
            "the floor refused: {command}: {}",
            decision.reason
        );
    }
}

#[test]
fn each_pattern_consumes_dash_c_and_git_dir_before_reading_the_subcommand() {
    // 7.4 says so in one line, and it is the whole difference between a floor
    // and a floor with a hole in it.
    let gate = Gate::new("standard");
    for command in [
        "git -C /elsewhere push --force",
        "git --git-dir=/elsewhere/.git push --force",
        "git --git-dir /elsewhere/.git push --force",
        "git --work-tree=/elsewhere -C /x reset --hard",
        "git -C /elsewhere branch -D topic",
    ] {
        assert!(
            gate.shell(command).denied_for("hydra/floor"),
            "the subcommand hid behind an option that takes a value: {command}"
        );
    }
}

#[test]
fn a_subcommand_from_a_shell_substitution_is_refused_rather_than_evaluated() {
    let gate = Gate::new("standard");
    for command in [
        "git $(echo push) --force",
        "git `echo push` --force",
        "git ${VERB} --force",
    ] {
        assert!(
            gate.shell(command).denied_for("hydra/floor"),
            "the floor tried to evaluate: {command}"
        );
    }
}

#[test]
fn the_floor_checks_every_segment_of_a_chained_command() {
    let gate = Gate::new("standard");
    for command in [
        "echo starting && git push --force",
        "cargo test; git reset --hard",
        "git status | head -3 && git clean -fdx",
    ] {
        assert!(
            gate.shell(command).denied_for("hydra/floor"),
            "the floor read only the first segment of: {command}"
        );
    }
}

#[test]
fn the_floor_refuses_the_aws_forms() {
    let gate = Gate::new("standard");
    for command in [
        "aws configure",
        "aws configure set region us-east-1",
        "aws sso login",
        "aws s3 cp local.txt s3://bucket/",
        "aws s3 mv a s3://b",
        "aws s3 rm s3://bucket/key",
        "aws s3 sync . s3://bucket",
        "aws s3 rb s3://bucket",
    ] {
        assert!(
            gate.shell(command).denied_for("hydra/floor"),
            "the floor let through: {command}"
        );
    }
    assert!(gate.shell("aws s3 ls s3://bucket").allowed);
    assert!(gate.shell("aws sts get-caller-identity").allowed);
}

#[test]
fn the_floor_refuses_the_slack_forms() {
    let gate = Gate::new("standard");
    for command in [
        "curl -X POST https://hooks.slack.com/services/T00/B00/XXX -d '{}'",
        "curl https://slack.com/api/chat.postMessage -d text=hi",
        "curl -F file=@x https://slack.com/api/files.upload",
        "curl https://slack.com/api/conversations.create -d name=x",
    ] {
        assert!(
            gate.shell(command).denied_for("hydra/floor"),
            "the floor let through: {command}"
        );
    }
    assert!(gate.shell("curl https://example.com/api").allowed);
}

#[test]
fn the_floor_applies_under_permissive_too() {
    // 7.4: exempting `permissive` would empty the floor on `agy` alone, which
    // is the one provider where it is the only thing present.
    assert!(
        Gate::new("permissive")
            .shell("git push --force")
            .denied_for("hydra/floor")
    );
}

#[test]
fn the_human_flag_lifts_the_floor_and_nothing_else() {
    // Harness spec 16. Never inheritable, never settable by a policy.
    let lifted = Gate::compile("standard", true);
    assert!(lifted.shell("git push --force").allowed);
    assert!(
        lifted
            .write_to(&lifted.runs.join("r.jsonl"))
            .denied_for("hydra/protected-path"),
        "--allow-floor-escape lifts the floor, not the path denials"
    );
}

#[test]
fn an_unparseable_command_is_refused_rather_than_guessed_at() {
    let gate = Gate::new("standard");
    assert!(
        gate.shell("git push \"unbalanced")
            .denied_for("hydra/floor")
    );
}

#[test]
fn the_written_script_is_executable_and_runs_on_its_own_shebang() {
    // The provider runs the script as a command, so this is the shape that
    // matters: exec bit set, and a shebang that does not go through PATH. On
    // `claude` a hook that fails to run fails open, which is why neither is
    // left to the caller.
    use std::os::unix::fs::PermissionsExt;

    let gate = Gate::new("judge");
    let mode = std::fs::metadata(&gate.script)
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700, "the gate script is not executable");

    let first = std::fs::read_to_string(&gate.script).unwrap();
    let shebang = first.lines().next().unwrap();
    assert_eq!(shebang, format!("#!{SHEBANG_PYTHON}"));
    assert!(Path::new(SHEBANG_PYTHON).exists());

    let mut child = Command::new(&gate.script)
        .arg("claude")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the script runs as a command");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"tool_name":"Bash","tool_input":{"command":"echo hi"}}"#)
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"], "deny",
        "judge allows nothing"
    );
}

#[test]
fn shell_is_refused_when_it_names_a_protected_path() {
    // 7.5 states the seven denials as delivered outright: "a hook sees the path
    // and refuses the write before it reaches disk". `Bash` carries no path, so
    // until this landed the deny list was consulted for `file_write` alone and
    // every one of the seven was reachable by redirect under any policy granting
    // shell. The gate script's own denial is 7.5's induction base case, and an
    // emptied script makes `claude` fail open (6.6), so that one composed into a
    // full bypass in two tool calls.
    let gate = Gate::new("standard");
    let record = gate.runs.join("r.jsonl");

    assert!(
        gate.write_to(&record).denied_for("hydra/protected-path"),
        "the path-carrying tool was always refused"
    );
    assert!(
        gate.shell(&format!("echo pwned > {}", record.display()))
            .denied_for("hydra/protected-path"),
        "the corpus is what a loop learns from and must not be able to edit"
    );
    assert!(
        gate.shell(&format!("cat /dev/null > {}", gate.script.display()))
            .denied_for("hydra/protected-path"),
        "7.5's base case: the run must not be able to empty its own gate"
    );
    assert!(
        gate.shell(&format!("rm -rf {}", gate.runs.display()))
            .denied_for("hydra/protected-path"),
        "a naming rule has to catch the verb it never enumerated"
    );
    assert!(
        gate.shell(&format!("tee --output={}/r.jsonl", gate.runs.display()))
            .denied_for("hydra/protected-path"),
        "a path behind a flag is still a path"
    );
}

#[test]
fn the_shell_refusal_is_coarse_on_purpose() {
    // Which token is a redirect target is not decidable without running the
    // command, and neither is which is an argument to a writing binary. So a
    // read that names a protected path is refused too. That is a false positive
    // in the same direction 7.4's floor already takes, and the precise access is
    // the file tools, which carry a path: `curate` scopes `file_read` to the
    // corpus and reads it that way.
    let gate = Gate::new("curate");
    assert!(
        gate.shell(&format!("cat {}/r.jsonl", gate.runs.display()))
            .denied_for("hydra/protected-path")
    );
}

#[test]
fn a_path_a_variable_carries_is_expanded_before_it_is_checked() {
    // The shell expands `$HOME` and `~`, so a rule that does not is a rule
    // against one spelling of the same path.
    let gate = Gate::new("standard");
    assert!(
        gate.shell("echo x > $HYDRA_PROBE_RUNS/r.jsonl")
            .denied_for("hydra/protected-path")
    );
    assert!(
        gate.shell("echo x > ${HYDRA_PROBE_RUNS}/r.jsonl")
            .denied_for("hydra/protected-path")
    );
}

#[test]
fn an_assignment_is_read_where_it_is_written_down() {
    // `D=<corpus>; echo x > $D/r.jsonl` sets the variable inside the command, so
    // the gate's own environment cannot expand `$D`. The assignment itself is a
    // token though, and its tail is the path, so the write is refused one
    // segment before it happens.
    let gate = Gate::new("standard");
    assert!(
        gate.shell(&format!("D={}; echo x > $D/r.jsonl", gate.runs.display()))
            .denied_for("hydra/protected-path")
    );
}

#[test]
fn a_protected_path_built_by_substitution_still_walks_past() {
    // **This pins what is still open.** 7.4 names the failure mode for the floor
    // and it is the same one here: silent permission on the spelling nobody
    // wrote down. A path the command assembles rather than spells is not caught,
    // because catching it means evaluating the command, which is the thing 7.4
    // rules out. Closing this needs a mechanism that does not read argv at all.
    //
    // The day it is closed, this test fails and whoever closed it rewrites it.
    // Spelling it inside a substitution is still spelling it, and that is
    // caught. What is not is a path the command never writes down.
    let gate = Gate::new("standard");
    assert!(
        gate.shell(&format!("echo x > $(echo {})/r.jsonl", gate.runs.display()))
            .denied_for("hydra/protected-path"),
        "a path spelled inside a substitution is still spelled"
    );
    // The path is never written down: the variable holds only its parent, so no
    // token is a protected path and no expansion the gate can do produces one.
    assert!(
        gate.shell(&format!(
            "P={}; echo x > $P/runs/r.jsonl",
            gate.root.display()
        ))
        .allowed,
        "if this now denies, the residual is closed and the comment is stale"
    );
}

#[test]
fn ordinary_shell_is_left_alone() {
    let gate = Gate::new("standard");
    assert!(gate.shell("cargo test --all").allowed);
    assert!(gate.shell("ls -la && git status").allowed);
    assert!(
        gate.shell("grep -rn TODO src/").allowed,
        "a relative path inside the worktree is not a protected path"
    );
}

