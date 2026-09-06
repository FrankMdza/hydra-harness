//! `hydra run`: the launch sequence, harness spec 6, and the view over it, 9.
//!
//! What a launch does, in order: build the child's environment from the allowlist,
//! read the provider's version, load the policy set and the preamble, refuse if the
//! concurrency cap is already met, mint a UUIDv7 run id, create a worktree and seed
//! it, compile the run's gate into its spawn directory, compose the provider's argv
//! and the whole of line 0, then hand all of it to [`crate::watch`], which spawns
//! the provider on a pty in a session of its own and owns the record from there.
//!
//! What it does not do, and what that costs the record, is in `IMPLEMENTATION.md`.
//! The short version: nothing is injected, so `skills` and `mcp` are absent rather
//! than null.
//!
//! **This half never touches the run file.** Contract 6 gives a run file one writer
//! and the detached half is it. `hydra run` tails the file like `hydra logs` does,
//! and everything below the launch is a reader.
//!
//! **Blocking is a view, so the timeout is free.** Harness spec 9: giving up on a
//! call only means it stops watching. Nothing is signalled, nothing is reaped, and
//! the run carries on writing its own record for `hydra status` to find.
//!
//! **The JSON this returns is read back off the file the run wrote.** Contract 17
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
use uuid::Uuid;

use crate::env::ChildEnv;
use crate::outcome::RunResult;
use crate::paths::Layout;
use crate::provider::{Provider, claude};
use crate::record::{Auth, RunStart, SCHEMA_VERSION, WorktreeRef};

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
    /// How long the call waits before it stops watching. Harness spec 9 defaults
    /// it to roughly ten minutes; zero returns the handle without waiting.
    pub timeout: std::time::Duration,
    /// The run id, when the caller minted it before the launch.
    ///
    /// **`hydra run` passes `None` and this function mints one.** `hydra curate`
    /// is the caller that cannot: it checks the skill store out at a path named
    /// after the run, and the gate scopes that run's writes to it, so the id has
    /// to exist before either. Nothing else about the launch changes, and the id
    /// still reaches the record the same way.
    pub run_id: Option<String>,
}

/// What a call gets back: the run, or a way to come back to it.
///
/// **Two shapes rather than one with nulls in it.** Harness spec 9 degrades a
/// blocking call to a handle, and a handle is not a thin result: the run has not
/// ended, so contract 17's derived fields have no values and printing them as null
/// would read as measurements. The caller switches on which shape arrived.
pub enum Call {
    Ended(Box<RunResult>),
    Handle(Handle),
}

/// What the caller gets when the view gave up first.
///
/// **`outcome` is the call's, not the record's**, and it is the one place in this
/// build where those two differ. Contract 8's row reads "the watcher gave up; the
/// child may still be running", which is a sentence no `run_end` can ever carry:
/// by the time a record has a closing line the child has stopped. So `timed_out`
/// is produced here, where the giving up happens, and the record keeps its own
/// answer for whoever reads it later. A run that outlived its call and then
/// finished cleanly is `completed` in the corpus, which is the honest lesson.
///
/// **There is no `outcome_source`.** Contract 8.2's three values all describe how
/// a record was read, and this outcome was not read from one. Inventing a fourth
/// is a decision for the map rather than a field for this struct.
#[derive(serde::Serialize)]
pub struct Handle {
    pub run_id: String,
    pub outcome: &'static str,
    /// Contract 5.2's answer at the moment the view stopped watching. False means
    /// the child died in the same breath, and `hydra status` is where that gets
    /// reconciled.
    pub alive: bool,
    pub pid: Option<u32>,
    /// The budget that ran out, as it was typed.
    pub timeout: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub run_file: String,
    /// The two commands that answer "what happened next". Harness spec 10 makes
    /// observation read-only, so these are the whole of what a caller may do to a
    /// run it is no longer watching.
    pub follow: String,
}

pub fn run(request: Request) -> Result<Call> {
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

    // Harness spec 9's cap, checked before anything is created. Concurrency comes
    // from the orchestrator's own parallel tool calls, so Hydra needs no scheduler,
    // only a refusal. Queueing here would hold a slot the caller cannot see and
    // cannot cancel.
    crate::live::guard(crate::live::LIVE_RUN_CAP, &crate::live::scan(&layout)?)?;

    let run_id = request
        .run_id
        .clone()
        .unwrap_or_else(|| Uuid::now_v7().to_string());
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
    let argv = claude::argv(&claude::Launch {
        task: &task,
        model: request.model.as_deref(),
        settings: &spawn.settings,
        policy: &request.policy,
    });

    // The whole of line 0 except the two fields that do not exist until a process
    // does. Composing it here keeps every provider-specific fact on this side of
    // the fork and leaves the detached half nothing to decide.
    let start = run_start(
        &request,
        &task,
        policies.sha(),
        &provider_version,
        &cwd,
        worktree_ref.clone(),
        child_env.names(),
    );

    let line_zero = serde_json::to_value(&start)?;
    let spec_path = layout.launch_spec(&run_id);

    // **A launch that fails here writes the record itself.** Everything from the
    // spec file to the fork happens before the detached half exists, so there is
    // nothing on the other side to write a run file, and contract 8.3's rule
    // still holds: under D10 anything not in a run record did not happen, and by
    // this point a worktree, a branch and a compiled gate are on disk with a run
    // id on them. The two writers cannot collide, because the failure is what
    // stops the second one from starting.
    let launched = crate::watch::write_spec(
        &spec_path,
        &crate::watch::Launch {
            run_id: run_id.clone(),
            provider: request.provider.to_string(),
            binary: request.provider.binary().to_string(),
            argv,
            cwd,
            run_file: run_file.clone(),
            model: request.model.clone(),
            run_start: line_zero.clone(),
        },
    )
    .and_then(|()| std::env::current_exe().context("locating the hydra binary"))
    .and_then(|exe| crate::watch::detach(&exe, &spec_path, &layout.watcher_log(&run_id)));

    let mut watcher = match launched {
        Ok(watcher) => watcher,
        Err(err) => {
            if let Err(second) =
                never_started(&run_file, &run_id, request.provider, &line_zero, &err)
            {
                eprintln!("hydra: this run could not be recorded either: {second:#}");
            }
            return Err(err);
        }
    };

    eprintln!("hydra: run {run_id}");
    eprintln!("hydra: record {}", run_file.display());
    if let Some(w) = &worktree_ref {
        eprintln!("hydra: worktree {} on {}", w.path, w.branch);
    }

    // Line 0 first, so a handle always carries a pid and a watcher that died on
    // its way up is reported as that rather than as a run the caller can poll.
    crate::watch::wait_for_start(&run_file, &mut watcher, LAUNCH_BUDGET).map_err(|err| {
        err.context(format!(
            "the run's own log is {}",
            layout.watcher_log(&run_id).display()
        ))
    })?;

    match crate::watch::wait_for_end(&run_file, &mut watcher, request.timeout)? {
        crate::watch::Waited::TimedOut => Ok(Call::Handle(handle(
            &run_id,
            &run_file,
            worktree_ref.as_ref(),
            request.timeout,
        )?)),
        crate::watch::Waited::WriterGone => Err(anyhow::anyhow!(
            "the run's writer exited without closing {}. Its log is {}",
            run_file.display(),
            layout.watcher_log(&run_id).display()
        )),
        crate::watch::Waited::Ended => {
            // Contract 17: the result is a projection of the record. So it is read
            // back off disk through the normalizer rather than assembled from what
            // this function still has in memory. The point is that the JSON cannot
            // carry a fact the source of truth does not have, which is the mistake
            // section 6 says ticket 11 made, and reading the file is the only way to
            // be sure of it.
            //
            // **A read that fails does not fail the command.** The record is whole
            // and synced by this line, so propagating the error would report a run
            // that worked as one that did not, and would take the run id and the
            // path to the file holding the answer down with it.
            let result = match crate::normalize::read_file(&run_file, &policies) {
                Ok(run) => result(
                    &run,
                    &run_file,
                    worktree_ref.as_ref(),
                    seed_tree.as_deref(),
                    &layout,
                ),
                Err(err) => {
                    eprintln!("hydra: the record is written and would not read back: {err:#}");
                    // The exit code is the detached half's to know, so it comes
                    // off the closing line rather than out of this process's
                    // memory. That read is a tail and a `serde_json` call, which
                    // is the half of the record that always parses even when the
                    // normalizer chokes on what is above it.
                    let exit_code = crate::live::closing_line(&run_file)
                        .ok()
                        .flatten()
                        .and_then(|end| end["exit_code"].as_i64())
                        .and_then(|code| i32::try_from(code).ok());
                    unread(&run_id, &run_file, exit_code, &err)
                }
            };

            // **A spawn that never started returns the error rather than the
            // JSON.** A null exit code with no `refusal` is a record whose child
            // never existed, and printing a projection of it would exit 0 on a run
            // that did not happen, which breaks any script reading the status. The
            // record is written either way, so nothing is lost for the corpus.
            if result.exit_code.is_none()
                && result.outcome_source != Some("refusal")
                && let Some(err) = result.error
            {
                anyhow::bail!("{err}");
            }
            Ok(Call::Ended(Box::new(result)))
        }
    }
}

/// How long a launch waits for its own detached half to write line 0.
///
/// This is not the call's timeout and is not configurable. It bounds one thing:
/// the gap between `fork` and the first `sync_data`, which is a process start and
/// a `pty` open. Ten seconds is far outside it on any machine that can run a
/// provider at all, and a watcher that has not written by then is broken rather
/// than slow.
pub const LAUNCH_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

/// Writes the record for a run whose detached half never started.
///
/// Contract 5.2's null pid: a run that never had a process. A reader must not take
/// that for dead-and-unharvested and go looking for a vendor transcript, and
/// contract 8.3's shape is the same one a refused spawn gets, `run_start` then
/// `run_end` with nothing between. It derives `failed`.
fn never_started(
    run_file: &std::path::Path,
    run_id: &str,
    provider: Provider,
    line_zero: &serde_json::Value,
    err: &anyhow::Error,
) -> Result<()> {
    let mut writer =
        crate::record::Writer::create(run_file, run_id, None, Some(provider.binary()), None)?;
    writer.hydra_line(line_zero)?;
    writer.hydra_line(&crate::record::RunEnd::streamed(
        None,
        Some(format!("{err:#}")),
    ))?;
    Ok(())
}

/// The handle a timed-out call gets, built from line 0 rather than from the run.
///
/// Nothing here normalizes. The run has not ended, so there is nothing to derive,
/// and reading the record back would cost a python process per recorded tool call
/// to produce a page of nulls.
fn handle(
    run_id: &str,
    run_file: &std::path::Path,
    worktree: Option<&WorktreeRef>,
    timeout: std::time::Duration,
) -> Result<Handle> {
    let view = crate::live::read_view(run_file)?;
    Ok(Handle {
        run_id: run_id.to_string(),
        outcome: crate::outcome::Outcome::TimedOut.as_str(),
        alive: view.is_live(),
        pid: view.pid,
        timeout: crate::cli::render_duration(timeout),
        branch: worktree.map(|w| w.branch.clone()),
        run_file: run_file.display().to_string(),
        follow: format!("hydra logs {run_id} --follow"),
    })
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

    // The store itself on every run but a curate pass, which gets its own
    // checkout of it: ouroboros spec 3.6 keeps the curator out of
    // `~/hydra/skills/` because the next launch commits that tree before pinning
    // it, and two writers to one working tree is the collision. The rule lives on
    // the layout because contract 7.7's read-time re-evaluation has to reach the
    // same answer with nothing but the run id and the policy name.
    let skill_store = layout.skill_store_scope(run_id, policy);

    crate::policy::write_gate(
        policies,
        &crate::policy::Spawn {
            policy,
            worktree: worktree.unwrap_or(&no_worktree),
            skill_store: &skill_store,
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

/// Line 0, composed before the fork.
///
/// **`pid` and `process_start_time` are null here and are the detached half's to
/// fill in.** They are contract 5.2's liveness pair and neither exists until a
/// process does, which is the one fact this side cannot know.
#[allow(clippy::too_many_arguments)]
fn run_start(
    request: &Request,
    task: &str,
    policy_sha: &str,
    provider_version: &str,
    cwd: &std::path::Path,
    worktree: Option<WorktreeRef>,
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
        pid: None,
        process_start_time: None,
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

    /// The record a launch writes when its detached half never started.
    ///
    /// **What triggers it is not tested and cannot cheaply be**, the same way the
    /// fallback below cannot: reaching it needs `detach` to fail, which needs a
    /// full disk or a binary that has moved out from under the process. So this
    /// pins the shape, which is the half that would otherwise be wrong without
    /// anyone noticing.
    #[test]
    fn a_launch_that_never_forked_still_writes_a_record() {
        let path =
            std::env::temp_dir().join(format!("hydra-never-started-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let run_id = "01a073ef-701c-7522-b5bb-56d5ecba0f57";
        let line_zero = serde_json::json!({
            "type": "run_start",
            "schema_version": SCHEMA_VERSION,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "cwd": "/tmp",
            "worktree": null,
            "policy": "standard",
            "policy_sha": "0000000000000000000000000000000000000000000000000000000000000000",
            "provider_version": "2.1.261 (Claude Code)",
            "provider_session_id": null,
            "task": "probe",
            "depth": 0,
            "pid": null,
            "process_start_time": null,
            "subscription": "anthropic",
        });
        never_started(
            &path,
            run_id,
            Provider::Claude,
            &line_zero,
            &anyhow::anyhow!("no space left on device"),
        )
        .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        crate::record::validate::check(&text, Some(run_id)).assert_ok();

        let end: serde_json::Value =
            serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
        // Contract 5.2: a run that never had a process. A reader must not take the
        // null for dead-and-unharvested.
        assert!(end["hydra"]["exit_code"].is_null());
        assert!(end["hydra"]["refusal"].is_null());
        assert!(
            end["hydra"]["error"]
                .as_str()
                .unwrap()
                .contains("no space left on device")
        );
        let _ = std::fs::remove_file(&path);
    }

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
