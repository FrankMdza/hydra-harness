use super::*;

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
pub(super) fn never_started(
    run_file: &std::path::Path,
    run_id: &str,
    provider: Provider,
    line_zero: &serde_json::Value,
    err: &anyhow::Error,
    parent_run_id: Option<&str>,
) -> Result<()> {
    let mut writer = crate::record::Writer::create(
        run_file,
        run_id,
        parent_run_id,
        Some(provider.binary()),
        None,
    )?;
    writer.hydra_line(line_zero)?;
    writer.hydra_line(&crate::record::RunEnd::streamed(
        None,
        Some(format!("{err:#}")),
    ))?;
    Ok(())
}

/// Writes the record of a run whose launcher stopped before it handed off, and
/// says whether it did.
///
/// Build ticket 147, map ticket 84: a seat still inside [`run`] when its
/// session ends dies with the session's process, and its run id is already on
/// the session's `seat_request` line. The record is [`never_started`]'s shape,
/// composed from the request alone, like the two refusals above the probe:
/// nothing says how far the launch got, so no provider fields, no worktree and
/// a null pid. `task` is the task composed with the preamble, which is what
/// line 0 would have carried.
///
/// **`Writer::create` is the arbiter.** A launch that got as far as writing its
/// own file, or a watcher that did, keeps it: the file exists, this returns
/// `false` and appends nothing, because contract 2 allows one writer per file
/// and the other writer may still be alive.
pub fn record_unlaunched(
    layout: &Layout,
    request: &Request,
    policy_sha: &str,
    task: &str,
    reason: &str,
) -> Result<bool> {
    let run_id = request
        .run_id
        .as_deref()
        .context("a run that never launched still needs the id it was minted")?;
    let lineage = Lineage::of(layout, request.parent.as_deref())?;
    let cwd = request.working_dir.clone().unwrap_or_default();
    let line_zero = serde_json::to_value(run_start(
        request,
        RunStartRequest {
            task,
            policy_sha,
            provider_version: None,
            cwd: &cwd,
            worktree: None,
            env_allowlist: None,
            skills: None,
            mcp: None,
            lineage: &lineage,
            scopes: None,
            gate_sha: None,
            sandbox: None,
        },
    ))?;
    match never_started(
        &layout.run_file(run_id),
        run_id,
        request.provider,
        &line_zero,
        &anyhow::anyhow!("{reason}"),
        lineage.parent_run_id(),
    ) {
        Ok(()) => Ok(true),
        Err(err)
            if err
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io| io.kind() == std::io::ErrorKind::AlreadyExists) =>
        {
            Ok(false)
        }
        Err(err) => Err(err),
    }
}

/// The handle a timed-out call gets, built from line 0 rather than from the run.
///
/// Nothing here normalizes. The run has not ended, so there is nothing to derive,
/// and reading the record back would cost a python process per recorded tool call
/// to produce a page of nulls.
pub(super) fn handle(
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
pub(super) fn unread(
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
    //
    // The base itself comes from the record before it comes from the caller.
    // Map ticket 49 put `base_tree` on `run_start.worktree`, and a record that
    // carries it needs no spawn directory to answer this question. `seed_tree`
    // is what a caller recomputed from the seed index, the only source a record
    // written before build ticket 23 has, and it is tried only when the record
    // itself says nothing.
    let recorded_base_tree = run
        .run_start()
        .and_then(|start| start.body["worktree"]["base_tree"].as_str());
    let base_tree = recorded_base_tree.or(seed_tree);
    let diff_stat = match (worktree, base_tree, !run.run_id.is_empty()) {
        (Some(worktree), Some(base), true) => crate::worktree::CheckoutGit::for_run(
            &run.run_id,
            std::path::Path::new(&worktree.path),
            run.run_start().map(|start| &start.body),
        )
        .and_then(|context| {
            crate::worktree::diff_stat(&context, &layout.stat_index(&run.run_id), base)
        })
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

/// The resolved inputs used to compose a run's first record line.
pub struct RunStartRequest<'a> {
    pub task: &'a str,
    pub policy_sha: &'a str,
    // `None` on a refused spawn, where no binary was asked (contract 8.3).
    pub provider_version: Option<&'a str>,
    pub cwd: &'a std::path::Path,
    pub worktree: Option<WorktreeRef>,
    // `None` on a refused spawn, where no child environment was built. Absent
    // rather than empty, because an empty list reads as a measurement.
    pub env_allowlist: Option<Vec<String>>,
    pub skills: Option<serde_json::Value>,
    pub mcp: Option<serde_json::Value>,
    pub lineage: &'a Lineage,
    pub scopes: Option<std::collections::BTreeMap<String, String>>,
    // `None` on a refused spawn and on a run with no gate, build ticket 49.
    pub gate_sha: Option<String>,
    // Null on a launch no seatbelt wrapped, build ticket 68.
    pub sandbox: Option<String>,
}

/// Line 0, composed before the fork.
///
/// **`pid` and `process_start_time` are null here and are the detached half's to
/// fill in.** They are contract 5.2's liveness pair and neither exists until a
/// process does, which is the one fact this side cannot know.
///
/// It is public for the same reason [`refuse`] is: `tests/delegation.rs` composes
/// the line a refused child gets and everything above it in [`run`] needs a
/// provider binary on PATH.
pub fn run_start(request: &Request, start: RunStartRequest<'_>) -> RunStart {
    let RunStartRequest {
        task,
        policy_sha,
        provider_version,
        cwd,
        worktree,
        env_allowlist,
        skills,
        mcp,
        lineage,
        scopes,
        gate_sha,
        sandbox,
    } = start;
    RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: env!("CARGO_PKG_VERSION").to_string(),
        command: request.command.clone(),
        launch_mode: Some("recorded".to_string()),
        provider_version: provider_version.map(str::to_string),
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
        // Harness spec 16: the flag was typed on this run whether or not the
        // launch got as far as compiling a gate, so it is filled on a refused
        // run too. The `scopes` beside it are not, because they are a fact about
        // a compile that may not have happened.
        floor_lifted: Some(request.floor_lifted),
        gate_sha,
        sandbox,
        scopes,
        cwd: cwd.display().to_string(),
        worktree,
        // 6.1 item 2: the parent's depth plus one, and 0 for a run a human
        // started. The cap has already refused anything past it.
        depth: lineage.depth,
        // Contract 8.3 wants the parent's policy beside the requested one, and a
        // run with no parent leaves the field out rather than null.
        parent_policy: lineage.parent_policy().map(str::to_string),
        pid: None,
        process_start_time: None,
        // Contract 5.3's three fields, or absent on a run with no checkout to
        // inject into. Absent rather than null: with nowhere to link, which sha
        // the run pinned is not a question with a missing answer.
        skills,
        // Contract 5.3's other group, and present on every recorded `claude` run
        // this build launches. It needs no checkout, because on this provider the
        // servers ride the command line, so a run outside a repo carries `mcp`
        // and no `skills`.
        mcp,
        auth: Some(Auth {
            // Authored from what Hydra did rather than sniffed. The allowlist passes
            // no key, so the child can only be on the stored subscription login.
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist,
        subscription: Some(request.provider.subscription().to_string()),
        resumed_from: request.resumed_from.clone(),
        may_edit: None,
    }
}
