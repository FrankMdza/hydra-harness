//! The spawn directory, harness spec 5, 6.5 and 6.6, without launching a provider.
//!
//! `tests/gate.rs` proves the compiled script answers correctly. This file proves
//! the run writes it where the run will find it, that `--settings` points at a
//! config naming that script and its dialect, and that both are real files created
//! fresh per spawn. `tests/mcp.rs` is the other half of the directory: what the
//! canonical document generates into `mcp.json` and what the launch check does
//! with it.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::run::write_spawn_config;

/// A layout per test, because the suite runs in parallel.
fn layout() -> (Layout, PathBuf) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-spawn-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    (layout, root)
}

fn settings_of(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn hook_command(settings: &serde_json::Value) -> String {
    settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn a_spawn_writes_a_gate_and_a_config_that_points_at_it() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let spawn =
        write_spawn_config(&layout, "r1", &set, Provider::Claude, "standard", Some(&worktree), false).unwrap();

    assert_eq!(spawn.gate, root.join("spawn/r1/gate"));
    assert_eq!(spawn.hook_config, root.join("spawn/r1/settings.json"));
    assert!(spawn.gate.is_file());
    assert!(spawn.hook_config.is_file());

    let command = hook_command(&settings_of(&spawn.hook_config));
    assert!(
        command.contains(spawn.gate.to_str().unwrap()),
        "--settings must point at the script this spawn wrote, got {command}"
    );
    assert!(
        command.ends_with(" claude"),
        "the dialect is argv[1] and is never inferred, got {command}"
    );
}

#[test]
fn the_gate_is_executable_and_is_a_real_file() {
    let (layout, _root) = layout();
    let set = PolicySet::discover().unwrap();
    let spawn = write_spawn_config(&layout, "r1", &set, Provider::Claude, "standard", None, false).unwrap();

    // 6.6: real files, never links. `cursor-agent` refuses a symlinked project
    // config silently and runs the command anyway, so this holds for all three.
    assert!(
        !spawn
            .gate
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        !spawn
            .hook_config
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );

    let mode = spawn.gate.metadata().unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700, "the provider runs the script as a command");
}

#[test]
fn every_spawn_gets_its_own_directory() {
    let (layout, _root) = layout();
    let set = PolicySet::discover().unwrap();
    let one = write_spawn_config(&layout, "r1", &set, Provider::Claude, "standard", None, false).unwrap();
    let two = write_spawn_config(&layout, "r2", &set, Provider::Claude, "read-only", None, false).unwrap();

    assert_ne!(one.gate, two.gate);
    let first = std::fs::read_to_string(&one.gate).unwrap();
    let second = std::fs::read_to_string(&two.gate).unwrap();
    assert_ne!(
        first, second,
        "two policies compiled to the same script, so one of them is not in force"
    );
    // The policy name is compiled into the script as JSON inside a Rust string
    // literal, so it reaches the file escaped.
    assert!(first.contains(r#"\"policy\":\"standard\""#));
    assert!(second.contains(r#"\"policy\":\"read-only\""#));
}

#[test]
fn the_gate_denies_writes_to_the_script_it_is_about_to_become() {
    let (layout, _root) = layout();
    let set = PolicySet::discover().unwrap();
    // 7.5's base case. The script is compiled before it is written, so a
    // placeholder path here would deny a file nothing ever writes to.
    let spawn = write_spawn_config(&layout, "r1", &set, Provider::Claude, "permissive", None, false).unwrap();
    let text = std::fs::read_to_string(&spawn.gate).unwrap();
    let resolved = std::fs::canonicalize(&spawn.gate)
        .unwrap()
        .to_string_lossy()
        .to_lowercase();
    assert!(
        text.contains(&resolved),
        "the compiled deny list does not name {resolved}"
    );
}

#[test]
fn a_run_with_no_worktree_scopes_writes_to_a_path_nothing_creates() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let spawn = write_spawn_config(&layout, "r1", &set, Provider::Claude, "standard", None, false).unwrap();
    let text = std::fs::read_to_string(&spawn.gate).unwrap();

    assert!(
        text.contains("no-worktree"),
        "with no repo the worktree scope must not fall back to the invocation directory"
    );
    assert!(!root.join("spawn/r1/no-worktree").exists());
}

/// Runs the hook command the config really carries, through a shell, the way the
/// provider runs it.
fn ask_through_the_hook(settings: &Path, payload: &str) -> String {
    let command = hook_command(&settings_of(settings));
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_hook_command_runs_the_gate_and_answers() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();
    let spawn =
        write_spawn_config(&layout, "r1", &set, Provider::Claude, "read-only", Some(&worktree), false).unwrap();

    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{{"file_path":"{}/x.txt","content":"x"}},"cwd":"{}"}}"#,
        worktree.display(),
        worktree.display()
    );
    let answer = ask_through_the_hook(&spawn.hook_config, &payload);
    assert!(
        answer.contains("\"deny\"") && answer.contains("hydra/capability"),
        "read-only should refuse a write through the real hook command, got {answer}"
    );
}

#[test]
fn a_rewritten_gate_fails_closed() {
    // 7.5's induction base case, and the half of it that does not depend on
    // reading argv. The provider snapshots the hook config at process start
    // (6.6), so the sha in the config is fixed before the child runs anything,
    // while the script is read fresh on every invocation. Emptying the script
    // used to mean an empty answer, which 6.6 measured `claude` treating as
    // permission to proceed.
    let (layout, _root) = layout();
    let set = PolicySet::discover().unwrap();
    let spawn = write_spawn_config(&layout, "r1", &set, Provider::Claude, "read-only", None, false).unwrap();

    let payload = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo hi"},"cwd":"/tmp"}"#;
    assert!(
        !ask_through_the_hook(&spawn.hook_config, payload).is_empty(),
        "the intact gate answers"
    );

    std::fs::write(&spawn.gate, "").unwrap();
    let answer = ask_through_the_hook(&spawn.hook_config, payload);
    assert!(
        answer.contains("\"deny\"") && answer.contains("hydra/gate-tampered"),
        "an emptied gate must deny rather than answer nothing, got {answer:?}"
    );

    std::fs::remove_file(&spawn.gate).unwrap();
    let answer = ask_through_the_hook(&spawn.hook_config, payload);
    assert!(
        answer.contains("hydra/gate-tampered"),
        "a deleted gate is the same case, got {answer:?}"
    );
}
