//! `hydra run --interactive`, harness spec 3's second launch mode, and the
//! harvest that completes it, section 10.
//!
//! **The vendor owns the terminal.** Hydra mints a run id and a `claude`
//! session id, writes `run_start`, execs the provider inheriting stdio in the
//! caller's own cwd, and waits for it to exit. No pty of Hydra's, no detached
//! half, no gate, no preamble: harness spec 4's second population. When the
//! process exits, [`harvest`] converts its own transcript into `run_end`, and
//! the same function is what `hydra status` calls on a dead run whose body
//! nobody harvested yet (section 10, "harvest is idempotent and any process
//! can do it").
//!
//! **`claude` only.** Harness spec 10: it is the one provider confirmed to
//! write an append-only transcript during the run, and the one whose
//! transcript path Hydra can predict before the process starts, off the same
//! `--session-id` the pre-flight probe already mints
//! ([`crate::provider::claude::transcript_path`]).

use std::path::Path;

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::env::ChildEnv;
use crate::outcome::RunResult;
use crate::paths::Layout;
use crate::provider::{Provider, claude};
use crate::record::{Auth, RunEnd, RunStart, SCHEMA_VERSION, Writer};

/// What a human typed. Nothing here is gated, so there is no policy, no
/// parent and no timeout: harness spec 3 gives this mode none of them, and
/// [`run`] blocks on the child until it exits whatever that takes.
pub struct Request {
    pub task: Option<String>,
    pub model: Option<String>,
}

/// Launches `claude` interactively and returns contract 17's projection once
/// it exits.
///
/// **This function is the whole of the launch and the wait.** There is no
/// handle to degrade to, because harness spec 9's timeout is a property of a
/// blocking view over a detached child, and an interactive run has neither: it
/// is the caller's own terminal, and giving up on it would leave the vendor's
/// TUI running with nobody home to read what it prints next.
pub fn run(request: Request) -> Result<RunResult> {
    let layout = Layout::discover()?;
    layout.ensure()?;

    let cwd = std::env::current_dir().context("reading the working directory")?;
    let run_id = Uuid::now_v7().to_string();
    let session = Uuid::now_v7().to_string();
    let run_file = layout.run_file(&run_id);

    let provider_version = Provider::Claude.version()?;
    let child_env = ChildEnv::build(Provider::Claude)?;

    // 6.5: the run's file, without `--strict-mcp-config` so the human's own
    // servers survive beside it. Nothing is admitted, because admission is a
    // policy question (7.1) and this run compiles no policy to ask it of;
    // an empty `mcpServers` is still a real file, the same shape a `judge`
    // run's is (`claude::mcp_config`'s own doc comment).
    let mcp_document = crate::mcp::Document::load()?;
    let mcp_file = crate::mcp::write_config(&layout, &run_id, Provider::Claude, &[], None)?;

    let argv = claude::interactive_argv(&claude::InteractiveLaunch {
        session: &session,
        task: request.task.as_deref(),
        model: request.model.as_deref(),
        mcp: Some(&mcp_file),
    });

    let mut child = std::process::Command::new(Provider::Claude.binary())
        .args(&argv)
        .current_dir(&cwd)
        .env_clear()
        .envs(child_env.vars().map(|(k, v)| (k.to_string(), v.to_string())))
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("starting {}", Provider::Claude.binary()))?;

    // Contract 5.2's liveness pair, filled from the process this call itself
    // owns rather than backfilled by a detached watcher: there is no watcher
    // here, `run` is the process holding the child.
    let pid = child.id();
    let process_start_time = crate::procinfo::start_time(pid)?;

    let start = RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: env!("CARGO_PKG_VERSION").to_string(),
        command: "run".to_string(),
        launch_mode: Some("interactive".to_string()),
        provider_version: Some(provider_version),
        // Known before the child starts, unlike a recorded run's: `claude`
        // takes `--session-id` rather than minting its own, so line 0 carries
        // it from the first write rather than arriving late on an init frame.
        provider_session_id: Some(session.clone()),
        // Verbatim, with no preamble: harness spec 3 gives an interactive run
        // none.
        task: request.task.clone(),
        policy: None,
        policy_sha: None,
        floor_lifted: None,
        scopes: None,
        cwd: cwd.display().to_string(),
        worktree: None,
        depth: 0,
        parent_policy: None,
        pid: Some(pid),
        process_start_time: process_start_time
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)),
        skills: None,
        mcp: Some(crate::mcp::record(&mcp_document.sha, &[], None)),
        auth: Some(Auth {
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(child_env.names()),
        subscription: Some(Provider::Claude.subscription().to_string()),
    };

    let mut writer = Writer::create(
        &run_file,
        &run_id,
        None,
        Some(Provider::Claude.binary()),
        request.model.as_deref(),
    )?;
    writer.hydra_line(&start)?;
    drop(writer);

    eprintln!("hydra: run {run_id}");
    eprintln!("hydra: record {}", run_file.display());

    let status = child.wait().context("waiting for claude to exit")?;

    if let Err(err) = harvest(&layout, &run_id, status.code()) {
        eprintln!("hydra: run {run_id} exited but did not harvest: {err:#}");
        eprintln!("hydra: `hydra status {run_id}` will retry it");
    }

    let policies = crate::policy::PolicySet::discover()?;
    let run = crate::normalize::read_file(&run_file, &policies)?;
    Ok(crate::run::result(&run, &run_file, None, None, &layout))
}

/// What one harvest attempt did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarvestOutcome {
    /// The run already carried a `run_end`. Contract 10: harvest is
    /// idempotent, and this is the case that makes it so.
    AlreadyDone,
    /// The transcript was read and wrapped, one line per record.
    Harvested { records: usize },
}

/// Converts `run_id`'s vendor transcript into its `run_end`, against the real
/// home directory.
///
/// `exit_code` is `Some` when the caller just watched the process exit and
/// `None` when harvest is running after the fact, on a run whose exit nothing
/// observed directly, which is `hydra status`'s case.
pub fn harvest(layout: &Layout, run_id: &str, exit_code: Option<i32>) -> Result<HarvestOutcome> {
    let home = crate::env::real_home()?;
    harvest_in(layout, run_id, exit_code, &home)
}

/// [`harvest`], with the home directory named rather than discovered.
///
/// **The one caller that needs this is a test.** `crate::env::real_home`
/// reads the passwd entry and ignores `HOME`, so a suite that wants to hand
/// harvest a transcript it wrote itself, without touching the developer's own
/// `~/.claude/projects/`, has to name the directory rather than let this
/// function find it. `tests/harvest.rs` and `tests/status.rs` are the callers.
pub fn harvest_in(
    layout: &Layout,
    run_id: &str,
    exit_code: Option<i32>,
    home: &Path,
) -> Result<HarvestOutcome> {
    let run_file = layout.run_file(run_id);
    if crate::live::has_run_end(&run_file)? {
        return Ok(HarvestOutcome::AlreadyDone);
    }

    let text = std::fs::read_to_string(&run_file)
        .with_context(|| format!("reading {}", run_file.display()))?;
    let first = text
        .lines()
        .next()
        .with_context(|| format!("{} is empty", run_file.display()))?;
    let envelope: serde_json::Value = serde_json::from_str(first)
        .with_context(|| format!("line 0 of {} did not parse", run_file.display()))?;
    anyhow::ensure!(
        envelope["provider"].as_str() == Some(Provider::Claude.binary()),
        "harvest reads only a claude transcript in this build, and run {run_id} is {:?}",
        envelope["provider"]
    );
    let start = &envelope["hydra"];
    anyhow::ensure!(
        start["launch_mode"].as_str() == Some("interactive"),
        "run {run_id} is not an interactive launch, so it has no vendor transcript to harvest"
    );
    let session = start["provider_session_id"]
        .as_str()
        .with_context(|| format!("run {run_id} carries no provider_session_id to harvest with"))?;
    let cwd = start["cwd"]
        .as_str()
        .with_context(|| format!("run {run_id} carries no cwd to predict a transcript path from"))?;
    let parent_run_id = envelope["parent_run_id"].as_str();
    let model = envelope["model"].as_str();

    let transcript_path = claude::transcript_path(home, Path::new(cwd), session);
    let transcript = std::fs::read_to_string(&transcript_path).with_context(|| {
        format!(
            "reading the transcript hydra predicted for run {run_id} at {}",
            transcript_path.display()
        )
    })?;

    let next_seq = crate::record::next_seq(&run_file)?;
    let mut writer = Writer::resume(
        &run_file,
        run_id,
        parent_run_id,
        envelope["provider"].as_str(),
        model,
        next_seq,
    )?;

    // Contract 6: at most one writer at a time. The check above already
    // proved there was no `run_end` when this call started; this one runs
    // again right before the first byte goes down, which is as much of a
    // guard against a second concurrent harvester as a file with no lock can
    // give.
    if crate::live::has_run_end(&run_file)? {
        return Ok(HarvestOutcome::AlreadyDone);
    }

    let mut records = 0usize;
    for line in transcript.lines() {
        if line.trim().is_empty() {
            continue;
        }
        writer.provider_line(line)?;
        records += 1;
    }

    let error = exit_code
        .is_none()
        .then(|| "harvested after the run already exited, so the exit code was not observed".to_string());
    writer.hydra_line(&RunEnd::harvested(exit_code, error))?;

    Ok(HarvestOutcome::Harvested { records })
}
