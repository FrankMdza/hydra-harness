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

use hydra::policy::replay::reason_code;
use hydra::provider::{Provider, codex};
use serde_json::{Value, json};

mod corpus;
use corpus::{Decision, Gate, OTHER, RUN, SHEBANG_PYTHON};

// ---------------------------------------------------------------------------
// The dialect
// ---------------------------------------------------------------------------

#[test]
fn the_batch_entry_answers_like_the_single_call_entry_on_every_dialect() {
    let claude = Gate::new("standard");
    let cursor = Gate::cursor("standard");
    let agy = Gate::agy("standard");

    let cases: [(&str, &Gate, Vec<Value>); 3] = [
        ("claude", &claude, {
            let other = claude.worktree.join("other-cwd");
            std::fs::create_dir_all(&other).unwrap();
            std::fs::create_dir_all(claude.worktree.join(".scratch-root")).unwrap();
            vec![
                claude.claude_payload(
                    "Read",
                    json!({ "file_path": "src/lib.rs" }),
                    &claude.worktree,
                ),
                claude.claude_payload(
                    "Bash",
                    json!({
                        "command": "HYDRA_ROOT=$PWD/.scratch-root cargo test --release --lib",
                        "description": "a probe",
                    }),
                    &claude.worktree,
                ),
                claude.claude_payload(
                    "Bash",
                    json!({
                        "command": "HYDRA_ROOT=$PWD/.scratch-root cargo test --release --lib",
                        "description": "another cwd",
                    }),
                    &other,
                ),
            ]
        }),
        ("cursor", &cursor, {
            let other = cursor.worktree.join("other-cwd");
            std::fs::create_dir_all(&other).unwrap();
            vec![
                cursor.cursor_payload(
                    "Write",
                    json!({
                        "file_path": cursor.worktree.join("src/new.rs").to_string_lossy(),
                        "content": "x",
                    }),
                    &cursor.worktree,
                ),
                cursor.cursor_payload(
                    "Shell",
                    json!({ "command": format!("ls {}", cursor.runs.to_string_lossy()) }),
                    &cursor.worktree,
                ),
                cursor.cursor_payload("Read", json!({ "file_path": "README.md" }), &other),
            ]
        }),
        ("agy", &agy, {
            let other = agy.worktree.join("other-cwd");
            std::fs::create_dir_all(&other).unwrap();
            vec![
                agy.agy_payload(
                    "read_file",
                    json!({ "TargetFile": "src/lib.rs" }),
                    &agy.worktree,
                ),
                agy.agy_payload(
                    "run_command",
                    json!({ "CommandLine": format!("ls {}", agy.runs.to_string_lossy()) }),
                    &agy.worktree,
                ),
                agy.agy_payload(
                    "write_to_file",
                    json!({
                        "TargetFile": other.join("planted.rs").to_string_lossy(),
                        "CodeContent": "x",
                    }),
                    &other,
                ),
            ]
        }),
    ];

    for (dialect, gate, payloads) in cases {
        // The single-call entry reads PWD from its environment, which on a
        // live run is the provider's and names the call's own directory. The
        // batch entry reads it off each payload instead, so each single call
        // here is handed the cwd its payload carries.
        let singles = payloads
            .iter()
            .map(|payload| {
                let cwd = payload["cwd"]
                    .as_str()
                    .or_else(|| payload["workspace_roots"][0].as_str())
                    .or_else(|| payload["workspacePaths"][0].as_str())
                    .map(Path::new);
                let (value, _) = gate.raw_answer_in(Some(dialect), payload.clone(), cwd);
                gate.decision_from(dialect, &value)
            })
            .collect::<Vec<_>>();
        let batch = gate.batch_ask(dialect, &payloads);
        assert_eq!(
            singles.len(),
            batch.len(),
            "{dialect}: the batch entry answered a different count"
        );
        for (index, (single, batched)) in singles.iter().zip(batch.iter()).enumerate() {
            assert_eq!(
                single.allowed, batched.allowed,
                "{dialect} payload {index}: allow mismatch (single {:?}, batch {:?})",
                single.reason, batched.reason
            );
            assert_eq!(
                single.reason, batched.reason,
                "{dialect} payload {index}: reason mismatch"
            );
        }
    }
}

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
    // 7.2 and rungs spec 3.6. The pass writes into its own git checkout of
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

#[test]
fn the_checkout_is_denied_and_the_own_worktree_still_writes() {
    // Map ticket 65: `hydra_home()` joins the deny list. The fixture's layout
    // root is a temp directory and `hydra_home()` is the real checkout, so a
    // write there is what a run under `permissive` could do to the binary and
    // the gate template before this ticket.
    let gate = Gate::new("permissive");
    let home = hydra::paths::hydra_home();
    for (what, path) in [
        ("the release binary", home.join("target/release/hydra")),
        ("the gate template", home.join("src/policy/gate.py")),
    ] {
        let decision = gate.write_to(&path);
        assert!(
            decision.denied_for("hydra/protected-path"),
            "permissive wrote to {what} at {}: {}",
            path.display(),
            decision.reason
        );
    }
    assert!(
        gate.write_to(&gate.worktree.join("notes.md")).allowed,
        "permissive's own worktree should still write"
    );
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
fn agent_gets_the_same_gate_answer_as_task_under_standard() {
    // Claude Code 2.1.281 renamed `Task` to `Agent`; the table keeps both.
    let gate = Gate::new("standard");
    let input = json!({ "prompt": "go" });
    let task = gate.ask_with_dialect(
        Some("claude"),
        gate.claude_payload("Task", input.clone(), &gate.worktree),
    );
    let agent = gate.ask_with_dialect(
        Some("claude"),
        gate.claude_payload("Agent", input, &gate.worktree),
    );
    assert_eq!(task.allowed, agent.allowed);
    assert_eq!(reason_code(&task.reason), reason_code(&agent.reason));
    assert!(
        agent.denied_for("hydra/capability"),
        "Agent was answered {}: {}",
        agent.allowed,
        agent.reason
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
    // Map ticket 51 and build ticket 113: the four tools Cursor runs without
    // asking the hook stay out of the compiled config, so a hook that never
    // fires is not a config the gate would answer differently for. ReadLints used to sit here as
    // `file_read` on `file_path`; dropping it changes nothing at run time
    // because the hook does not fire.
    let gate = Gate::cursor("standard");
    let config = compiled_config(&gate.script);
    for name in [
        "ReadLints",
        "getMcpToolsToolCall",
        "awaitToolCall",
        "WebSearch",
    ] {
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
fn a_claude_run_may_read_its_own_tool_results() {
    // Build ticket 123: Claude Code 2.1.281 writes large tool output under
    // `~/.claude/projects/<slug>/<session>/tool-results/` and reads it back.
    let gate = Gate::new("standard");
    let own_dir = hydra::provider::claude::projects_dir(&gate.worktree).unwrap();
    // Nothing is created under the real home: `gate.py` resolves the path with
    // `realpath`, which only needs the existing prefix, `~/.claude/projects`.
    let tool_result = own_dir.join("session-id/tool-results/x.txt");

    assert!(
        gate.read_from(&tool_result).allowed,
        "Read of own tool-results: {}",
        gate.read_from(&tool_result).reason
    );

    let other = gate.worktree.join("other-worktree");
    let sibling = hydra::provider::claude::projects_dir(&other)
        .unwrap()
        .join("session-id/tool-results/x.txt");
    assert!(
        gate.read_from(&sibling).denied_for("hydra/out-of-scope"),
        "Read of another slug's tool-results"
    );

    let settings = hydra::env::real_home()
        .unwrap()
        .join(".claude/settings.json");
    assert!(
        gate.read_from(&settings).denied_for("hydra/out-of-scope"),
        "Read of ~/.claude/settings.json"
    );

    assert!(
        gate.write_to(&own_dir.join("x.txt"))
            .denied_for("hydra/out-of-scope"),
        "Write into the carved-out directory"
    );
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

fn rewrite_compiled_config(script: &Path, config: &Value) {
    let text = std::fs::read_to_string(script).unwrap();
    let inner = serde_json::to_string(config).unwrap();
    let literal = serde_json::to_string(&inner).unwrap();
    let line = format!("HYDRA_CONFIG = json.loads({literal})");
    let replaced = text
        .lines()
        .filter(|existing| existing.starts_with("HYDRA_CONFIG = json.loads("))
        .count();
    assert_eq!(replaced, 1, "the compiled gate should hold one config line");
    let rewritten = text
        .lines()
        .map(|existing| {
            if existing.starts_with("HYDRA_CONFIG = json.loads(") {
                line.as_str()
            } else {
                existing
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(script, format!("{rewritten}\n")).unwrap();
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
fn a_protected_path_split_or_assembled_still_walks_past() {
    // **These pin what is still open**, build ticket 77. The raw pass is a
    // substring test, so a denied path is refused when its prefix appears whole
    // and in canonical form. A path split by a quote, assembled by
    // concatenation, percent-encoded, or spelled with `..`, `//` or `/./` inside
    // an interpreter or nested-shell token is not. Neither is one split across
    // an MCP call's fields.
    let gate = Gate::new("standard");
    let root = gate.root.display();
    let runs = gate.runs.display();
    assert!(
        gate.shell(&format!("cat {}/x.jsonl", runs))
            .denied_for("hydra/protected-path"),
        "the plain spelling must stay refused"
    );

    // A quote splits the prefix inside a nested shell.
    // If this now denies, the residual is closed and this comment is stale;
    // rewrite the assertion and SECURITY.md.
    let split_quote = format!("sh -c 'cat {}/ru\"ns\"/x.jsonl'", root);
    assert!(
        gate.shell(&split_quote).allowed,
        "if this now denies, the residual is closed and the comment is stale"
    );

    // Concatenation inside an interpreter token.
    // If this now denies, the residual is closed and this comment is stale;
    // rewrite the assertion and SECURITY.md.
    let python_concat = format!("python3 -c \"open('{}/ru' + 'ns/x.jsonl')\"", root);
    assert!(
        gate.shell(&python_concat).allowed,
        "if this now denies, the residual is closed and the comment is stale"
    );

    // The same `worktrees/..` launder reaches skills, skills-snapshots, pruned,
    // spawn/<other-run> and skill-worktrees/<other-run>; one pin is enough.
    // If this now denies, the residual is closed and this comment is stale;
    // rewrite the assertion and SECURITY.md.
    let dotdot = format!("sh -c 'cat {}/worktrees/../runs/x.jsonl'", root);
    assert!(
        gate.shell(&dotdot).allowed,
        "if this now denies, the residual is closed and the comment is stale"
    );

    // A doubled slash and a dot segment.
    // If this now denies, the residual is closed and this comment is stale;
    // rewrite the assertion and SECURITY.md.
    for doubled in [
        format!("sh -c 'cat {}//runs/x.jsonl'", root),
        format!("sh -c 'cat {}/./runs/x.jsonl'", root),
    ] {
        assert!(
            gate.shell(&doubled).allowed,
            "if this now denies, the residual is closed and the comment is stale"
        );
    }

    // Percent-encoding in a fetch URL.
    // If this now denies, the residual is closed and this comment is stale;
    // rewrite the assertion and SECURITY.md.
    let encoded = format!("file://{}/ru%6es/x.jsonl", root);
    assert!(
        gate.ask("WebFetch", json!({ "url": encoded, "prompt": "read it" }))
            .allowed,
        "if this now denies, the residual is closed and the comment is stale"
    );

    // An MCP call under `permissive`, with a doubled slash or the path split
    // across two fields. Both spellings and the control below are built from
    // this gate's own root, so the pins hold only while the gate that is asked
    // really lets its own corpus through.
    let permissive = Gate::new("permissive");
    let permissive_root = permissive.root.display();
    // If this now denies, the residual is closed and this comment is stale;
    // rewrite the assertion and SECURITY.md.
    for arguments in [
        json!({ "path": format!("{}//runs/x.jsonl", permissive_root) }),
        json!({
            "dir": format!("{}/ru", permissive_root),
            "name": "ns/x.jsonl",
        }),
    ] {
        assert!(
            permissive.ask("mcp__x__write", arguments).allowed,
            "if this now denies, the residual is closed and the comment is stale"
        );
    }
    let corpus = format!("{}/runs/x.jsonl", permissive_root);
    assert_eq!(
        Path::new(&corpus),
        permissive.runs.join("x.jsonl"),
        "the control names the same corpus the respellings reach for"
    );
    assert!(
        permissive
            .ask("mcp__x__write", json!({ "path": corpus }))
            .denied_for("hydra/protected-path"),
        "the plain MCP spelling must stay refused"
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
const OWN_CONFIG_FILES: [&str; 9] = [
    ".cursor/hooks.json",
    ".cursor/mcp.json",
    ".agents/hooks.json",
    ".agents/mcp_config.json",
    ".codex/hooks.json",
    ".codex/config.toml",
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".mcp.json",
];

/// The skill roots Hydra links into, one per provider. A write into one
/// replaces a link, so the roots are denied whole.
const OWN_SKILL_ROOTS: [&str; 4] = [
    ".claude/skills",
    ".cursor/skills",
    ".agents/skills",
    ".codex/skills",
];

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
            Provider::Codex => self.apply_patch(&format!(
                "*** Begin Patch\n*** Add File: {}\n+x\n*** End Patch\n",
                path
            )),
        }
    }
}

#[test]
fn a_run_cannot_rewrite_its_own_hook_or_provider_config_under_any_policy() {
    // Every provider's files on every provider's gate: a `claude` run has no
    // `.cursor/hooks.json`, and one it writes lands on the branch `hydra reap`
    // commits all the same.
    for provider in [
        Provider::Claude,
        Provider::CursorAgent,
        Provider::Agy,
        Provider::Codex,
    ] {
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
    for provider in [
        Provider::Claude,
        Provider::CursorAgent,
        Provider::Agy,
        Provider::Codex,
    ] {
        let gate = Gate::compile_for(provider, "standard", false);
        for rel in [
            "src/new.rs",
            ".cursor/.gitignore",
            ".agents/.gitignore",
            ".codex/.gitignore",
        ] {
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

// --- build ticket 59 ------------------------------------------------------

#[test]
fn a_shell_facility_carrying_no_command_is_denied_under_every_policy() {
    // `WriteShellStdin` on `cursor-agent` and `send_command_input` on `agy`
    // write to the stdin of a command that is already running. The command was
    // judged when it started; what is typed into it afterwards is not, and the
    // gate has no string to read, so neither the pattern floor nor the path
    // denials of 7.5 apply. They were `shell` rows until this ticket, which is
    // how a run under `permissive` could feed a protected path to a shell it had
    // already opened.
    for policy in hydra::policy::NAMES {
        let cursor = Gate::compile_for(Provider::CursorAgent, policy, false);
        let said = cursor.cursor_tool("WriteShellStdin", json!({ "input": "echo hello\n" }));
        assert!(
            said.denied_for("hydra/capability"),
            "{policy} allowed WriteShellStdin: {}",
            said.reason
        );

        let agy = Gate::compile_for(Provider::Agy, policy, false);
        let said = agy.agy_tool("send_command_input", json!({ "Input": "echo hello\n" }));
        assert!(
            said.denied_for("hydra/capability"),
            "{policy} allowed send_command_input: {}",
            said.reason
        );
    }
}

#[test]
fn reading_a_running_commands_output_is_still_shell() {
    // The other half of the pair, and the reason they no longer share a
    // capability: collecting output is not a command the gate never saw. It
    // stays `shell`, so a policy that denies shell still denies it and one that
    // grants shell still grants it.
    let denied = Gate::agy("read-only");
    let said = denied.agy_tool("command_status", json!({}));
    assert!(
        said.denied_for("hydra/capability"),
        "read-only granted a shell facility: {}",
        said.reason
    );
    let granted = Gate::agy("standard");
    let said = granted.agy_tool("command_status", json!({}));
    assert!(
        said.allowed,
        "standard refused a read of a running command's output: {}",
        said.reason
    );
}

#[test]
fn an_mcp_call_naming_a_protected_path_is_refused_under_permissive() {
    // Build ticket 59's second half. `evaluate` reached the denials of 7.5
    // through two doors, a `file_write` with a path and a `shell` with a command
    // string, and an MCP call is neither: its arguments are whatever its server
    // declared, so the table names no path field and `target_path` reads
    // nothing. `config/mcp.md` admits no writing server today, so nothing had
    // fired.
    let gate = Gate::compile_for(Provider::Claude, "permissive", false);
    let corpus = gate.runs.join("stolen.jsonl");

    for arguments in [
        json!({ "path": corpus.to_string_lossy() }),
        // At depth, because an MCP server's arguments are its own shape.
        json!({ "edits": [{ "target": { "file": corpus.to_string_lossy() } }] }),
    ] {
        let decision = gate.ask("mcp__writer__put", arguments);
        assert!(
            decision.denied_for("hydra/protected-path"),
            "permissive let an MCP call write the corpus: {}",
            decision.reason
        );
    }

    // And an MCP call that names nothing protected is still allowed, so the
    // check is a deny list rather than a wall in front of every server.
    let ordinary = gate.ask(
        "mcp__writer__put",
        json!({ "path": gate.worktree.join("note.md").to_string_lossy() }),
    );
    assert!(
        ordinary.allowed,
        "the check refused an MCP call inside the run's own worktree: {}",
        ordinary.reason
    );
}

#[test]
fn apply_patch_adding_inside_the_worktree_is_allowed_under_standard() {
    let gate = Gate::compile_for(Provider::Codex, "standard", false);
    let inside = gate.worktree.join("src/new.rs");
    let patch = format!(
        "*** Begin Patch\n*** Add File: {}\n+fn main() {{}}\n*** End Patch\n",
        inside.display()
    );
    let decision = gate.apply_patch(&patch);
    assert!(
        decision.allowed,
        "standard refused a patch inside the worktree: {}",
        decision.reason
    );
}

#[test]
fn apply_patch_updating_under_the_corpus_is_refused() {
    let gate = Gate::compile_for(Provider::Codex, "standard", false);
    let target = gate.runs.join("r.jsonl");
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-x\n+y\n*** End Patch\n",
        target.display()
    );
    let decision = gate.apply_patch(&patch);
    assert!(
        decision.denied_for("hydra/protected-path"),
        "standard let a patch reach the corpus: {}",
        decision.reason
    );
}

#[test]
fn apply_patch_with_one_path_inside_and_one_outside_is_refused() {
    let gate = Gate::compile_for(Provider::Codex, "standard", false);
    let inside = gate.worktree.join("src/inside.rs");
    let patch = format!(
        "*** Begin Patch\n*** Add File: {}\n+x\n*** Update File: src/old.rs\n*** Move to: /tmp/outside.rs\n*** End Patch\n",
        inside.display()
    );
    let decision = gate.apply_patch(&patch);
    assert!(
        decision.denied_for("hydra/out-of-scope"),
        "standard let a patch name a path outside the worktree: {}",
        decision.reason
    );
}

#[test]
fn codex_judge_is_not_refused_at_preflight() {
    assert!(
        codex::unsupported_policy("judge").is_none(),
        "judge on codex is routable since build ticket 61"
    );
    let message = codex::unsupported_policy("read-only").expect("read-only stays refused");
    assert!(message.contains("build ticket 48 item 4"), "{message}");
    assert!(message.contains("build ticket 61"), "{message}");
}

// Five shapes measured on 2026-09-22 against a compiled gate of a live run,
// before the fixes they pin. A gate that dies is exit 1 and nothing on stdout,
// which `codex` reads as permission and `claude` as a non-blocking error.

#[test]
fn a_payload_whose_cwd_is_not_a_string_is_answered_rather_than_crashed() {
    let gate = Gate::new("standard");
    let decision = gate.ask_with_dialect(
        Some("claude"),
        json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": "ls" },
            "cwd": 5,
        }),
    );
    assert_eq!(decision.exit, 0, "the gate died: {}", decision.reason);
    assert!(
        decision.allowed,
        "a benign command with a malformed cwd was refused: {}",
        decision.reason
    );
}

#[test]
fn a_single_call_whose_cwd_holds_a_nul_is_answered_as_before_the_batch_entry() {
    // The single-call entry is the live hook's. It reads PWD from the
    // environment the provider handed it and never from the payload, so a cwd
    // `os.environ` would refuse cannot reach `os.environ` on this path.
    // Measured 2026-09-22 on master's gate before build ticket 86: this payload
    // was answered, allowed, exit 0.
    let gate = Gate::new("standard");
    let decision = gate.ask_with_dialect(
        Some("claude"),
        json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": "ls" },
            "cwd": "/tmp/a\u{0}b",
        }),
    );
    assert_eq!(decision.exit, 0, "the gate died: {}", decision.reason);
    assert!(
        decision.allowed && decision.reason.starts_with("hydra/allow"),
        "the single-call answer changed: {}",
        decision.reason
    );
}

#[test]
fn evaluate_raise_is_answered_as_a_deny() {
    // `decide_payload`'s catch-all around `evaluate`, not the batch entry's PWD
    // handler. A scope path that is not a string dies in `inside`.
    let gate = Gate::new("standard");
    let mut config = compiled_config(&gate.script);
    config["scopes"]["worktree"] = json!(5);
    rewrite_compiled_config(&gate.script, &config);
    let decision = gate.ask("Read", json!({ "file_path": "src/lib.rs" }));
    assert!(
        decision.denied_for("hydra/gate-error"),
        "a broken scope was answered {}: {}",
        decision.allowed,
        decision.reason
    );
    assert!(
        decision.reason.contains("while evaluating this call"),
        "the batch PWD handler answered instead: {}",
        decision.reason
    );
}

#[test]
fn a_batch_payload_whose_cwd_holds_a_nul_costs_that_payload_alone() {
    // The batch entry sets PWD from each payload's cwd, and `os.environ`
    // raises ValueError on a NUL. That is one deny for that payload, and the
    // payload after it is answered as the single-call entry answers it.
    let gate = Gate::new("standard");
    let bad = gate.claude_payload(
        "Bash",
        json!({ "command": "ls" }),
        Path::new("/tmp/a\u{0}b"),
    );
    let good = gate.claude_payload("Read", json!({ "file_path": "src/lib.rs" }), &gate.worktree);
    let answers = gate.batch_ask("claude", &[bad, good.clone()]);
    assert_eq!(answers.len(), 2);
    assert!(
        answers[0].denied_for("hydra/gate-error"),
        "a NUL cwd in a batch was answered {}: {}",
        answers[0].allowed,
        answers[0].reason
    );
    let single = gate.ask_with_dialect(Some("claude"), good);
    assert_eq!(answers[1].allowed, single.allowed, "{}", answers[1].reason);
    assert_eq!(answers[1].reason, single.reason);
}

#[test]
fn a_shell_row_whose_command_is_not_a_string_is_refused() {
    let gate = Gate::new("standard");
    for input in [json!({ "command": 123 }), json!({ "command": null })] {
        let decision = gate.ask("Bash", input.clone());
        assert!(
            decision.denied_for("hydra/bad-payload"),
            "{input} was answered {}: {}",
            decision.allowed,
            decision.reason
        );
    }
}

#[test]
fn a_write_that_names_no_path_is_refused_under_permissive_too() {
    let gate = Gate::new("permissive");
    let decision = gate.ask("Write", json!({ "content": "x" }));
    assert!(
        decision.denied_for("hydra/no-path"),
        "permissive let a write with no path through: {}",
        decision.reason
    );
}

#[test]
fn a_payload_nested_past_the_interpreter_limit_is_a_deny_and_not_a_traceback() {
    // `json.loads` raises RecursionError on this, which is not a ValueError,
    // and before 2026-09-22 escaped `main` as a traceback with exit 1.
    let gate = Gate::new("standard");
    let nested = format!("{}1{}", "[".repeat(5000), "]".repeat(5000));
    let mut child = Command::new(SHEBANG_PYTHON)
        .arg(&gate.script)
        .arg("claude")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 is on PATH");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(nested.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "the gate died: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("hydra/bad-payload"),
        "a document nested past the limit was answered: {stdout}"
    );
}

#[test]
fn a_fetch_whose_url_spells_the_corpus_is_refused() {
    let gate = Gate::new("standard");
    let url = format!("file://{}/r.jsonl", gate.runs.display());
    let decision = gate.ask("WebFetch", json!({ "url": url, "prompt": "read it" }));
    assert!(
        decision.denied_for("hydra/protected-path"),
        "a file URL naming the corpus was answered {}: {}",
        decision.allowed,
        decision.reason
    );
}

// ---------------------------------------------------------------------------
// Build ticket 77: the gate reads the payload it was compiled for
// ---------------------------------------------------------------------------

#[test]
fn a_shell_tool_input_that_is_not_an_object_is_refused() {
    let gate = Gate::new("standard");
    let runs = gate.runs.join("x.jsonl");
    for tool_input in [
        json!(format!("cat {}", runs.display())),
        json!([format!("cat {}", runs.display())]),
    ] {
        let decision = gate.ask_with_dialect(
            Some("claude"),
            json!({
                "cwd": gate.worktree.to_string_lossy(),
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": tool_input,
            }),
        );
        assert!(
            decision.denied_for("hydra/bad-payload"),
            "{tool_input} was answered {}: {}",
            decision.allowed,
            decision.reason
        );
    }
}

#[test]
fn a_claude_gate_refuses_another_dialects_envelope() {
    let gate = Gate::new("standard");
    let runs = gate.runs.join("x.jsonl");
    let inside = gate.worktree.join("a.txt");

    let with_agy = gate.ask_with_dialect(
        Some("claude"),
        json!({
            "cwd": gate.worktree.to_string_lossy(),
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": format!("cat {}", runs.display()) },
            "toolCall": {
                "name": "Read",
                "args": { "file_path": inside.to_string_lossy() },
            },
        }),
    );
    assert!(
        with_agy.denied_for("hydra/wrong-envelope"),
        "agy's toolCall walked past: {}",
        with_agy.reason
    );

    let with_cursor = gate.ask_with_dialect(
        Some("claude"),
        json!({
            "cwd": gate.worktree.to_string_lossy(),
            "hook_event_name": "beforeReadFile",
            "file_path": inside.to_string_lossy(),
            "tool_name": "Bash",
            "tool_input": { "command": format!("cat {}", runs.display()) },
        }),
    );
    assert!(
        with_cursor.denied_for("hydra/wrong-envelope"),
        "cursor's beforeReadFile walked past: {}",
        with_cursor.reason
    );
}

#[test]
fn the_floor_refuses_slack_on_a_fetch_url_and_not_in_a_search_query() {
    let gate = Gate::new("standard");
    let slack_url = "https://slack.com/api/files.upload";
    assert!(
        gate.shell(&format!("curl {}", slack_url))
            .denied_for("hydra/floor"),
        "the shell floor still refuses curl to Slack"
    );
    assert!(
        gate.ask("WebFetch", json!({ "url": slack_url, "prompt": "post" }))
            .denied_for("hydra/floor"),
        "WebFetch to the same Slack method must meet the same floor"
    );
    assert!(
        gate.ask("WebFetch", json!({ "url": "https://example.com" }))
            .allowed,
        "an ordinary fetch is still allowed"
    );
    // A search query sends nothing to Slack, so the floor does not read it.
    let search = gate.ask(
        "WebSearch",
        json!({ "query": "slack api files.upload rate limits" }),
    );
    assert!(
        search.allowed,
        "a search that mentions files.upload was refused: {}",
        search.reason
    );
    // The floor reads the URL field and nothing beside it.
    assert!(
        gate.ask(
            "WebFetch",
            json!({ "url": "https://example.com", "prompt": "compare with files.upload" })
        )
        .allowed,
        "the prompt beside the URL is not the URL"
    );
}

#[test]
fn an_agy_run_command_whose_args_are_not_an_object_is_refused() {
    let gate = Gate::agy("standard");
    let runs = gate.runs.join("x.jsonl");
    for args in [
        json!(format!("cat {}", runs.display())),
        json!([format!("cat {}", runs.display())]),
    ] {
        let decision = gate.agy_tool("run_command", args.clone());
        assert!(
            decision.denied_for("hydra/bad-payload"),
            "{args} was answered {}: {}",
            decision.allowed,
            decision.reason
        );
    }
}

#[test]
fn a_missing_tool_input_is_allowed_for_a_tool_that_sends_none() {
    // `Task` and an `mcp__` tool carry no field the gate resolves a path from,
    // so a payload without `tool_input` reads as `{}` and `permissive` grants
    // both. Only a `tool_input` that is present and not an object is refused.
    let gate = Gate::new("permissive");
    for tool in ["Task", "Agent", "mcp__server__tool"] {
        let decision = gate.ask_with_dialect(
            Some("claude"),
            json!({
                "cwd": gate.worktree.to_string_lossy(),
                "hook_event_name": "PreToolUse",
                "tool_name": tool,
            }),
        );
        assert!(
            decision.allowed,
            "{tool} without tool_input was refused: {}",
            decision.reason
        );
    }
}

#[test]
fn a_tool_input_that_is_present_and_not_an_object_is_refused_even_when_null() {
    // Absent reads as `{}`. Present means the key is there, whatever it holds,
    // so an explicit null is refused like a bool, a number, a string or a list.
    let gate = Gate::new("permissive");
    for tool_input in [json!(null), json!(true), json!(1), json!("x"), json!([])] {
        let decision = gate.ask_with_dialect(
            Some("claude"),
            json!({
                "cwd": gate.worktree.to_string_lossy(),
                "hook_event_name": "PreToolUse",
                "tool_name": "Task",
                "tool_input": tool_input,
            }),
        );
        assert!(
            decision.denied_for("hydra/bad-payload"),
            "Task with tool_input {tool_input} was answered {}: {}",
            decision.allowed,
            decision.reason
        );
    }
    let missing = gate.ask_with_dialect(
        Some("claude"),
        json!({
            "cwd": gate.worktree.to_string_lossy(),
            "hook_event_name": "PreToolUse",
            "tool_name": "Task",
        }),
    );
    assert!(
        missing.allowed,
        "Task without tool_input was refused: {}",
        missing.reason
    );
}

#[test]
fn an_agy_args_that_is_present_and_not_an_object_is_refused_even_when_null() {
    // `wait` is a `provider_native` row `permissive` grants, and it names no
    // field the gate reads. A missing `args` is absent and allowed; an `args`
    // that is there and not an object is refused.
    let gate = Gate::agy("permissive");
    let payload = |call: Value| {
        json!({
            "conversationId": "b631e11b-4a99-43db-9f37-11a373cdd98a",
            "stepIdx": 2,
            "toolCall": call,
            "workspacePaths": [gate.worktree.to_string_lossy()],
        })
    };
    for args in [json!(null), json!(true), json!(1), json!("x"), json!([])] {
        let decision = gate.agy_ask(payload(json!({ "name": "wait", "args": args })));
        assert!(
            decision.denied_for("hydra/bad-payload"),
            "wait with args {args} was answered {}: {}",
            decision.allowed,
            decision.reason
        );
    }
    let missing = gate.agy_ask(payload(json!({ "name": "wait" })));
    assert!(
        missing.allowed,
        "wait without args was refused: {}",
        missing.reason
    );
}

#[test]
fn a_foreign_tool_call_key_is_refused_whatever_it_holds() {
    // Carrying `toolCall` is enough on a claude or a cursor gate. The call
    // beside it is an ordinary read the gate would otherwise allow.
    let claude = Gate::new("standard");
    let cursor = Gate::cursor("standard");
    for value in [json!([]), json!(null), json!(1), json!("run_command")] {
        let with_claude = claude.ask_with_dialect(
            Some("claude"),
            json!({
                "cwd": claude.worktree.to_string_lossy(),
                "hook_event_name": "PreToolUse",
                "tool_name": "Read",
                "tool_input": { "file_path": "src/lib.rs" },
                "toolCall": value,
            }),
        );
        assert!(
            with_claude.denied_for("hydra/wrong-envelope"),
            "toolCall {value} on a claude gate was answered {}: {}",
            with_claude.allowed,
            with_claude.reason
        );
        let with_cursor = cursor.cursor_ask(json!({
            "conversation_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
            "tool_name": "Read",
            "tool_input": { "file_path": "README.md" },
            "hook_event_name": "preToolUse",
            "workspace_roots": [cursor.worktree.to_string_lossy()],
            "toolCall": value,
        }));
        assert!(
            with_cursor.denied_for("hydra/wrong-envelope"),
            "toolCall {value} on a cursor gate was answered {}: {}",
            with_cursor.allowed,
            with_cursor.reason
        );
    }
}

#[test]
fn a_cursor_gate_refuses_agys_envelope() {
    let gate = Gate::cursor("standard");
    let runs = gate.runs.join("x.jsonl");
    let inside = gate.worktree.join("a.txt");
    let decision = gate.cursor_ask(json!({
        "conversation_id": "8ccaa19c-fa4f-42d5-82a4-732806f7a312",
        "tool_name": "Shell",
        "tool_input": { "command": format!("cat {}", runs.display()) },
        "hook_event_name": "preToolUse",
        "workspace_roots": [gate.worktree.to_string_lossy()],
        "toolCall": {
            "name": "view_file",
            "args": { "AbsolutePath": inside.to_string_lossy() },
        },
    }));
    assert!(
        decision.denied_for("hydra/wrong-envelope"),
        "agy's toolCall walked past a cursor gate: {}",
        decision.reason
    );
}

#[test]
fn an_agy_gate_refuses_cursors_envelope() {
    let gate = Gate::agy("standard");
    let runs = gate.runs.join("x.jsonl");
    let decision = gate.agy_ask(json!({
        "hook_event_name": "beforeReadFile",
        "file_path": gate.worktree.join("a.txt").to_string_lossy(),
        "toolCall": {
            "name": "run_command",
            "args": { "CommandLine": format!("cat {}", runs.display()) },
        },
        "workspacePaths": [gate.worktree.to_string_lossy()],
    }));
    assert!(
        decision.denied_for("hydra/wrong-envelope"),
        "cursor's beforeReadFile walked past an agy gate: {}",
        decision.reason
    );
}
