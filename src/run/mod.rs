//! `hydra run`: the launch sequence, harness spec 6, and the view over it, 9.
//!
//! What a launch does, in order: build the child's environment from the allowlist,
//! read the provider's version, load the policy set, the preamble and the canonical
//! MCP document, read the parent's record for its depth and its policy, refuse if
//! the concurrency cap is already met, mint a UUIDv7 run id, refuse a child whose
//! policy the parent's does not contain, create a worktree and seed it, write the
//! run's MCP config, pin the skill store
//! and link a snapshot of it into the checkout, ask the provider what it resolved
//! and refuse on a skill collision or an MCP shortfall, run Cursor's fatal
//! pre-flight when the provider is `cursor-agent`, compile the run's gate into its
//! spawn directory, compose the provider's argv and the whole of line 0, then hand
//! all of it to [`crate::watch`], which spawns the provider on a pty in a session
//! of its own and owns the record from there.
//!
//! What it does not do, and what that costs the record, is in `IMPLEMENTATION.md`.
//! The short version: `skills` is absent on a run with no checkout to inject into,
//! `mcp` is present on every recorded `claude` run because on that provider the
//! servers ride the command line rather than the worktree, and a `cursor-agent`
//! run needs a checkout of Hydra's own for everything, so it is refused before a
//! run id exists when there is none.
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
use crate::provider::{Provider, agy, claude, codex, cursor};
use crate::record::{Auth, RunStart, SCHEMA_VERSION, WorktreeRef};

pub use crate::spawn_config::{SpawnFiles, write_spawn_config};

mod lineage;
mod record;
mod refuse;

pub use lineage::{Lineage, TooDeep};
pub use record::{LAUNCH_BUDGET, RunStartRequest, record_unlaunched, result, run_start};
pub use refuse::{RefuseRequest, refuse, refuse_foreign_checkout, refuse_launch_inside_checkout};

use record::{handle, never_started, unread};
use refuse::{handoff_preflight_refusals, refuse_escalation, refuse_handoff, refuse_narrowing};

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
    /// `--parent`, the run id of the run that asked for this one.
    ///
    /// It is a run id and never a path, because the record under
    /// `~/hydra/runs/<run-id>.jsonl` is the only thing that says how deep the
    /// parent is and what policy bounds it. A parent with no record is refused
    /// before a run id exists (6.1 item 2).
    pub parent: Option<String>,
    /// The run id, when the caller minted it before the launch.
    ///
    /// **`hydra run` passes `None` and this function mints one.** `hydra curate`
    /// is the caller that cannot: it checks the skill store out at a path named
    /// after the run, and the gate scopes that run's writes to it, so the id has
    /// to exist before either. Nothing else about the launch changes, and the id
    /// still reaches the record the same way.
    pub run_id: Option<String>,
    /// `run` or `resume`. Harness spec 17's ninth command mints `resume`.
    pub command: String,
    /// The run whose provider session this launch continues, when `command` is
    /// `resume`.
    pub resumed_from: Option<String>,
    /// The provider's own session id, passed on its resume flag when set.
    pub resume_session_id: Option<String>,
    /// Where to seed the worktree from, when the caller is not the cwd Hydra was
    /// invoked in. `hydra resume` passes the source run's `cwd`.
    pub working_dir: Option<PathBuf>,
    /// Prints nothing on stderr. Build ticket 142: a gated session's launcher
    /// sets it, because its stderr is the terminal `claude` is drawing on. The
    /// lines go nowhere, since the seat's own record carries the run id, the
    /// worktree and any refusal they would have named.
    pub quiet: bool,
    /// Called once, when the run's detached half exists and this process can
    /// exit without taking the run with it. Build ticket 142: the launcher
    /// waits on it before the harvest ends the session's process.
    pub handed_off: Option<HandedOff>,
}

/// What [`Request::handed_off`] calls.
pub type HandedOff = Box<dyn FnOnce() + Send>;

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

/// What the pre-run checks produced before a run id is minted.
struct PreflightInputs {
    invoked_in: PathBuf,
    repo: Option<PathBuf>,
    policies: crate::policy::PolicySet,
    task: String,
    mcp_document: crate::mcp::Document,
    root_named: bool,
}

/// What exists once the layout is open and the run id is reserved.
struct ReservedRun {
    layout: Layout,
    lineage: Lineage,
    run_id: String,
    run_file: PathBuf,
    reservation: Option<crate::live::Reservation>,
    /// [`Request::quiet`], here because every failure path reads this struct.
    quiet: bool,
}

/// Machine facts read after the attempt has been judged.
struct MachineReady {
    request: Request,
    preflight: PreflightInputs,
    reserved: ReservedRun,
    child_env: ChildEnv,
    provider_version: String,
    hydra_env: std::collections::BTreeMap<String, String>,
}

/// A checkout and the spawn-side paths that hang off it.
struct WorktreeReady {
    cwd: PathBuf,
    worktree_ref: Option<WorktreeRef>,
    seed_tree: Option<String>,
}

/// What the run is handed before the spawn: MCP, skills, gate, line 0.
struct HandoffReady {
    mcp_file: Option<PathBuf>,
    injected: Option<InjectedSkills>,
    spawn: SpawnFiles,
    line_zero: serde_json::Value,
}

/// Context for composing line 0 after the worktree exists.
struct LineZeroParts<'a, 's> {
    request: &'a Request,
    policies: &'a crate::policy::PolicySet,
    task: &'a str,
    provider_version: &'a str,
    cwd: &'a std::path::Path,
    worktree_ref: Option<&'a WorktreeRef>,
    env_allowlist: Vec<String>,
    injected: Option<&'s InjectedSkills>,
    mcp_document: &'a crate::mcp::Document,
    injected_names: &'s [String],
    mcp_resolved: Option<&'s [String]>,
    lineage: &'a Lineage,
}

/// Handoff state recorded for a `never_started` line 0 if a later step fails.
struct PostWorktree {
    mcp_plan_injected: Vec<String>,
    mcp_resolved: Option<Vec<String>>,
    injected: Option<InjectedSkills>,
}

pub fn run(request: Request) -> Result<Call> {
    // **What the attempt was is decided before what the machine is.** 6.1's items
    // 2 and 3 run first here, ahead of the environment, the provider's version and
    // the two repo files, because 7.3 wants an attempted escalation on disk and
    // those three can all fail on a machine where the child was never going to
    // start. Reading the version first would turn a child reaching for `permissive`
    // on a binary nobody installed into an error with nothing written, which is the
    // invisible attempt the containment check exists to prevent.
    let preflight = preflight_inputs(&request)?;
    let reserved = reserve_run(&request, &preflight)?;
    if let Some(call) = refuse_escalation(&request, &preflight, &reserved)? {
        return Ok(call);
    }
    if let Some(call) = refuse_narrowing(&request, &preflight, &reserved)? {
        return Ok(call);
    }
    let mut machine = prepare_machine(request, preflight, reserved)?;
    let worktree = phase_worktree(&machine)?;
    let mut recording = PostWorktree::new();
    let handoff = match phase_handoff(&machine, &worktree, &mut recording) {
        Ok(PhaseHandoff::Done(call)) => return call,
        Ok(PhaseHandoff::Ready(handoff)) => handoff,
        Err(err) => return Err(recording.fail(&machine, &worktree, err, true, None)),
    };
    phase_spawn(&mut machine, &worktree, handoff)
}

enum PhaseHandoff {
    Done(Result<Call>),
    Ready(HandoffReady),
}

fn preflight_inputs(request: &Request) -> Result<PreflightInputs> {
    let invoked_in = match &request.working_dir {
        Some(dir) => dir.clone(),
        None => std::env::current_dir().context("reading the working directory")?,
    };
    // Read before a run id exists: a policy name that is not one of the five, or a
    // preamble file that has gone missing, is a fact about this machine rather than
    // a refused spawn. Line 0 needs all three whichever way the launch goes.
    let policies = crate::policy::PolicySet::discover()?;
    policies.get(&request.policy)?;
    let preamble = crate::preamble::load()?;
    // Contract 5.1: the task is recorded verbatim, preamble and all. It is
    // composed here rather than beside the argv because a launch refused by 6.1
    // item 3 records line 0 too, and line 0 carries the task.
    let task = crate::preamble::compose(&preamble, &request.task);
    // Protected config #5, read here for the same reason: a document that will
    // not parse is a fact about this machine rather than a refused spawn. The
    // refusal it can produce is 6.1 item 6's, and that one needs a run id.
    let mcp_document = crate::mcp::Document::load()?;
    let root_named = std::env::var_os("HYDRA_ROOT").is_some();
    // A run that unsets the marker of build ticket 57 is a human to the check
    // below, and `HYDRA_ROOT` then points its record somewhere the kernel does
    // not deny. The seatbelt is the one thing about its ancestry a run cannot
    // shed, so the variable is refused from inside one before any root is
    // read. Reviewed 2026-09-22: `env -u HYDRA_RUN_ID HYDRA_ROOT=/tmp/x hydra
    // run --policy permissive` launched from a `standard` run.
    if root_named && crate::sandbox::inside_seatbelt() {
        anyhow::bail!(
            "HYDRA_ROOT is set and this process is inside the seatbelt Hydra put around a \
             run, so the record was going to land where the kernel does not deny it. A run \
             cannot move the root. Unset the variable, or launch from outside the run."
        );
    }
    refuse_launch_inside_checkout(&Layout::discover()?, &invoked_in)?;
    let repo = crate::worktree::repo_root(&invoked_in);
    Ok(PreflightInputs {
        invoked_in,
        repo,
        policies,
        task,
        mcp_document,
        root_named,
    })
}

fn reserve_run(request: &Request, preflight: &PreflightInputs) -> Result<ReservedRun> {
    let layout = Layout::discover()?;
    reserve_on_layout(request, preflight, layout)
}

fn reserve_on_layout(
    request: &Request,
    preflight: &PreflightInputs,
    layout: Layout,
) -> Result<ReservedRun> {
    crate::paths::refuse_non_ascii_roots(&layout, &crate::paths::hydra_home())?;
    refuse_foreign_checkout(&layout, &crate::paths::hydra_home(), preflight.root_named)?;
    layout.ensure()?;
    // 6.1 item 2, ahead of item 4 below in the order the spec lists them. The
    // parent's own record is the only thing that says how deep this child is and
    // what policy bounds it, so a `--parent` this machine has no record of is
    // refused here rather than launched bounded by nothing.
    let lineage = Lineage::of(&layout, request.parent.as_deref())?;
    let run_id = request
        .run_id
        .clone()
        .unwrap_or_else(|| Uuid::now_v7().to_string());
    let run_file = layout.run_file(&run_id);
    let reservation = crate::live::reserve(&layout, &run_id, crate::live::LIVE_RUN_CAP)?;
    Ok(ReservedRun {
        layout,
        lineage,
        run_id,
        run_file,
        reservation: Some(reservation),
        quiet: request.quiet,
    })
}

fn prepare_machine(
    request: Request,
    preflight: PreflightInputs,
    reserved: ReservedRun,
) -> Result<MachineReady> {
    // A pair this build does not route is a fact about the attempt, not the
    // machine: reading the version first would turn `codex` under `read-only`
    // into "not on PATH" on a box with no binary. The face says the same words
    // before it asks a seatbelt.
    if request.provider == Provider::Codex
        && let Some(remediation) = codex::unsupported_policy(&request.policy)
    {
        anyhow::bail!(remediation);
    }
    // The machine's own facts, read once the attempt has been judged. A wrong
    // PATH, a missing HOME, a provider binary that is not installed and a repo
    // that already carries the provider's two files are all errors rather than
    // refused spawns, the way they were before delegation existed.
    let (child_env, hydra_env) = compose_child_env(
        request.provider,
        &reserved.layout,
        &reserved.run_id,
        preflight.repo.as_deref(),
    )?;
    let provider_version = request.provider.version()?;
    // Both providers that read their hook config out of the worktree get the same
    // check against their own two files.
    match request.provider {
        Provider::CursorAgent => worktree_preconditions(
            "cursor-agent",
            preflight.repo.as_deref(),
            &[cursor::hooks_file, cursor::mcp_file],
        )?,
        Provider::Agy => worktree_preconditions(
            "agy",
            preflight.repo.as_deref(),
            &[agy::hooks_file, agy::mcp_file],
        )?,
        Provider::Codex => worktree_preconditions(
            "codex",
            preflight.repo.as_deref(),
            &[codex::hooks_file, codex::mcp_file],
        )?,
        Provider::Claude => {}
    }
    Ok(MachineReady {
        request,
        preflight,
        reserved,
        child_env,
        provider_version,
        hydra_env,
    })
}

/// The child's environment, and the part of it Hydra set rather than passed.
///
/// **The second map is what reaches the child.** The launcher never execs the
/// provider. The detached half does, and it rebuilds the allowlist out of its
/// own environment, which drops every name the allowlist does not hold. So a
/// variable Hydra sets reaches the child only by value on the launch spec.
/// Until build ticket 97 the marker went on the first map alone:
/// `run_start.env_allowlist` named `HYDRA_RUN_ID`, no recorded child carried
/// it, and build ticket 57's check read a recorded run's relaunch as a human's.
fn compose_child_env(
    provider: Provider,
    layout: &Layout,
    run_id: &str,
    repo: Option<&std::path::Path>,
) -> Result<(ChildEnv, std::collections::BTreeMap<String, String>)> {
    let mut child_env = ChildEnv::build(provider)?;
    let mut hydra_env = std::collections::BTreeMap::new();
    // 6.2's second category and build ticket 57's whole mechanism: the child is
    // told which run it is, so that a `hydra run` it launches has something to
    // check itself against. The name lands in `run_start.env_allowlist` like
    // every other name passed, so a reader can see the child was marked.
    hydra_env.insert(crate::containment::MARKER.to_string(), run_id.to_string());
    // `CURSOR_DATA_DIR` moves Cursor's per-project MCP approval state into the
    // run's spawn directory, so approving an injected server writes under a path
    // Hydra created rather than into `~/.cursor/`. It is a path Hydra computed
    // and not a secret the launcher inherited, so it may travel by value.
    if provider == Provider::CursorAgent {
        let data_dir = layout.cursor_data_dir(run_id);
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating {}", data_dir.display()))?;
        hydra_env.insert(
            "CURSOR_DATA_DIR".to_string(),
            data_dir.display().to_string(),
        );
    }
    // Map ticket 74: a run with a checkout writes git objects into its own
    // directory under the spawn directory and reads the parent's as an
    // alternate. Two paths Hydra computed, so they travel by value too.
    if let Some(repo) = repo {
        for (name, value) in crate::worktree::prepare_run_objects(layout, run_id, repo)? {
            hydra_env.insert(name.to_string(), value);
        }
    }
    for (name, value) in &hydra_env {
        child_env.set(name, value.clone());
    }
    Ok((child_env, hydra_env))
}

fn phase_worktree(machine: &MachineReady) -> Result<WorktreeReady> {
    let worktree = match &machine.preflight.repo {
        Some(repo) => Some(crate::worktree::create(
            repo,
            &machine.reserved.layout.worktree(&machine.reserved.run_id),
            &format!("hydra/{}", machine.reserved.run_id),
            &machine.reserved.layout.seed_index(&machine.reserved.run_id),
        )?),
        None => None,
    };
    let seed_tree = worktree.as_ref().and_then(|w| w.seed_tree.clone());
    let cwd = worktree
        .as_ref()
        .map(|w| w.path.clone())
        .unwrap_or_else(|| machine.preflight.invoked_in.clone());
    let worktree_ref = worktree.as_ref().map(|w| WorktreeRef {
        path: w.path.display().to_string(),
        branch: w.branch.clone(),
        base_tree: w.seed_tree.clone(),
    });
    Ok(WorktreeReady {
        cwd,
        worktree_ref,
        seed_tree,
    })
}

impl PostWorktree {
    fn new() -> Self {
        PostWorktree {
            mcp_plan_injected: Vec::new(),
            mcp_resolved: None,
            injected: None,
        }
    }

    fn parts<'a, 's>(
        &'s self,
        machine: &'a MachineReady,
        worktree: &'a WorktreeReady,
    ) -> LineZeroParts<'a, 's> {
        LineZeroParts {
            request: &machine.request,
            policies: &machine.preflight.policies,
            task: &machine.preflight.task,
            provider_version: &machine.provider_version,
            cwd: &worktree.cwd,
            worktree_ref: worktree.worktree_ref.as_ref(),
            env_allowlist: machine.child_env.names(),
            injected: self.injected.as_ref(),
            mcp_document: &machine.preflight.mcp_document,
            injected_names: &self.mcp_plan_injected,
            mcp_resolved: self.mcp_resolved.as_deref(),
            lineage: &machine.reserved.lineage,
        }
    }

    fn fail(
        &self,
        machine: &MachineReady,
        worktree: &WorktreeReady,
        err: anyhow::Error,
        announce: bool,
        spawn: Option<&SpawnFiles>,
    ) -> anyhow::Error {
        announce_launch_failure(
            &machine.reserved,
            worktree.worktree_ref.as_ref(),
            announce,
            &err,
        );
        match compose_line_zero(self.parts(machine, worktree), spawn) {
            Ok(line_zero) => record_never_started(
                &machine.reserved,
                machine.request.provider,
                &line_zero,
                &err,
            ),
            Err(e) if !machine.reserved.quiet => {
                eprintln!("hydra: this run could not be recorded either: {e:#}");
            }
            Err(_) => {}
        }
        err
    }
}

fn announce_launch_failure(
    reserved: &ReservedRun,
    worktree: Option<&WorktreeRef>,
    announce: bool,
    err: &anyhow::Error,
) {
    if !announce || reserved.quiet {
        return;
    }
    eprintln!("hydra: run {} did not launch", reserved.run_id);
    eprintln!("hydra: {err:#}");
    eprintln!("hydra: record {}", reserved.run_file.display());
    if let Some(worktree) = worktree {
        eprintln!("hydra: worktree {} on {}", worktree.path, worktree.branch);
    }
}

fn record_never_started(
    reserved: &ReservedRun,
    provider: Provider,
    line_zero: &serde_json::Value,
    err: &anyhow::Error,
) {
    if let Err(second) = never_started(
        &reserved.run_file,
        &reserved.run_id,
        provider,
        line_zero,
        err,
        reserved.lineage.parent_run_id(),
    ) && !reserved.quiet
    {
        eprintln!("hydra: this run could not be recorded either: {second:#}");
    }
}

fn fail_after_handoff(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    handoff: &HandoffReady,
    err: anyhow::Error,
    announce: bool,
) -> anyhow::Error {
    announce_launch_failure(
        &machine.reserved,
        worktree.worktree_ref.as_ref(),
        announce,
        &err,
    );
    record_never_started(
        &machine.reserved,
        machine.request.provider,
        &handoff.line_zero,
        &err,
    );
    err
}

fn compose_line_zero(
    parts: LineZeroParts<'_, '_>,
    spawn: Option<&SpawnFiles>,
) -> Result<serde_json::Value> {
    // The whole of line 0 except the two fields that do not exist until a process
    // does, as a function of the two the steps below decide.
    // Composing it here keeps every provider-specific fact on this side of the
    // fork and leaves the detached half nothing to decide, and a refused launch
    // writes the same line 0 a spawned one would, because contract 8.3 wants the
    // attempt on disk in full.
    //
    // **`scopes` is absent until a gate is compiled.** Since build ticket 375
    // the compile comes before the probe and so before every handoff refusal,
    // which carry it. A failure before the compile resolved no scope, so
    // contract 5.1's rule applies there: the key is absent rather than filled
    // with what the compile would have produced had it run.
    let (scopes, gate_sha, sandbox) = match spawn {
        Some(spawn) => (
            Some(&spawn.scopes),
            Some(&spawn.gate_sha),
            spawn.sandbox_sha.as_deref(),
        ),
        None => (None, None, None),
    };
    let start = run_start(
        parts.request,
        RunStartRequest {
            task: parts.task,
            policy_sha: parts.policies.sha(),
            provider_version: Some(parts.provider_version),
            cwd: parts.cwd,
            worktree: parts.worktree_ref.cloned(),
            env_allowlist: Some(parts.env_allowlist.clone()),
            skills: parts.injected.map(|i| i.record.clone()),
            mcp: Some(crate::mcp::record(
                &parts.mcp_document.sha,
                parts.injected_names,
                parts.mcp_resolved,
            )),
            lineage: parts.lineage,
            scopes: scopes.cloned(),
            gate_sha: gate_sha.map(|sha| sha.to_string()),
            sandbox: sandbox.map(str::to_string),
        },
    );
    Ok(serde_json::to_value(&start)?)
}

fn phase_handoff(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    recording: &mut PostWorktree,
) -> Result<PhaseHandoff> {
    // 6.5: the document decides which servers this run's policy admits and the
    // launch shape decides whether any of them can be delivered. The file is
    // written before the probe below, because the probe carries the run's own MCP
    // flags and that is what makes its answer the run's rather than the machine's.
    // A launch that cannot deliver gets no file, and the check that refuses it
    // runs after the skill collision check, where 6.1 puts it.
    let checkout = match &worktree.worktree_ref {
        Some(_) => crate::mcp::Checkout::Hydra,
        None => crate::mcp::Checkout::None,
    };
    let mcp_plan = crate::mcp::plan(
        &machine.preflight.mcp_document,
        machine.request.provider,
        &machine.request.policy,
        checkout,
    );
    recording.mcp_plan_injected = mcp_plan.injected_names();
    let mcp_file = match mcp_plan.undeliverable {
        Some(_) => None,
        None => Some(crate::mcp::write_config(
            &machine.reserved.layout,
            &machine.reserved.run_id,
            machine.request.provider,
            mcp_plan.injected(),
            worktree
                .worktree_ref
                .as_ref()
                .map(|w| std::path::Path::new(&w.path)),
        )?),
    };
    let linked = link_into_checkout(machine, worktree, recording)?;

    // Build ticket 375: the gate and the profile are written before the probe,
    // because the probe runs the provider's binary and has to run inside the
    // wrapper the run gets. The bubblewrap profile reads the gate off disk, so
    // the whole spawn config moves and not the profile alone. Every refusal
    // below therefore comes after a compiled gate, and its line 0 carries the
    // scopes that gate resolved.
    let spawn = write_spawn_config(
        &machine.reserved.layout,
        &machine.reserved.run_id,
        &machine.preflight.policies,
        machine.request.provider,
        &machine.request.policy,
        worktree
            .worktree_ref
            .as_ref()
            .map(|w| std::path::Path::new(&w.path)),
        machine.preflight.repo.as_deref(),
        recording.injected.as_ref().map(|i| i.store_sha.as_str()),
        machine.request.floor_lifted,
    )?;
    if let Some(linked) = &linked
        && let Err(err) = probe_checkout(
            machine,
            worktree,
            linked,
            mcp_file.as_deref(),
            spawn.sandbox_profile.as_deref(),
            recording,
        )
    {
        return Ok(PhaseHandoff::Done(Err(recording.fail(
            machine,
            worktree,
            err,
            true,
            Some(&spawn),
        ))));
    }

    // 6.1 item 5. A hit fails the run as its own outcome, and the record is the
    // whole of what the caller gets back. It is read before the pre-flight below
    // approves anything, so a launch refused here has started no server process,
    // and on `cursor-agent` its record says nobody asked about servers, which is
    // the true answer.
    if let Some(injected) = recording.injected.as_ref()
        && !injected.collisions.is_empty()
    {
        let remediation = crate::skills::collision::remediation(&injected.collisions);
        return Ok(refuse_handoff(
            machine,
            worktree,
            &compose_line_zero(recording.parts(machine, worktree), Some(&spawn))?,
            crate::record::Refusal::SkillCollision,
            remediation,
        ));
    }

    // 6.1 item 7, the pre-flight Cursor alone gets, run here because its listing
    // is the provider's own report of what the run will start, which is what
    // contract 5.3's `mcp.resolved` is on this provider, and line 0 carries it.
    // Its verdict is read after item 6 below, in 6.1's order. A pre-flight that
    // could not run at all is the same refusal with the error as its remediation:
    // nothing spawned, and the record says why.
    let preflight = match (
        machine.request.provider,
        &worktree.worktree_ref,
        mcp_plan.undeliverable,
    ) {
        (Provider::CursorAgent, Some(worktree_ref), None) => {
            match cursor::preflight(
                std::path::Path::new(&worktree_ref.path),
                &machine.child_env,
                &recording.mcp_plan_injected,
                spawn.sandbox_profile.as_deref(),
            ) {
                Ok(preflight) => Some(Ok(preflight)),
                Err(err) => Some(Err(format!(
                    "the Cursor MCP pre-flight could not run, so whether the injected \
                     servers would start is unknown and the run is refused: {err:#}"
                ))),
            }
        }
        _ => None,
    };
    let mcp_resolved: Option<Vec<String>> = match &preflight {
        Some(Ok(preflight)) => Some(preflight.resolved()),
        Some(Err(_)) => None,
        None => recording
            .injected
            .as_ref()
            .and_then(|injected| injected.mcp_resolved.clone()),
    };
    recording.mcp_resolved = mcp_resolved.clone();
    let refused_line_zero = compose_line_zero(recording.parts(machine, worktree), Some(&spawn))?;
    if let Some(done) = handoff_preflight_refusals(
        machine,
        worktree,
        recording,
        &mcp_plan,
        &refused_line_zero,
        &preflight,
    ) {
        return Ok(done);
    }

    // Line 0 again, after the pre-flight refusals had their chance to change
    // what the recording holds.
    let line_zero = compose_line_zero(recording.parts(machine, worktree), Some(&spawn))?;
    Ok(PhaseHandoff::Ready(HandoffReady {
        mcp_file,
        injected: std::mem::take(&mut recording.injected),
        spawn,
        line_zero,
    }))
}

/// 6.4: the store is pinned, snapshotted and linked into the checkout before
/// anything asks the provider a question. A run with no checkout has nowhere to
/// inject into, so it gets no links and its record leaves `skills` absent
/// rather than null: the question does not apply. A `judge` run is not injected
/// either, since the policy allows nothing and the diff rides the prompt, and
/// ticket 25 stopped the links. The MCP group is present either way, which is
/// 6.5's asymmetry showing up in the record: on `claude` the servers ride the
/// command line and need no checkout to land in.
///
/// The answer lands in `recording` even when a step fails partway, so the
/// failure record names what reached the checkout; see [`link_skills`].
///
/// `None` means nothing was linked, and then nothing is probed either.
fn link_into_checkout(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    recording: &mut PostWorktree,
) -> Result<Option<crate::skills::Injected>> {
    let Some(worktree_ref) = &worktree.worktree_ref else {
        return Ok(None);
    };
    if machine.request.policy == "judge" {
        return Ok(None);
    }
    link_skills(
        &machine.reserved.layout,
        &machine.reserved.run_id,
        machine.request.provider,
        std::path::Path::new(&worktree_ref.path),
        &mut recording.injected,
    )
    .map(Some)
}

/// Asks the provider what it resolved for the checkout [`link_into_checkout`]
/// linked, inside the run's own profile. Build ticket 375 moved this after
/// `write_spawn_config`, so the probe and the run see the same wrapper.
fn probe_checkout(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    linked: &crate::skills::Injected,
    mcp_file: Option<&std::path::Path>,
    profile: Option<&std::path::Path>,
    recording: &mut PostWorktree,
) -> Result<()> {
    let Some(worktree_ref) = &worktree.worktree_ref else {
        return Ok(());
    };
    probe_skills(
        machine.request.provider,
        std::path::Path::new(&worktree_ref.path),
        crate::mcp::Probe {
            policy: &machine.request.policy,
            config: mcp_file,
        },
        profile,
        linked,
        &mut recording.injected,
        machine.reserved.quiet,
    )
}

/// **The gate mount is checked on `agy` and on nothing else, free.** 6.6's
/// table says this provider reads its hook config only when the worktree
/// arrives as `--add-dir`, "without which it reports `hooks: []` rather than
/// an error", and that silence is the worst failure mode on the ticket: a run
/// that looks gated and is not. One read-only slash command rules it out for
/// no tokens, measured. `claude` needs no equivalent, because its config is an
/// argument it either accepted or refused to start with, and Cursor's
/// `failClosed` config would block rather than leak.
///
/// A launch that fails here writes the record itself, the same shape a spawn
/// that could not fork gets: contract 8.3 wants the attempt on disk, and by
/// this point a worktree, a branch and a compiled gate are on disk with a run
/// id on them. It is not one of contract 6's four refusals, so it is not
/// dressed as one: this is a launch that could not start, and it derives
/// `failed`. The caller writes that record through [`fail_after_handoff`].
fn agy_gate_mounted(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    handoff: &HandoffReady,
) -> Result<()> {
    if machine.request.provider != Provider::Agy {
        return Ok(());
    }
    let checkout = match worktree.worktree_ref.as_ref() {
        Some(w) => PathBuf::from(&w.path),
        None => anyhow::bail!("an agy run needs a checkout and this one has none"),
    };
    let mounted = agy::hooks_mounted(&checkout, handoff.spawn.sandbox_profile.as_deref())
        .with_context(|| {
            format!(
                "asking agy which hooks it loaded for {}, which is what proves the gate is \
             in front of the run",
                checkout.display()
            )
        })?;
    if agy::gate_is_mounted(&mounted, &handoff.spawn.hook_config) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "{}",
            agy::unmounted_remediation(&mounted, &handoff.spawn.hook_config)
        ))
    }
}

fn phase_spawn(
    machine: &mut MachineReady,
    worktree: &WorktreeReady,
    handoff: HandoffReady,
) -> Result<Call> {
    // Under D10 anything not in a run record did not happen. One path covers every
    // error after the worktree exists: [`fail_after_handoff`] and
    // [`PostWorktree::fail`] both reach [`never_started`], whether the failure is
    // in handoff, argv build, agy hook mount or detach.
    if let Some(msg) = crate::sandbox::launch_refusal_for_policy(&machine.request.policy) {
        return Err(fail_after_handoff(
            machine,
            worktree,
            &handoff,
            anyhow::anyhow!(msg),
            true,
        ));
    }

    if let Err(err) = agy_gate_mounted(machine, worktree, &handoff) {
        // The same three lines `refuse` prints, for the same reason: a
        // checkout, a branch and a compiled gate are on disk with this run
        // id on them, and an error naming none of them leaves a human with
        // an orphan worktree and nothing to type into `hydra reap`.
        return Err(fail_after_handoff(machine, worktree, &handoff, err, true));
    }

    let mut argv = match build_argv(machine, worktree, &handoff) {
        Ok(argv) => argv,
        Err(err) => return Err(fail_after_handoff(machine, worktree, &handoff, err, true)),
    };
    if let Some(session_id) = &machine.request.resume_session_id {
        let hook_file = if machine.request.provider == Provider::Codex {
            Some(codex::hook_file(
                &machine.reserved.layout,
                &machine.reserved.run_id,
            ))
        } else {
            None
        };
        append_resume_args(
            &mut argv,
            machine.request.provider,
            session_id,
            hook_file.as_deref(),
        );
    }

    let spec_path = machine
        .reserved
        .layout
        .launch_spec(&machine.reserved.run_id);
    // Everything from the spec file to the fork happens before the detached half
    // exists, so there is nothing on the other side to write a run file. The two
    // writers cannot collide, because the failure is what stops the second one
    // from starting.
    let launched = crate::watch::write_spec(
        &spec_path,
        &crate::watch::Launch {
            run_id: machine.reserved.run_id.clone(),
            // Contract 4's edge, carried across the fork. The detached half owns
            // every line of the record, so this is the only way the parent's id
            // reaches any of them.
            parent_run_id: machine.reserved.lineage.parent_run_id().map(str::to_string),
            provider: machine.request.provider.to_string(),
            binary: machine.request.provider.binary().to_string(),
            argv,
            cwd: worktree.cwd.clone(),
            run_file: machine.reserved.run_file.clone(),
            model: machine.request.model.clone(),
            hydra_env: machine.hydra_env.clone(),
            run_start: handoff.line_zero.clone(),
            sandbox_profile: handoff.spawn.sandbox_profile.clone(),
            skill_snapshot: if machine.request.provider.reads_skills_by_shell() {
                handoff.injected.as_ref().map(|injected| {
                    machine
                        .reserved
                        .layout
                        .skill_snapshot_scope(&machine.reserved.run_id, Some(&injected.store_sha))
                })
            } else {
                None
            },
        },
    )
    .and_then(|()| std::env::current_exe().context("locating the hydra binary"))
    .and_then(|exe| {
        crate::watch::detach(
            &exe,
            &spec_path,
            &machine
                .reserved
                .layout
                .watcher_log(&machine.reserved.run_id),
            &machine.child_env,
        )
    });

    let mut watcher = match launched {
        Ok(watcher) => watcher,
        Err(err) => return Err(fail_after_handoff(machine, worktree, &handoff, err, false)),
    };

    if let Some(handed_off) = machine.request.handed_off.take() {
        handed_off();
    }

    if !machine.reserved.quiet {
        eprintln!("hydra: run {}", machine.reserved.run_id);
        eprintln!("hydra: record {}", machine.reserved.run_file.display());
        if let Some(w) = &worktree.worktree_ref {
            eprintln!("hydra: worktree {} on {}", w.path, w.branch);
        }
    }

    // Line 0 first, so a handle always carries a pid and a watcher that died on
    // its way up is reported as that rather than as a run the caller can poll.
    crate::watch::wait_for_start(&machine.reserved.run_file, &mut watcher, LAUNCH_BUDGET).map_err(
        |err| {
            err.context(format!(
                "the run's own log is {}",
                machine
                    .reserved
                    .layout
                    .watcher_log(&machine.reserved.run_id)
                    .display()
            ))
        },
    )?;

    machine.reserved.reservation = None;
    wait_for_call_end(machine, worktree, &mut watcher)
}

fn build_argv(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    handoff: &HandoffReady,
) -> Result<Vec<String>> {
    let task = &machine.preflight.task;
    let argv = match machine.request.provider {
        Provider::Claude => claude::argv(&claude::Launch {
            task,
            model: machine.request.model.as_deref(),
            settings: &handoff.spawn.hook_config,
            policy: &machine.request.policy,
            mcp: handoff.mcp_file.as_deref(),
        }),
        // Nothing provider-specific rides the argv. The hook config, the MCP
        // config and the skill links are all read from cwd, and the data
        // directory is in the environment.
        Provider::CursorAgent => cursor::argv(&cursor::Launch {
            task,
            model: machine.request.model.as_deref(),
        }),
        // The one flag the skills, the hook config and the MCP config all ride
        // in on. A launch with no worktree was refused above, so the checkout is
        // here rather than optional.
        Provider::Agy => agy::argv(&agy::Launch {
            task,
            model: machine.request.model.as_deref(),
            worktree: worktree
                .worktree_ref
                .as_ref()
                .map(|w| std::path::Path::new(w.path.as_str()))
                .context("an agy run needs a checkout and this one has none")?,
        }),
        Provider::Codex => codex::argv(&codex::Launch {
            task,
            model: machine.request.model.as_deref(),
            worktree: worktree
                .worktree_ref
                .as_ref()
                .map(|w| std::path::Path::new(w.path.as_str()))
                .context("a codex run needs a checkout and this one has none")?,
            hook_file: &codex::hook_file(&machine.reserved.layout, &machine.reserved.run_id),
        }),
    };
    Ok(argv)
}

fn wait_for_call_end(
    machine: &MachineReady,
    worktree: &WorktreeReady,
    watcher: &mut std::process::Child,
) -> Result<Call> {
    match crate::watch::wait_for_end(&machine.reserved.run_file, watcher, machine.request.timeout)?
    {
        crate::watch::Waited::TimedOut => Ok(Call::Handle(handle(
            &machine.reserved.run_id,
            &machine.reserved.run_file,
            worktree.worktree_ref.as_ref(),
            machine.request.timeout,
        )?)),
        crate::watch::Waited::WriterGone => Err(anyhow::anyhow!(
            "the run's writer exited without closing {}. Its log is {}",
            machine.reserved.run_file.display(),
            machine
                .reserved
                .layout
                .watcher_log(&machine.reserved.run_id)
                .display()
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
            let result = match crate::normalize::read_file(
                &machine.reserved.run_file,
                &machine.preflight.policies,
            ) {
                Ok(run) => result(
                    &run,
                    &machine.reserved.run_file,
                    worktree.worktree_ref.as_ref(),
                    worktree.seed_tree.as_deref(),
                    &machine.reserved.layout,
                ),
                Err(err) => {
                    if !machine.reserved.quiet {
                        eprintln!("hydra: the record is written and would not read back: {err:#}");
                    }
                    // The exit code is the detached half's to know, so it comes
                    // off the closing line rather than out of this process's
                    // memory. That read is a tail and a `serde_json` call, which
                    // is the half of the record that always parses even when the
                    // normalizer chokes on what is above it.
                    let exit_code = crate::live::closing_line(&machine.reserved.run_file)
                        .ok()
                        .flatten()
                        .and_then(|end| end["exit_code"].as_i64())
                        .and_then(|code| i32::try_from(code).ok());
                    unread(
                        &machine.reserved.run_id,
                        &machine.reserved.run_file,
                        exit_code,
                        &err,
                    )
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

/// Two facts about the repo a run stands in, for the two providers that read
/// their config out of the worktree, checked before a run id exists.
///
/// **No repo, no gate.** Both read their hook config out of the workspace, and a
/// run outside a repo would run where Hydra was invoked: a directory Hydra does
/// not own and may not write into (D9). A provider Hydra cannot gate is not
/// launched (harness spec 2), so the refusal is here and not a record, the same
/// as a wrong PATH. On `agy` the workspace is not even cwd: the directory has to
/// arrive as `--add-dir`, and a run with no checkout has none to pass.
///
/// **A repo that carries the provider's own two files is refused too.** The
/// worktree is seeded from this repo (6.3), and Hydra has to write both files
/// into it. Writing over the human's copies would edit their work and `hydra
/// reap` would commit the edit; loading theirs instead of Hydra's would launch an
/// ungated run. Neither is a shape 6.6 allows, so the launch stops and says
/// which file.
fn worktree_preconditions(
    binary: &str,
    repo: Option<&std::path::Path>,
    files: &[fn(&std::path::Path) -> PathBuf],
) -> Result<()> {
    let Some(repo) = repo else {
        anyhow::bail!(
            "{binary} reads its hook config out of the worktree, so a run outside a git \
             repo has nowhere Hydra owns to put a gate and is not launched (harness spec 2). \
             Run it from inside a repository."
        );
    };
    for file in files {
        let file = file(repo);
        if file.exists() {
            anyhow::bail!(
                "{} exists, and Hydra writes that file into every {binary} worktree it \
                 seeds from this repo. It will not write over the human's copy and it will \
                 not launch a run that reads it instead of the gate, so move the file \
                 aside for the run.",
                file.display()
            );
        }
    }
    Ok(())
}

/// What the injection half of the launch produced: the `skills` group, the
/// verdict of 6.1's fifth pre-flight check, and the MCP half of the one answer
/// the probe gave.
pub(crate) struct InjectedSkills {
    pub(crate) record: serde_json::Value,
    pub(crate) collisions: Vec<crate::skills::collision::Collision>,
    /// The sha this run pinned, contract 5.3's `skills.store_sha`. The gate's
    /// `skill_snapshot` scope is derived from it through
    /// [`Layout::skill_snapshot_scope`], which is the same function contract
    /// 7.7's re-evaluation calls with the sha read back off the record.
    pub(crate) store_sha: String,
    /// Contract 5.3's `mcp.resolved`, from the same probe. `None` is unanswered,
    /// never an empty listing.
    pub(crate) mcp_resolved: Option<Vec<String>>,
}

/// Pins the store, snapshots it and links it into the checkout, with no process.
///
/// **The answer goes in `slot`, and so does a partial one.** Once anything is
/// in the checkout, `slot` holds a `skills` group naming it, before any later
/// step can fail, so the failure record [`PostWorktree::fail`] writes lists
/// what reap must hold back. Build ticket 99, the third hiring panel's H40.
///
/// Split from [`probe_skills`] by build ticket 375. The run's profile scopes the
/// snapshot by the sha pinned here, so the profile can only be written after
/// this half, and the probe has to run inside that profile. Build ticket 377
/// removed `inject_skills`, which ran both halves with no profile, once the
/// gated interactive launch stopped calling it.
pub(crate) fn link_skills(
    layout: &Layout,
    run_id: &str,
    provider: Provider,
    worktree: &std::path::Path,
    slot: &mut Option<InjectedSkills>,
) -> Result<crate::skills::Injected> {
    let store = crate::skills::Store::open(layout)?;
    let store_sha = store.pin(run_id)?;
    let snapshot = store.snapshot(layout, &store_sha)?;
    let partial = |injected: &crate::skills::Injected| InjectedSkills {
        record: crate::skills::record(injected, None),
        collisions: Vec::new(),
        store_sha: store_sha.clone(),
        mcp_resolved: None,
    };
    let injected = match crate::skills::inject(worktree, provider, &snapshot, &store_sha) {
        Ok(injected) => injected,
        Err(failure) => {
            *slot = Some(partial(&failure.partial));
            return Err(failure.error);
        }
    };
    *slot = Some(partial(&injected));
    Ok(injected)
}

/// Asks the provider what it resolved for what [`link_skills`] linked, and runs
/// the collision check against the answer.
///
/// **One probe answers both `resolved` fields.** The skill listing and the MCP
/// tool list arrive in the same transcript, so the launch pays for one process and
/// reads two answers off it. The MCP config is already written by the time this
/// runs, and the probe carries it, which is what makes the answer the run's.
///
/// **An unanswerable listing does not refuse the run.** The probe is free and it
/// is still a second process that can fail, and contract 5.3 already requires a
/// reader to treat an absent `resolved` as unanswered rather than as an empty
/// listing. The cost is that the third of the three collision tests does not
/// run, which is said out loud rather than swallowed: the other two still do.
///
/// `profile` is the run's own launch wrapper. The probe runs the provider's
/// binary with the human's settings, so it runs inside the same profile the run
/// will, and `None` only where nothing wraps the run either.
pub(crate) fn probe_skills(
    provider: Provider,
    worktree: &std::path::Path,
    mcp: crate::mcp::Probe<'_>,
    profile: Option<&std::path::Path>,
    injected: &crate::skills::Injected,
    slot: &mut Option<InjectedSkills>,
    quiet: bool,
) -> Result<()> {
    let store_sha = injected.store_sha.clone();
    let resolved = match provider.resolve(worktree, mcp, profile) {
        Ok(resolved) => resolved,
        Err(err) => {
            if !quiet {
                eprintln!(
                    "hydra: {provider} would not say what it resolved, so the record \
                     cannot: {err:#}"
                );
            }
            crate::provider::Resolved::default()
        }
    };
    // 6.5 gives the fatal pre-flight to Cursor alone, so a server that would not
    // start is a line on stderr and a difference between `injected` and
    // `resolved` in the record, rather than a refused launch.
    for failure in resolved.failed_mcp.iter().filter(|_| !quiet) {
        eprintln!("hydra: an injected MCP server did not start: {failure}");
    }

    let home = crate::env::real_home()?;
    let user = crate::skills::index::Index::user_scope(&home)?;
    let project =
        crate::skills::index::Index::project_scope_without(&injected.root, &injected.linked)?;
    let collisions = crate::skills::collision::check(
        injected,
        provider,
        &home,
        &user,
        &project,
        resolved.skills.as_deref(),
    );

    *slot = Some(InjectedSkills {
        record: crate::skills::record(injected, resolved.skills.as_deref()),
        collisions,
        store_sha,
        mcp_resolved: resolved.mcp,
    });
    Ok(())
}

/// Inserts the provider's resume flag before the task, harness spec section 5.
pub fn append_resume_args(
    argv: &mut Vec<String>,
    provider: Provider,
    session_id: &str,
    hook_file: Option<&std::path::Path>,
) {
    match provider {
        Provider::Codex => {
            let hook_file = hook_file.expect("codex resume needs the per-spawn hook file");
            let task = argv
                .pop()
                .expect("every recorded argv ends with the task string");
            let model = take_model_flag(argv);
            *argv = codex::resume_args(session_id, hook_file);
            if let Some(model) = model {
                argv.push("-m".to_string());
                argv.push(model);
            }
            argv.push(task);
        }
        _ => {
            let resume = match provider {
                Provider::Claude => claude::resume_args(session_id),
                Provider::CursorAgent => cursor::resume_args(session_id),
                Provider::Agy => agy::resume_args(session_id),
                Provider::Codex => unreachable!(),
            };
            match provider {
                Provider::Agy => {
                    argv.splice(0..0, resume);
                }
                _ => {
                    let task = argv
                        .pop()
                        .expect("every recorded argv ends with the task string");
                    argv.extend(resume);
                    argv.push(task);
                }
            }
        }
    }
}

/// The model name when `-m` is the last flag pair on a recorded argv.
fn take_model_flag(argv: &mut Vec<String>) -> Option<String> {
    if argv.len() >= 2 && argv[argv.len() - 2] == "-m" {
        let model = argv.pop().unwrap();
        argv.pop();
        Some(model)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Re-execute repository fixtures without the object pointers of the launching run.
    // An explicit composition map would not isolate their setup commands.
    fn isolated_object_store_test(name: &str) -> bool {
        const CHILD: &str = "HYDRA_RUN_OBJECT_STORE_TEST_CHILD";
        let name = format!("run::tests::{name}");
        if std::env::var(CHILD).as_deref() == Ok(name.as_str()) {
            return true;
        }
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &name, "--nocapture"])
            .env(CHILD, &name)
            .env_remove("GIT_OBJECT_DIRECTORY")
            .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "child failed:\n{stdout}\n{stderr}");
        assert!(
            stdout.contains(&format!("test {name} ... ok")),
            "the child did not run the named test:\n{stdout}\n{stderr}"
        );
        false
    }

    #[test]
    fn every_name_hydra_sets_travels_to_the_detached_half() {
        // The watcher rebuilds the allowlist from its own environment, so a name
        // outside it reaches the child only through the spec's `hydra_env`. The
        // record's `env_allowlist` is `child_env.names()`, and a name there that
        // does not travel is a record claiming a variable the child never had.
        let root = std::env::temp_dir().join(format!("hydra-compose-env-{}", std::process::id()));
        let layout = Layout::at(&root);
        let run_id = "01996d3c-8f2a-7c31-9b44-000000000097";
        for provider in [
            Provider::Claude,
            Provider::CursorAgent,
            Provider::Agy,
            Provider::Codex,
        ] {
            let (child_env, hydra_env) =
                compose_child_env(provider, &layout, run_id, None).unwrap();
            assert_eq!(
                hydra_env
                    .get(crate::containment::MARKER)
                    .map(String::as_str),
                Some(run_id),
                "{provider:?}: the containment marker does not reach the child"
            );
            for name in ["GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES"] {
                assert!(
                    !hydra_env.contains_key(name),
                    "{provider:?}: {name} set without a repo"
                );
                assert!(
                    child_env.vars().all(|(key, _)| key != name),
                    "{provider:?}: {name} reached a child without a repo"
                );
            }

            let rebuilt = ChildEnv::build(provider).unwrap().names();
            for name in child_env.names() {
                assert!(
                    rebuilt.contains(&name) || hydra_env.contains_key(&name),
                    "{provider:?}: {name} is on line 0 and never reaches the child"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_with_a_repo_is_handed_its_own_object_directory() {
        if !isolated_object_store_test("a_run_with_a_repo_is_handed_its_own_object_directory") {
            return;
        }

        // Map ticket 74. The seatbelt denies the parent's `objects/`, so a run
        // whose git is not pointed at its own store cannot `git add` at all.
        let root = std::env::temp_dir().join(format!("hydra-run-objects-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let init = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap();
        assert!(init.success());
        let common = crate::worktree::git_common_dir(&repo).unwrap();
        let layout = Layout::at(root.join("hydra"));
        let run_id = "01996d3c-8f2a-7c31-9b44-000000000103";
        let (child_env, hydra_env) =
            compose_child_env(Provider::Claude, &layout, run_id, Some(&repo)).unwrap();
        let store = layout.run_objects(run_id);
        assert_eq!(
            hydra_env.get("GIT_OBJECT_DIRECTORY"),
            Some(&store.display().to_string())
        );
        assert_eq!(
            hydra_env.get("GIT_ALTERNATE_OBJECT_DIRECTORIES"),
            Some(&common.join("objects").display().to_string())
        );
        assert!(store.join("pack").is_dir(), "the store was not created");
        assert!(
            child_env
                .names()
                .contains(&"GIT_OBJECT_DIRECTORY".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

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
            None,
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

    #[test]
    fn inject_skills_does_not_treat_a_codex_copy_as_a_collision_with_itself() {
        if !isolated_object_store_test(
            "inject_skills_does_not_treat_a_codex_copy_as_a_collision_with_itself",
        ) {
            return;
        }

        use std::process::Command;

        use crate::paths::Layout;
        use crate::provider::Provider;
        use crate::skills::Store;

        fn git(dir: &std::path::Path, args: &[&str]) {
            let out = Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }

        let base = std::env::temp_dir().join(format!("hydra-inject-skills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "test@example.invalid"]);
        git(&repo, &["config", "user.name", "hydra tests"]);
        std::fs::write(repo.join("committed.txt"), "one\n").unwrap();
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", "first"]);

        let layout = Layout::at(base.join("hydra"));
        layout.ensure().unwrap();
        let run_id = "01a0cfa1-507d-7a61-99b1-18b7f5a30b0b";
        let worktree = layout.worktree(run_id);
        crate::worktree::create(
            &repo,
            &worktree,
            &format!("hydra/{run_id}"),
            &layout.seed_index(run_id),
        )
        .unwrap();

        let store = Store::open(&layout).unwrap();
        let skill_dir = layout.skills().join("hydra-g2-inject-probe");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: hydra-g2-inject-probe\ndescription: A probe.\n---\n\nbody\n",
        )
        .unwrap();
        store.pin(run_id).unwrap();

        let mut slot = None;
        let linked = link_skills(&layout, run_id, Provider::Codex, &worktree, &mut slot).unwrap();
        probe_skills(
            Provider::Codex,
            &worktree,
            crate::mcp::Probe {
                policy: "standard",
                config: None,
            },
            None,
            &linked,
            &mut slot,
            false,
        )
        .unwrap();
        let injected = slot.unwrap();
        assert!(
            injected.collisions.is_empty(),
            "the launch-time check read the copy as a project skill: {:?}",
            injected.collisions
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_failed_injection_leaves_the_names_it_wrote_where_the_failure_record_reads_them() {
        if !isolated_object_store_test(
            "a_failed_injection_leaves_the_names_it_wrote_where_the_failure_record_reads_them",
        ) {
            return;
        }

        // Build ticket 99, H40. `phase_handoff` passes `recording.injected` as
        // the slot and `PostWorktree::fail` composes line 0 from it, so what the
        // slot holds after an error is what the `never_started` record says.
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        use crate::paths::Layout;
        use crate::provider::Provider;
        use crate::skills::Store;

        fn git(dir: &std::path::Path, args: &[&str]) {
            let out = Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {} failed", args.join(" "));
        }

        let base =
            std::env::temp_dir().join(format!("hydra-inject-partial-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "test@example.invalid"]);
        git(&repo, &["config", "user.name", "hydra tests"]);
        std::fs::write(repo.join("committed.txt"), "one\n").unwrap();
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", "first"]);

        let layout = Layout::at(base.join("hydra"));
        layout.ensure().unwrap();
        let run_id = "01a09900-0000-7000-8000-000000000040";
        let worktree = layout.worktree(run_id);
        crate::worktree::create(
            &repo,
            &worktree,
            &format!("hydra/{run_id}"),
            &layout.seed_index(run_id),
        )
        .unwrap();

        let store = Store::open(&layout).unwrap();
        for name in ["alpha", "beta"] {
            let dir = layout.skills().join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: A probe.\n---\n\nbody\n"),
            )
            .unwrap();
        }
        std::fs::write(layout.skills().join("beta/extra.md"), "extra\n").unwrap();
        let sha = store.pin(run_id).unwrap();
        let unreadable = store.snapshot(&layout, &sha).unwrap().join("beta/extra.md");
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

        let mut slot = None;
        let failed = link_skills(&layout, run_id, Provider::Codex, &worktree, &mut slot);
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert!(failed.is_err(), "the copy of beta should have failed");
        let recorded = slot.expect("the failure left no `skills` group to record");
        assert_eq!(recorded.record["store_sha"], sha);
        assert_eq!(
            recorded.record["linked"],
            serde_json::json!(["alpha", "beta"])
        );

        let _ = Command::new("chmod")
            .args(["-R", "u+w"])
            .arg(&base)
            .output();
        let _ = std::fs::remove_dir_all(&base);
    }
}
