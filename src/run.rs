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
//! null, and the spawn is not detached, so nothing times out.
//!
//! **The JSON this returns is read back off the file it just wrote.** Contract 17
//! makes the result a projection of the record, and reading the record is the only
//! way to keep that true rather than merely intended.
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
use uuid::Uuid;

use crate::env::ChildEnv;
use crate::outcome::RunResult;
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

pub fn run(request: Request) -> Result<RunResult> {
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
            &layout.seed_index(&run_id),
        )?),
        None => None,
    };
    let seed_tree = worktree.as_ref().and_then(|w| w.seed_tree.clone());
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

    let mut tee_error: Option<String> = None;
    let provider = request.provider;

    // The session id is observed and not kept. Contract 17 leaves it off the
    // result and 5.1 has a reader take it from `raw` on the init frame, so a
    // second copy in memory would have no consumer.
    let teed = spawned.tee(|line| {
        if let Some(model) = provider.observe(line).model {
            writer.set_model(model);
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
    drop(writer);

    // Contract 17: the result is a projection of the record. So it is read back
    // off disk through the normalizer rather than assembled from what this
    // function still has in memory. The point is that the JSON cannot carry a
    // fact the source of truth does not have, which is the mistake section 6 says
    // ticket 11 made, and reading the file is the only way to be sure of it.
    //
    // **A read that fails does not fail the command.** The record is whole and
    // synced by this line, so propagating the error would report a run that
    // worked as one that did not, and would take the run id and the path to the
    // file holding the answer down with it.
    match crate::normalize::read_file(&run_file, &policies) {
        Ok(run) => Ok(result(
            &run,
            &run_file,
            worktree_ref.as_ref(),
            seed_tree.as_deref(),
            &layout,
        )),
        Err(err) => {
            eprintln!("hydra: the record is written and would not read back: {err:#}");
            Ok(unread(&run_id, &run_file, exit_code, &err))
        }
    }
}

/// What is left of contract 17's projection when the record will not read back.
///
/// Four fields Hydra knows without the normalizer, and nulls where a derivation
/// would have gone. `error` carries the read failure rather than the record's own,
/// which is the one place this struct is not a projection of the file, and it is
/// what stops `outcome: null` reading as a measurement: contract 6 defines `error`
/// as text Hydra captured that is not on the stream, and this is exactly that.
fn unread(
    run_id: &str,
    run_file: &std::path::Path,
    exit_code: Option<i32>,
    err: &anyhow::Error,
) -> RunResult {
    RunResult {
        run_id: run_id.to_string(),
        outcome: None,
        outcome_source: None,
        exit_code,
        error: Some(format!("the record would not read back: {err:#}")),
        result_text: None,
        branch: None,
        diff_stat: None,
        usage: crate::outcome::Usage::unavailable(),
        run_file: run_file.display().to_string(),
    }
}

/// Contract 17's JSON, off a normalized run.
///
/// Split out from [`run`] so a test can build the projection from a record on
/// disk without launching a provider.
///
/// The run id comes off the record rather than from the caller, so the scratch
/// index and the returned `run_id` cannot name two different runs.
pub fn result(
    run: &crate::normalize::Run,
    run_file: &std::path::Path,
    worktree: Option<&WorktreeRef>,
    seed_tree: Option<&str>,
    layout: &Layout,
) -> RunResult {
    let derived = crate::outcome::derive(run);
    let end = run.run_end();

    // The diff stat is the one returned field with no record equivalent, computed
    // from git at return time. It needs a checkout, the tree the seed left, and a
    // run id, so a run outside a repo has none of the three and the field is
    // absent.
    //
    // The run id is on that list because of what a torn line does. A record whose
    // line 0 did not parse normalizes to an empty `run_id` with a
    // `Finding::Limit` rather than failing the read, and `Layout::spawn("")` is
    // `~/hydra/spawn/` itself, so the scratch index would be one shared file for
    // every such record. Two of them at once would stage into each other.
    let diff_stat = match (worktree, seed_tree, !run.run_id.is_empty()) {
        (Some(worktree), Some(base), true) => crate::worktree::diff_stat(
            std::path::Path::new(&worktree.path),
            &layout.stat_index(&run.run_id),
            base,
        )
        .inspect_err(|err| {
            // The record is already whole at this point, so a failed stat is a
            // missing field rather than a failed run. It is said out loud because
            // an absent `diff_stat` otherwise reads as "no checkout".
            eprintln!("hydra: no diff stat for this run: {err:#}");
        })
        .ok()
        .map(|stat| crate::outcome::DiffStat {
            covers: "what the run changed in its worktree, against the tree Hydra \
                     seeded it with from the parent's uncommitted work",
            base_tree: base.to_string(),
            files_changed: stat.files_changed,
            insertions: stat.insertions,
            deletions: stat.deletions,
        }),
        _ => None,
    };

    RunResult {
        run_id: run.run_id.clone(),
        outcome: derived.map(|d| d.outcome.as_str()),
        outcome_source: derived.map(|d| d.source.as_str()),
        exit_code: end
            .and_then(|end| end.body["exit_code"].as_i64())
            .and_then(|code| i32::try_from(code).ok()),
        error: end.and_then(|end| end.body["error"].as_str().map(str::to_string)),
        result_text: crate::outcome::final_assistant_text(run),
        branch: worktree.map(|w| w.branch.clone()),
        diff_stat,
        usage: crate::outcome::Usage::of(run),
        run_file: run_file.display().to_string(),
    }
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

    // Read back rather than hashed in memory, so the sha is of the bytes that
    // reached the disk. The hook config carries it and the provider snapshots the
    // config at process start, which is what makes a mid-run rewrite of the script
    // fail closed.
    let written =
        std::fs::read(&gate).with_context(|| format!("reading back {}", gate.display()))?;
    let gate_sha = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(&written))
    };

    std::fs::write(
        &settings,
        serde_json::to_string_pretty(&claude::settings(&gate, &gate_sha))?,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The fallback's field shape, contract 17's list with the derivations null.
    ///
    /// **What triggers it is not tested and cannot cheaply be.** Reaching it
    /// needs `read_file` to fail on a record `run` has just written and synced,
    /// and `run` launches a provider. So this pins what a caller gets, which is
    /// the half that would otherwise be wrong without anyone noticing.
    #[test]
    fn a_record_that_will_not_read_back_still_returns_the_run_id_and_the_path() {
        let err = anyhow::anyhow!("python3 is not on PATH");
        let result = unread(
            "01a073ef-701c-7522-b5bb-56d5ecba0f57",
            std::path::Path::new("/Users/nobody/hydra/runs/r.jsonl"),
            Some(0),
            &err,
        );
        let json = serde_json::to_value(&result).unwrap();

        assert_eq!(json["run_id"], "01a073ef-701c-7522-b5bb-56d5ecba0f57");
        assert_eq!(json["run_file"], "/Users/nobody/hydra/runs/r.jsonl");
        assert_eq!(json["exit_code"], 0);
        assert!(json["outcome"].is_null());
        assert!(json["outcome_source"].is_null());
        assert!(json["result_text"].is_null());
        assert_eq!(json["usage"]["usage_source"], "unavailable");
        // The null outcome has to be explained by something, or it reads as a
        // measurement. `error` is what explains it.
        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("python3 is not on PATH"),
            "{json}"
        );
        // No checkout was asked about, so 5.5's rule leaves both out.
        assert!(json.get("branch").is_none());
        assert!(json.get("diff_stat").is_none());
    }
}
