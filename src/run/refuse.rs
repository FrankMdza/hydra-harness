use super::*;

/// Map ticket 61. A launch from a checkout that is not the data root, with
/// `HYDRA_ROOT` unset, refuses before anything is written into the root.
///
/// Whether the variable is set arrives as an argument rather than being read
/// here, so a test can ask the question under a CI runner that sets it, which
/// is what failed run 35686710205 on 2026-09-22: the workflow exports
/// `HYDRA_ROOT` for the corpus walks, and the check returned early.
pub fn refuse_foreign_checkout(
    layout: &Layout,
    checkout: &std::path::Path,
    root_named: bool,
) -> Result<()> {
    if root_named {
        return Ok(());
    }
    let root = layout.root();
    if same_path(checkout, root) {
        return Ok(());
    }
    anyhow::bail!(
        "this launch would write into {}, but it was run from checkout {}, which is not \
         that root. Set HYDRA_ROOT to {} or to another directory if you meant a different \
         data root.",
        root.display(),
        checkout.display(),
        root.display()
    )
}

/// Build ticket 99. A launch from under `worktrees/` refuses before any git
/// runs there.
///
/// A run's checkout holds a `.git` file the run can rewrite, and launch
/// discovery runs plain git in the directory it was started from. So from
/// inside a checkout, `git worktree add` followed that file to whatever
/// repository it named and ran that repository's hooks outside the seatbelt:
/// the third hiring panel's H47, reproduced with plain git and a decoy
/// `post-checkout` hook. Nothing a human means to do starts a run there.
///
/// Since build ticket 105 the same holds under `skill-worktrees/`, where a
/// curate pass's checkout of the skill store keeps a `.git` file the pass can
/// write, and `hydra evolve` refuses too, before its own discovery.
pub fn refuse_launch_inside_checkout(layout: &Layout, dir: &std::path::Path) -> Result<()> {
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let dir = canonical(dir);
    for checkouts in [layout.worktrees(), layout.skill_worktrees()] {
        let checkouts = canonical(&checkouts);
        if dir.starts_with(&checkouts) {
            anyhow::bail!(
                "{} is under {}, where Hydra keeps its runs' checkouts. A checkout's `.git` \
                 file is one its run could rewrite, so Hydra runs no git from there. Launch \
                 from the repository the checkout came from.",
                dir.display(),
                checkouts.display()
            );
        }
    }
    Ok(())
}

fn same_path(left: &std::path::Path, right: &std::path::Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

/// Inputs for [`refuse`], the way [`RunStartRequest`] carries line 0.
pub struct RefuseRequest<'a> {
    pub run_file: &'a std::path::Path,
    pub run_id: &'a str,
    pub provider: Provider,
    pub line_zero: &'a serde_json::Value,
    pub refusal: crate::record::Refusal,
    pub remediation: String,
    pub policies: &'a crate::policy::PolicySet,
    pub worktree: Option<&'a WorktreeRef>,
    pub seed_tree: Option<&'a str>,
    pub layout: &'a Layout,
    pub parent_run_id: Option<&'a str>,
    /// [`Request::quiet`], carried to the two lines a refusal prints.
    pub quiet: bool,
}

pub(super) fn refuse_escalation(
    request: &Request,
    preflight: &PreflightInputs,
    reserved: &ReservedRun,
) -> Result<Option<Call>> {
    // Build ticket 57, read before the lineage because it is the question the
    // lineage cannot ask: `Lineage::of` is handed `--parent` and believes it, and
    // a flag left off reaches it as a human at depth 0. This reads the marker
    // Hydra set in the child's own environment instead, so the answer comes from
    // the launch rather than from the caller's argv.
    let caller = crate::containment::caller(&reserved.layout)?;
    // 6.1 item 3, and the one pre-flight check whose refusal is a run record. It
    // runs after the id is minted, which is out of the spec's order on purpose:
    // contract 8.3 makes this refusal a run file and a run file needs a run id.
    // Only a temporary reservation exists so far. The refusal drops it and
    // leaves the record of the attempt, which is what 7.3 asks for.
    // Build ticket 57, beside 6.1 item 3 and for the same reason: a refused
    // escalation is a run record (contract 8.3), and a run record needs a run id.
    // It goes first because it is the wider refusal. A caller inside a run that
    // names no parent has not yet been measured against any policy, so
    // `lineage.narrowing` below would compare its request against the empty bound
    // of a human at depth 0 and allow anything.
    let Some(remediation) = caller.escalation(request.parent.as_deref()) else {
        return Ok(None);
    };
    let line_zero = serde_json::to_value(run_start(
        request,
        RunStartRequest {
            task: &preflight.task,
            policy_sha: preflight.policies.sha(),
            provider_version: None,
            cwd: &preflight.invoked_in,
            worktree: None,
            env_allowlist: None,
            skills: None,
            mcp: None,
            lineage: &reserved.lineage,
            scopes: None,
            gate_sha: None,
            sandbox: None,
        },
    ))?;
    Ok(Some(refuse(RefuseRequest {
        run_file: &reserved.run_file,
        run_id: &reserved.run_id,
        provider: request.provider,
        line_zero: &line_zero,
        refusal: crate::record::Refusal::Containment,
        remediation,
        policies: &preflight.policies,
        worktree: None,
        seed_tree: None,
        layout: &reserved.layout,
        parent_run_id: reserved.lineage.parent_run_id(),
        quiet: reserved.quiet,
    })?))
}

pub(super) fn refuse_narrowing(
    request: &Request,
    preflight: &PreflightInputs,
    reserved: &ReservedRun,
) -> Result<Option<Call>> {
    let Some(remediation) = reserved
        .lineage
        .narrowing(&preflight.policies, &request.policy)?
    else {
        return Ok(None);
    };
    // Contract 8.3: no provider fields on a refused spawn. Nothing asked the
    // binary for its version and nothing built the child's environment, so
    // both are absent rather than guessed, and `skills` and `mcp` follow. The
    // envelope still names the provider, which is contract 4's field and a
    // fact about the request rather than about a process. `scopes` is absent
    // for the same reason and `floor_lifted` is not: the flag was typed on
    // this run, and no gate was compiled to resolve a path (contract 5.1).
    let line_zero = serde_json::to_value(run_start(
        request,
        RunStartRequest {
            task: &preflight.task,
            policy_sha: preflight.policies.sha(),
            provider_version: None,
            cwd: &preflight.invoked_in,
            worktree: None,
            env_allowlist: None,
            skills: None,
            mcp: None,
            lineage: &reserved.lineage,
            scopes: None,
            gate_sha: None,
            sandbox: None,
        },
    ))?;
    Ok(Some(refuse(RefuseRequest {
        run_file: &reserved.run_file,
        run_id: &reserved.run_id,
        provider: request.provider,
        line_zero: &line_zero,
        refusal: crate::record::Refusal::PolicyNarrowing,
        remediation,
        policies: &preflight.policies,
        worktree: None,
        seed_tree: None,
        layout: &reserved.layout,
        parent_run_id: reserved.lineage.parent_run_id(),
        quiet: reserved.quiet,
    })?))
}

pub(super) fn refuse_handoff(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    line_zero: &serde_json::Value,
    refusal: crate::record::Refusal,
    remediation: String,
) -> PhaseHandoff {
    PhaseHandoff::Done(refuse(RefuseRequest {
        run_file: &machine.reserved.run_file,
        run_id: &machine.reserved.run_id,
        provider: machine.request.provider,
        line_zero,
        refusal,
        remediation,
        policies: &machine.preflight.policies,
        worktree: worktree.worktree_ref.as_ref(),
        seed_tree: worktree.seed_tree.as_deref(),
        layout: &machine.reserved.layout,
        parent_run_id: machine.reserved.lineage.parent_run_id(),
        quiet: machine.reserved.quiet,
    }))
}

pub(super) fn handoff_preflight_refusals(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    recording: &mut PostWorktree,
    mcp_plan: &crate::mcp::Plan<'_>,
    refused_line_zero: &serde_json::Value,
    preflight: &Option<Result<cursor::Preflight, String>>,
) -> Option<PhaseHandoff> {
    if machine.request.provider == Provider::Codex {
        let remediation = match codex::login_status() {
            Ok(codex::LoginStatus::LoggedIn) => None,
            Ok(codex::LoginStatus::NotLoggedIn) => Some(codex::auth_remediation().to_string()),
            Err(err) => Some(format!(
                "the codex auth pre-flight could not run, so whether this launch is logged in \
                 is unknown and the run is refused: {err:#}"
            )),
        };
        if let Some(remediation) = remediation {
            return Some(refuse_handoff(
                machine,
                worktree,
                refused_line_zero,
                crate::record::Refusal::AuthPreflight,
                remediation,
            ));
        }
    }

    // 6.1 item 6, one item after it. A run whose policy admits a server this
    // launch cannot deliver is refused rather than launched short, which is
    // section 2's rule: a run that quietly comes up short is what it forbids.
    if mcp_plan.shortfall().is_some() {
        return Some(refuse_handoff(
            machine,
            worktree,
            refused_line_zero,
            crate::record::Refusal::McpUnavailable,
            mcp_plan.remediation(),
        ));
    }

    // 6.1 item 7. Any injected server the listing did not report ready fails the
    // run, because on this provider that listing is the one that lies: the
    // human's same-named server may be starting with the human's token while
    // the name reads as not loaded. Same refusal code as item 6; harness spec 12
    // says which check fired is a field Hydra sets, and here it is the
    // remediation.
    if let Some(preflight) = preflight {
        let remediation = match preflight {
            Ok(preflight) => {
                let short = preflight.shortfall(&recording.mcp_plan_injected);
                if short.is_empty() {
                    None
                } else {
                    Some(preflight.remediation(&short))
                }
            }
            Err(why) => Some(why.clone()),
        };
        if let Some(remediation) = remediation {
            return Some(refuse_handoff(
                machine,
                worktree,
                refused_line_zero,
                crate::record::Refusal::McpUnavailable,
                remediation,
            ));
        }
    }

    None
}

/// Writes the record for a launch one of 6.1's pre-flight checks refused.
///
/// Contract 8.3: no child process starts, so without the record the attempt is
/// invisible, and under D10 anything not in a run record did not happen. The shape
/// is `run_start` then `run_end` with a `refusal` on it, which derives its outcome
/// from `refusal` rather than from anything on a stream.
///
/// **One function for every check that refuses**, because the three steps are the
/// same three every time and only the code and the remediation change. Three of
/// contract 6's four checks are built and reach here: the policy containment of
/// 6.1 item 3, the skill collision of item 5 and the MCP launch check of item 6.
///
/// It is public so a test can drive the whole shape without launching a provider.
///
/// The checkout, the branch and the links stay where they are. `hydra reap` is
/// what removes them, the same as for a run that spawned, and a human who wants to
/// see what was refused can look at the directory.
pub fn refuse(request: RefuseRequest<'_>) -> Result<Call> {
    if !request.quiet {
        eprintln!("hydra: run {} refused", request.run_id);
        eprintln!("hydra: {}", request.remediation);
    }

    // Contract 4: the edge goes on every line of the child's file, including the
    // two a refused spawn gets, because a refused child is still a child.
    let mut writer = crate::record::Writer::create(
        request.run_file,
        request.run_id,
        request.parent_run_id,
        Some(request.provider.binary()),
        None,
    )?;
    writer.hydra_line(request.line_zero)?;
    writer.hydra_line(&crate::record::RunEnd::refused(
        request.refusal,
        request.remediation,
    ))?;
    drop(writer);

    // Read back through the normalizer for the same reason the spawned path does:
    // contract 17 makes the result a projection of the record, and reading the
    // file is the only way to keep that true rather than merely intended.
    //
    // **A failed read propagates here and does not on the spawned path**, and the
    // asymmetry is deliberate. There, the record is whole and the run worked, so
    // reporting a read failure as a failed run would be a lie. Here the run was
    // refused, the file is two lines Hydra just wrote with no tool call in it for
    // the gate replay to choke on, and a read that fails is a real fault rather
    // than a missing projection.
    let run = crate::normalize::read_file(request.run_file, request.policies)?;
    Ok(Call::Ended(Box::new(result(
        &run,
        request.run_file,
        request.worktree,
        request.seed_tree,
        request.layout,
    ))))
}
