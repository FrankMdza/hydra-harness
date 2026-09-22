//! `hydra run --interactive`, harness spec 3's second launch mode, and the
//! harvest that completes it, section 10.
//!
//! **The vendor owns the terminal.** Hydra mints a run id and a `claude`
//! session id, writes `run_start`, execs the provider inheriting stdio in the
//! caller's own cwd or in a worktree when `--policy` is set, and waits for it
//! to exit. No pty of Hydra's, no detached half, no preamble: harness spec 4's
//! second population when no policy is named, and a gated run with a human at
//! the keyboard when one is. When the process exits, [`harvest`] converts its
//! own transcript into `run_end`, and the same function is what `hydra status`
//! calls on a dead run whose body nobody harvested yet (section 10, "harvest is
//! idempotent and any process can do it").
//!
//! **`claude` only.** Harness spec 10: it is the one provider confirmed to
//! write an append-only transcript during the run, and the one whose
//! transcript path Hydra can predict before the process starts, off the same
//! `--session-id` the pre-flight probe already mints
//! ([`crate::provider::claude::transcript_path`]).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::env::ChildEnv;
use crate::outcome::RunResult;
use crate::paths::Layout;
use crate::provider::{Provider, claude};
use crate::record::{Auth, RunStart, SCHEMA_VERSION, WorktreeRef, Writer};
use crate::run::{SpawnFiles, inject_skills, write_spawn_config};

/// What a human typed. Without `--policy` nothing here is gated. With it the
/// launch compiles a gate, seeds a worktree and links the snapshot the way a
/// recorded run does before the spawn.
pub struct Request {
    pub task: Option<String>,
    pub model: Option<String>,
    /// One of the five names when this run should be gated, and none by default.
    pub policy: Option<String>,
}

/// What a gated interactive launch writes before it execs the provider.
///
/// Public so `tests/spawn.rs` can read the gate and the settings without
/// launching anything.
pub struct PolicyLaunch {
    pub cwd: PathBuf,
    pub worktree: WorktreeRef,
    pub spawn: SpawnFiles,
    pub mcp_file: Option<PathBuf>,
    pub skills: Option<serde_json::Value>,
    pub mcp: serde_json::Value,
    pub seed_tree: Option<String>,
    pub scopes: std::collections::BTreeMap<String, String>,
}

/// Compiles the gate, seeds a worktree and links the snapshot, harness spec 3
/// with map ticket 50's paragraph.
///
/// Split out of [`run`] because `tests/spawn.rs` checks what it writes without
/// launching a provider, and because the gated and ungated halves of
/// [`run`] share nothing below this point except the harvest.
pub fn prepare_policy_launch(
    layout: &Layout,
    run_id: &str,
    policy: &str,
    policies: &crate::policy::PolicySet,
    repo: &Path,
    _invoked_in: &Path,
) -> Result<PolicyLaunch> {
    policies.get(policy)?;

    let mcp_document = crate::mcp::Document::load()?;

    let worktree = crate::worktree::create(
        repo,
        &layout.worktree(run_id),
        &format!("hydra/{run_id}"),
        &layout.seed_index(run_id),
    )?;
    let seed_tree = worktree.seed_tree.clone();
    let cwd = worktree.path.clone();
    let worktree_ref = WorktreeRef {
        path: worktree.path.display().to_string(),
        branch: worktree.branch.clone(),
        base_tree: worktree.seed_tree.clone(),
    };

    let mcp_plan = crate::mcp::plan(
        &mcp_document,
        Provider::Claude,
        policy,
        crate::mcp::Checkout::Hydra,
    );
    if mcp_plan.shortfall().is_some() {
        anyhow::bail!("{}", mcp_plan.remediation());
    }
    let mcp_file = crate::mcp::write_config(
        layout,
        run_id,
        Provider::Claude,
        mcp_plan.injected(),
        Some(&worktree.path),
    )?;
    let injected_names = mcp_plan.injected_names();

    let injected = if policy != "judge" {
        Some(inject_skills(
            layout,
            run_id,
            Provider::Claude,
            &worktree.path,
            crate::mcp::Probe {
                policy,
                config: Some(&mcp_file),
            },
        )?)
    } else {
        None
    };

    if let Some(injected) = &injected
        && !injected.collisions.is_empty()
    {
        anyhow::bail!(
            "{}",
            crate::skills::collision::remediation(&injected.collisions)
        );
    }

    let store_sha = injected.as_ref().map(|i| i.store_sha.as_str());
    let spawn = write_spawn_config(
        layout,
        run_id,
        policies,
        Provider::Claude,
        policy,
        Some(&worktree.path),
        Some(repo),
        store_sha,
        false,
    )?;

    let scopes = spawn.scopes.clone();
    Ok(PolicyLaunch {
        cwd,
        worktree: worktree_ref,
        spawn,
        mcp_file: Some(mcp_file),
        skills: injected.as_ref().map(|i| i.record.clone()),
        mcp: crate::mcp::record(
            &mcp_document.sha,
            &injected_names,
            injected
                .as_ref()
                .and_then(|injected| injected.mcp_resolved.as_deref()),
        ),
        seed_tree,
        scopes,
    })
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
    if request.policy.is_some() {
        run_gated(request)
    } else {
        run_ungated(request)
    }
}

fn run_gated(request: Request) -> Result<RunResult> {
    let layout = Layout::discover()?;
    layout.ensure()?;

    let invoked_in = std::env::current_dir().context("reading the working directory")?;
    let repo = crate::worktree::repo_root(&invoked_in).with_context(|| {
        "a gated interactive run needs a git repo to seed a worktree from. \
         Run it from inside a repository."
    })?;

    let policy = request
        .policy
        .as_deref()
        .context("run_gated called without a policy")?;
    let policies = crate::policy::PolicySet::discover()?;

    let run_id = Uuid::now_v7().to_string();
    let session = Uuid::now_v7().to_string();
    let run_file = layout.run_file(&run_id);

    let prep = prepare_policy_launch(&layout, &run_id, policy, &policies, &repo, &invoked_in)?;

    let provider_version = Provider::Claude.version()?;
    let mut child_env = ChildEnv::build(Provider::Claude)?;
    // Build ticket 57. This arm compiles a gate and seeds a worktree the way a
    // recorded run does, so 7.3 applies to it and the child is marked. The
    // ungated arm below is not marked: it compiles no gate, claims no
    // containment, and marking it would refuse the `hydra run` a human types in
    // their own session.
    child_env.set(crate::containment::MARKER, run_id.clone());

    let argv = claude::interactive_argv(&claude::InteractiveLaunch {
        session: &session,
        task: request.task.as_deref(),
        model: request.model.as_deref(),
        mcp: prep.mcp_file.as_deref(),
        settings: Some(&prep.spawn.hook_config),
        policy: Some(policy),
    });

    let mut child = std::process::Command::new(Provider::Claude.binary())
        .args(&argv)
        .current_dir(&prep.cwd)
        .env_clear()
        .envs(
            child_env
                .vars()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        )
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("starting {}", Provider::Claude.binary()))?;

    let pid = child.id();
    let process_start_time = crate::procinfo::start_time(pid)?;

    let start = RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: env!("CARGO_PKG_VERSION").to_string(),
        command: "run".to_string(),
        launch_mode: Some("interactive".to_string()),
        provider_version: Some(provider_version),
        provider_session_id: Some(session.clone()),
        task: request.task.clone(),
        policy: Some(policy.to_string()),
        policy_sha: Some(policies.sha().to_string()),
        floor_lifted: Some(false),
        // An interactive run launched without `--policy` compiles no gate; with
        // one it compiles the same gate a recorded run does, and build ticket 49
        // wants the sha here too. Filled when the gate is compiled, null until.
        gate_sha: None,
        sandbox: None,
        scopes: Some(prep.scopes.clone()),
        cwd: prep.cwd.display().to_string(),
        worktree: Some(prep.worktree.clone()),
        depth: 0,
        parent_policy: None,
        pid: Some(pid),
        process_start_time: process_start_time
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)),
        skills: prep.skills.clone(),
        mcp: Some(prep.mcp.clone()),
        auth: Some(Auth {
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(child_env.names()),
        subscription: Some(Provider::Claude.subscription().to_string()),
        resumed_from: None,
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
    eprintln!(
        "hydra: worktree {} on {}",
        prep.worktree.path, prep.worktree.branch
    );

    let status = child.wait().context("waiting for claude to exit")?;

    if let Err(err) = harvest(&layout, &run_id, status.code()) {
        eprintln!("hydra: run {run_id} exited but did not harvest: {err:#}");
        eprintln!("hydra: `hydra status {run_id}` will retry it");
    }

    let run = crate::normalize::read_file(&run_file, &policies)?;
    Ok(crate::run::result(
        &run,
        &run_file,
        Some(&prep.worktree),
        prep.seed_tree.as_deref(),
        &layout,
    ))
}

fn run_ungated(request: Request) -> Result<RunResult> {
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
        settings: None,
        policy: None,
    });

    let mut child = std::process::Command::new(Provider::Claude.binary())
        .args(&argv)
        .current_dir(&cwd)
        .env_clear()
        .envs(
            child_env
                .vars()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        )
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
        // The gated interactive path compiles a gate like a recorded run does;
        // build ticket 49 wants its sha here and this path does not thread it out
        // of `prepare` yet, so it is null and the replay treats the record as
        // unmarked rather than as a leak.
        gate_sha: None,
        sandbox: None,
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
        resumed_from: None,
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
    // Contract 6: at most one writer at a time. Held from before the first
    // `has_run_end` read to the end of the function, so a second harvester
    // blocks here and then finds the closing line the first one wrote.
    let _lock = crate::record::lock_exclusive(&run_file)?;
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
    let cwd = start["cwd"].as_str().with_context(|| {
        format!("run {run_id} carries no cwd to predict a transcript path from")
    })?;
    let parent_run_id = envelope["parent_run_id"].as_str();
    let model = envelope["model"].as_str();

    // A session that exits before its first turn writes no transcript at all,
    // measured 2026-09-08 on a consent dialog answered with exit. That run is
    // over and has nothing to harvest, and a `run_end` saying so is what keeps
    // `hydra status` from re-reporting the same missing file forever.
    let transcript_path = claude::transcript_path(home, Path::new(cwd), session);
    let transcript = match std::fs::read_to_string(&transcript_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let next_seq = crate::record::next_seq(&run_file)?;
            let mut writer = Writer::resume(
                &run_file,
                run_id,
                parent_run_id,
                envelope["provider"].as_str(),
                model,
                next_seq,
            )?;
            if crate::live::has_run_end(&run_file)? {
                return Ok(HarvestOutcome::AlreadyDone);
            }
            let error = format!(
                "the session wrote no transcript at {}, so there is nothing to harvest",
                transcript_path.display()
            );
            writer.hydra_line(&crate::record::RunEnd::harvested(exit_code, Some(error)))?;
            return Ok(HarvestOutcome::Harvested { records: 0 });
        }
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "reading the transcript hydra predicted for run {run_id} at {}",
                    transcript_path.display()
                )
            });
        }
    };

    let next_seq = crate::record::next_seq(&run_file)?;
    let mut writer = Writer::resume(
        &run_file,
        run_id,
        parent_run_id,
        envelope["provider"].as_str(),
        model,
        next_seq,
    )?;

    // The lock above is the guard now; this second read stays as the cheap
    // check it always was.
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

    let error = exit_code.is_none().then(|| {
        "harvested after the run already exited, so the exit code was not observed".to_string()
    });
    writer.hydra_line(&crate::record::RunEnd::harvested(exit_code, error))?;

    Ok(HarvestOutcome::Harvested { records })
}
