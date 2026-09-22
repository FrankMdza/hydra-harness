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
use hydra::run::{refuse_foreign_checkout, write_spawn_config};

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

/// An empty repository at `dir`. One `git init` is enough: nothing here commits,
/// and `--git-common-dir` answers as soon as the directory is a repository.
fn git_init(dir: &Path) {
    let out = std::process::Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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
fn a_launch_from_a_foreign_checkout_is_refused_before_writing() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-spawn-foreign-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    let checkout = root.join("checkout");
    std::fs::create_dir_all(&checkout).unwrap();
    let err = refuse_foreign_checkout(&layout, &checkout).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains(root.display().to_string().as_str()),
        "{message}"
    );
    assert!(
        message.contains(checkout.display().to_string().as_str()),
        "{message}"
    );
    assert!(message.contains("HYDRA_ROOT"), "{message}");
    assert!(
        !layout.runs().exists(),
        "the refusal wrote into the data root"
    );
}

#[test]
fn a_spawn_writes_a_gate_and_a_config_that_points_at_it() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();

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
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "standard",
        None,
        None,
        None,
        false,
    )
    .unwrap();

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
    let one = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "standard",
        None,
        None,
        None,
        false,
    )
    .unwrap();
    let two = write_spawn_config(
        &layout,
        "r2",
        &set,
        Provider::Claude,
        "read-only",
        None,
        None,
        None,
        false,
    )
    .unwrap();

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
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "permissive",
        None,
        None,
        None,
        false,
    )
    .unwrap();
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
fn the_parent_repos_git_directory_reaches_the_deny_list() {
    // 7.5's one denied path outside `~/hydra`. `hydra run` knows the parent
    // because it ran `git worktree add` from it, and that directory is shared by
    // every worktree: it holds every run's branch and, on a machine where the
    // parent is Hydra's own checkout, Hydra's own history. Ticket 40 measured a
    // `standard` run's shell allowed to `rm -rf` it.
    //
    // A real repository rather than a directory named `.git`, because the path
    // is read back out of `git rev-parse --git-common-dir` and git answers that
    // for a repository alone. From inside a linked worktree the same call
    // answers with the parent's directory, which is the whole reason it is the
    // call rather than a `join(".git")`.
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let repo = root.join("parent");
    std::fs::create_dir_all(&repo).unwrap();
    git_init(&repo);
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "permissive",
        Some(&worktree),
        Some(&repo),
        None,
        false,
    )
    .unwrap();
    let text = std::fs::read_to_string(&spawn.gate).unwrap();
    let git_dir = std::fs::canonicalize(repo.join(".git"))
        .unwrap()
        .to_string_lossy()
        .to_lowercase();
    assert!(
        text.contains(&git_dir),
        "the compiled deny list does not name {git_dir}"
    );

    // A run outside a repo has no parent to protect, and nothing is invented
    // for it.
    let orphan = write_spawn_config(
        &layout,
        "r2",
        &set,
        Provider::Claude,
        "permissive",
        None,
        None,
        None,
        false,
    )
    .unwrap();
    assert!(
        !std::fs::read_to_string(&orphan.gate)
            .unwrap()
            .contains(&git_dir)
    );
}

#[test]
fn a_run_with_no_worktree_scopes_writes_to_a_path_nothing_creates() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "standard",
        None,
        None,
        None,
        false,
    )
    .unwrap();
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
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "read-only",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();

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

// ---------------------------------------------------------------------------
// cursor-agent: the config lives in the worktree, 6.6's table
// ---------------------------------------------------------------------------

fn cursor_hook(config: &serde_json::Value, event: &str) -> String {
    config["hooks"][event][0]["command"]
        .as_str()
        .unwrap_or_else(|| panic!("no {event} hook in {config}"))
        .to_string()
}

#[test]
fn a_cursor_spawn_writes_the_hook_config_into_the_worktree() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::CursorAgent,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();

    // The script stays outside the worktree it guards (7.5); only the pointer
    // to it goes inside, because that is where this provider reads it from.
    assert_eq!(spawn.gate, root.join("spawn/r1/gate"));
    assert_eq!(spawn.hook_config, worktree.join(".cursor/hooks.json"));
    assert!(spawn.hook_config.is_file());
    assert!(
        !spawn
            .hook_config
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink(),
        "cursor-agent refuses a symlinked project config silently and runs the command anyway"
    );

    let config = settings_of(&spawn.hook_config);
    assert_eq!(config["version"], 1);
    for event in ["preToolUse", "beforeReadFile"] {
        let command = cursor_hook(&config, event);
        assert!(
            command.contains(spawn.gate.to_str().unwrap()),
            "{event}: {command}"
        );
        assert!(command.ends_with(" cursor"), "{event}: {command}");
        assert_eq!(config["hooks"][event][0]["failClosed"], true, "{event}");
    }

    // The two files Hydra writes into `.cursor/` are kept out of `git status`
    // by name, and nothing else in that directory is.
    let ignore = std::fs::read_to_string(worktree.join(".cursor/.gitignore")).unwrap();
    assert!(ignore.contains("/hooks.json"));
    assert!(ignore.contains("/mcp.json"));
    assert!(!ignore.contains('*'));
}

#[test]
fn a_cursor_spawn_without_a_worktree_is_refused() {
    // A run outside a repo has nowhere Hydra owns to put the config, and a
    // provider Hydra cannot gate is not launched (harness spec 2).
    let (layout, _root) = layout();
    let set = PolicySet::discover().unwrap();
    let err = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::CursorAgent,
        "standard",
        None,
        None,
        None,
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("worktree"), "{err:#}");
}

#[test]
fn a_checkout_that_already_holds_a_hook_config_is_refused_rather_than_overwritten() {
    // The worktree is seeded from the human's repo. Their `.cursor/hooks.json`
    // is theirs: writing over it edits their work, and loading it instead of
    // Hydra's launches an ungated run.
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(worktree.join(".cursor")).unwrap();
    std::fs::write(
        worktree.join(".cursor/hooks.json"),
        "{\"version\":1,\"hooks\":{}}",
    )
    .unwrap();

    let err = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::CursorAgent,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err:#}");
    assert_eq!(
        std::fs::read_to_string(worktree.join(".cursor/hooks.json")).unwrap(),
        "{\"version\":1,\"hooks\":{}}",
        "the human's file was touched"
    );
}

#[test]
fn the_cursor_hook_command_gates_a_read_through_before_read_file() {
    // The command the config really carries, through a shell, with the payload
    // shape measured on 2026.09.02-c22c1a3: no `tool_name`, `file_path` at the
    // top level, and the content beside it.
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::CursorAgent,
        "read-only",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();
    let config = settings_of(&spawn.hook_config);

    let run = |command: &str, payload: &str| -> serde_json::Value {
        let mut child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
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
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
            panic!(
                "not JSON: {:?} / {:?}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        })
    };

    let outside = run(
        &cursor_hook(&config, "beforeReadFile"),
        r#"{"hook_event_name":"beforeReadFile","file_path":"/etc/hosts","content":"x","attachments":[]}"#,
    );
    assert_eq!(outside["permission"], "deny", "{outside}");
    assert!(
        outside["user_message"]
            .as_str()
            .unwrap()
            .contains("hydra/out-of-scope"),
        "{outside}"
    );

    let inside = run(
        &cursor_hook(&config, "beforeReadFile"),
        &format!(
            r#"{{"hook_event_name":"beforeReadFile","file_path":"{}/a.txt","content":"x","attachments":[]}}"#,
            worktree.display()
        ),
    );
    assert_eq!(inside["permission"], "allow", "{inside}");

    let write = run(
        &cursor_hook(&config, "preToolUse"),
        &format!(
            r#"{{"hook_event_name":"preToolUse","tool_name":"Write","tool_input":{{"file_path":"{}/a.txt","content":"x"}},"tool_use_id":"t"}}"#,
            worktree.display()
        ),
    );
    assert_eq!(write["permission"], "deny", "{write}");
    assert!(
        write["user_message"]
            .as_str()
            .unwrap()
            .contains("hydra/capability"),
        "{write}"
    );

    // The tamper wrapper speaks the same shape, or a rewritten script would be
    // an invalid response that `failClosed` blocks without a reason.
    std::fs::write(&spawn.gate, "").unwrap();
    let tampered = run(
        &cursor_hook(&config, "preToolUse"),
        r#"{"hook_event_name":"preToolUse","tool_name":"Shell","tool_input":{"command":"echo hi"}}"#,
    );
    assert_eq!(tampered["permission"], "deny", "{tampered}");
    assert!(
        tampered["user_message"]
            .as_str()
            .unwrap()
            .contains("hydra/gate-tampered"),
        "{tampered}"
    );
}

#[test]
fn a_cursor_gate_carves_out_its_own_cursor_data_for_reads() {
    // Build ticket 38: the compiled config, not the policy file, is what names
    // the run's own `cursor-data/`. A `claude` launch carries the key and an
    // empty list.
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let cursor = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::CursorAgent,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();
    let config = compiled_config(&cursor.gate);
    let expected = std::fs::canonicalize(layout.spawn("r1"))
        .unwrap()
        .join("cursor-data")
        .to_string_lossy()
        .to_lowercase();
    let own = config["own_read_paths"]
        .as_array()
        .unwrap_or_else(|| panic!("own_read_paths missing from {config}"));
    assert!(
        own.iter().any(|v| v.as_str() == Some(expected.as_str())),
        "cursor-agent should carve out {expected}, got {own:?}"
    );

    let claude = write_spawn_config(
        &layout,
        "r2",
        &set,
        Provider::Claude,
        "standard",
        None,
        None,
        None,
        false,
    )
    .unwrap();
    let config = compiled_config(&claude.gate);
    let own = config["own_read_paths"]
        .as_array()
        .unwrap_or_else(|| panic!("own_read_paths missing from {config}"));
    assert!(
        own.is_empty(),
        "a claude launch should carry no own_read_paths entry, got {own:?}"
    );
}

/// The policy the compiler wrote into a gate script, read back.
fn compiled_config(gate: &Path) -> serde_json::Value {
    let script = std::fs::read_to_string(gate).unwrap();
    let line = script
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
fn a_rewritten_gate_fails_closed() {
    // 7.5's induction base case, and the half of it that does not depend on
    // reading argv. The provider snapshots the hook config at process start
    // (6.6), so the sha in the config is fixed before the child runs anything,
    // while the script is read fresh on every invocation. Emptying the script
    // used to mean an empty answer, which 6.6 measured `claude` treating as
    // permission to proceed.
    let (layout, _root) = layout();
    let set = PolicySet::discover().unwrap();
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "read-only",
        None,
        None,
        None,
        false,
    )
    .unwrap();

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

// ---------------------------------------------------------------------------
// agy: the config lives in the worktree too, and the shape is its own
// ---------------------------------------------------------------------------

/// The gate command out of an `agy` hook config, which nests its handlers under
/// a `matcher` group rather than listing them flat.
fn agy_hook(config: &serde_json::Value) -> String {
    config["hydra-gate"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap_or_else(|| panic!("no PreToolUse handler in {config}"))
        .to_string()
}

#[test]
fn an_agy_spawn_writes_the_hook_config_into_the_worktree() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Agy,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();

    // The script stays outside the worktree it guards (7.5); only the pointer
    // goes inside, and this provider needs `--add-dir` to read even that.
    assert_eq!(spawn.gate, root.join("spawn/r1/gate"));
    assert_eq!(spawn.hook_config, worktree.join(".agents/hooks.json"));
    assert!(spawn.hook_config.is_file());
    assert!(
        !spawn
            .hook_config
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink(),
        "6.6 asks for real files, never links, on every provider"
    );

    let config = settings_of(&spawn.hook_config);
    let command = agy_hook(&config);
    assert!(command.contains(spawn.gate.to_str().unwrap()), "{command}");
    // The dialect is on the command line and never inferred (6.6).
    assert!(command.ends_with(" agy"), "{command}");
    assert_eq!(config["hydra-gate"]["PreToolUse"][0]["matcher"], "*");
    assert_eq!(config["hydra-gate"]["enabled"], true);
    // The schema has no switch for failing closed, so nothing here writes one:
    // a broken hook denies every tool on this provider and there is no flag.
    assert!(!config.to_string().contains("failClosed"));

    // The two files Hydra writes into `.agents/` are kept out of `git status` by
    // name. A `*` here would hide anything the child wrote in a `.agents/` the
    // human's repo tracks, and this provider reads `rules/` and `plugins/` out
    // of the same directory.
    let ignore = std::fs::read_to_string(worktree.join(".agents/.gitignore")).unwrap();
    assert!(ignore.contains("/hooks.json"));
    assert!(ignore.contains("/mcp_config.json"));
    assert!(!ignore.contains('*'));
}

#[test]
fn an_agy_spawn_without_a_worktree_is_refused() {
    let (layout, _root) = layout();
    let set = PolicySet::discover().unwrap();
    let err = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Agy,
        "standard",
        None,
        None,
        None,
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("worktree"), "{err:#}");
}

#[test]
fn an_agy_checkout_that_already_holds_a_hook_config_is_refused() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(worktree.join(".agents")).unwrap();
    std::fs::write(worktree.join(".agents/hooks.json"), "{}").unwrap();

    let err = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Agy,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err:#}");
    assert_eq!(
        std::fs::read_to_string(worktree.join(".agents/hooks.json")).unwrap(),
        "{}",
        "the human's file was touched"
    );
}

#[test]
fn the_agy_hook_command_gates_a_call_in_the_shape_the_provider_sends() {
    // The command the config really carries, through a shell, with the payload
    // measured on 1.1.26: the call nested under `toolCall`, camelCase, and no
    // `cwd` field anywhere.
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Agy,
        "read-only",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();

    let command = agy_hook(&settings_of(&spawn.hook_config));
    let ask = |payload: String| {
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
        String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
    };

    let inside = ask(format!(
        r#"{{"toolCall":{{"name":"view_file","args":{{"AbsolutePath":"{}/a.txt"}}}},"stepIdx":2,"workspacePaths":["{}"]}}"#,
        worktree.display(),
        worktree.display()
    ));
    assert!(inside.contains("\"allow\""), "{inside}");

    let outside = ask(format!(
        r#"{{"toolCall":{{"name":"view_file","args":{{"AbsolutePath":"/etc/hosts"}}}},"stepIdx":3,"workspacePaths":["{}"]}}"#,
        worktree.display()
    ));
    assert!(
        outside.contains("\"deny\"") && outside.contains("hydra/out-of-scope"),
        "{outside}"
    );

    // An emptied script is a deny in this provider's dialect too, and it is the
    // wrapper rather than the gate that says so.
    std::fs::write(&spawn.gate, "").unwrap();
    let tampered = ask(format!(
        r#"{{"toolCall":{{"name":"view_file","args":{{"AbsolutePath":"{}/a.txt"}}}},"stepIdx":4,"workspacePaths":["{}"]}}"#,
        worktree.display(),
        worktree.display()
    ));
    assert!(
        tampered.contains("hydra/gate-tampered") && tampered.contains("\"deny\""),
        "{tampered}"
    );
}

#[test]
fn an_interactive_policy_launch_writes_a_gate_and_settings() {
    let (layout, root) = layout();
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_init(&repo);
    let config = |key: &str, value: &str| {
        let out = std::process::Command::new("git")
            .args(["config", key, value])
            .current_dir(&repo)
            .output()
            .unwrap();
        assert!(out.status.success());
    };
    config("user.email", "test@example.invalid");
    config("user.name", "hydra tests");
    std::fs::write(repo.join("README"), "seed\n").unwrap();
    let out = std::process::Command::new("git")
        .args(["add", "README"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "git add failed");
    let out = std::process::Command::new("git")
        .args(["commit", "-qm", "seed", "README"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "git commit failed");

    let policies = PolicySet::discover().unwrap();
    let prep = hydra::interactive::prepare_policy_launch(
        &layout,
        "r1",
        "read-only",
        &policies,
        &repo,
        &repo,
    )
    .unwrap();

    assert_eq!(prep.spawn.gate, root.join("spawn/r1/gate"));
    assert_eq!(prep.spawn.hook_config, root.join("spawn/r1/settings.json"));
    assert!(prep.spawn.gate.is_file());
    assert!(prep.spawn.hook_config.is_file());

    let command = hook_command(&settings_of(&prep.spawn.hook_config));
    assert!(
        command.contains(prep.spawn.gate.to_str().unwrap()),
        "--settings must point at the script this spawn wrote, got {command}"
    );
    assert!(
        command.ends_with(" claude"),
        "the dialect is argv[1] and is never inferred, got {command}"
    );
}

// ---------------------------------------------------------------------------
// codex: hooks.json in the worktree and Claude's dialect on the command line
// ---------------------------------------------------------------------------

fn codex_hook(config: &serde_json::Value) -> String {
    config["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap_or_else(|| panic!("no PreToolUse handler in {config}"))
        .to_string()
}

#[test]
fn a_codex_spawn_writes_the_hook_config_into_the_worktree() {
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Codex,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();

    assert_eq!(spawn.gate, root.join("spawn/r1/gate"));
    assert_eq!(spawn.hook_config, worktree.join(".codex/hooks.json"));
    assert!(spawn.hook_config.is_file());
    assert!(
        !spawn
            .hook_config
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink(),
        "6.6 asks for real files, never links, on every provider"
    );

    let config = settings_of(&spawn.hook_config);
    let command = codex_hook(&config);
    assert!(command.contains(spawn.gate.to_str().unwrap()), "{command}");
    assert!(command.ends_with(" claude"), "{command}");
    assert_eq!(config["hooks"]["PreToolUse"][0]["matcher"], "*");

    // The hook the command line names, measured as the only route that fires
    // inside a worktree: the same Python as the inline command, in a file
    // beside the gate.
    let hook = root.join("spawn/r1/hook.py");
    let source = std::fs::read_to_string(&hook).unwrap();
    assert!(source.contains(spawn.gate.to_str().unwrap()), "{source}");
    assert!(source.contains(&spawn.gate_sha), "{source}");

    let ignore = std::fs::read_to_string(worktree.join(".codex/.gitignore")).unwrap();
    assert!(ignore.contains("/hooks.json"));
    assert!(ignore.contains("/config.toml"));
    assert!(!ignore.contains('*'));
}

// ---------------------------------------------------------------------------
// macOS seatbelt: profile beside the gate, build ticket 68
// ---------------------------------------------------------------------------

fn canonical_subpath(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

#[test]
fn a_spawn_writes_a_sandbox_profile_that_denies_hydra_paths() {
    if !hydra::sandbox::wraps_launches() {
        return;
    }
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();

    let profile = spawn.sandbox_profile.expect("macOS spawn writes a profile");
    assert!(profile.is_file(), "profile is a real file beside the gate");
    let text = std::fs::read_to_string(&profile).unwrap();
    let worktree_canon = canonical_subpath(&worktree).to_lowercase();

    let gate = layout.gate_script("r1");
    let policy_spawn = hydra::policy::Spawn {
        provider: Provider::Claude,
        policy: "standard",
        layout: &layout,
        worktree: &worktree,
        skill_store: &layout.skills(),
        corpus: &layout.runs(),
        skill_snapshot: &layout.skill_snapshot_scope("r1", None),
        skill_checkout: None,
        parent_git_dir: None,
        gate_script: &gate,
        floor_lifted: false,
    };
    for path in hydra::policy::denied_write_paths(&set, &policy_spawn) {
        let resolved = canonical_subpath(&path).to_lowercase();
        if hydra::policy::own_write_paths(&policy_spawn)
            .iter()
            .any(|own| resolved == canonical_subpath(own).to_lowercase())
        {
            continue;
        }
        assert!(
            text.to_lowercase().contains(&resolved),
            "profile does not deny {resolved}"
        );
    }
    assert!(
        !text.to_lowercase().contains(&format!(
            r#"(deny file-write* (subpath "{}"))"#,
            worktree_canon
        )),
        "profile denies the run's own worktree"
    );
}

#[test]
fn the_seatbelt_refuses_a_corpus_write_and_allows_the_worktree() {
    if !hydra::sandbox::exec_available() {
        return;
    }
    let (layout, root) = layout();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();
    layout.ensure().unwrap();

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();
    let profile = spawn.sandbox_profile.unwrap();
    let corpus = canonical_subpath(&layout.runs());
    let wt = canonical_subpath(&worktree);

    let corpus_write = std::process::Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(&profile)
        .arg("/bin/sh")
        .arg("-c")
        .arg(format!(
            "echo x > '{corpus}/seatbelt-test' 2>&1; echo rc=$?"
        ))
        .output()
        .unwrap();
    let corpus_out = String::from_utf8_lossy(&corpus_write.stdout);
    assert!(
        corpus_out.contains("rc=1") || corpus_out.to_lowercase().contains("not permitted"),
        "corpus write should fail, got {corpus_out}"
    );

    let worktree_write = std::process::Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(&profile)
        .arg("/bin/sh")
        .arg("-c")
        .arg(format!("echo x > '{wt}/seatbelt-test' 2>&1; echo rc=$?"))
        .output()
        .unwrap();
    let wt_out = String::from_utf8_lossy(&worktree_write.stdout);
    assert!(
        wt_out.contains("rc=0"),
        "worktree write should succeed, got {wt_out}"
    );
}
