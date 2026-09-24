//! Does `claude` still drop the servers a human's settings define?
//!
//! Every other test in this suite launches no provider. This one does, and it is
//! worth the exception: harness spec 6.5's account of what a `judge` run gets is
//! a measurement of one build, ticket 23 read 2.1.257 one way and ticket 43 read
//! 2.1.263 the other, and nothing in Hydra notices when the answer moves. This
//! runs the free pre-flight probe under the `judge` argv and asserts the empty
//! set that argv is supposed to buy.
//!
//! **It costs no quota.** The probe passes a model name no catalog holds, so the
//! binary refuses before it calls the API and still writes the transcript that
//! carries the answer.
//!
//! **It never reads or writes the human's own config.** Both stub servers go
//! under a scratch `HOME` and a scratch cwd, and every `claude` call here gets
//! that `HOME`, the version check included. That is also what makes the
//! measurement possible: a user-scope server in the real `~/.claude.json` is the
//! human's to add, and ticket 43 refused to add one.
//!
//! **It skips rather than fails when `claude` is not on the path**, with the
//! reason on stdout, because a machine without the binary has nothing to say
//! about the binary.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::env::ChildEnv;
use hydra::mcp::Probe;
use hydra::provider::{Provider, claude, codex};

/// The stdio server both scopes point at, one tool and no dependencies.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp-probe-server.py")
}

/// A `HOME` and a cwd with one stub server each, in the two scopes settings
/// define.
struct Scratch {
    root: PathBuf,
    home: PathBuf,
    cwd: PathBuf,
}

fn scratch() -> Scratch {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-drift-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let scratch = Scratch {
        home: root.join("home"),
        cwd: root.join("cwd"),
        root,
    };
    std::fs::create_dir_all(&scratch.home).unwrap();
    std::fs::create_dir_all(&scratch.cwd).unwrap();

    let server = |name: &str| {
        serde_json::json!({
            "mcpServers": {
                name: {
                    "command": "/usr/bin/python3",
                    "args": [fixture().display().to_string()],
                }
            }
        })
    };
    // User scope and project scope, which are the two `--restricted` is supposed
    // to drop. A server on `--mcp-config` is the third and it survives the flag,
    // which is the whole reason `judge` carries `--strict-mcp-config` instead.
    std::fs::write(
        scratch.home.join(".claude.json"),
        serde_json::to_string_pretty(&server("hydra-user-scope")).unwrap(),
    )
    .unwrap();
    std::fs::write(
        scratch.cwd.join(".mcp.json"),
        serde_json::to_string_pretty(&server("hydra-project-scope")).unwrap(),
    )
    .unwrap();
    scratch
}

impl Scratch {
    /// The version string a failure message carries, or `None` when there is no
    /// binary to measure.
    ///
    /// Under the scratch `HOME` like every other call here, so a machine whose
    /// real home is unreadable behaves the same as one whose home is fine.
    fn version(&self) -> Option<String> {
        let out = Command::new("claude")
            .arg("--version")
            .env("HOME", &self.home)
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// The environment the probe runs under: the launch's own allowlist with the
    /// scratch `HOME` put back.
    ///
    /// `ChildEnv::build` drops a `HOME` that disagrees with the passwd entry,
    /// which is the right rule for a launch and the wrong one here, so the
    /// variable goes on afterwards.
    fn env(&self) -> ChildEnv {
        let mut env = ChildEnv::build(Provider::Claude).unwrap();
        env.set("HOME", self.home.display().to_string());
        env
    }

    /// A passing test leaves nothing behind. A failing one leaves the transcript,
    /// which is the evidence.
    fn clean(self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Runs Hydra's own probe for this launch shape and returns what it resolved.
///
/// The argv, the transcript path and both readers are the launch's, so a passing
/// test measures the pair a record depends on rather than a command written
/// twice.
fn resolve(scratch: &Scratch, shape: Probe<'_>) -> hydra::provider::Resolved {
    claude::resolve_in(&scratch.cwd, shape, &scratch.home, &scratch.env())
        .expect("the probe left no transcript")
}

#[test]
fn a_judge_run_still_starts_none_of_the_servers_settings_define() {
    let scratch = scratch();
    let Some(version) = scratch.version() else {
        println!("skipped: `claude` is not on the PATH, so there is nothing to measure");
        return;
    };

    let resolved = resolve(
        &scratch,
        Probe {
            policy: "judge",
            config: None,
        },
    );
    assert_eq!(
        resolved.mcp,
        Some(Vec::new()),
        "{version} resolved a server on the `judge` argv. Harness spec 6.5 says \
         that argv starts none, contract 5.3's `mcp.resolved` claims the empty \
         set on every `judge` record, and the gate refuses `mcp__` calls under \
         that policy, so what a server would add is tool descriptions in a \
         judge's context. Re-measure 6.5 and 6.7 against this build."
    );
    // The absence is Hydra's rule and not a missing attachment: a restricted probe
    // still writes a `skill_listing`, measured 2026-09-06 as the binary's own
    // twelve bundled skills with a user-scope and a project-scope skill on disk
    // and neither of them in it.
    assert!(
        resolved.skills.is_none(),
        "{version} answered the skill question on a `--restricted` probe, which \
         lists none of the skills a run is linked, so the answer would tell the \
         collision check that every one of them lost to something else"
    );
    scratch.clean();
}

#[test]
fn the_same_stubs_load_when_the_flags_come_off() {
    // The other half of the measurement, and the one that keeps the first from
    // passing for the wrong reason: a probe that resolved nothing because the
    // stubs were never readable would look identical.
    //
    // **`standard` with no config file is a shape no launch produces**, since
    // every non-`judge` `claude` launch writes a file even when it admits
    // nothing. That is fine here. This control measures the binary rather than a
    // launch, and it is the only shape `mcp_flags` has that passes no MCP flag at
    // all, which is what the vendor's default needs to be asked through.
    let scratch = scratch();
    let Some(version) = scratch.version() else {
        println!("skipped: `claude` is not on the PATH, so there is nothing to measure");
        return;
    };

    let servers = claude::resolved_servers(&transcript(&scratch));
    assert_eq!(
        servers,
        Some(vec![
            "hydra-project-scope".to_string(),
            "hydra-user-scope".to_string()
        ]),
        "{version} did not load the stub servers with no flags passed, so the \
         test above proves nothing about the flags"
    );
    scratch.clean();
}

/// The unflagged probe's transcript, read raw.
///
/// `resolve` would answer `None` here whatever the transcript said, because a
/// probe carrying no flags does not answer for a run's servers. That rule is the
/// thing under test one level up, so this control reads the file instead.
fn transcript(scratch: &Scratch) -> String {
    let session = "01a07600-0000-7000-8000-00000000000d";
    let shape = Probe {
        policy: "standard",
        config: None,
    };
    let out = Command::new("claude")
        .args(claude::probe_argv(session, shape))
        .current_dir(&scratch.cwd)
        .env_clear()
        .envs(scratch.env().vars())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("running the probe");

    let path = claude::transcript_path(&scratch.home, &scratch.cwd, session);
    std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "the probe left no transcript at {}: {err}. It exited {} and said: {}{}",
            path.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim(),
            String::from_utf8_lossy(&out.stdout).trim(),
        )
    })
}

/// Returns the version string printed by `claude --version`, or `None` when `claude`
/// is not on the PATH.
fn claude_version() -> Option<String> {
    let out = Command::new("claude").arg("--version").output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

struct CleanDir(PathBuf);

impl Drop for CleanDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Does `claude` still write thinking text when the run's own settings ask for it?
///
/// Build ticket 17. Claude Code 2.1.72 started emptying every thinking block on
/// the stream, and `claude::settings` carries `showThinkingSummaries` to turn the
/// text back on. The setting has regressed twice in the binary's history and one
/// model has been reported ignoring it, so this runs the real thing rather than
/// reading the argv. It costs a few hundred tokens on haiku, which is why it runs
/// only under `HYDRA_DRIFT_PAID=1` and skips otherwise, unlike the two free probes
/// above it. The failure names the version so a regression says which build.
#[test]
fn a_claude_run_still_writes_its_thinking_text() {
    if std::env::var("HYDRA_DRIFT_PAID").as_deref() != Ok("1") {
        println!("skipped: HYDRA_DRIFT_PAID is not set to 1");
        return;
    }
    let Some(version) = claude_version() else {
        println!("skipped: `claude` is not on the PATH, so there is nothing to measure");
        return;
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let scratch_dir = std::env::temp_dir().join(format!(
        "hydra-drift-thinking-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&scratch_dir);
    std::fs::create_dir_all(&scratch_dir).unwrap();
    let _clean = CleanDir(scratch_dir.clone());

    let settings_file = scratch_dir.join("settings.json");
    let settings = hydra::provider::claude::settings(Path::new("/tmp/hydra-drift-gate.py"), "0000");
    std::fs::write(
        &settings_file,
        serde_json::to_string_pretty(&settings).unwrap(),
    )
    .unwrap();

    let child = Command::new("claude")
        .arg("--print")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--model")
        .arg("haiku")
        .arg("--settings")
        .arg(&settings_file)
        .arg("Think hard step by step, then answer: what is 17*23?")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("running claude");

    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    let output = match rx.recv_timeout(std::time::Duration::from_secs(120)) {
        Ok(res) => res.expect("waiting for claude"),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
            let _ = waiter.join();
            let _ = std::fs::remove_dir_all(&scratch_dir);
            panic!("{version} timed out after 120 seconds");
        }
        Err(err) => {
            let _ = std::fs::remove_dir_all(&scratch_dir);
            panic!("channel error: {err}");
        }
    };

    let _ = std::fs::remove_dir_all(&scratch_dir);

    let mut thinking_blocks = Vec::new();
    let mut thinking_tokens: Option<u64> = None;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if val["type"] == "assistant" {
            if let Some(blocks) = val["message"]["content"].as_array() {
                for block in blocks {
                    if block["type"] == "thinking" {
                        thinking_blocks.push(block.clone());
                    }
                }
            }
        } else if val["type"] == "result"
            && let Some(tokens) = val["usage"]["output_tokens_details"]["thinking_tokens"].as_u64()
        {
            thinking_tokens = Some(tokens);
        }
    }

    let count_str = match thinking_tokens {
        Some(count) => format!("{count}"),
        None => "absent".to_string(),
    };

    assert!(
        !thinking_blocks.is_empty(),
        "{version} wrote no thinking blocks (thinking_tokens: {count_str})"
    );
    assert!(
        thinking_blocks
            .iter()
            .any(|b| b["thinking"].as_str().is_some_and(|s| !s.is_empty())),
        "{version} wrote thinking blocks but none had non-empty thinking text (thinking_tokens: {count_str})"
    );
}

/// Returns the version string printed by `codex --version`, or `None` when `codex`
/// is not on the PATH.
fn codex_version() -> Option<String> {
    let out = Command::new("codex").arg("--version").output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn codex_login_status_still_reports_whether_the_binary_is_logged_in() {
    let Some(version) = codex_version() else {
        println!("skipped: `codex` is not on the PATH, so there is nothing to measure");
        return;
    };

    match codex::login_status() {
        Ok(codex::LoginStatus::LoggedIn) => {
            println!("measured: {version} reports a logged-in ChatGPT account");
        }
        Ok(codex::LoginStatus::NotLoggedIn) => {
            println!("measured: {version} reports not logged in");
        }
        Err(err) => panic!("{version} login status probe failed: {err:#}"),
    }
}

/// What `codex` does with a call when its `PreToolUse` hook dies without
/// answering. Build ticket 101, the third hiring panel's H15.
///
/// `SECURITY.md` and `src/provider/codex.rs` say the call runs: a hook that
/// exits non-zero with no JSON fails open on this provider, measured by hand on
/// 0.154.0 as probe p3 in build ticket 48's outcome, and nothing re-measured it
/// since. This launches `codex` with the argv Hydra builds, hook override and
/// all, pointing at a hook that records that it fired and then crashes with an
/// uncaught exception, the way a gate with a bug the catch-all misses would. The
/// task asks for one shell command that creates a file.
///
/// **It asserts the measured behaviour, fail open, and says so when it moves.**
/// A `codex` that starts refusing on a dead hook fails this test with a message
/// naming the version, and the sentences in `SECURITY.md` and `codex.rs` are
/// then wrong in the safe direction and need re-measuring.
///
/// **It costs quota**, one short `codex exec` turn, so it runs only under
/// `HYDRA_DRIFT_PAID=1` with `codex` on the `PATH`, and skips otherwise with the
/// reason on stdout. Everything it writes is under a scratch directory: the
/// repository `codex` runs in, the hook and the hook's marker.
#[test]
fn a_codex_hook_that_dies_without_answering_lets_the_call_run() {
    if std::env::var("HYDRA_DRIFT_PAID").as_deref() != Ok("1") {
        println!("skipped: HYDRA_DRIFT_PAID is not set to 1");
        return;
    }
    let Some(version) = codex_version() else {
        println!("skipped: `codex` is not on the PATH, so there is nothing to measure");
        return;
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-drift-codex-hook-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let _clean = CleanDir(root.clone());
    // A repository, because a Hydra run is always inside one and `codex exec`
    // refuses a directory that is not.
    for args in [
        &["init", "-q", "-b", "main"][..],
        &[
            "-c",
            "user.email=drift@example.invalid",
            "-c",
            "user.name=drift",
            "commit",
            "--allow-empty",
            "-q",
            "-m",
            "root",
        ][..],
    ] {
        let out = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    }

    let fired = root.join("hook-fired");
    let hook = root.join("hook.py");
    std::fs::write(
        &hook,
        format!(
            "import pathlib\n\
             pathlib.Path({fired:?}).write_text('fired\\n')\n\
             raise RuntimeError('the gate died before it answered')\n",
            fired = fired.display().to_string()
        ),
    )
    .unwrap();

    let target = repo.join("probe-ran");
    let task = "Run exactly this one shell command and nothing else: touch probe-ran \
                Then reply with the single word done.";
    let argv = codex::argv(&codex::Launch {
        task,
        model: None,
        worktree: &repo,
        hook_file: &hook,
    });
    let env = ChildEnv::build(Provider::Codex).unwrap();
    let child = Command::new("codex")
        .args(&argv)
        .current_dir(&repo)
        .env_clear()
        .envs(env.vars())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("running codex");
    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    let output = match rx.recv_timeout(std::time::Duration::from_secs(180)) {
        Ok(result) => result.expect("waiting for codex"),
        Err(_) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
            let _ = waiter.join();
            panic!("{version} did not finish the probe in 180 seconds");
        }
    };

    let said = format!(
        "exit {}; stdout: {}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    assert!(
        fired.exists(),
        "{version}: the hook never fired, so this measured nothing about a dead hook. {said}"
    );
    assert!(
        target.exists(),
        "{version} refused the call its hook died on: a dead hook no longer fails open on \
         `codex`. SECURITY.md and src/provider/codex.rs say it does, so re-measure and \
         amend both. {said}"
    );
    println!("measured: {version} ran the call its hook died on");
}
