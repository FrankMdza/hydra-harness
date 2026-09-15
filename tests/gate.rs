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

use hydra::paths::Layout;
use hydra::policy::{PolicySet, Spawn, write_gate};
use hydra::provider::Provider;
use serde_json::{Value, json};

/// What the gate script's shebang names, and what every test below runs it on.
const SHEBANG_PYTHON: &str = "/usr/bin/python3";

/// The run every fixture path is keyed by.
const RUN: &str = "own";

/// A sibling run, whose worktree, spawn directory and store checkout this run
/// owns nothing of. Half the deny list of 7.5 exists to keep these apart.
const OTHER: &str = "other";

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
///
/// Every path comes off a [`Layout`] rooted at `root`, so a probe naming
/// `worktrees/other` names what a real sibling run would hold and the deny list
/// of 7.5 is checked against the layout it was written for.
struct Gate {
    root: PathBuf,
    script: PathBuf,
    worktree: PathBuf,
    runs: PathBuf,
    skills: PathBuf,
    /// The run's own snapshot of the store, `~/hydra/skills-snapshots/<sha>/` on
    /// a real launch. The links inside the worktree point here.
    snapshot: PathBuf,
    /// The `curate` pass's own checkout of the store, and `None` under every
    /// other policy, which is what makes the carve-out of 7.5 per spawn.
    checkout: Option<PathBuf>,
    /// The parent repo's `.git`, which a real launch knows because it ran
    /// `git worktree add` from it.
    parent_git: PathBuf,
}

impl Gate {
    fn compile(policy: &str, floor_lifted: bool) -> Self {
        Self::compile_for(Provider::Claude, policy, floor_lifted)
    }

    fn compile_for(provider: Provider, policy: &str, floor_lifted: bool) -> Self {
        Self::build(provider, policy, floor_lifted, true)
    }

    /// The gate a run outside a repo gets. It has no checkout, so `hydra run`
    /// points its `worktree` scope at `spawn/<run-id>/no-worktree`, a path
    /// nothing creates and nothing carves out of the denials.
    fn without_a_checkout(policy: &str) -> Self {
        Self::build(Provider::Claude, policy, false, false)
    }

    fn build(provider: Provider, policy: &str, floor_lifted: bool, has_checkout: bool) -> Self {
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
    fn build_at(
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
    fn layout(&self) -> Layout {
        Layout::at(&self.root)
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

    /// A gate compiled for `agy`, which answers `decision` plus `reason`.
    fn agy(policy: &str) -> Self {
        Self::compile_for(Provider::Agy, policy, false)
    }

    /// One skill linked into `agy`'s own root rather than `claude`'s, so a read
    /// through the link is the same question on this provider's spelling.
    fn link_agy_skill(&self, name: &str) -> PathBuf {
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
    fn agy_ask(&self, payload: Value) -> Decision {
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
    fn agy_tool(&self, tool: &str, args: Value) -> Decision {
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
// The path denials, harness spec 7.5
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
fn curate_writes_inside_its_own_checkout_and_nowhere_else() {
    // 7.2 and ouroboros spec 3.6. The pass writes into its own git checkout of
    // the store and never into `~/hydra/skills/`, which 7.5 denies to every
    // policy since ticket 40: a curator writing the live working tree hands its
    // half-finished proposal to whichever run launches next.
    let gate = Gate::new("curate");
    let checkout = gate.checkout.clone().expect("a curate gate carries one");
    assert!(
        gate.write_to(&checkout.join("proposal.md")).allowed,
        "curate's whole write surface is its own checkout"
    );
    assert!(
        gate.read_from(&checkout.join("naming/SKILL.md")).allowed,
        "a curator that cannot read what it wrote cannot revise it"
    );
    assert!(
        gate.write_to(&gate.skills.join("naming/SKILL.md"))
            .denied_for("hydra/protected-path"),
        "the store's working tree is denied to every policy, curate included"
    );
    assert!(
        gate.read_from(&gate.skills.join("naming/SKILL.md")).allowed,
        "the deny list is a write list, and curate reads the store it proposes against"
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

#[test]
fn a_run_with_no_checkout_writes_nowhere_at_all() {
    // 7.5 leaves the run's own spawn directory denied and carves nothing out of
    // it, and a run outside a repo is scoped to `no-worktree` inside that
    // directory. So the `worktree` scope resolves, the write lands under a
    // denied parent, and nothing carves it back out. `src/run.rs` says such a
    // run can read and shell out and write nowhere, and this is that sentence
    // as an assertion.
    let gate = Gate::without_a_checkout("standard");
    let decision = gate.write_to(&gate.worktree.join("out.txt"));
    assert!(
        decision.denied_for("hydra/protected-path"),
        "a run with no checkout wrote inside its own spawn directory: {}",
        decision.reason
    );
    // Reading is untouched, which is the half of the sentence that stays true.
    assert!(gate.read_from(&gate.worktree.join("out.txt")).allowed);
}

// ---------------------------------------------------------------------------
// The data-loss matrix of map ticket 40
// ---------------------------------------------------------------------------

/// The five, in the order the matrix writes its columns.
const MATRIX_POLICIES: [&str; 5] = ["judge", "curate", "read-only", "standard", "permissive"];

/// Asserts one cell. `want` is `allow` or the code the refusal carries after
/// `hydra/`, because a deny for the wrong reason is a deny the next edit moves
/// back.
fn matrix_cell(policy: &str, probe: &str, want: &str, decision: &Decision) {
    if want == "allow" {
        assert!(
            decision.allowed,
            "{policy} refused {probe}, which ticket 40 decision 11 keeps allowed: {}",
            decision.reason
        );
    } else {
        assert!(
            decision.denied_for(&format!("hydra/{want}")),
            "{policy} on {probe} wanted {want} and answered: {}",
            decision.reason
        );
    }
}

#[test]
fn a_write_the_matrix_allowed_on_2026_09_06_is_protected_now() {
    // Ticket 40 measured 32 probes per policy on 2026-09-06 and this is that
    // matrix after decision 9. Everything under `~/hydra` that belongs to
    // another run, to the corpus or to the store is denied before the policy is
    // consulted, and the two cells that stay allowed are the carve-out: a run
    // writes only inside what Hydra created for it.
    for (column, policy) in MATRIX_POLICIES.iter().enumerate() {
        let gate = Gate::new(policy);
        let layout = gate.layout();
        let protected = "protected-path";
        let rows: [(&str, PathBuf, [&str; 5]); 13] = [
            ("runs/r.jsonl", gate.runs.join("r.jsonl"), [protected; 5]),
            (
                "pruned/r.jsonl",
                layout.pruned().join("r.jsonl"),
                [protected; 5],
            ),
            (
                "worktrees/other/x",
                layout.worktree(OTHER).join("x"),
                [protected; 5],
            ),
            (
                "worktrees/own/x",
                gate.worktree.join("x"),
                ["capability", "out-of-scope", "capability", "allow", "allow"],
            ),
            ("spawn/own/gate", gate.script.clone(), [protected; 5]),
            (
                "spawn/own/seed-index",
                layout.seed_index(RUN),
                [protected; 5],
            ),
            (
                "spawn/other/gate",
                layout.gate_script(OTHER),
                [protected; 5],
            ),
            (
                "skills/naming/SKILL.md",
                gate.skills.join("naming/SKILL.md"),
                [protected; 5],
            ),
            (
                "skills/.git/HEAD",
                gate.skills.join(".git/HEAD"),
                [protected; 5],
            ),
            (
                "skills-snapshots/deadbeef/SKILL.md",
                gate.snapshot.join("SKILL.md"),
                [protected; 5],
            ),
            (
                "skill-worktrees/other/p.md",
                layout.skill_checkout(OTHER).join("p.md"),
                [protected; 5],
            ),
            (
                // The one row the carve-out turns on: `curate` owns this
                // checkout and the other four policies were never given one.
                "skill-worktrees/own/p.md",
                layout.skill_checkout(RUN).join("p.md"),
                [protected, "allow", protected, protected, protected],
            ),
            (
                "parent/.git/HEAD",
                gate.parent_git.join("HEAD"),
                [protected; 5],
            ),
        ];
        for (probe, path, want) in rows {
            matrix_cell(policy, probe, want[column], &gate.write_to(&path));
        }
    }
}

/// What one shell probe has to come back as.
///
/// `Refused` is the row where the path check and the pattern floor both apply
/// and which of them answers first is not worth pinning. Everywhere else the
/// code is the assertion, because a deny for the wrong reason is a deny the next
/// edit moves back.
#[derive(Clone, Copy)]
enum Want {
    Code(&'static str),
    Refused,
}

#[test]
fn a_shell_command_the_matrix_allowed_is_refused_now_unless_it_is_the_runs_own_scratch() {
    // The same matrix through the capability that carries no path. `standard`
    // was measured deleting a sibling's worktree, the store's `.git`, the
    // snapshots and the parent repo's `.git`, and running `git gc`,
    // `git prune` and `git worktree remove --force`. None of that loses the
    // record and every one of them loses something a record cannot rebuild.
    //
    // The last five rows are decision 11. The run's own scratch stays
    // discardable, because those lose only the run's own uncommitted edits and
    // the record carries every tool call either way.
    for policy in ["standard", "permissive"] {
        let gate = Gate::new(policy);
        let layout = gate.layout();
        let runs = gate.runs.display().to_string();
        let protected = Want::Code("protected-path");
        let floor = Want::Code("floor");
        let allow = Want::Code("allow");
        let rows: [(&str, String, Want); 27] = [
            ("the corpus", format!("rm -rf {runs}"), protected),
            (
                "the corpus by rename",
                format!("mv {runs} {runs}.old"),
                protected,
            ),
            (
                "the corpus by find",
                format!("find {runs} -delete"),
                protected,
            ),
            (
                "the pruned records",
                format!("rm -rf {}", layout.pruned().display()),
                protected,
            ),
            (
                "a sibling's worktree",
                format!("rm -rf {}", layout.worktree(OTHER).display()),
                protected,
            ),
            (
                "every worktree",
                format!("rm -rf {}", layout.worktrees().display()),
                protected,
            ),
            (
                "a sibling's spawn directory",
                format!("rm -rf {}", layout.spawn(OTHER).display()),
                protected,
            ),
            (
                "its own spawn directory",
                format!("rm -rf {}", layout.spawn(RUN).display()),
                protected,
            ),
            (
                "the store's git directory",
                format!("rm -rf {}", gate.skills.join(".git").display()),
                protected,
            ),
            (
                "the snapshots",
                format!("rm -rf {}", layout.snapshots().display()),
                protected,
            ),
            (
                "a sibling's store checkout",
                format!("rm -rf {}", layout.skill_checkout(OTHER).display()),
                protected,
            ),
            (
                "the parent repo's git directory",
                format!("rm -rf {}", gate.parent_git.display()),
                protected,
            ),
            (
                "a sibling's worktree through git",
                format!(
                    "git worktree remove --force {}",
                    layout.worktree(OTHER).display()
                ),
                Want::Refused,
            ),
            ("git gc", "git gc --prune=now".to_string(), floor),
            ("git prune", "git prune --expire=now".to_string(), floor),
            (
                "git worktree remove --force",
                "git worktree remove --force /somewhere/else".to_string(),
                floor,
            ),
            (
                "git worktree prune",
                "git worktree prune".to_string(),
                floor,
            ),
            (
                "git worktree add",
                "git worktree add /somewhere/else".to_string(),
                floor,
            ),
            (
                "git branch -D",
                "git branch -D hydra/other".to_string(),
                floor,
            ),
            ("git reset --hard", "git reset --hard".to_string(), floor),
            ("git clean -fdx", "git clean -fdx".to_string(), floor),
            (
                "git update-ref -d",
                "git update-ref -d refs/heads/x".to_string(),
                floor,
            ),
            ("git worktree list", "git worktree list".to_string(), allow),
            ("git checkout -- .", "git checkout -- .".to_string(), allow),
            ("git stash drop", "git stash drop".to_string(), allow),
            ("git clean -f", "git clean -f".to_string(), allow),
            (
                "its own worktree",
                format!("rm -rf {}", gate.worktree.display()),
                allow,
            ),
        ];
        for (probe, command, want) in rows {
            let decision = gate.shell(&command);
            match want {
                Want::Code(code) => matrix_cell(policy, probe, code, &decision),
                Want::Refused => assert!(
                    !decision.allowed,
                    "{policy} reached {probe}: {}",
                    decision.reason
                ),
            }
        }
    }

    // The three policies with no shell refuse every one of those, and the
    // reason is capability or a path denial depending on which the command
    // names first. Only the refusal is asserted.
    for policy in ["judge", "curate", "read-only"] {
        let gate = Gate::new(policy);
        for command in [
            "rm -rf",
            "git gc --prune=now",
            "git worktree list",
            "git clean -f",
        ] {
            let decision = gate.shell(command);
            assert!(
                !decision.allowed,
                "{policy} has no shell and ran {command}: {}",
                decision.reason
            );
        }
    }
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
            .denied_for("hydra/protected-path"),
        "the store's working tree is denied to every policy since ticket 40"
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
    // Out of the run's own worktree, up past the directory that holds every
    // run's worktree, and into the corpus.
    assert!(
        gate.write_to(&gate.worktree.join("../../runs/stolen.jsonl"))
            .denied_for("hydra/protected-path")
    );
    // One level up is a sibling's ground rather than the corpus, and the deny
    // list names the parent for exactly this reason: siblings appear and
    // disappear while a run is live.
    assert!(
        gate.write_to(&gate.worktree.join("../stolen"))
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
    // 2026-09-06, and the test below is where it lives now. `ToolSearch`
    // followed it on map ticket 43, same day, and sits in the same test.
    for tool in [
        "CronCreate",
        "SendMessage",
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
fn every_policy_that_may_invoke_a_skill_may_also_fetch_a_tool_schema() {
    // Map ticket 43, 2026-09-06. `claude` 2.1.263 hands MCP tools to a print-mode
    // run deferred: the probe's `deferred_tools_delta` listed
    // `mcp__hydra-probe__ping` beside fifteen deferred built-ins, and a deferred
    // tool needs `ToolSearch` before anything can call it. A policy that admits a
    // server and denies `ToolSearch` has admitted a listing entry the model
    // cannot reach, which is ticket 39's failure on a second tool.
    //
    // `ToolSearch` fetches schemas and executes nothing. The gate judges the call
    // that follows, which is why this sits beside `Skill` rather than beside
    // `mcp_call`.
    for policy in ["read-only", "standard", "curate", "permissive"] {
        let decision = Gate::new(policy).ask("ToolSearch", json!({ "query": "select:Read" }));
        assert!(
            decision.allowed,
            "{policy} may invoke a skill and cannot fetch a tool schema: {}",
            decision.reason
        );
    }
    let judge = Gate::new("judge").ask("ToolSearch", json!({ "query": "select:Read" }));
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
fn a_write_through_a_skill_link_lands_on_a_denied_snapshot() {
    // The snapshot is a read scope and appears on no policy's `file_write`. A
    // run that could edit it could edit its own instructions mid-session.
    //
    // Ticket 40 moved the refusal from the policy's scope check to 7.5's deny
    // list, which is what closes the same write under `permissive`: `anywhere`
    // used to reach a snapshot and now stops at the denial.
    for policy in ["standard", "permissive"] {
        let gate = Gate::new(policy);
        let link = gate.link_skill("hydra-sentinel");
        assert!(
            gate.write_to(&link.join("SKILL.md"))
                .denied_for("hydra/protected-path"),
            "{policy} rewrote its own instructions mid-session"
        );
        assert!(
            gate.write_to(&link.join("new-file.md"))
                .denied_for("hydra/protected-path")
        );
    }
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
            .cursor_tool(
                "Shell",
                json!({ "command": "echo hi", "cwd": "", "timeout": 30000 })
            )
            .denied_for("hydra/capability")
    );
    assert!(
        read_only
            .cursor_tool(
                "Write",
                json!({ "file_path": "/etc/hosts", "content": "x" })
            )
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
fn the_cursor_gate_config_carries_no_unhooked_tool() {
    // Map ticket 51: the three tools Cursor runs without asking the hook stay
    // out of the compiled config, so a hook that never fires is not a config
    // the gate would answer differently for. ReadLints used to sit here as
    // `file_read` on `file_path`; dropping it changes nothing at run time
    // because the hook does not fire.
    let gate = Gate::cursor("standard");
    let config = compiled_config(&gate.script);
    for name in ["ReadLints", "getMcpToolsToolCall", "awaitToolCall"] {
        assert!(
            config["tools"].get(name).is_none(),
            "{name} is unhooked and the gate never sees it"
        );
        assert!(
            config["path_fields"].get(name).is_none(),
            "{name} has no path field in the compiled config"
        );
    }
    assert_eq!(config["tools"]["Read"], "file_read");
    assert_eq!(config["path_fields"]["Read"], "file_path");
}

#[test]
fn a_cursor_run_may_read_its_own_cursor_data() {
    // Build ticket 38: Cursor answers `awaitToolCall` by reading the
    // background task's output under the run's own `cursor-data/`, and that
    // read fires the hook. Spawn is a denied parent, so without the carve-out
    // the await is refused as out-of-scope.
    for policy in ["standard", "read-only"] {
        let gate = Gate::cursor(policy);
        let own = gate
            .layout()
            .cursor_data_dir(RUN)
            .join("projects/slug/terminals/1.txt");
        std::fs::create_dir_all(own.parent().unwrap()).unwrap();
        std::fs::write(&own, "slept\n").unwrap();

        let before = gate.cursor_read(&own);
        assert!(
            before.allowed,
            "{policy} beforeReadFile of own cursor-data: {}",
            before.reason
        );
        let read = gate.cursor_tool("Read", json!({ "file_path": own.to_string_lossy() }));
        assert!(
            read.allowed,
            "{policy} Read of own cursor-data: {}",
            read.reason
        );

        let sibling = gate
            .layout()
            .spawn(OTHER)
            .join("cursor-data/projects/slug/terminals/1.txt");
        std::fs::create_dir_all(sibling.parent().unwrap()).unwrap();
        std::fs::write(&sibling, "slept\n").unwrap();
        assert!(
            gate.cursor_read(&sibling).denied_for("hydra/out-of-scope"),
            "{policy} should refuse a sibling's spawn"
        );
        assert!(
            gate.cursor_tool("Read", json!({ "file_path": sibling.to_string_lossy() }))
                .denied_for("hydra/out-of-scope"),
            "{policy} Read of a sibling's spawn"
        );

        let spawn_root = gate.layout().spawns();
        assert!(
            gate.cursor_read(&spawn_root)
                .denied_for("hydra/out-of-scope"),
            "{policy} should refuse ~/hydra/spawn/ itself"
        );
        assert!(
            gate.cursor_tool("Read", json!({ "file_path": spawn_root.to_string_lossy() }))
                .denied_for("hydra/out-of-scope"),
            "{policy} Read of ~/hydra/spawn/"
        );
    }

    let judge = Gate::cursor("judge");
    let own = judge
        .layout()
        .cursor_data_dir(RUN)
        .join("projects/slug/terminals/1.txt");
    std::fs::create_dir_all(own.parent().unwrap()).unwrap();
    std::fs::write(&own, "slept\n").unwrap();
    assert!(
        judge.cursor_read(&own).denied_for("hydra/capability"),
        "judge still refuses a read of own cursor-data"
    );
    assert!(
        judge
            .cursor_tool("Read", json!({ "file_path": own.to_string_lossy() }))
            .denied_for("hydra/capability")
    );
}

fn compiled_config(script: &Path) -> Value {
    let text = std::fs::read_to_string(script).unwrap();
    let line = text
        .lines()
        .find(|line| line.starts_with("HYDRA_CONFIG = json.loads("))
        .expect("the compiled gate carries a config line");
    let literal = line
        .trim_start_matches("HYDRA_CONFIG = json.loads(")
        .trim_end_matches(')');
    let inner: String = serde_json::from_str(literal).unwrap();
    serde_json::from_str(&inner).unwrap()
}

#[test]
fn the_path_denials_hold_through_cursors_shell_too() {
    let gate = Gate::cursor("permissive");
    let target = gate.runs.join("r.jsonl");
    let decision = gate.cursor_tool(
        "Shell",
        json!({ "command": format!("echo x >> {}", target.display()), "cwd": "", "timeout": 30000 }),
    );
    assert!(
        decision.denied_for("hydra/protected-path"),
        "{}",
        decision.reason
    );
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
    // 7.5 states the denials as delivered outright: "a hook sees the path
    // and refuses the write before it reaches disk". `Bash` carries no path, so
    // until this landed the deny list was consulted for `file_write` alone and
    // every entry on it was reachable by redirect under any policy granting
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

// ---------------------------------------------------------------------------
// The agy dialect, harness spec 6.6, measured on 1.1.26
// ---------------------------------------------------------------------------

#[test]
fn an_agy_gate_answers_in_the_shape_its_own_reference_documents() {
    let gate = Gate::agy("standard");
    let decision = gate.agy_tool(
        "run_command",
        json!({
            "CommandLine": "echo T10PROBE",
            "Cwd": gate.worktree.to_string_lossy(),
            "WaitMsBeforeAsync": 5000,
        }),
    );
    assert!(decision.allowed, "{}", decision.reason);
    assert!(decision.reason.starts_with("hydra/allow:"));
    assert_eq!(decision.exit, 0);
}

#[test]
fn the_call_is_read_out_of_the_nested_toolcall_and_not_beside_it() {
    // The payload this provider sends carries no `tool_name`, no `tool_input`
    // and no `cwd`. A gate that read the claude shape here would find no tool
    // name and deny everything, which fails closed and gates nothing usefully.
    let gate = Gate::agy("read-only");
    let inside = gate.worktree.join("a.txt");
    let read = gate.agy_tool(
        "view_file",
        json!({ "AbsolutePath": inside.to_string_lossy() }),
    );
    assert!(read.allowed, "{}", read.reason);
    assert!(
        !read.denied_for("hydra/no-tool-name"),
        "the nested toolCall was not unwrapped: {}",
        read.reason
    );

    let outside = gate.agy_tool("view_file", json!({ "AbsolutePath": "/etc/hosts" }));
    assert!(
        outside.denied_for("hydra/out-of-scope"),
        "{}",
        outside.reason
    );
}

#[test]
fn agys_measured_field_names_are_the_ones_the_gate_reads() {
    // The seven measured on a real hook payload, each with the field the gate
    // scopes on. A wrong field name here is a `hydra/no-path` deny on every
    // scoped policy, which fails closed and makes the provider unusable.
    let gate = Gate::agy("standard");
    let inside = gate.worktree.join("probe.txt");
    for (tool, args) in [
        (
            "view_file",
            json!({ "AbsolutePath": inside.to_string_lossy() }),
        ),
        (
            "list_dir",
            json!({ "DirectoryPath": gate.worktree.to_string_lossy() }),
        ),
        (
            "write_to_file",
            json!({ "TargetFile": inside.to_string_lossy(), "CodeContent": "x" }),
        ),
        // Measured 2026-09-08 on 1.1.27: `SearchPath`, not the `SearchDirectory`
        // the table carried, which denied a run's first search `hydra/no-path`.
        (
            "grep_search",
            json!({ "Query": "usage", "SearchPath": gate.worktree.to_string_lossy() }),
        ),
        // Measured the same day on run 01a08008-cb50.
        (
            "find_by_name",
            json!({ "Pattern": "*.rs", "SearchDirectory": gate.worktree.to_string_lossy() }),
        ),
        // And on run 01a0801d-77b1.
        (
            "replace_file_content",
            json!({ "TargetFile": inside.to_string_lossy(), "ReplacementContent": "x" }),
        ),
    ] {
        let decision = gate.agy_tool(tool, args);
        assert!(decision.allowed, "{tool}: {}", decision.reason);
    }
    // And the same three outside it, which the scope refuses.
    for (tool, args) in [
        ("view_file", json!({ "AbsolutePath": "/etc/hosts" })),
        ("list_dir", json!({ "DirectoryPath": "/etc" })),
        ("grep_search", json!({ "Query": "x", "SearchPath": "/etc" })),
        (
            "find_by_name",
            json!({ "Pattern": "*.conf", "SearchDirectory": "/etc" }),
        ),
        (
            "replace_file_content",
            json!({ "TargetFile": "/etc/hosts", "ReplacementContent": "x" }),
        ),
        (
            "write_to_file",
            json!({ "TargetFile": "/tmp/t10-outside.txt", "CodeContent": "x" }),
        ),
    ] {
        let decision = gate.agy_tool(tool, args);
        assert!(
            decision.denied_for("hydra/out-of-scope"),
            "{tool}: {}",
            decision.reason
        );
    }
}

#[test]
fn a_relative_path_resolves_against_the_workspace_the_flag_named() {
    // No `cwd` field on this provider's payload, so the gate reads
    // `args.Cwd` when the tool sends one and `workspacePaths[0]` otherwise.
    // Without either, a relative path would resolve against whatever directory
    // the gate process happened to start in, which is `.agents/`.
    let gate = Gate::agy("read-only");
    let inside = gate.agy_tool("view_file", json!({ "AbsolutePath": "a.txt" }));
    assert!(inside.allowed, "{}", inside.reason);

    let outside = gate.agy_tool("view_file", json!({ "AbsolutePath": "../escape.txt" }));
    assert!(
        outside.denied_for("hydra/out-of-scope"),
        "{}",
        outside.reason
    );
}

#[test]
fn agy_names_its_mcp_tool_and_carries_no_prefix() {
    // `call_mcp_tool` is a row in the table rather than a prefix, so this
    // provider's compiled config has no `mcp_prefix` at all. An empty string
    // there would have matched every tool name and called the whole provider an
    // MCP call.
    let standard = Gate::agy("standard");
    assert!(
        standard
            .agy_tool("call_mcp_tool", json!({}))
            .denied_for("hydra/capability")
    );
    assert!(
        Gate::agy("permissive")
            .agy_tool("call_mcp_tool", json!({}))
            .allowed
    );
    // The other two providers' spellings mean nothing here.
    assert!(
        Gate::agy("permissive")
            .agy_tool("mcp__hydra__ping", json!({}))
            .denied_for("hydra/unknown-tool")
    );
}

#[test]
fn the_path_denials_hold_through_agys_shell_too() {
    let gate = Gate::agy("permissive");
    let target = gate.runs.join("r.jsonl");
    let decision = gate.agy_tool(
        "run_command",
        json!({ "CommandLine": format!("echo x >> {}", target.display()) }),
    );
    assert!(
        decision.denied_for("hydra/protected-path"),
        "{}",
        decision.reason
    );
}

#[test]
fn the_pattern_floor_reads_agys_own_command_field() {
    // `CommandLine`, not `command`. A floor pointed at the wrong field sees an
    // empty string and refuses nothing.
    let gate = Gate::agy("standard");
    let decision = gate.agy_tool(
        "run_command",
        json!({ "CommandLine": "git push --force origin main" }),
    );
    assert!(decision.denied_for("hydra/floor"), "{}", decision.reason);
}

#[test]
fn a_claude_tool_name_means_nothing_to_an_agy_gate() {
    let gate = Gate::agy("permissive");
    assert!(
        gate.agy_tool("Bash", json!({ "command": "echo hi" }))
            .denied_for("hydra/unknown-tool")
    );
}

#[test]
fn an_agy_gate_refuses_to_be_asked_in_another_dialect() {
    let gate = Gate::agy("permissive");
    let (value, _) = gate.raw_answer(
        Some("claude"),
        json!({ "tool_name": "run_command", "tool_input": { "CommandLine": "echo hi" } }),
    );
    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("hydra/wrong-dialect"),
        "{value}"
    );
}

#[test]
fn a_read_through_an_agy_skill_link_lands_in_the_snapshot() {
    // 6.4 and ticket 39, on this provider's root. The provider names the link
    // path as the skill's base directory, and the gate resolves the link before
    // comparing, so a sibling opened through it is inside the snapshot scope
    // and the same path opened for writing is denied outright, since ticket 40
    // put the snapshots on 7.5's list.
    let gate = Gate::agy("read-only");
    let link = gate.link_agy_skill("t10-skill");
    let read = gate.agy_tool(
        "view_file",
        json!({ "AbsolutePath": link.join("SKILL.md").to_string_lossy() }),
    );
    assert!(read.allowed, "{}", read.reason);

    let write = Gate::agy("standard");
    let link = write.link_agy_skill("t10-skill");
    let refused = write.agy_tool(
        "write_to_file",
        json!({
            "TargetFile": link.join("SKILL.md").to_string_lossy(),
            "CodeContent": "rewritten",
        }),
    );
    assert!(
        refused.denied_for("hydra/protected-path"),
        "a run may read its own skills and write nowhere near them: {}",
        refused.reason
    );
}

#[test]
fn agys_subagent_tools_belong_to_permissive_alone() {
    // 7.6: an in-process subagent gets no run id and writes no run file, so
    // every policy but `permissive` refuses all four of this provider's
    // spellings.
    for tool in [
        "define_subagent",
        "invoke_subagent",
        "manage_subagents",
        "browser_subagent",
    ] {
        for policy in ["judge", "read-only", "standard", "curate"] {
            let decision = Gate::agy(policy).agy_tool(tool, json!({}));
            assert!(
                decision.denied_for("hydra/capability"),
                "{policy} allowed {tool}: {}",
                decision.reason
            );
        }
        assert!(
            Gate::agy("permissive").agy_tool(tool, json!({})).allowed,
            "{tool}"
        );
    }
}

// ---------------------------------------------------------------------------
// The run's own hook and provider config, build ticket 44
// ---------------------------------------------------------------------------

/// The files inside the worktree that decide what gates the run, spelled out
/// here rather than read off the library so a dropped entry fails a test.
const OWN_CONFIG_FILES: [&str; 7] = [
    ".cursor/hooks.json",
    ".cursor/mcp.json",
    ".agents/hooks.json",
    ".agents/mcp_config.json",
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".mcp.json",
];

/// The skill roots Hydra links into, one per provider. A write into one
/// replaces a link, so the roots are denied whole.
const OWN_SKILL_ROOTS: [&str; 3] = [".claude/skills", ".cursor/skills", ".agents/skills"];

impl Gate {
    /// A write in the dialect this gate was compiled for, so the same probe
    /// asks each provider's own write tool with its own field name.
    fn write_as(&self, provider: Provider, path: &Path) -> Decision {
        let path = path.to_string_lossy();
        match provider {
            Provider::Claude => self.write_to(Path::new(path.as_ref())),
            Provider::CursorAgent => {
                self.cursor_tool("Write", json!({ "file_path": path, "content": "x" }))
            }
            Provider::Agy => self.agy_tool(
                "write_to_file",
                json!({ "TargetFile": path, "CodeContent": "x" }),
            ),
        }
    }
}

#[test]
fn a_run_cannot_rewrite_its_own_hook_or_provider_config_under_any_policy() {
    // Every provider's files on every provider's gate: a `claude` run has no
    // `.cursor/hooks.json`, and one it writes lands on the branch `hydra reap`
    // commits all the same.
    for provider in [Provider::Claude, Provider::CursorAgent, Provider::Agy] {
        for policy in MATRIX_POLICIES {
            let gate = Gate::compile_for(provider, policy, false);
            for rel in OWN_CONFIG_FILES {
                let decision = gate.write_as(provider, &gate.worktree.join(rel));
                assert!(
                    decision.denied_for("hydra/protected-path"),
                    "{provider} under {policy} let a write reach {rel}: {}",
                    decision.reason
                );
            }
            for root in OWN_SKILL_ROOTS {
                let target = gate.worktree.join(root).join("planted/SKILL.md");
                let decision = gate.write_as(provider, &target);
                assert!(
                    decision.denied_for("hydra/protected-path"),
                    "{provider} under {policy} let a write reach {root}: {}",
                    decision.reason
                );
            }
        }
    }
}

#[test]
fn the_config_denial_takes_nothing_else_out_of_the_worktree() {
    // The ordinary coding case still passes on every dialect under `standard`,
    // and so does the ignore file Hydra drops beside its config: a repo may
    // track its own, and the ignore file decides what reap stages rather than
    // what the gate allows.
    for provider in [Provider::Claude, Provider::CursorAgent, Provider::Agy] {
        let gate = Gate::compile_for(provider, "standard", false);
        for rel in ["src/new.rs", ".cursor/.gitignore", ".agents/.gitignore"] {
            let decision = gate.write_as(provider, &gate.worktree.join(rel));
            assert!(
                decision.allowed,
                "{provider} under standard refused {rel}: {}",
                decision.reason
            );
        }
    }
}

#[test]
fn the_compiled_gate_carries_the_config_denials_resolved_inside_the_worktree() {
    let gate = Gate::cursor("standard");
    let config = compiled_config(&gate.script);
    let listed: Vec<String> = config["protected_worktree_paths"]
        .as_array()
        .unwrap_or_else(|| panic!("protected_worktree_paths missing from {config}"))
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(listed.len(), OWN_CONFIG_FILES.len(), "{listed:?}");
    // The skill roots ride on their own key, read by the write check alone.
    let roots = config["write_denied_worktree_paths"]
        .as_array()
        .unwrap_or_else(|| panic!("write_denied_worktree_paths missing from {config}"));
    assert_eq!(roots.len(), OWN_SKILL_ROOTS.len(), "{roots:?}");
    let worktree = std::fs::canonicalize(&gate.worktree)
        .unwrap()
        .to_string_lossy()
        .to_lowercase();
    for entry in &listed {
        assert!(
            entry.starts_with(&worktree),
            "{entry} is not inside the resolved worktree {worktree}"
        );
    }
    assert!(
        listed
            .iter()
            .any(|entry| entry.ends_with(".cursor/hooks.json")),
        "{listed:?}"
    );
}

#[test]
fn a_denied_path_inside_a_larger_token_is_still_named() {
    // Build ticket 56. The token pass sees whole tokens, so a path spelled in
    // full inside one walked through: an adversarial review on 2026-09-14 got
    // `allow` for all three of these against a real compiled gate. The function
    // claims to refuse a command that names a denied path, and these name one.
    let gate = Gate::new("standard");
    let runs = gate.runs.display();
    for command in [
        format!("sh -c 'cat {runs}/x.jsonl'"),
        format!("bash -lc \"echo pwn >> {runs}/x.jsonl\""),
        format!("python3 -c \"open('{runs}/pwn.jsonl','w')\""),
        format!("echo x > $'{runs}/x'"),
        format!("zsh -c \"rm -rf {runs}\""),
    ] {
        let decision = gate.shell(&command);
        assert!(
            decision.denied_for("hydra/protected-path"),
            "{command}: {}",
            decision.reason
        );
    }
}

#[test]
fn the_raw_pass_leaves_an_ordinary_command_alone() {
    // The coarseness has a bound: the raw pass looks for denied paths and
    // nothing else, so the ordinary working day still passes, and so does a
    // command naming the run's own worktree, which the own carve-out covers.
    let gate = Gate::new("standard");
    let worktree = gate.worktree.display().to_string();
    for command in [
        "cargo test --release".to_string(),
        "git status --short".to_string(),
        "cat .claude/skills/unslop/SKILL.md".to_string(),
        format!("ls -la {worktree}/src"),
        format!("rg TODO {worktree}"),
    ] {
        let decision = gate.shell(&command);
        assert!(
            decision.allowed,
            "{command} was refused: {}",
            decision.reason
        );
    }
}

#[test]
fn a_shell_command_naming_the_skill_root_is_how_two_providers_read_a_skill() {
    // Run 01a09405-f451 ran `ls -la .claude/skills/` under standard and the
    // gate allowed it; the first version of build ticket 44 refused it, and the
    // corpus walk caught the difference. The roots are denied to writes only.
    // `Gate::shell` speaks the claude dialect, which is the one the run in
    // question ran under; the other two dialects share the floor code path.
    let gate = Gate::new("standard");
    for command in [
        "ls -la .claude/skills/",
        "cat .claude/skills/unslop/SKILL.md",
        "cat .cursor/skills/unslop/SKILL.md",
        "head -20 .agents/skills/unslop/SKILL.md",
    ] {
        let decision = gate.shell(command);
        assert!(
            decision.allowed,
            "standard refused a skill read, {command}: {}",
            decision.reason
        );
    }
}

#[test]
fn a_shell_command_naming_the_hook_config_is_refused_whole() {
    // The same list through the capability that carries no path, resolved
    // against the worktree the run is in. A read is refused too, which is the
    // coarseness 7.5 accepts on purpose.
    let gate = Gate::new("standard");
    for command in [
        "cat .cursor/hooks.json",
        "echo '{}' > .cursor/hooks.json",
        "rm -f .agents/hooks.json",
        "cp /dev/null .claude/settings.json",
    ] {
        let decision = gate.shell(command);
        assert!(
            decision.denied_for("hydra/protected-path"),
            "{command}: {}",
            decision.reason
        );
    }
    assert!(
        gate.shell("cat .cursor/.gitignore").allowed,
        "the ignore file is not on the list"
    );
}

#[test]
fn a_layout_root_rust_and_python_used_to_fold_differently_still_denies_the_corpus() {
    // Build ticket 60. Rust's to_lowercase left ß alone while Python's casefold
    // turned it into ss, so every deny entry and scope compiled for such a root
    // could not match and the gate failed open with no message. Harness spec
    // 15.2 asks for macOS filesystem case-insensitivity, which is ASCII-only.
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let suffix = format!(
        "fold-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    for (label, segment) in [("ascii control", "Hydra"), ("non-ascii root", "Straße")] {
        let root = std::env::temp_dir().join(segment).join(&suffix);
        let gate = Gate::build_at(Provider::Claude, "permissive", false, true, root);
        let decision = gate.write_to(&gate.runs.join("leak.jsonl"));
        assert!(
            decision.denied_for("hydra/protected-path"),
            "{label}: permissive wrote to the corpus at {}: {}",
            gate.runs.join("leak.jsonl").display(),
            decision.reason
        );
    }
}

#[test]
#[ignore = "build ticket 58: fails until a human adds -c and --config-env to \
            consume_before_subcommand in policies/floor.json, which is protected config #1. \
            Run it with `cargo test --test gate -- --ignored`."]
fn a_config_override_does_not_hide_the_subcommand_from_the_floor() {
    // Build ticket 58. `git -c key=value` and `git --config-env key=VAR` take a
    // separate value the way `-C` does, so a parser that does not consume it
    // reads the value as the subcommand and every push rule below stops
    // running. Measured on the compiled gate: `git push --force origin main`
    // was refused and `git -c core.pager=cat push --force origin main` was
    // allowed. The fix is two entries in `consume_before_subcommand` in
    // `policies/floor.json`, which is protected config #1 and a human's to make.
    let gate = Gate::new("standard");
    for command in [
        "git -c core.pager=cat push --force origin main",
        "git -c user.name=x -c user.email=y reset --hard",
        "git --config-env=http.proxy=PROXY push --force origin main",
        "git --config-env http.proxy=PROXY branch -D topic",
    ] {
        assert!(
            gate.shell(command).denied_for("hydra/floor"),
            "the subcommand hid behind a config override: {command}"
        );
    }
}
