//! The gate script, fed synthetic hook payloads on stdin.
//!
//! No provider is launched here. Ticket 01 compiles the script and asserts its
//! answers; ticket 02 is what wires it into a recorded `claude` run and finds
//! out whether the provider honours them.
//!
//! Every payload below is the shape ticket 26 measured coming out of `claude`
//! 2.1.257: `tool_name`, `tool_input`, `cwd`, and the rest of the envelope the
//! gate ignores.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::policy::{PolicySet, Spawn, write_gate};
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
}

impl Gate {
    fn compile(policy: &str, floor_lifted: bool) -> Self {
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
        let spawn_dir = root.join("spawn");
        for dir in [&worktree, &runs, &skills, &spawn_dir] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let script = spawn_dir.join("gate");

        let set = PolicySet::discover().unwrap();
        write_gate(
            &set,
            &Spawn {
                policy,
                worktree: &worktree,
                skill_store: &skills,
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
        }
    }

    fn new(policy: &str) -> Self {
        Self::compile(policy, false)
    }

    fn ask_with_dialect(&self, dialect: Option<&str>, payload: Value) -> Decision {
        // The interpreter the shebang names, never the one on this shell's PATH.
        // `python3` here is Homebrew's 3.14.7 and the gate runs on the 3.9.6
        // macOS ships, so testing the first would leave the second unmeasured.
        let mut command = Command::new(SHEBANG_PYTHON);
        command.arg(&self.script);
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
    let env_allowlist = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("env-allowlist.json");

    for policy in ["permissive", "curate"] {
        let gate = Gate::new(policy);
        let targets = [
            (
                "the delegation policy set",
                policies_dir.join("standard.json"),
            ),
            ("the environment allowlist", env_allowlist.clone()),
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
fn the_nineteen_provider_native_tools_belong_to_permissive_alone() {
    for tool in ["CronCreate", "SendMessage", "ToolSearch", "Skill"] {
        assert!(
            !Gate::new("standard").ask(tool, json!({})).allowed,
            "{tool} is provider_native and standard does not grant it"
        );
        assert!(Gate::new("permissive").ask(tool, json!({})).allowed);
    }
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
