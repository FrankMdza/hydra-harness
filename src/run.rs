//! `hydra run`: the launch sequence, harness spec 6.
//!
//! What this slice does, in order: build the child's environment from the
//! allowlist, read the provider's version, load the policy set and the preamble,
//! mint a UUIDv7 run id, create a worktree and seed it, compile the run's gate into
//! its spawn directory, spawn the provider on a pty with `--settings` pointing at
//! the hook config, write `run_start`, tee every frame verbatim, and write `run_end`
//! when the child exits.
//!
//! What it does not do, and what that costs the record, is in `IMPLEMENTATION.md`.
//! The short version: no injection, so `skills` and `mcp` are absent rather than
//! null, and nothing derives `permission_decision` from the gate's answers yet.
//!
//! **The gate is compiled before the child starts and never rewritten.** 6.6 asks
//! for real files, never links, rewritten at every spawn. The spawn directory is
//! per run and is created here, so every run gets a fresh script and a fresh config
//! and the next run in the same directory inherits nothing.
//!
//! **`run_start` is written after the spawn, not before it.** Contract section 5
//! says the line is written before the child process starts, and 5.2 puts the
//! child's pid on it. Both cannot hold literally, since a pid does not exist until
//! the process does. What run_end's derivation actually leans on is that the pid is
//! in the file, and the pty buffers everything the child writes until the tee loop
//! reads it, so nothing is lost by spawning first. `run_start` is still line 0.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::SecondsFormat;
use serde::Serialize;
use uuid::Uuid;

use crate::env::ChildEnv;
use crate::paths::Layout;
use crate::provider::{Provider, claude};
use crate::record::{Auth, RunEnd, RunStart, SCHEMA_VERSION, WorktreeRef, Writer};

pub struct Request {
    pub provider: Provider,
    pub task: String,
    pub model: Option<String>,
    /// One of the five names, never a list. A name is something a human wrote
    /// once; a list is security composed at call time by the least accountable
    /// party in the chain (7.2).
    pub policy: String,
    /// `--allow-floor-escape`, harness spec 16. Never inheritable and never
    /// settable by a policy.
    pub floor_lifted: bool,
}

/// What `hydra run` prints on stdout (D7).
///
/// Contract section 17 makes this a projection of the record plus a diff stat. Most
/// of that projection needs the normalizer, which is not built, so this returns the
/// fields it can fill honestly and names the rest rather than inventing them. A
/// `usage` of null and an `outcome` of null would both read as measurements.
#[derive(Serialize)]
pub struct Outcome {
    pub run_id: String,
    pub provider: String,
    pub provider_session_id: Option<String>,
    pub exit_code: Option<i32>,
    pub run_file: String,
    pub worktree: Option<WorktreeRef>,
    pub not_yet_derived: Vec<&'static str>,
}

pub fn run(request: Request) -> Result<Outcome> {
    // Pre-flight that refuses before a run id exists. A wrong PATH or a missing HOME
    // is a fact about this machine rather than a refused spawn, and the contract's
    // four refusal values are all Hydra-side policy checks this slice does not run.
    let child_env = ChildEnv::build(request.provider)?;
    let provider_version = request.provider.version()?;

    // Both are read before a run id exists, for the same reason the environment is:
    // a policy name that is not one of the five, or a preamble file that has gone
    // missing, is a fact about this machine rather than a refused spawn, and the
    // contract's four refusal values are all Hydra-side policy checks this slice
    // does not run.
    let policies = crate::policy::PolicySet::discover()?;
    policies.get(&request.policy)?;
    let preamble = crate::preamble::load()?;

    let layout = Layout::discover()?;
    layout.ensure()?;

    let run_id = Uuid::now_v7().to_string();
    let invoked_in = std::env::current_dir().context("reading the working directory")?;

    let worktree = match crate::worktree::repo_root(&invoked_in) {
        Some(repo) => Some(crate::worktree::create(
            &repo,
            &layout.worktree(&run_id),
            &format!("hydra/{run_id}"),
        )?),
        None => None,
    };
    let cwd: PathBuf = worktree
        .as_ref()
        .map(|w| w.path.clone())
        .unwrap_or_else(|| invoked_in.clone());
    let worktree_ref = worktree.as_ref().map(|w| WorktreeRef {
        path: w.path.display().to_string(),
        branch: w.branch.clone(),
    });

    let spawn = write_spawn_config(
        &layout,
        &run_id,
        &policies,
        &request.policy,
        worktree_ref.as_ref().map(|w| std::path::Path::new(&w.path)),
        request.floor_lifted,
    )?;

    // Contract 5.1: the task is recorded verbatim, preamble and all.
    let task = crate::preamble::compose(&preamble, &request.task);

    let run_file = layout.run_file(&run_id);
    let mut writer = Writer::create(
        &run_file,
        &run_id,
        None,
        Some(request.provider.binary()),
        request.model.as_deref(),
    )?;

    eprintln!("hydra: run {run_id}");
    eprintln!("hydra: record {}", run_file.display());
    if let Some(w) = &worktree_ref {
        eprintln!("hydra: worktree {} on {}", w.path, w.branch);
    }

    let argv = claude::argv(&claude::Launch {
        task: &task,
        model: request.model.as_deref(),
        settings: &spawn.settings,
        policy: &request.policy,
    });

    let spawned = crate::pty::spawn(
        request.provider.binary(),
        &argv,
        &cwd,
        child_env
            .vars()
            .map(|(k, v)| (k.to_string(), v.to_string())),
    );

    let mut spawned = match spawned {
        Ok(spawned) => spawned,
        Err(err) => {
            // A run that never had a process. Contract 5.2: pid null, and a reader
            // must not read that as dead-and-unharvested.
            writer.hydra_line(&run_start(
                &request,
                &task,
                policies.sha(),
                &provider_version,
                &cwd,
                worktree_ref.clone(),
                None,
                None,
                child_env.names(),
            ))?;
            writer.hydra_line(&RunEnd::streamed(None, Some(format!("{err:#}"))))?;
            return Err(err);
        }
    };

    let pid = spawned.pid;
    let start_time =
        crate::procinfo::start_time(pid)?.map(|t| t.to_rfc3339_opts(SecondsFormat::Micros, true));

    writer.hydra_line(&run_start(
        &request,
        &task,
        policies.sha(),
        &provider_version,
        &cwd,
        worktree_ref.clone(),
        Some(pid),
        start_time,
        child_env.names(),
    ))?;

    let mut session_id: Option<String> = None;
    let mut tee_error: Option<String> = None;
    let provider = request.provider;

    let teed = spawned.tee(|line| {
        let observed = provider.observe(line);
        if let Some(model) = observed.model {
            writer.set_model(model);
        }
        if let Some(id) = observed.session_id {
            session_id.get_or_insert(id);
        }
        writer.provider_line(line)?;
        Ok(())
    });
    if let Err(err) = teed {
        tee_error = Some(format!("{err:#}"));
    }

    let exit_code = spawned.wait()?;

    if spawned.lossy_lines > 0 {
        let note = format!(
            "{} line(s) the child wrote were not valid UTF-8 and were stored lossily",
            spawned.lossy_lines
        );
        tee_error = Some(match tee_error {
            Some(existing) => format!("{existing}; {note}"),
            None => note,
        });
    }

    writer.hydra_line(&RunEnd::streamed(exit_code, tee_error))?;

    Ok(Outcome {
        run_id,
        provider: request.provider.binary().to_string(),
        provider_session_id: session_id,
        exit_code,
        run_file: run_file.display().to_string(),
        worktree: worktree_ref,
        not_yet_derived: vec![
            "outcome",
            "outcome_source",
            "usage",
            "result_text",
            "diff_stat",
        ],
    })
}

/// What one spawn writes into `~/hydra/spawn/<run-id>/`.
pub struct SpawnFiles {
    /// The compiled gate, `0o700`.
    pub gate: PathBuf,
    /// The hook config `--settings` points at.
    pub settings: PathBuf,
}

/// Compiles the run's gate and writes the config that delivers it.
///
/// Split out from [`run`] because it is the half of the launch that can be checked
/// without spending a subscription: `tests/spawn.rs` calls it against a temporary
/// layout and reads what it wrote. The other half needs a provider.
///
/// **Real files, never links, rewritten at every spawn** (6.6). The spawn directory
/// is per run and the two files are created inside it, so a second run in the same
/// directory is not a case that can arise.
pub fn write_spawn_config(
    layout: &Layout,
    run_id: &str,
    policies: &crate::policy::PolicySet,
    policy: &str,
    worktree: Option<&std::path::Path>,
    floor_lifted: bool,
) -> Result<SpawnFiles> {
    // The gate is compiled against the path it will really live at, because it
    // denies writes to itself and a placeholder would deny the wrong file.
    let gate = layout.gate_script(run_id);
    let settings = layout.settings_file(run_id);

    // With no repo there is no worktree, and the child runs where Hydra was
    // invoked. Scoping the `worktree` capability to that directory would hand the
    // run write access to whatever the human happened to be standing in, so the
    // scope points at a path inside the spawn directory that nothing creates. A
    // `standard` run outside a repo can therefore read and shell out and write
    // nowhere, which is the honest reading of D9: Hydra writes only under a path
    // it created, and a run with no checkout has none.
    let no_worktree = layout.spawn(run_id).join("no-worktree");

    crate::policy::write_gate(
        policies,
        &crate::policy::Spawn {
            policy,
            worktree: worktree.unwrap_or(&no_worktree),
            skill_store: &layout.skills(),
            runs: &layout.runs(),
            gate_script: &gate,
            floor_lifted,
        },
    )?;

    std::fs::write(
        &settings,
        serde_json::to_string_pretty(&claude::settings(&gate))?,
    )
    .with_context(|| format!("writing {}", settings.display()))?;

    Ok(SpawnFiles { gate, settings })
}

#[allow(clippy::too_many_arguments)]
fn run_start(
    request: &Request,
    task: &str,
    policy_sha: &str,
    provider_version: &str,
    cwd: &std::path::Path,
    worktree: Option<WorktreeRef>,
    pid: Option<u32>,
    process_start_time: Option<String>,
    env_allowlist: Vec<String>,
) -> RunStart {
    RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: env!("CARGO_PKG_VERSION").to_string(),
        command: "run".to_string(),
        launch_mode: Some("recorded".to_string()),
        provider_version: Some(provider_version.to_string()),
        // Late on every provider. It lands on `claude`'s init frame, and a reader
        // takes it from `raw` rather than expecting a rewrite of line 0.
        provider_session_id: None,
        // The preamble and the caller's task, composed. Contract 5.1 stores the
        // whole string, so the preamble needs no field and no sha of its own.
        task: Some(task.to_string()),
        policy: Some(request.policy.clone()),
        // The sha of the policy directory, the pattern floor included. Contract
        // 5.1: the permission decisions in a record are re-derived from the
        // policy, so a reader a month later needs it exactly as it was.
        policy_sha: Some(policy_sha.to_string()),
        cwd: cwd.display().to_string(),
        worktree,
        // Delegation is not built, so every run is one a human started.
        depth: 0,
        pid,
        process_start_time,
        // Absent rather than null: nothing was injected, so which sha the run pinned
        // is not a question with a missing answer.
        skills: None,
        mcp: None,
        auth: Some(Auth {
            // Authored from what Hydra did rather than sniffed. The allowlist passes
            // no key, so the child can only be on the stored subscription login.
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(env_allowlist),
        subscription: Some(request.provider.subscription().to_string()),
    }
}
