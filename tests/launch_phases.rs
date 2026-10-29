//! Build ticket 92, G4: a post-worktree handoff failure through [`hydra::run::run`]
//! writes `never_started` rather than leaving only an orphan checkout.

#[path = "common/git.rs"]
mod fixture_git;

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use hydra::paths::Layout;
use hydra::provider::Provider;
use hydra::run::{Request, run};

fn scratch_root() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "hydra-launch-phases-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn git_init(dir: &std::path::Path) {
    let out = fixture_git::command()
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    fixture_git::command()
        .args(["config", "user.email", "test@example.invalid"])
        .current_dir(dir)
        .status()
        .unwrap();
    fixture_git::command()
        .args(["config", "user.name", "hydra tests"])
        .current_dir(dir)
        .status()
        .unwrap();
    fixture_git::command()
        .args(["commit", "-q", "--allow-empty", "-m", "seed"])
        .current_dir(dir)
        .status()
        .unwrap();
}

#[test]
fn a_post_worktree_handoff_failure_through_run_writes_never_started() {
    if hydra::sandbox::inside_seatbelt() {
        eprintln!("skip: HYDRA_ROOT is refused from inside the seatbelt");
        return;
    }
    if !Command::new("claude")
        .arg("--version")
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skip: claude is not installed");
        return;
    }

    const CHILD: &str = "HYDRA_LAUNCH_PHASES_TEST_CHILD";
    const TEST: &str = "a_post_worktree_handoff_failure_through_run_writes_never_started";
    let root = if std::env::var_os(CHILD).is_some() {
        PathBuf::from(std::env::var_os("HYDRA_ROOT").unwrap())
    } else {
        let root = scratch_root();
        let out = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, "1")
            .env("HYDRA_ROOT", &root)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "child failed:\n{stdout}\n{stderr}");
        assert!(
            stdout.contains(&format!("test {TEST} ... ok")),
            "the child did not run the named test:\n{stdout}\n{stderr}"
        );
        let _ = std::fs::remove_dir_all(&root);
        return;
    };
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();

    let run_id = "01a09200-0000-7000-8000-000000000002";
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_init(&repo);

    std::fs::create_dir_all(layout.gate_script(run_id)).unwrap();

    let result = run(Request {
        provider: Provider::Claude,
        task: "probe".into(),
        model: None,
        policy: "judge".into(),
        floor_lifted: false,
        timeout: Duration::from_secs(0),
        parent: None,
        run_id: Some(run_id.to_string()),
        command: "run".into(),
        resumed_from: None,
        resume_session_id: None,
        working_dir: Some(repo),
        quiet: false,
        handed_off: None,
    });

    assert!(
        result.is_err(),
        "run should fail when the gate path is blocked"
    );

    let run_file = layout.run_file(run_id);
    assert!(run_file.is_file(), "no record was written for {run_id}");
    let text = std::fs::read_to_string(&run_file).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let end: serde_json::Value = serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
    assert!(end["hydra"]["exit_code"].is_null());
    assert!(end["hydra"]["refusal"].is_null());
    assert!(
        end["hydra"]["error"].as_str().unwrap().contains("gate"),
        "{end}"
    );

    let start: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(start["hydra"]["type"].as_str(), Some("run_start"));
    assert!(start["hydra"]["worktree"].is_object());

    let _ = std::fs::remove_dir_all(&root);
}

/// Build ticket 375. A stub `claude` on `PATH` answers `--version`, and when it
/// sees the probe model it writes once into the run's own worktree and once into
/// the corpus. The profile allows the first and denies the second. Before the
/// ticket the probe ran bare and both writes landed; build 370 measured the same
/// shape committing onto the run's branch in the parent's `refs/heads/`.
const PROBE_STUB: &str = r#"#!/bin/sh
case " $* " in
  *" --version "*) echo "2.1.281 (Claude Code)"; exit 0 ;;
  *" hydra-skill-preflight-not-a-model "*)
    : > probe-ran
    : > "$ESCAPE" 2>/dev/null
    exit 1 ;;
esac
: > run-ran
printf '{"type":"system","subtype":"init","model":"stub-1","session_id":"stub"}\n'
printf '{"type":"result","subtype":"success","is_error":false,"result":"done"}\n'
"#;

#[test]
fn the_probe_runs_inside_the_runs_wrapper() {
    if hydra::sandbox::inside_seatbelt() {
        eprintln!("skip: inside a seatbelt, where the binary refuses HYDRA_ROOT");
        return;
    }
    if !hydra::sandbox::wraps_launches() {
        eprintln!("skip: nothing wraps a launch on this machine, so nothing wraps the probe");
        return;
    }
    use std::os::unix::fs::PermissionsExt;

    let root = scratch_root();
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_init(&repo);

    let escape = layout.runs().join("probe-escaped");
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let stub = bin.join("claude");
    std::fs::write(
        &stub,
        PROBE_STUB.replace("\"$ESCAPE\"", &format!("'{}'", escape.display())),
    )
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args([
            "run",
            "--provider",
            "claude",
            "--policy",
            "standard",
            "--timeout",
            "60s",
            "--task",
            "probe stub",
        ])
        .current_dir(&repo)
        .env("HYDRA_ROOT", &root)
        .env("PATH", &path)
        .env_remove("HYDRA_RUN_ID")
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let worktree = std::fs::read_dir(root.join("worktrees"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .unwrap_or_else(|| panic!("no worktree was seeded: {said}"));
    // The control: the probe ran, and wrote where the profile lets a run write.
    assert!(
        worktree.join("probe-ran").exists(),
        "the probe never ran: {said}"
    );
    assert!(
        !escape.exists(),
        "the probe wrote into the corpus, so it ran outside the run's wrapper: {said}"
    );
    // A probe that fails is a line on stderr, and the launch goes on.
    assert!(
        worktree.join("run-ran").exists(),
        "the run did not launch: {said}"
    );
    let run_file = std::fs::read_dir(layout.runs())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .unwrap_or_else(|| panic!("no record was written: {said}"));
    let text = std::fs::read_to_string(&run_file).unwrap();
    let end: serde_json::Value = serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
    assert_eq!(end["hydra"]["type"].as_str(), Some("run_end"), "{text}");
    assert!(end["hydra"]["refusal"].is_null(), "{end}");

    let _ = std::fs::remove_dir_all(&root);
}

/// Writes `script` as `bin/<binary>` under `root`, with `$ESCAPE` pointing at a
/// file in the corpus, and returns the `PATH` that finds it first and that file.
fn install_probe_stub(root: &std::path::Path, binary: &str, script: &str) -> (String, PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let escape = Layout::at(root).runs().join("probe-escaped");
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let stub = bin.join(binary);
    std::fs::write(
        &stub,
        script.replace("\"$ESCAPE\"", &format!("'{}'", escape.display())),
    )
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    (path, escape)
}

/// The one checkout a launch under `root` seeded.
fn seeded_worktree(root: &std::path::Path, said: &str) -> PathBuf {
    std::fs::read_dir(root.join("worktrees"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .unwrap_or_else(|| panic!("no worktree was seeded: {said}"))
}

/// Build ticket 377. The session the stub stands in for exits at once, so the
/// launch needs no terminal. Before the ticket a gated interactive session ran
/// its skill probe before any profile existed, and the probe wrote into the
/// corpus.
const INTERACTIVE_STUB: &str = r#"#!/bin/sh
case " $* " in
  *" --version "*) echo "2.1.281 (Claude Code)"; exit 0 ;;
  *" hydra-skill-preflight-not-a-model "*)
    : > probe-ran
    : > "$ESCAPE" 2>/dev/null
    exit 1 ;;
esac
exit 0
"#;

#[test]
fn the_interactive_probe_runs_inside_the_sessions_wrapper() {
    if hydra::sandbox::inside_seatbelt() {
        eprintln!("skip: inside a seatbelt, where the binary refuses HYDRA_ROOT");
        return;
    }
    // The gated interactive mode narrows a seatbelt profile and refuses a
    // bubblewrap one, so only macOS gets as far as the probe.
    if !cfg!(target_os = "macos") {
        eprintln!("skip: the gated interactive mode runs under seatbelt alone");
        return;
    }

    // The seat socket lives under the spawn directory, and a root under macOS's
    // `/var/folders/...` temp dir pushes its path past `SUN_LEN`, which aborts
    // the launch and removes the worktree this test reads.
    let root = PathBuf::from(format!("/tmp/hlp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    Layout::at(&root).ensure().unwrap();
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_init(&repo);
    let (path, escape) = install_probe_stub(&root, "claude", INTERACTIVE_STUB);

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args([
            "run",
            "--interactive",
            "--provider",
            "claude",
            "--policy",
            "standard",
            "--task",
            "probe stub",
        ])
        .current_dir(&repo)
        .env("HYDRA_ROOT", &root)
        .env("PATH", &path)
        .env_remove("HYDRA_RUN_ID")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let worktree = seeded_worktree(&root, &said);
    // The control: the probe ran, and wrote where the profile lets a session write.
    assert!(
        worktree.join("probe-ran").exists(),
        "the probe never ran: {said}"
    );
    assert!(
        !escape.exists(),
        "the probe wrote into the corpus, so it ran outside the session's wrapper: {said}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Build ticket 377. Every `cursor-agent mcp` call of the pre-flight writes
/// once into the run's worktree and once into the corpus, and answers with an
/// empty listing so the launch goes on.
const CURSOR_STUB: &str = r#"#!/bin/sh
case " $* " in
  *" --version "*) echo "2026.10.01-stub"; exit 0 ;;
  " mcp "*)
    : > preflight-ran
    : > "$ESCAPE" 2>/dev/null
    exit 0 ;;
esac
: > run-ran
exit 0
"#;

#[test]
fn the_cursor_preflight_runs_inside_the_runs_wrapper() {
    if hydra::sandbox::inside_seatbelt() {
        eprintln!("skip: inside a seatbelt, where the binary refuses HYDRA_ROOT");
        return;
    }
    if !hydra::sandbox::wraps_launches() {
        eprintln!("skip: nothing wraps a launch on this machine, so nothing wraps the pre-flight");
        return;
    }

    let root = scratch_root();
    let _ = std::fs::remove_dir_all(&root);
    Layout::at(&root).ensure().unwrap();
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_init(&repo);
    let (path, escape) = install_probe_stub(&root, "cursor-agent", CURSOR_STUB);

    let out = Command::new(env!("CARGO_BIN_EXE_hydra"))
        .args([
            "run",
            "--provider",
            "cursor-agent",
            "--policy",
            "standard",
            "--timeout",
            "60s",
            "--task",
            "preflight stub",
        ])
        .current_dir(&repo)
        .env("HYDRA_ROOT", &root)
        .env("PATH", &path)
        .env_remove("HYDRA_RUN_ID")
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let worktree = seeded_worktree(&root, &said);
    assert!(
        worktree.join("preflight-ran").exists(),
        "the pre-flight never ran: {said}"
    );
    assert!(
        !escape.exists(),
        "the pre-flight wrote into the corpus, so it ran outside the run's wrapper: {said}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
