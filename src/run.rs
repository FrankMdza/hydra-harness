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

/// What the parent's record says about a child, harness spec 6.1 items 2 and 3.
///
/// **The parent is a run id and its record is the only source.** Contract 4 makes
/// `parent_run_id` an edge the parent writes at spawn and never reads back off the
/// child, so the two facts a launch needs about its parent, how deep it is and
/// what policy bounds it, come off the parent's own line 0.
///
/// **Both checks run before the child exists and only one of them writes a
/// record.** [`Lineage::of`] refuses a tree already at the cap with a typed error
/// and no run id, because contract 8.3 promises a record for the narrowing
/// refusal and for nothing else. [`Lineage::narrowing`] is the one that writes,
/// and 7.3 says why: attempted escalation is exactly the signal the corpus should
/// carry.
#[derive(Debug)]
pub struct Lineage {
    parent: Option<Parent>,
    /// The parent's depth plus one, and 0 for a run a human started.
    pub depth: u32,
}

/// The two fields a child reads off its parent's record.
#[derive(Debug)]
struct Parent {
    run_id: String,
    /// `None` only for a harness parent, contract 5.5: a `hydra evolve` pass has
    /// no policy because no gate was compiled for it, and its children are the
    /// proposer and the judges the human's own command spawns. Any other record
    /// that states no policy states no bound, and [`Lineage::of`] refuses it
    /// rather than reading the absence as permission.
    policy: Option<String>,
}

impl Lineage {
    /// Reads the parent's record and computes the child's depth, 6.1 item 2.
    ///
    /// Three ways this refuses, all of them before a run id exists and all of them
    /// errors rather than records: a parent this machine has no record of, a
    /// parent whose record states no policy, and a child that would land past
    /// [`crate::live::MAX_DEPTH`].
    pub fn of(layout: &Layout, parent: Option<&str>) -> Result<Self> {
        let Some(parent) = parent else {
            return Ok(Self {
                parent: None,
                depth: 0,
            });
        };

        let run_file = layout.run_file(parent);
        let Some(start) = crate::live::opening_line(&run_file)? else {
            anyhow::bail!(
                "there is no run record at {}, so how deep {parent} is and what policy \
                 bounds it cannot be read, and a child of it would be launched bounded by \
                 nothing. `hydra status` lists the runs this machine has.",
                run_file.display()
            );
        };

        // A harness run, contract 5.5: `command: evolve`, no provider, no policy,
        // because nothing was gated. Spec 4.10 makes it the parent of the proposer
        // and the four judges so a pass can be grouped and a Ctrl-C leaves no
        // orphans, and 7.3's bound is the human's who typed the pass, which is no
        // bound. Measured 2026-09-08 on the first real pass, 01a08127-6f58, which
        // this refusal ended before the proposer existed.
        let harness_parent = start["command"].as_str() == Some("evolve")
            && start["launch_mode"].is_null()
            && start["policy"].is_null();
        let policy = match start["policy"].as_str() {
            Some(policy) => Some(policy.to_string()),
            None if harness_parent => None,
            None => anyhow::bail!(
                "run {parent} has no policy on its record, which is a run written before \
                 the gate reached a provider or an interactive one (contract 5.6). Harness \
                 spec 7.3 bounds a child by its parent's policy and this parent states \
                 none, so it cannot delegate. Start the work at depth 0 with `hydra run`."
            ),
        };

        let depth = start["depth"].as_u64().and_then(|d| u32::try_from(d).ok());
        let Some(depth) = depth else {
            anyhow::bail!(
                "run {parent} has no usable depth on its record, and a child's depth is \
                 its parent's plus one (harness spec 6.1). The record is {}.",
                run_file.display()
            );
        };

        // Compared rather than incremented, because a record claiming a depth of
        // `u32::MAX` is a torn or hand-edited line and adding one to it panics.
        if depth >= crate::live::MAX_DEPTH {
            return Err(TooDeep {
                parent_run_id: parent.to_string(),
                parent_depth: depth,
                cap: crate::live::MAX_DEPTH,
            }
            .into());
        }

        Ok(Self {
            parent: Some(Parent {
                run_id: parent.to_string(),
                policy,
            }),
            depth: depth + 1,
        })
    }

    pub fn parent_run_id(&self) -> Option<&str> {
        self.parent.as_ref().map(|parent| parent.run_id.as_str())
    }

    pub fn parent_policy(&self) -> Option<&str> {
        self.parent
            .as_ref()
            .and_then(|parent| parent.policy.as_deref())
    }

    /// 6.1 item 3, the containment test. `None` when the child may run, and the
    /// remediation a human reads when it may not.
    ///
    /// **Containment is a partial order and this is where that costs something.**
    /// `curate` writes to its checkout of the store and cannot touch a worktree,
    /// `standard` is the reverse, and neither contains the other. 7.3 refuses an
    /// incomparable pair rather than resolving it, so a `standard` parent cannot
    /// delegate a `curate` child and the remediation says which case this is.
    pub fn narrowing(
        &self,
        policies: &crate::policy::PolicySet,
        child: &str,
    ) -> Result<Option<String>> {
        let Some(parent) = &self.parent else {
            return Ok(None);
        };
        // A harness parent bounds nothing: see [`Parent::policy`].
        let Some(parent_policy) = parent.policy.as_deref() else {
            return Ok(None);
        };
        let bound = policies.get(parent_policy).with_context(|| {
            format!(
                "run {} names a policy this binary does not have, so there is nothing for \
                 7.3 to compare against",
                parent.run_id
            )
        })?;
        let asked = policies.get(child)?;
        if bound.contains(asked) {
            return Ok(None);
        }

        // Two different failures with one refusal, and the words matter to
        // whoever reads them: one is an attempted escalation, the other is 7.3's
        // accepted cost showing up in a legitimate request.
        let relation = if asked.contains(bound) {
            format!("`{child}` is wider than `{parent_policy}`")
        } else {
            format!(
                "`{child}` and `{parent_policy}` are incomparable, because each grants \
                 something the other does not"
            )
        };
        Ok(Some(format!(
            "{relation}, and harness spec 7.3 never lets a child hold a capability its \
             parent does not. Run {} is the parent. Ask for `{parent_policy}` or a policy \
             it contains, or start this work at depth 0 yourself.",
            parent.run_id
        )))
    }
}

/// The depth cap refused this spawn, 6.1 item 2.
///
/// **A typed error rather than a record**, which is the shape 6.1 item 4 gives the
/// concurrency cap. Contract 8.3 promises a run record for the narrowing refusal
/// and for nothing else, and the caller here is itself a run whose record already
/// carries the `delegate` call and the error it got back. A run id minted to say
/// "this never ran" would put a row in the corpus with no policy, no task and no
/// tree under it.
#[derive(Debug)]
pub struct TooDeep {
    pub parent_run_id: String,
    pub parent_depth: u32,
    pub cap: u32,
}

impl std::fmt::Display for TooDeep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "run {} is at depth {} and the cap is {}, so a child of it would be depth {} \
             and this spawn is refused. A child that delegates is how three subscriptions \
             get drained overnight (harness spec 9), and section 7.6 closes the cap's \
             blind spot by denying in-process subagents. Start the work you wanted at \
             depth 0.",
            self.parent_run_id,
            self.parent_depth,
            self.cap,
            self.parent_depth.saturating_add(1)
        )
    }
}

impl std::error::Error for TooDeep {}

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

fn same_path(left: &std::path::Path, right: &std::path::Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

pub fn run(request: Request) -> Result<Call> {
    // **What the attempt was is decided before what the machine is.** 6.1's items
    // 2 and 3 run first here, ahead of the environment, the provider's version and
    // the two repo files, because 7.3 wants an attempted escalation on disk and
    // those three can all fail on a machine where the child was never going to
    // start. Reading the version first would turn a child reaching for `permissive`
    // on a binary nobody installed into an error with nothing written, which is the
    // invisible attempt the containment check exists to prevent.
    let invoked_in = match &request.working_dir {
        Some(dir) => dir.clone(),
        None => std::env::current_dir().context("reading the working directory")?,
    };
    let repo = crate::worktree::repo_root(&invoked_in);

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
    let layout = Layout::discover()?;
    crate::paths::refuse_non_ascii_roots(&layout, &crate::paths::hydra_home())?;
    refuse_foreign_checkout(&layout, &crate::paths::hydra_home(), root_named)?;
    layout.ensure()?;

    // Build ticket 57, read before the lineage because it is the question the
    // lineage cannot ask: `Lineage::of` is handed `--parent` and believes it, and
    // a flag left off reaches it as a human at depth 0. This reads the marker
    // Hydra set in the child's own environment instead, so the answer comes from
    // the launch rather than from the caller's argv.
    let caller = crate::containment::caller(&layout)?;

    // 6.1 item 2, ahead of item 4 below in the order the spec lists them. The
    // parent's own record is the only thing that says how deep this child is and
    // what policy bounds it, so a `--parent` this machine has no record of is
    // refused here rather than launched bounded by nothing.
    let lineage = Lineage::of(&layout, request.parent.as_deref())?;

    let run_id = request
        .run_id
        .clone()
        .unwrap_or_else(|| Uuid::now_v7().to_string());
    let reservation = crate::live::reserve(&layout, &run_id, crate::live::LIVE_RUN_CAP)?;
    let run_file = layout.run_file(&run_id);

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
    if let Some(remediation) = caller.escalation(request.parent.as_deref()) {
        let line_zero = serde_json::to_value(run_start(
            &request,
            &task,
            policies.sha(),
            None,
            &invoked_in,
            None,
            None,
            None,
            None,
            &lineage,
            None,
            None,
            None,
        ))?;
        return refuse(
            &run_file,
            &run_id,
            request.provider,
            &line_zero,
            crate::record::Refusal::Containment,
            remediation,
            &policies,
            None,
            None,
            &layout,
            lineage.parent_run_id(),
        );
    }

    if let Some(remediation) = lineage.narrowing(&policies, &request.policy)? {
        // Contract 8.3: no provider fields on a refused spawn. Nothing asked the
        // binary for its version and nothing built the child's environment, so
        // both are absent rather than guessed, and `skills` and `mcp` follow. The
        // envelope still names the provider, which is contract 4's field and a
        // fact about the request rather than about a process. `scopes` is absent
        // for the same reason and `floor_lifted` is not: the flag was typed on
        // this run, and no gate was compiled to resolve a path (contract 5.1).
        let line_zero = serde_json::to_value(run_start(
            &request,
            &task,
            policies.sha(),
            None,
            &invoked_in,
            None,
            None,
            None,
            None,
            &lineage,
            None,
            None,
            None,
        ))?;
        return refuse(
            &run_file,
            &run_id,
            request.provider,
            &line_zero,
            crate::record::Refusal::PolicyNarrowing,
            remediation,
            &policies,
            None,
            None,
            &layout,
            lineage.parent_run_id(),
        );
    }

    // The machine's own facts, read once the attempt has been judged. A wrong
    // PATH, a missing HOME, a provider binary that is not installed and a repo
    // that already carries the provider's two files are all errors rather than
    // refused spawns, the way they were before delegation existed.
    let mut child_env = ChildEnv::build(request.provider)?;
    // 6.2's second category and build ticket 57's whole mechanism: the child is
    // told which run it is, so that a `hydra run` it launches has something to
    // check itself against. The name lands in `run_start.env_allowlist` like
    // every other name passed, so a reader can see the child was marked.
    child_env.set(crate::containment::MARKER, run_id.clone());
    let provider_version = request.provider.version()?;
    if request.provider == Provider::Codex
        && let Some(remediation) = codex::unsupported_policy(&request.policy)
    {
        anyhow::bail!(remediation);
    }
    // Both providers that read their hook config out of the worktree get the same
    // check against their own two files.
    match request.provider {
        Provider::CursorAgent => worktree_preconditions(
            "cursor-agent",
            repo.as_deref(),
            &[cursor::hooks_file, cursor::mcp_file],
        )?,
        Provider::Agy => {
            worktree_preconditions("agy", repo.as_deref(), &[agy::hooks_file, agy::mcp_file])?
        }
        Provider::Codex => worktree_preconditions(
            "codex",
            repo.as_deref(),
            &[codex::hooks_file, codex::mcp_file],
        )?,
        Provider::Claude => {}
    }

    // 6.2's second category: a variable Hydra sets rather than one it passes
    // through. `CURSOR_DATA_DIR` moves Cursor's per-project MCP approval state
    // into the run's spawn directory, so approving an injected server writes
    // under a path Hydra created rather than into `~/.cursor/`. It is set before
    // line 0 is composed so the name lands in `env_allowlist`, and it travels to
    // the detached half by value, because it is a path Hydra computed and not a
    // secret the launcher inherited.
    let mut hydra_env = std::collections::BTreeMap::new();
    if request.provider == Provider::CursorAgent {
        let data_dir = layout.cursor_data_dir(&run_id);
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating {}", data_dir.display()))?;
        let value = data_dir.display().to_string();
        child_env.set("CURSOR_DATA_DIR", value.clone());
        hydra_env.insert("CURSOR_DATA_DIR".to_string(), value);
    }

    let worktree = match &repo {
        Some(repo) => Some(crate::worktree::create(
            repo,
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
        base_tree: w.seed_tree.clone(),
    });

    // 6.5: the document decides which servers this run's policy admits and the
    // launch shape decides whether any of them can be delivered. The file is
    // written before the probe below, because the probe carries the run's own MCP
    // flags and that is what makes its answer the run's rather than the machine's.
    // A launch that cannot deliver gets no file, and the check that refuses it
    // runs after the skill collision check, where 6.1 puts it.
    let checkout = match &worktree_ref {
        Some(_) => crate::mcp::Checkout::Hydra,
        None => crate::mcp::Checkout::None,
    };
    let mcp_plan = crate::mcp::plan(&mcp_document, request.provider, &request.policy, checkout);
    let mcp_file = match mcp_plan.undeliverable {
        Some(_) => None,
        None => Some(crate::mcp::write_config(
            &layout,
            &run_id,
            request.provider,
            mcp_plan.injected(),
            worktree_ref.as_ref().map(|w| std::path::Path::new(&w.path)),
        )?),
    };

    // 6.4: the store is pinned, snapshotted and linked into the checkout before
    // anything asks the provider a question. A run with no checkout has nowhere to
    // inject into, so it gets no links and its record leaves `skills` absent
    // rather than null: the question does not apply. A `judge` run is not injected
    // either, since the policy allows nothing and the diff rides the prompt, and
    // ticket 25 stopped the links. The MCP group is present either way, which is
    // 6.5's asymmetry showing up in the record: on `claude` the servers ride the
    // command line and need no checkout to land in.
    let injected = match &worktree_ref {
        Some(worktree) if request.policy != "judge" => Some(inject_skills(
            &layout,
            &run_id,
            request.provider,
            std::path::Path::new(&worktree.path),
            crate::mcp::Probe {
                policy: &request.policy,
                config: mcp_file.as_deref(),
            },
        )?),
        _ => None,
    };

    let injected_names = mcp_plan.injected_names();

    // The whole of line 0 except the two fields that do not exist until a process
    // does, as a function of the two the steps below decide.
    // Composing it here keeps every provider-specific fact on this side of the
    // fork and leaves the detached half nothing to decide, and a refused launch
    // writes the same line 0 a spawned one would, because contract 8.3 wants the
    // attempt on disk in full.
    //
    // **`scopes` is absent until a gate is compiled**, which is below the two
    // pre-flight refusals. A refused run resolved no scope, so contract 5.1's
    // rule applies: the key is absent rather than filled with what the compile
    // would have produced had it run.
    let line_zero = |mcp_resolved: Option<&[String]>,
                     scopes: Option<&std::collections::BTreeMap<String, String>>,
                     gate_sha: Option<&str>,
                     sandbox: Option<&str>|
     -> Result<serde_json::Value> {
        let start = run_start(
            &request,
            &task,
            policies.sha(),
            Some(&provider_version),
            &cwd,
            worktree_ref.clone(),
            Some(child_env.names()),
            injected.as_ref().map(|i| i.record.clone()),
            Some(crate::mcp::record(
                &mcp_document.sha,
                &injected_names,
                mcp_resolved,
            )),
            &lineage,
            scopes.cloned(),
            gate_sha.map(str::to_string),
            sandbox.map(str::to_string),
        );
        Ok(serde_json::to_value(&start)?)
    };

    // 6.1 item 5. A hit fails the run as its own outcome, and the record is the
    // whole of what the caller gets back. It is read before the pre-flight below
    // approves anything, so a launch refused here has started no server process,
    // and on `cursor-agent` its record says nobody asked about servers, which is
    // the true answer.
    if let Some(injected) = &injected
        && !injected.collisions.is_empty()
    {
        let remediation = crate::skills::collision::remediation(&injected.collisions);
        return refuse(
            &run_file,
            &run_id,
            request.provider,
            &line_zero(injected.mcp_resolved.as_deref(), None, None, None)?,
            crate::record::Refusal::SkillCollision,
            remediation,
            &policies,
            worktree_ref.as_ref(),
            seed_tree.as_deref(),
            &layout,
            lineage.parent_run_id(),
        );
    }

    // 6.1 item 7, the pre-flight Cursor alone gets, run here because its listing
    // is the provider's own report of what the run will start, which is what
    // contract 5.3's `mcp.resolved` is on this provider, and line 0 carries it.
    // Its verdict is read after item 6 below, in 6.1's order. A pre-flight that
    // could not run at all is the same refusal with the error as its remediation:
    // nothing spawned, and the record says why.
    let preflight = match (request.provider, &worktree_ref, mcp_plan.undeliverable) {
        (Provider::CursorAgent, Some(worktree), None) => {
            match cursor::preflight(
                std::path::Path::new(&worktree.path),
                &child_env,
                &injected_names,
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
        None => injected
            .as_ref()
            .and_then(|injected| injected.mcp_resolved.clone()),
    };
    // No gate is compiled yet, so the two refusals below write a line 0 with no
    // `scopes` on it. The launched path recomposes it after `write_spawn_config`.
    let refused_line_zero = line_zero(mcp_resolved.as_deref(), None, None, None)?;

    if request.provider == Provider::Codex {
        let remediation = match codex::login_status() {
            Ok(codex::LoginStatus::LoggedIn) => None,
            Ok(codex::LoginStatus::NotLoggedIn) => Some(codex::auth_remediation().to_string()),
            Err(err) => Some(format!(
                "the codex auth pre-flight could not run, so whether this launch is logged in \
                 is unknown and the run is refused: {err:#}"
            )),
        };
        if let Some(remediation) = remediation {
            return refuse(
                &run_file,
                &run_id,
                request.provider,
                &refused_line_zero,
                crate::record::Refusal::AuthPreflight,
                remediation,
                &policies,
                worktree_ref.as_ref(),
                seed_tree.as_deref(),
                &layout,
                lineage.parent_run_id(),
            );
        }
    }

    // 6.1 item 6, one item after it. A run whose policy admits a server this
    // launch cannot deliver is refused rather than launched short, which is
    // section 2's rule: a run that quietly comes up short is what it forbids.
    if mcp_plan.shortfall().is_some() {
        return refuse(
            &run_file,
            &run_id,
            request.provider,
            &refused_line_zero,
            crate::record::Refusal::McpUnavailable,
            mcp_plan.remediation(),
            &policies,
            worktree_ref.as_ref(),
            seed_tree.as_deref(),
            &layout,
            lineage.parent_run_id(),
        );
    }

    // 6.1 item 7. Any injected server the listing did not report ready fails the
    // run, because on this provider that listing is the one that lies: the
    // human's same-named server may be starting with the human's token while
    // the name reads as not loaded. Same refusal code as item 6; harness spec 12
    // says which check fired is a field Hydra sets, and here it is the
    // remediation.
    if let Some(preflight) = &preflight {
        let remediation = match preflight {
            Ok(preflight) => {
                let short = preflight.shortfall(&injected_names);
                if short.is_empty() {
                    None
                } else {
                    Some(preflight.remediation(&short))
                }
            }
            Err(why) => Some(why.clone()),
        };
        if let Some(remediation) = remediation {
            return refuse(
                &run_file,
                &run_id,
                request.provider,
                &refused_line_zero,
                crate::record::Refusal::McpUnavailable,
                remediation,
                &policies,
                worktree_ref.as_ref(),
                seed_tree.as_deref(),
                &layout,
                lineage.parent_run_id(),
            );
        }
    }

    let spawn = write_spawn_config(
        &layout,
        &run_id,
        &policies,
        request.provider,
        &request.policy,
        worktree_ref.as_ref().map(|w| std::path::Path::new(&w.path)),
        repo.as_deref(),
        injected.as_ref().map(|i| i.store_sha.as_str()),
        request.floor_lifted,
    )?;

    // Line 0 again, now that the gate has been compiled and the paths it
    // resolved are known. Everything above it wrote `refused_line_zero`, which
    // carries no `scopes` because no gate existed to name any.
    let line_zero = line_zero(
        mcp_resolved.as_deref(),
        Some(&spawn.scopes),
        Some(&spawn.gate_sha),
        spawn.sandbox_sha.as_deref(),
    )?;

    if crate::sandbox::wraps_launches() && !crate::sandbox::exec_available() {
        let err = anyhow::anyhow!(
            "/usr/bin/sandbox-exec is missing on this Mac, so Hydra refuses to launch \
             a recorded run without the seatbelt rather than run bare"
        );
        eprintln!("hydra: run {run_id} did not launch");
        eprintln!("hydra: {err:#}");
        eprintln!("hydra: record {}", run_file.display());
        if let Some(worktree) = &worktree_ref {
            eprintln!("hydra: worktree {} on {}", worktree.path, worktree.branch);
        }
        if let Err(second) = never_started(
            &run_file,
            &run_id,
            request.provider,
            &line_zero,
            &err,
            lineage.parent_run_id(),
        ) {
            eprintln!("hydra: this run could not be recorded either: {second:#}");
        }
        return Err(err);
    }

    // **The gate mount is checked on `agy` and on nothing else, free.** 6.6's
    // table says this provider reads its hook config only when the worktree
    // arrives as `--add-dir`, "without which it reports `hooks: []` rather than
    // an error", and that silence is the worst failure mode on the ticket: a run
    // that looks gated and is not. One read-only slash command rules it out for
    // no tokens, measured. `claude` needs no equivalent, because its config is an
    // argument it either accepted or refused to start with, and Cursor's
    // `failClosed` config would block rather than leak.
    //
    // A launch that fails here writes the record itself, the same shape a spawn
    // that could not fork gets: contract 8.3 wants the attempt on disk, and by
    // this point a worktree, a branch and a compiled gate are on disk with a run
    // id on them. It is not one of contract 6's four refusals, so it is not
    // dressed as one: this is a launch that could not start, and it derives
    // `failed`.
    if request.provider == Provider::Agy {
        let checkout = worktree_ref
            .as_ref()
            .map(|w| std::path::PathBuf::from(&w.path))
            .context("an agy run needs a checkout and this one has none")?;
        let mounted = agy::hooks_mounted(&checkout).with_context(|| {
            format!(
                "asking agy which hooks it loaded for {}, which is what proves the gate is \
                 in front of the run",
                checkout.display()
            )
        });
        let verdict = match &mounted {
            Ok(mounted) if agy::gate_is_mounted(mounted, &spawn.hook_config) => Ok(()),
            Ok(mounted) => Err(anyhow::anyhow!(
                "{}",
                agy::unmounted_remediation(mounted, &spawn.hook_config)
            )),
            Err(err) => Err(anyhow::anyhow!("{err:#}")),
        };
        if let Err(err) = verdict {
            // The same three lines `refuse` prints, for the same reason: a
            // checkout, a branch and a compiled gate are on disk with this run
            // id on them, and an error naming none of them leaves a human with
            // an orphan worktree and nothing to type into `hydra reap`.
            eprintln!("hydra: run {run_id} did not launch");
            eprintln!("hydra: {err:#}");
            eprintln!("hydra: record {}", run_file.display());
            if let Some(worktree) = &worktree_ref {
                eprintln!("hydra: worktree {} on {}", worktree.path, worktree.branch);
            }
            if let Err(second) = never_started(
                &run_file,
                &run_id,
                request.provider,
                &line_zero,
                &err,
                lineage.parent_run_id(),
            ) {
                eprintln!("hydra: this run could not be recorded either: {second:#}");
            }
            return Err(err);
        }
    }

    let mut argv = match request.provider {
        Provider::Claude => claude::argv(&claude::Launch {
            task: &task,
            model: request.model.as_deref(),
            settings: &spawn.hook_config,
            policy: &request.policy,
            mcp: mcp_file.as_deref(),
        }),
        // Nothing provider-specific rides the argv. The hook config, the MCP
        // config and the skill links are all read from cwd, and the data
        // directory is in the environment.
        Provider::CursorAgent => cursor::argv(&cursor::Launch {
            task: &task,
            model: request.model.as_deref(),
        }),
        // The one flag the skills, the hook config and the MCP config all ride
        // in on. A launch with no worktree was refused above, so the checkout is
        // here rather than optional.
        Provider::Agy => agy::argv(&agy::Launch {
            task: &task,
            model: request.model.as_deref(),
            worktree: worktree_ref
                .as_ref()
                .map(|w| std::path::Path::new(w.path.as_str()))
                .context("an agy run needs a checkout and this one has none")?,
        }),
        Provider::Codex => codex::argv(&codex::Launch {
            task: &task,
            model: request.model.as_deref(),
            worktree: worktree_ref
                .as_ref()
                .map(|w| std::path::Path::new(w.path.as_str()))
                .context("a codex run needs a checkout and this one has none")?,
            hook_file: &codex::hook_file(&layout, &run_id),
        }),
    };
    if let Some(session_id) = &request.resume_session_id {
        let hook_file = if request.provider == Provider::Codex {
            Some(codex::hook_file(&layout, &run_id))
        } else {
            None
        };
        append_resume_args(
            &mut argv,
            request.provider,
            session_id,
            hook_file.as_deref(),
        );
    }

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
            // Contract 4's edge, carried across the fork. The detached half owns
            // every line of the record, so this is the only way the parent's id
            // reaches any of them.
            parent_run_id: lineage.parent_run_id().map(str::to_string),
            provider: request.provider.to_string(),
            binary: request.provider.binary().to_string(),
            argv,
            cwd,
            run_file: run_file.clone(),
            model: request.model.clone(),
            hydra_env,
            run_start: line_zero.clone(),
            sandbox_profile: spawn.sandbox_profile.clone(),
        },
    )
    .and_then(|()| std::env::current_exe().context("locating the hydra binary"))
    .and_then(|exe| crate::watch::detach(&exe, &spec_path, &layout.watcher_log(&run_id)));

    let mut watcher = match launched {
        Ok(watcher) => watcher,
        Err(err) => {
            if let Err(second) = never_started(
                &run_file,
                &run_id,
                request.provider,
                &line_zero,
                &err,
                lineage.parent_run_id(),
            ) {
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

    drop(reservation);
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

/// Pins the store, snapshots it, links it into the checkout and asks the provider
/// what it resolved.
///
/// Split out of [`run`] because it is four steps with one failure mode each and
/// none of them is about launching a process.
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
pub(crate) fn inject_skills(
    layout: &Layout,
    run_id: &str,
    provider: Provider,
    worktree: &std::path::Path,
    mcp: crate::mcp::Probe<'_>,
) -> Result<InjectedSkills> {
    let store = crate::skills::Store::open(layout)?;
    let store_sha = store.pin(run_id)?;
    let snapshot = store.snapshot(layout, &store_sha)?;
    let injected = crate::skills::inject(worktree, provider, &snapshot, &store_sha)?;

    let resolved = match provider.resolve(worktree, mcp) {
        Ok(resolved) => resolved,
        Err(err) => {
            eprintln!(
                "hydra: {provider} would not say what it resolved, so the record \
                 cannot: {err:#}"
            );
            crate::provider::Resolved::default()
        }
    };
    // 6.5 gives the fatal pre-flight to Cursor alone, so a server that would not
    // start is a line on stderr and a difference between `injected` and
    // `resolved` in the record, rather than a refused launch.
    for failure in &resolved.failed_mcp {
        eprintln!("hydra: an injected MCP server did not start: {failure}");
    }

    let home = crate::env::real_home()?;
    let user = crate::skills::index::Index::user_scope(&home)?;
    let project = crate::skills::index::Index::project_scope(&injected.root)?;
    let collisions = crate::skills::collision::check(
        &injected,
        provider,
        &home,
        &user,
        &project,
        resolved.skills.as_deref(),
    );

    Ok(InjectedSkills {
        record: crate::skills::record(&injected, resolved.skills.as_deref()),
        collisions,
        store_sha,
        mcp_resolved: resolved.mcp,
    })
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
#[allow(clippy::too_many_arguments)]
pub fn refuse(
    run_file: &std::path::Path,
    run_id: &str,
    provider: Provider,
    line_zero: &serde_json::Value,
    refusal: crate::record::Refusal,
    remediation: String,
    policies: &crate::policy::PolicySet,
    worktree: Option<&WorktreeRef>,
    seed_tree: Option<&str>,
    layout: &Layout,
    parent_run_id: Option<&str>,
) -> Result<Call> {
    eprintln!("hydra: run {run_id} refused");
    eprintln!("hydra: {remediation}");

    // Contract 4: the edge goes on every line of the child's file, including the
    // two a refused spawn gets, because a refused child is still a child.
    let mut writer = crate::record::Writer::create(
        run_file,
        run_id,
        parent_run_id,
        Some(provider.binary()),
        None,
    )?;
    writer.hydra_line(line_zero)?;
    writer.hydra_line(&crate::record::RunEnd::refused(refusal, remediation))?;
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
    let run = crate::normalize::read_file(run_file, policies)?;
    Ok(Call::Ended(Box::new(result(
        &run, run_file, worktree, seed_tree, layout,
    ))))
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

/// Line 0, composed before the fork.
///
/// **`pid` and `process_start_time` are null here and are the detached half's to
/// fill in.** They are contract 5.2's liveness pair and neither exists until a
/// process does, which is the one fact this side cannot know.
///
/// It is public for the same reason [`refuse`] is: `tests/delegation.rs` composes
/// the line a refused child gets and everything above it in [`run`] needs a
/// provider binary on PATH.
#[allow(clippy::too_many_arguments)]
pub fn run_start(
    request: &Request,
    task: &str,
    policy_sha: &str,
    // `None` on a refused spawn, where no binary was asked (contract 8.3).
    provider_version: Option<&str>,
    cwd: &std::path::Path,
    worktree: Option<WorktreeRef>,
    // `None` on a refused spawn, where no child environment was built. Absent
    // rather than empty, because an empty list reads as a measurement.
    env_allowlist: Option<Vec<String>>,
    skills: Option<serde_json::Value>,
    mcp: Option<serde_json::Value>,
    lineage: &Lineage,
    scopes: Option<std::collections::BTreeMap<String, String>>,
    // `None` on a refused spawn and on a run with no gate, build ticket 49.
    gate_sha: Option<String>,
    // Null on a launch no seatbelt wrapped, build ticket 68.
    sandbox: Option<String>,
) -> RunStart {
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
    }
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
}
