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
