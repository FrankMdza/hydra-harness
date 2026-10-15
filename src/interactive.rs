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
//! **`claude` and, ungated, the other three.** Harness spec 10: `claude` writes an
//! append-only transcript during the run, at a path Hydra predicts before the
//! process starts off the same `--session-id` the pre-flight probe already
//! mints ([`crate::provider::claude::transcript_path`]). `codex` takes no
//! session id, so its harvest finds the session file after the fact
//! ([`crate::provider::codex::find_interactive_session`]), build ticket 255.
//! `cursor-agent` is the same shape over a SQLite chat store
//! ([`crate::provider::cursor::find_interactive_chat`]), build ticket 266.
//! `agy` keeps one SQLite store per conversation and its transcript in a
//! directory of the same uuid
//! ([`crate::provider::agy::find_interactive_conversation`]), build ticket 267.
//! The gated mode stays `claude` only: it compiles a `claude` gate and wraps a
//! `claude` argv.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::env::ChildEnv;
use crate::outcome::RunResult;
use crate::paths::Layout;
use crate::provider::{Provider, agy, claude, codex, cursor};
use crate::record::{Auth, RunStart, SCHEMA_VERSION, WorktreeRef, Writer};
use crate::run::{SpawnFiles, inject_skills, write_spawn_config};

/// What a human typed. Without `--policy` nothing here is gated. With it the
/// launch compiles a gate, seeds a worktree and links the snapshot the way a
/// recorded run does before the spawn.
pub struct Request {
    /// `claude`, or any of the other three when no policy is named.
    pub provider: Provider,
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
    /// Map ticket 74, the private object directory a recorded run gets, as
    /// the two variables the child's environment carries.
    pub object_env: Vec<(&'static str, String)>,
}

/// Undoes a gated launch that failed after its checkout was made and before
/// anything wrote its record: the checkout, its branch, its context file and
/// the run's spawn directory. The review of build ticket 99, item 3, which
/// found H39 and H40 still open on this path.
///
/// Rollback rather than a failure record, as `worktree::create` chose in build
/// ticket 99: nothing ran in the checkout, so it holds nothing reap would need
/// to keep, and a skill linked or copied into it goes with it.
pub fn abandon_policy_launch(
    layout: &Layout,
    run_id: &str,
    repo: &Path,
    worktree: &WorktreeRef,
    err: anyhow::Error,
) -> anyhow::Error {
    let undone = crate::worktree::roll_back(repo, Path::new(&worktree.path), &worktree.branch)
        .and_then(|()| match std::fs::remove_dir_all(layout.spawn(run_id)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        });
    match undone {
        Ok(()) => err,
        Err(undo) => err.context(format!(
            "and the checkout at {} and branch {} could not be removed: {undo:#}",
            worktree.path, worktree.branch
        )),
    }
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
    let worktree_ref = WorktreeRef {
        path: worktree.path.display().to_string(),
        branch: worktree.branch.clone(),
        base_tree: worktree.seed_tree.clone(),
    };
    // Every step below can fail, and no record exists yet to say so.
    finish_policy_launch(
        layout,
        run_id,
        policy,
        policies,
        repo,
        &mcp_document,
        &worktree,
        worktree_ref.clone(),
    )
    .map_err(|err| abandon_policy_launch(layout, run_id, repo, &worktree_ref, err))
}

/// What [`prepare_policy_launch`] does once the checkout exists.
#[allow(clippy::too_many_arguments)]
fn finish_policy_launch(
    layout: &Layout,
    run_id: &str,
    policy: &str,
    policies: &crate::policy::PolicySet,
    repo: &Path,
    mcp_document: &crate::mcp::Document,
    worktree: &crate::worktree::Worktree,
    worktree_ref: WorktreeRef,
) -> Result<PolicyLaunch> {
    let seed_tree = worktree.seed_tree.clone();
    let cwd = worktree.path.clone();

    let mcp_plan = crate::mcp::plan(
        mcp_document,
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

    let mut injected = None;
    if policy != "judge" {
        inject_skills(
            layout,
            run_id,
            Provider::Claude,
            &worktree.path,
            crate::mcp::Probe {
                policy,
                config: Some(&mcp_file),
            },
            &mut injected,
            false,
        )?;
    }

    if let Some(injected) = &injected
        && !injected.collisions.is_empty()
    {
        anyhow::bail!(
            "{}",
            crate::skills::collision::remediation(&injected.collisions)
        );
    }

    let store_sha = injected.as_ref().map(|i| i.store_sha.as_str());
    let mut spawn = write_spawn_config(
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
    // Build ticket 250: the session a human types into writes nothing under
    // the home directory outside its worktree and `claude`'s own state.
    if let Some(profile) = &spawn.sandbox_profile {
        spawn.sandbox_sha = Some(crate::sandbox::narrow_for_copilot(profile, &worktree.path)?);
    }
    // The seatbelt this launch runs under denies the parent's `objects/`.
    let object_env = crate::worktree::prepare_run_objects(layout, run_id, repo)?;

    let scopes = spawn.scopes.clone();
    Ok(PolicyLaunch {
        object_env,
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

/// The names of the approved shapes `policy` may ask for, read from `panels`
/// at launch for the start line, build ticket 247.
///
/// A file that does not load names none, and the launch goes on: the launcher
/// refuses every `panel` request against it anyway, in its own words.
pub fn askable_shapes(
    panels: &Path,
    policies: &crate::policy::PolicySet,
    policy: &str,
) -> Vec<String> {
    match crate::panels::load(panels, policies) {
        Ok(loaded) => loaded
            .shapes
            .into_iter()
            .filter(|shape| shape.askers.iter().any(|asker| asker == policy))
            .map(|shape| shape.name)
            .collect(),
        Err(err) => {
            eprintln!(
                "hydra: {} does not load, so the session is told of no shape: {err:#}",
                panels.display()
            );
            Vec::new()
        }
    }
}

/// The binary and argv a gated interactive launch execs.
///
/// Public so `tests/spawn.rs` can read the seatbelt wrap without starting a
/// session.
pub fn gated_exec_argv(
    prep: &PolicyLaunch,
    policy: &str,
    session: &str,
    task: Option<&str>,
    model: Option<&str>,
    shapes: &[String],
) -> (String, Vec<String>) {
    let argv = claude::interactive_argv(&claude::InteractiveLaunch {
        session,
        task,
        model,
        mcp: prep.mcp_file.as_deref(),
        settings: Some(&prep.spawn.hook_config),
        policy: Some(policy),
        shapes,
    });
    match prep.spawn.sandbox_profile.as_deref() {
        Some(profile) => {
            let (binary, wrapped) =
                crate::sandbox::wrap_command(Provider::Claude.binary(), &argv, profile);
            (binary.to_string(), wrapped)
        }
        None => (Provider::Claude.binary().to_string(), argv),
    }
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
    refuse_provider(request.provider, request.policy.is_some())?;
    if request.policy.is_some() {
        run_gated(request)
    } else {
        run_ungated(request)
    }
}

/// The providers each interactive mode launches, build tickets 255, 266 and
/// 267: `claude` gated or not, the other three ungated.
pub fn refuse_provider(provider: Provider, gated: bool) -> Result<()> {
    match provider {
        Provider::Claude => Ok(()),
        _ if !gated => Ok(()),
        other => anyhow::bail!(
            "hydra run --interactive --policy is claude only: the gated interactive mode \
             compiles a claude gate. Drop --policy to launch {} ungated",
            other.binary()
        ),
    }
}

fn run_gated(request: Request) -> Result<RunResult> {
    let layout = Layout::discover()?;
    crate::paths::refuse_non_ascii_roots(&layout, &crate::paths::hydra_home())?;
    layout.ensure()?;

    let invoked_in = std::env::current_dir().context("reading the working directory")?;
    crate::run::refuse_launch_inside_checkout(&layout, &invoked_in)?;
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

    // Everything that can fail without a checkout fails before one exists. The
    // review of build ticket 99, item 3: these three used to come after the
    // worktree, and a failure left a checkout and a branch no record named.
    let provider_version = Provider::Claude.version()?;
    let mut child_env = ChildEnv::build(Provider::Claude)?;
    if crate::sandbox::wraps_launches() && !crate::sandbox::exec_available() {
        anyhow::bail!(
            "/usr/bin/sandbox-exec is missing on this Mac, so Hydra refuses to launch \
             an interactive run without the seatbelt rather than run bare"
        );
    }

    let prep = prepare_policy_launch(&layout, &run_id, policy, &policies, &repo, &invoked_in)?;

    // Build ticket 57. This arm compiles a gate and seeds a worktree the way a
    // recorded run does, so 7.3 applies to it and the child is marked. The
    // ungated arm below is not marked: it compiles no gate, claims no
    // containment, and marking it would refuse the `hydra run` a human types in
    // their own session.
    child_env.set(crate::containment::MARKER, run_id.clone());
    for (name, value) in &prep.object_env {
        child_env.set(name, value.clone());
    }

    let shapes = askable_shapes(&crate::panels::path(), &policies, policy);
    let (binary, argv) = gated_exec_argv(
        &prep,
        policy,
        &session,
        request.task.as_deref(),
        request.model.as_deref(),
        &shapes,
    );

    // Build ticket 140, map 82 answer 1. Bound before the exec, so the socket
    // is there when `claude` starts the face, and in the spawn directory, which
    // the run cannot write: it can neither unlink this socket nor bind its own.
    // Served on a thread because this one blocks in `record_then_wait`.
    let launcher = std::sync::Arc::new(crate::seats::Launcher::new(
        crate::seats::Session {
            run_id: run_id.clone(),
            policy: policy.to_string(),
            model: request.model.clone(),
            run_file: run_file.clone(),
            working_dir: invoked_in.clone(),
        },
        policies.dir().to_path_buf(),
        crate::panels::path(),
        crate::paths::Layout::at(layout.root()),
        Box::new(crate::run::run),
    ));
    let served = crate::seats::serve(launcher, &crate::seats::socket_path(&layout, &run_id))
        .map_err(|err| abandon_policy_launch(&layout, &run_id, &repo, &prep.worktree, err))?;

    let mut child = std::process::Command::new(&binary)
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
        .with_context(|| format!("starting {binary}"))
        .map_err(|err| abandon_policy_launch(&layout, &run_id, &repo, &prep.worktree, err))?;

    let start_time = crate::procinfo::start_time(child.id());

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
        gate_sha: Some(prep.spawn.gate_sha.clone()),
        sandbox: prep.spawn.sandbox_sha.clone(),
        sandbox_kind: prep.spawn.sandbox_sha.as_ref().map(|_| "seatbelt"),
        host: crate::record::Host::here(),
        scopes: Some(prep.scopes.clone()),
        cwd: prep.cwd.display().to_string(),
        worktree: Some(prep.worktree.clone()),
        depth: 0,
        parent_policy: None,
        // Both filled by `record_then_wait`, from the process and its start time.
        pid: None,
        process_start_time: None,
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
        may_edit: None,
    };

    eprintln!("hydra: run {run_id}");
    eprintln!("hydra: record {}", run_file.display());
    eprintln!(
        "hydra: worktree {} on {}",
        prep.worktree.path, prep.worktree.branch
    );

    let status = record_then_wait(
        Provider::Claude,
        &run_file,
        &run_id,
        request.model.as_deref(),
        start,
        &mut child,
        start_time,
        Some(&prep.worktree),
    )?;
    // Before the harvest, which writes `run_end`: no request line may land
    // after it. `claude` has exited, so the terminal is the human's again and
    // a seat this process is about to take with it is said out loud.
    for seat in served.stop() {
        eprintln!(
            "hydra: seat {seat} of run {run_id} had not reached its own watcher when the \
             session ended, and stopped with it"
        );
    }

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
    let provider = request.provider;
    let layout = Layout::discover()?;
    crate::paths::refuse_non_ascii_roots(&layout, &crate::paths::hydra_home())?;
    layout.ensure()?;

    let cwd = std::env::current_dir().context("reading the working directory")?;
    let run_id = Uuid::now_v7().to_string();
    let run_file = layout.run_file(&run_id);

    let provider_version = provider.version()?;
    let child_env = ChildEnv::build(provider)?;

    let mcp_document = crate::mcp::Document::load()?;
    let (session, argv) = match provider {
        Provider::Codex => {
            // `codex` mints its own session id and takes none, so `run_start`
            // carries null and the harvest finds the file by its start. Its MCP
            // config lives inside a checkout Hydra made (6.5), and this launch
            // makes none, so the human's own `~/.codex/config.toml` is all it
            // reads, the way a `claude` session keeps the human's servers.
            let argv = codex::interactive_argv(request.task.as_deref(), request.model.as_deref());
            (None, argv)
        }
        Provider::CursorAgent => {
            // The same as `codex`: no chat id goes in, the harvest finds the
            // chat by its start, and the MCP config Hydra writes lives in a
            // checkout (6.5) this launch does not make, so the human's own
            // Cursor config is what the TUI reads.
            let argv = cursor::interactive_argv(request.task.as_deref(), request.model.as_deref());
            (None, argv)
        }
        Provider::Agy => {
            // The same again: `agy` mints its own conversation id, and its MCP
            // config lives in `.agents/` of a checkout (6.5) this launch does
            // not make.
            let argv = agy::interactive_argv(request.task.as_deref(), request.model.as_deref());
            (None, argv)
        }
        _ => {
            // 6.5: the run's file, without `--strict-mcp-config` so the human's
            // own servers survive beside it. Nothing is admitted, because
            // admission is a policy question (7.1) and this run compiles no
            // policy to ask it of; an empty `mcpServers` is still a real file,
            // the same shape a `judge` run's is (`claude::mcp_config`'s own doc
            // comment).
            let mcp_file = crate::mcp::write_config(&layout, &run_id, Provider::Claude, &[], None)?;
            let session = Uuid::now_v7().to_string();
            let argv = claude::interactive_argv(&claude::InteractiveLaunch {
                session: &session,
                task: request.task.as_deref(),
                model: request.model.as_deref(),
                mcp: Some(&mcp_file),
                settings: None,
                policy: None,
                shapes: &[],
            });
            (Some(session), argv)
        }
    };

    let mut child = std::process::Command::new(provider.binary())
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
        .with_context(|| format!("starting {}", provider.binary()))?;

    // Contract 5.2's liveness pair, filled from the process this call itself
    // owns rather than backfilled by a detached watcher: there is no watcher
    // here, `run` is the process holding the child.
    let start_time = crate::procinfo::start_time(child.id());

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
        sandbox_kind: None,
        host: crate::record::Host::here(),
        launch_mode: Some("interactive".to_string()),
        provider_version: Some(provider_version),
        // Known before the child starts on `claude`, unlike a recorded run's:
        // it takes `--session-id` rather than minting its own, so line 0
        // carries it from the first write rather than arriving late on an init
        // frame. Null on the other three, whose ids only their own files
        // name.
        provider_session_id: session,
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
        pid: None,
        process_start_time: None,
        skills: None,
        mcp: Some(crate::mcp::record(&mcp_document.sha, &[], None)),
        auth: Some(Auth {
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(child_env.names()),
        subscription: Some(provider.subscription().to_string()),
        resumed_from: None,
        may_edit: None,
    };

    eprintln!("hydra: run {run_id}");
    eprintln!("hydra: record {}", run_file.display());

    let status = record_then_wait(
        provider,
        &run_file,
        &run_id,
        request.model.as_deref(),
        start,
        &mut child,
        start_time,
        None,
    )?;

    if let Err(err) = harvest(&layout, &run_id, status.code()) {
        eprintln!("hydra: run {run_id} exited but did not harvest: {err:#}");
        eprintln!("hydra: `hydra status {run_id}` will retry it");
    }

    let policies = crate::policy::PolicySet::discover()?;
    let run = crate::normalize::read_file(&run_file, &policies)?;
    Ok(crate::run::result(&run, &run_file, None, None, &layout))
}

/// Writes an interactive run's `run_start` once `provider` is running, and
/// then waits for that process whatever the write did.
///
/// **Nothing after the spawn returns early.** The review of build ticket 99,
/// item 3, left two steps here that could fail with the session already on
/// the human's terminal: reading the process's start time and writing the
/// record. Each one used to return at once. That left no record, and it left
/// `claude` running with nobody waiting on it. Rolling the checkout back
/// would pull it out from under a live session, so neither failure undoes
/// anything. Build ticket 112.
///
/// A start time that will not read gets a failure record: `run_start` and then
/// a `run_end` whose `error` names the pid. Contract 5.2 allows a pid only
/// beside its start time, so `run_start` carries neither and the pid lives in
/// the error. A record that cannot be written at all is said on stderr with
/// the pid and the checkout. Either way this waits for the process to exit
/// and then answers the error, and `checkout` stays for `hydra reap`.
///
/// `start_time` is [`crate::procinfo::start_time`]'s answer for the child,
/// taken by the caller so a test can hand in a failure.
#[allow(clippy::too_many_arguments)]
pub fn record_then_wait(
    provider: Provider,
    run_file: &Path,
    run_id: &str,
    model: Option<&str>,
    mut start: RunStart,
    child: &mut std::process::Child,
    start_time: Result<Option<chrono::DateTime<chrono::Utc>>>,
    checkout: Option<&WorktreeRef>,
) -> Result<std::process::ExitStatus> {
    let pid = child.id();
    let binary = provider.binary();
    let written = match start_time {
        Ok(start_time) => {
            start.pid = Some(pid);
            start.process_start_time =
                start_time.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
            write_start(provider, run_file, run_id, model, &start, None)
        }
        Err(err) => {
            let failure = format!(
                "{binary} started as pid {pid}, and its start time could not be read, so this \
                 record cannot tell that process from a later one with the same pid: {err:#}"
            );
            start.pid = None;
            start.process_start_time = None;
            write_start(provider, run_file, run_id, model, &start, Some(&failure))
                .and_then(|()| Err(anyhow::anyhow!(failure)))
        }
    };
    let written = written.map_err(|err| {
        let err = match checkout {
            Some(checkout) => err.context(format!(
                "{binary} keeps running as pid {pid} in {} on {}, and the checkout stays for \
                 `hydra reap`",
                checkout.path, checkout.branch
            )),
            None => err.context(format!("{binary} keeps running as pid {pid}")),
        };
        eprintln!("hydra: run {run_id}: {err:#}");
        err
    });
    let status = child
        .wait()
        .with_context(|| format!("waiting for {binary} to exit"));
    match written {
        Ok(()) => status,
        Err(err) => match status {
            Ok(status) => Err(err.context(format!("{binary} exited: {status}"))),
            Err(wait) => Err(err.context(format!("{wait:#}"))),
        },
    }
}

/// `run_start`, and a closing line carrying `failure` when there is one.
fn write_start(
    provider: Provider,
    run_file: &Path,
    run_id: &str,
    model: Option<&str>,
    start: &RunStart,
    failure: Option<&str>,
) -> Result<()> {
    let mut writer = Writer::create(run_file, run_id, None, Some(provider.binary()), model)?;
    writer.hydra_line(start)?;
    if let Some(failure) = failure {
        writer.hydra_line(&crate::record::RunEnd::harvested(
            None,
            Some(failure.to_string()),
        ))?;
    }
    Ok(())
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
    let provider = envelope["provider"].as_str();
    anyhow::ensure!(
        matches!(provider, Some("claude" | "codex" | "cursor-agent" | "agy")),
        "harvest reads only a claude, codex, cursor-agent or agy transcript, and run {run_id} \
         is {provider:?}"
    );
    let start = &envelope["hydra"];
    anyhow::ensure!(
        start["launch_mode"].as_str() == Some("interactive"),
        "run {run_id} is not an interactive launch, so it has no vendor transcript to harvest"
    );
    let cwd = start["cwd"]
        .as_str()
        .with_context(|| format!("run {run_id} carries no cwd to find a transcript from"))?;
    let parent_run_id = envelope["parent_run_id"].as_str();
    let model = envelope["model"].as_str();

    let found = match provider {
        Some("cursor-agent") => cursor_chat(home, Path::new(cwd), started_at(&envelope, run_id)?)?,
        Some("agy") => agy_conversation(home, Path::new(cwd), started_at(&envelope, run_id)?)?,
        _ => transcript_file(home, &envelope, run_id, cwd)?,
    };
    let (lines, note) = match found {
        Found::Lines { lines, note } => (lines, note),
        Found::Nothing(why) => {
            return close_without_transcript(layout, &run_file, run_id, exit_code, &envelope, why);
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
    for line in &lines {
        writer.provider_line(line)?;
        records += 1;
    }

    let unobserved = exit_code.is_none().then(|| {
        "harvested after the run already exited, so the exit code was not observed".to_string()
    });
    let error = match (unobserved, note) {
        (Some(unobserved), Some(note)) => Some(format!("{unobserved}; {note}")),
        (unobserved, note) => unobserved.or(note),
    };
    let error = stop_noted(layout, run_id, error);
    let error = crate::watch::gate_refusals_at_close(
        &layout.spawn(run_id).join("gate-refusals.jsonl"),
        &mut writer,
        error,
    );
    writer.hydra_line(&crate::record::RunEnd::harvested(exit_code, error))?;

    Ok(HarvestOutcome::Harvested { records })
}

/// What a harvest found to wrap: the vendor's lines and a note for
/// `run_end.error`, or why there is nothing.
enum Found {
    Lines {
        lines: Vec<String>,
        note: Option<String>,
    },
    Nothing(String),
}

/// The process's own start, which the vendor's own session start follows;
/// the envelope's time when the record carries none.
fn started_at(envelope: &serde_json::Value, run_id: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    envelope["hydra"]["process_start_time"]
        .as_str()
        .or_else(|| envelope["ts"].as_str())
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&chrono::Utc))
        .with_context(|| format!("run {run_id} carries no start time to find a session by"))
}

/// The lines of a `claude` transcript or a `codex` session file.
fn transcript_file(
    home: &Path,
    envelope: &serde_json::Value,
    run_id: &str,
    cwd: &str,
) -> Result<Found> {
    let (transcript_path, note) = if envelope["provider"].as_str() == Some("codex") {
        let since = started_at(envelope, run_id)?;
        match codex::find_interactive_session(home, Path::new(cwd), since)? {
            Some(found) => {
                let note = (found.later > 0).then(|| {
                    format!(
                        "harvested codex session {}, the first of {} that began in {cwd} after \
                         the run started",
                        found.session_id,
                        found.later + 1
                    )
                });
                (found.path, note)
            }
            None => {
                return Ok(Found::Nothing(format!(
                    "the session wrote no codex session file under {} that ran in {cwd} \
                     and began after {since}, so there is nothing to harvest",
                    codex::sessions_dir(home).display()
                )));
            }
        }
    } else {
        let session = envelope["hydra"]["provider_session_id"]
            .as_str()
            .with_context(|| {
                format!("run {run_id} carries no provider_session_id to harvest with")
            })?;
        (claude::transcript_path(home, Path::new(cwd), session), None)
    };

    // A session that exits before its first turn writes no transcript at all,
    // measured 2026-09-08 on a consent dialog answered with exit. That run is
    // over and has nothing to harvest, and a `run_end` saying so is what keeps
    // `hydra status` from re-reporting the same missing file forever.
    match std::fs::read_to_string(&transcript_path) {
        Ok(text) => Ok(Found::Lines {
            lines: text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_string)
                .collect(),
            note,
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Found::Nothing(format!(
            "the session wrote no transcript at {}, so there is nothing to harvest",
            transcript_path.display()
        ))),
        Err(err) => Err(err).with_context(|| {
            format!(
                "reading the transcript hydra found for run {run_id} at {}",
                transcript_path.display()
            )
        }),
    }
}

/// The messages of the `cursor-agent` chat a run wrote, build ticket 266,
/// one record line each in conversation order. The note says what the
/// harvest could not place or read, since neither is in the lines.
fn cursor_chat(home: &Path, cwd: &Path, since: chrono::DateTime<chrono::Utc>) -> Result<Found> {
    let Some(found) = cursor::find_interactive_chat(home, cwd, since)? else {
        return Ok(Found::Nothing(format!(
            "the session wrote no cursor-agent chat under {} that ran in {} and began after \
             {since}, so there is nothing to harvest",
            cursor::chats_dir(home).display(),
            cwd.display()
        )));
    };
    let store = found.dir.join("store.db");
    if !store.exists() {
        return Ok(Found::Nothing(format!(
            "cursor-agent chat {} has no store.db, so there is nothing to harvest",
            found.chat_id
        )));
    }
    let chat = cursor::read_chat(&store)?;
    if chat.messages.is_empty() {
        return Ok(Found::Nothing(format!(
            "cursor-agent chat {} holds no message, so there is nothing to harvest",
            found.chat_id
        )));
    }
    let mut notes = Vec::new();
    if found.later > 0 {
        notes.push(format!(
            "harvested cursor-agent chat {}, the first of {} that began in {} after the run \
             started",
            found.chat_id,
            found.later + 1,
            cwd.display()
        ));
    }
    if chat.unordered > 0 {
        notes.push(format!(
            "messages no state blob placed are last, in id order: {}",
            chat.unordered
        ));
    }
    if chat.unread > 0 {
        notes.push(format!(
            "protobuf blobs that name no message were left unread, and any usage the chat \
             recorded is among them: {}",
            chat.unread
        ));
    }
    Ok(Found::Lines {
        lines: chat.messages,
        note: (!notes.is_empty()).then(|| notes.join("; ")),
    })
}

/// The transcript of the `agy` conversation a run wrote, build ticket 267,
/// one record line per transcript line and one per step output. The note
/// says what the lines cannot: a later conversation, a store that would not
/// read, an output with no step, and steps whose calls meet their outputs by
/// order alone.
fn agy_conversation(
    home: &Path,
    cwd: &Path,
    since: chrono::DateTime<chrono::Utc>,
) -> Result<Found> {
    let Some(found) = agy::find_interactive_conversation(home, cwd, since)? else {
        return Ok(Found::Nothing(format!(
            "the session wrote no agy conversation under {} that ran in {} and began after \
             {since}, so there is nothing to harvest",
            agy::data_dir(home).join("conversations").display(),
            cwd.display()
        )));
    };
    let Some(transcript) = agy::read_transcript(&found.brain)? else {
        return Ok(Found::Nothing(format!(
            "agy conversation {} has no transcript at {}, so there is nothing to harvest",
            found.conversation_id,
            agy::transcript_path(&found.brain).display()
        )));
    };
    let mut notes = Vec::new();
    if found.later > 0 {
        notes.push(format!(
            "harvested agy conversation {}, the first of {} that began in {} after the run \
             started",
            found.conversation_id,
            found.later + 1,
            cwd.display()
        ));
    }
    if found.unreadable > 0 {
        notes.push(format!(
            "conversation stores begun after the run that sqlite3 could not read, so their \
             workspace is unknown: {}",
            found.unreadable
        ));
    }
    if transcript.unplaced > 0 {
        notes.push(format!(
            "step outputs with no transcript line for their step are last, in step order: {}",
            transcript.unplaced
        ));
    }
    if transcript.multi_call_steps > 0 {
        notes.push(format!(
            "steps that made more than one tool call, whose outputs were joined to calls by \
             step order because the transcript carries no call id: {}",
            transcript.multi_call_steps
        ));
    }
    Ok(Found::Lines {
        lines: transcript.lines,
        note: (!notes.is_empty()).then(|| notes.join("; ")),
    })
}

/// The `run_end` of a session that left nothing to harvest, with `error`
/// saying where the harvest looked.
fn close_without_transcript(
    layout: &Layout,
    run_file: &Path,
    run_id: &str,
    exit_code: Option<i32>,
    envelope: &serde_json::Value,
    error: String,
) -> Result<HarvestOutcome> {
    let next_seq = crate::record::next_seq(run_file)?;
    let mut writer = Writer::resume(
        run_file,
        run_id,
        envelope["parent_run_id"].as_str(),
        envelope["provider"].as_str(),
        envelope["model"].as_str(),
        next_seq,
    )?;
    if crate::live::has_run_end(run_file)? {
        return Ok(HarvestOutcome::AlreadyDone);
    }
    let error = stop_noted(layout, run_id, Some(error));
    let error = crate::watch::gate_refusals_at_close(
        &layout.spawn(run_id).join("gate-refusals.jsonl"),
        &mut writer,
        error,
    );
    writer.hydra_line(&crate::record::RunEnd::harvested(exit_code, error))?;
    Ok(HarvestOutcome::Harvested { records: 0 })
}

/// `error` with the stop note in front when `hydra stop` marked this run,
/// build ticket 169. The harvest is what closes an interactive session, so it
/// is the one writer besides the watcher that has to say a human stopped it.
fn stop_noted(layout: &Layout, run_id: &str, error: Option<String>) -> Option<String> {
    if !layout.stop_marker(run_id).exists() {
        return error;
    }
    let note = crate::watch::STOP_NOTE;
    Some(match error {
        Some(existing) => format!("{note}; {existing}"),
        None => note.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gated_launch_whose_panels_name_two_shapes_names_both() {
        let dir = std::env::temp_dir().join(format!("hydra-askable-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let panels = dir.join("panels.json");
        let seat = serde_json::json!({ "provider": "claude", "policy": "judge", "role": "r" });
        let document = serde_json::json!({ "shapes": [
            { "name": "review", "askers": ["standard"], "cap_per_hour": 2, "seats": [seat] },
            { "name": "triage", "askers": ["standard", "read-only"], "cap_per_hour": 1, "seats": [seat] },
            { "name": "elsewhere", "askers": ["permissive"], "cap_per_hour": 1, "seats": [seat] },
        ]});
        std::fs::write(&panels, document.to_string()).unwrap();
        let policies =
            crate::policy::PolicySet::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("policies"))
                .unwrap();

        let shapes = askable_shapes(&panels, &policies, "standard");
        assert_eq!(
            shapes,
            ["review", "triage"],
            "a shape this policy may not ask for is not named"
        );
        let argv = claude::interactive_argv(&claude::InteractiveLaunch {
            session: "s1",
            task: None,
            model: None,
            mcp: None,
            settings: Some(Path::new("/tmp/spawn/r1/settings.json")),
            policy: Some("standard"),
            shapes: &shapes,
        });
        let at = argv
            .iter()
            .position(|a| a == "--append-system-prompt")
            .unwrap();
        assert!(
            argv[at + 1].ends_with("the `panel` tool: review, triage."),
            "{argv:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Build tickets 255, 266 and 267: `codex`, `cursor-agent` and `agy`
    /// launch interactively only without a policy, and the refusal says the
    /// gated mode is `claude` only.
    #[test]
    fn the_other_three_are_interactive_only_ungated() {
        assert!(refuse_provider(Provider::Claude, true).is_ok());
        assert!(refuse_provider(Provider::Claude, false).is_ok());
        assert!(refuse_provider(Provider::Codex, false).is_ok());
        assert!(refuse_provider(Provider::CursorAgent, false).is_ok());
        let gated = refuse_provider(Provider::Codex, true)
            .unwrap_err()
            .to_string();
        assert!(gated.contains("claude only"), "{gated}");
        let gated = refuse_provider(Provider::CursorAgent, true)
            .unwrap_err()
            .to_string();
        assert!(gated.contains("launch cursor-agent ungated"), "{gated}");
        assert!(refuse_provider(Provider::Agy, false).is_ok());
        let gated = refuse_provider(Provider::Agy, true)
            .unwrap_err()
            .to_string();
        assert!(gated.contains("launch agy ungated"), "{gated}");
    }

    #[test]
    fn a_missing_panels_file_names_no_shape() {
        let policies =
            crate::policy::PolicySet::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("policies"))
                .unwrap();
        let missing = std::env::temp_dir().join("hydra-askable-none/panels.json");
        assert!(askable_shapes(&missing, &policies, "standard").is_empty());
    }
}
