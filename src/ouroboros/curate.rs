//! `hydra curate`: Ouroboros rung one, invoked and never scheduled.
//!
//! One pass reads the corpus, launches a recorded run under the `curate` policy
//! with what it found, and commits whatever that run wrote into its checkout of
//! the skill store onto `ouroboros/curate/<run-id>`. A human merges the branch or
//! does not. `docs/ouroboros-spec.md` section 3 is the design.
//!
//! **It is a `hydra run` like any other** (3.5), so it goes through
//! [`crate::run::run`]: one launch sequence, one record, one writer. Four things
//! are the pass's own rather than the launch's: which runs the curator is told
//! about, where the `skill_checkout` scope points, the pin before the checkout,
//! and the commit afterwards.
//!
//! **The pass gets no run record of its own.** Contract 5.5 says why, and the test
//! it gives is whether the supervisor decided anything after the run's file
//! closed. Committing work the run already did is not a decision, and a commit is
//! self-describing. `hydra evolve` is the command that fails that test.
//!
//! **Nothing lands.** Three mechanisms, and only the first is a gate: writes are
//! scoped to a checkout that is not the store's working tree, the commit goes to a
//! branch nothing merges, and no code path here merges one.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use uuid::Uuid;

use crate::outcome::RunResult;
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::provider::Provider;
use crate::run::{Call, Handle, Request};
use crate::skills::Store;
use crate::worktree::{commit_everything, git};

use super::corpus::{self, Learnable, Selection};

/// What the human asked for.
pub struct Pass {
    /// A model inside the provider. Section 3.5: which model curates is a config
    /// value, and until there is a config for it, it is a flag with no default.
    pub model: Option<String>,
    /// How long the call watches before it degrades to a handle, the same budget
    /// `hydra run` takes.
    pub timeout: Duration,
    /// Read the corpus, print what the curator would be told, launch nothing.
    pub dry_run: bool,
}

/// What one pass did.
pub enum Done {
    /// The corpus was read and nothing was launched.
    Considered { brief: String, selection: Selection },
    /// The run ended inside the call's budget, and this is what came of it.
    Ran {
        result: Box<RunResult>,
        proposal: Proposal,
    },
    /// The run outlived the call. **Nothing is committed and nothing is removed**:
    /// the checkout holds whatever the curator has written so far, and a commit
    /// made by a supervisor that is no longer watching would be a commit of a
    /// half-written proposal.
    Handed {
        handle: Box<Handle>,
        checkout: PathBuf,
        branch: String,
    },
}

/// Where a proposal ended up.
pub struct Proposal {
    pub branch: String,
    /// The commit the pass made, or `None` when the curator wrote nothing.
    pub commit: Option<String>,
    pub checkout_removed: bool,
    /// A branch with nothing on it is litter and goes, which is `git branch -d`'s
    /// decision rather than this module's: it refuses a branch that is not fully
    /// merged, so the one commit a real proposal made is what keeps its branch.
    pub branch_removed: bool,
    /// Why the checkout or the branch is still there, in git's own words.
    pub left: Vec<String>,
}

/// Reads the corpus, runs the curator, commits what it proposed.
pub fn curate(pass: Pass) -> Result<Done> {
    let layout = Layout::discover()?;
    let policies = PolicySet::discover()?;
    // Minted here rather than by the launch, because the checkout the gate scopes
    // to is named after the run and has to exist before the gate is compiled.
    let run_id = Uuid::now_v7().to_string();
    // The path the gate will scope this run's writes to, from the same rule the
    // read-time re-evaluation uses. Asking the layout rather than composing it
    // here is what keeps the checkout and the scope the same directory.
    let checkout = layout.skill_checkout(&run_id);
    let branch = format!("ouroboros/curate/{run_id}");

    let selection = corpus::select(&layout, &policies)?;
    let brief = brief(&run_id, &checkout, &layout, &selection);

    if pass.dry_run {
        return Ok(Done::Considered { brief, selection });
    }

    let store = store(&layout)?;
    // The pass pins before it checks out, and the launch pins again a store this
    // already left clean, so both get one sha. Map ticket 41 chose this over
    // moving the checkout after the launch: the store's only writers are a
    // human and `Store::pin`, so a curate pass committing the store is Hydra
    // doing what the deny list already reserves for it. A checkout of `HEAD`
    // taken before the launch's own commit would read the store as it was
    // before this pass tidied it.
    let sha = Store::open(&layout)?.pin(&run_id)?;
    check_out(&store, &checkout, &branch, &sha)?;

    let call = crate::run::run(Request {
        provider: Provider::Claude,
        task: brief,
        model: pass.model,
        policy: "curate".to_string(),
        floor_lifted: false,
        timeout: pass.timeout,
        // Rung one is started by a human typing `hydra curate`, so its run is
        // depth 0 and bounded by nothing above it (ouroboros spec 3.6).
        parent: None,
        run_id: Some(run_id.clone()),
    });

    let call = match call {
        Ok(call) => call,
        Err(err) => {
            // The run never started, so the checkout holds a clean copy of the
            // store's HEAD and the branch holds nothing. Both are litter, and
            // taking them back is not a removal of anyone's work.
            if let Err(second) = undo_checkout(&store, &checkout, &branch) {
                eprintln!("hydra: {} is still there: {second:#}", checkout.display());
            }
            return Err(err);
        }
    };

    match call {
        Call::Handle(handle) => Ok(Done::Handed {
            handle: Box::new(handle),
            checkout,
            branch,
        }),
        Call::Ended(result) => {
            let proposal = propose(&store, &checkout, &branch, &run_id, &result)?;
            Ok(Done::Ran { result, proposal })
        }
    }
}

/// The skill store, or a refusal naming what is missing.
///
/// Public because the halves of this pass that need no subscription are the ones
/// worth testing, which is why `hydra::run::write_spawn_config` is public too.
///
/// **Ticket 07 is what creates it.** Nothing in this build writes
/// `~/hydra/skills/`, and a curator with nowhere to propose into is a run that
/// spends a subscription to find that out. So the pass refuses before it mints
/// anything, the way a bad `--policy` does.
pub fn store(layout: &Layout) -> Result<PathBuf> {
    let store = layout.skills();
    if !store.is_dir() {
        bail!(
            "there is no skill store at {}, so a curator has nothing to propose into. \
             The store is ticket 07; `hydra curate --dry-run` reads the corpus without it.",
            store.display()
        );
    }
    git(&store, &["rev-parse", "--git-dir"])
        .with_context(|| format!("{} is not a git repository", store.display()))?;
    git(&store, &["rev-parse", "--verify", "HEAD"]).with_context(|| {
        format!(
            "{} has no commits, so there is nothing to branch a proposal from",
            store.display()
        )
    })?;
    Ok(store)
}

/// A git worktree of the store at `rev`, on the branch the proposal will land on.
///
/// **Never the store's own working tree** (3.6). A run pins the store at a sha and
/// links into an immutable snapshot, and a dirty store is committed before that
/// pinning, so a curator writing into the live tree would collide with the next
/// launch's commit. The checkout is not seeded with anything: a proposal is a
/// change against what the store has committed.
///
/// **`rev` is the sha the run will pin**, which is why [`curate`] pins before it
/// calls this. Checking out `HEAD` before the launch commits a dirty store would
/// hand the curator yesterday's store while its record says today's.
pub fn check_out(store: &Path, checkout: &Path, branch: &str, rev: &str) -> Result<()> {
    if let Some(parent) = checkout.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    git(
        store,
        &[
            "worktree",
            "add",
            "-b",
            branch,
            &checkout.to_string_lossy(),
            rev,
        ],
    )
    .with_context(|| format!("checking the skill store out at {}", checkout.display()))?;
    Ok(())
}

/// Takes back a checkout nothing wrote to.
fn undo_checkout(store: &Path, checkout: &Path, branch: &str) -> Result<()> {
    // No `--force`, so a checkout holding changes refuses to go. Nothing has run
    // by the time this is called, but the guarantee should not depend on that.
    git(store, &["worktree", "remove", &checkout.to_string_lossy()])?;
    git(store, &["branch", "-d", branch])?;
    Ok(())
}

/// Commits what the curator left, then takes the checkout back.
///
/// **The order is the guarantee, and git enforces it rather than this function.**
/// `git commit` leaves the checkout clean, and only a clean checkout is one
/// `git worktree remove` will take without `--force`, which this never passes. A
/// commit that did not happen therefore stops the removal instead of preceding it.
/// That is `hydra reap`'s rule, applied to the one other place Hydra removes a
/// directory a model wrote into.
pub fn propose(
    store: &Path,
    checkout: &Path,
    branch: &str,
    run_id: &str,
    result: &RunResult,
) -> Result<Proposal> {
    let mut proposal = Proposal {
        branch: branch.to_string(),
        commit: None,
        checkout_removed: false,
        branch_removed: false,
        left: Vec::new(),
    };

    proposal.commit = commit_everything(checkout, &message(run_id, result))?;

    match git(store, &["worktree", "remove", &checkout.to_string_lossy()]) {
        Ok(_) => proposal.checkout_removed = true,
        Err(err) => proposal.left.push(format!("{err:#}")),
    }

    if proposal.checkout_removed {
        match git(store, &["branch", "-d", branch]) {
            Ok(_) => proposal.branch_removed = true,
            Err(err) => proposal.left.push(format!("{err:#}")),
        }
    }
    Ok(proposal)
}

/// The commit message, which is where everything the pass decided lives.
///
/// **Hydra's facts first, the model's words second and labelled as words.** The
/// curator's rule against taking an intent declaration from a summary afterwards
/// exists because a model's free text stopped matching what it did. This pass
/// makes the commit after the run, so a summary afterwards is all there is; what
/// it can do is keep the two apart, so a reader knows which half is a fact.
fn message(run_id: &str, result: &RunResult) -> String {
    let mut text = format!("hydra curate: a proposal from run {run_id}\n\n");
    text.push_str(
        "Ouroboros rung one proposes and does not land. This branch is the proposal.\n\n",
    );
    text.push_str(&format!("run: {run_id}\n"));
    text.push_str(&format!("record: {}\n", result.run_file));
    text.push_str("policy: curate\n");
    text.push_str(&format!(
        "outcome: {}/{}\n",
        result.outcome.unwrap_or("none"),
        result.outcome_source.unwrap_or("none")
    ));
    if let Some(said) = &result.result_text {
        text.push_str("\nWhat the curator said it did. The diff is what it did.\n\n");
        text.push_str(said.trim());
        text.push('\n');
    }
    text
}

/// What the curator is told, and the whole of it: contract 5.1 stores this
/// verbatim as `run_start.task`, behind the preamble the launch composes.
///
/// **The corpus goes in as a list of run ids rather than as their contents.** Rung
/// one gets the directory (4.3) and reads what it wants through the `corpus`
/// scope, so the brief's job is to say which runs Hydra's own filters left in it
/// and which they took out.
pub fn brief(run_id: &str, checkout: &Path, layout: &Layout, selection: &Selection) -> String {
    let mut text = String::new();
    text.push_str(
        "You are Ouroboros rung one, Hydra's curator. A human typed `hydra curate`. \
         You read run records and propose changes to the skill store.\n\n",
    );

    text.push_str("## What you may write\n\n");
    text.push_str(&format!(
        "{}, and nothing else. It is a git worktree of the skill store at {}, on branch \
         ouroboros/curate/{run_id}. When you exit, Hydra commits whatever you left there \
         onto that branch, and a human merges it or does not. Nothing you write lands on \
         its own, so a proposal that is wrong costs a branch nobody merged.\n\n",
        checkout.display(),
        layout.skills().display(),
    ));
    text.push_str(&format!(
        "Do not write into {} itself. The gate refuses it, and the next run to pin the \
         store commits that working tree first, so a second writer there would collide \
         with a launch.\n\n",
        layout.skills().display()
    ));

    text.push_str("## What you may read\n\n");
    text.push_str(&format!(
        "{}, one file per run, and your own checkout of the store. The `curate` policy \
         grants you no shell and no network, so the file tools are the whole of what you \
         have.\n\n",
        layout.runs().display()
    ));

    text.push_str("## The corpus\n\n");
    text.push_str(&format!(
        "{} run(s) rung one may learn from, most signal first. Contract 8's outcome \
         column and 8.6's harness-run kind are what selected them; the record is on disk \
         in full either way.\n\n",
        selection.learn.len()
    ));
    for run in &selection.learn {
        text.push_str(&line(run));
    }
    if selection.learn.is_empty() {
        text.push_str("  (none, which is a fact about the corpus and not an error)\n");
    }
    text.push('\n');

    let skipped = selection.skipped_counts();
    if !skipped.is_empty() {
        text.push_str(&format!(
            "{} run(s) are not on that list, and none of them is a gap in the record:\n",
            selection.skipped.len()
        ));
        for (why, count) in skipped {
            text.push_str(&format!("  {count}: {why}\n"));
        }
        text.push('\n');
    }

    let classes = selection.classes();
    text.push_str("## The five classes, by frequency\n\n");
    text.push_str(&format!("  abandoned run: {}\n", classes.abandoned));
    text.push_str(&format!(
        "  human rejection: {} (no producer yet: it needs an interactive run, and this \
         build launches none)\n",
        classes.corrections
    ));
    text.push_str(&format!(
        "  repeated tool errors: {} run(s) with at least one failed tool\n",
        classes.tool_errors
    ));
    text.push_str(&format!(
        "  retry loop: {} run(s) whose record names a parent. Every record written \
         before ticket 11 has none\n",
        classes.retries
    ));
    text.push_str(&format!(
        "  clean-looking failed run: {}\n\n",
        classes.clean_looking
    ));

    text.push_str("## What a proposal may and may not rest on\n\n");
    text.push_str(
        "- Name the failure class and the skill each proposal responds to. That name is \
         the measurement: the next pass counts the same signal and sees whether it \
         dropped.\n",
    );
    text.push_str(
        "- A call Hydra's own gate refused is a policy bug rather than a skill bug. The \
         `denied` count on a run's line above is that, and no skill edit follows from \
         it.\n",
    );
    text.push_str(
        "- An `unclassified` tool result is a number to rank on and never one to diagnose \
         from. The adapter says it cannot tell a denial from a missing file on that \
         provider, and guessing produces a curator that rewrites a skill because a file \
         was missing.\n",
    );
    text.push_str(
        "- A signal absent from a whole population is absent because that population has \
         no producer for it, not because nothing happened. The counts above say which.\n",
    );
    text.push_str(
        "- Scope every edit to a skill the loop itself created. A skill a human wrote is \
         the human's, and a pinned skill is never touched.\n",
    );
    text.push_str(
        "- Check that anything you claim to merge into exists, and rewrite the references \
         to any skill you rename or consolidate. A lookup by name that misses silently is \
         a duty that quietly stops firing.\n\n",
    );

    text.push_str("## What to leave behind\n\n");
    text.push_str(
        "Write your proposal into the checkout above and say, in the files you write, \
         which failure class and which run ids each change answers. Follow the store's own \
         layout rather than inventing one. If the corpus does not support a change, write \
         nothing: an empty pass is a real answer and costs a branch that is deleted \
         rather than merged.\n",
    );
    text
}

/// One run, as a line the curator can act on.
fn line(run: &Learnable) -> String {
    let mut text = format!(
        "  {}  {}  {}/{}  policy={}  calls={} failed={} unclassified={} denied={}",
        run.run_id,
        run.provider,
        run.outcome_name(),
        run.source_name(),
        run.policy.as_deref().unwrap_or("none"),
        run.signals.tool_calls,
        run.signals.failed_tools,
        run.signals.unclassified_tools,
        run.signals.denied_by_hydra,
    );
    if run.signals.abandoned {
        text.push_str("  [abandoned: no run_end and the pid is gone]");
    }
    if run.signals.clean_looking_failure(run.outcome.as_ref()) {
        text.push_str("  [ended clean with a failure inside it]");
    }
    text.push('\n');
    for said in &run.signals.disagreements {
        text.push_str(&format!(
            "      the record and the policy disagree: {said}\n"
        ));
    }
    for said in &run.signals.limits {
        text.push_str(&format!("      what this record cannot say: {said}\n"));
    }
    text
}

/// What a human reads back.
pub fn report(done: &Done) -> String {
    match done {
        Done::Considered { brief, selection } => {
            format!(
                "{brief}\n--\n{} run(s) to learn from, {} skipped. Nothing was launched.\n",
                selection.learn.len(),
                selection.skipped.len()
            )
        }
        Done::Ran { result, proposal } => {
            let mut text = format!(
                "curated in run {}\n  outcome {}/{}\n  record {}\n",
                result.run_id,
                result.outcome.unwrap_or("none"),
                result.outcome_source.unwrap_or("none"),
                result.run_file,
            );
            match &proposal.commit {
                Some(sha) => text.push_str(&format!(
                    "  proposed {} on {}\n  merge it or do not; nothing here will\n",
                    &sha[..12.min(sha.len())],
                    proposal.branch
                )),
                None => text.push_str("  the curator wrote nothing, so there is no proposal\n"),
            }
            if proposal.branch_removed {
                text.push_str(&format!("  branch {} removed\n", proposal.branch));
            }
            for left in &proposal.left {
                text.push_str(&format!("  left behind: {left}\n"));
            }
            text
        }
        Done::Handed {
            handle,
            checkout,
            branch,
        } => format!(
            "run {} outlived the call and is still going\n  watch it with `{}`\n  \
             nothing is committed: what it has written so far is in {}, and {} is where \
             a commit would go\n",
            handle.run_id,
            handle.follow,
            checkout.display(),
            branch,
        ),
    }
}
