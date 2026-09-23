//! `hydra status`, harness spec 10 and 17.
//!
//! It reconciles live pids against open run files and reports the worktrees left
//! behind. **It appends to no run file**, not even one it finds abandoned:
//! contract 6 gives a file one writer at a time and ownership transfers only to a
//! process that takes over the writing, which this never does. The one thing it
//! does write is the diff stat's scratch index in the run's own spawn directory,
//! the same file `hydra run` stages through, and that is a git index rather than
//! anything a reader of the record can see.
//!
//! **The worked scenario of harness spec 10 is what this command is for.** You
//! close the Ghostty window on a session with two children in flight. Hydra dies,
//! the children keep running because they were never in that terminal's session,
//! and the next `hydra status` derives all of it from the pids in the files:
//! which runs are still going, which ones ended without anyone writing their
//! closing line, and which worktrees are sitting there waiting for a `hydra reap`
//! you type.
//!
//! **Harvest is here too, now.** Section 10 has `hydra status` complete a dead
//! interactive run from the vendor's transcript, "any process can do it." An
//! abandoned run whose envelope names `claude` and whose `launch_mode` is
//! `interactive` gets one harvest attempt before this command reads it, so a
//! session left running behind a closed terminal is completed rather than
//! merely reported.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::live::{RunView, State};
use crate::paths::Layout;
use crate::policy::PolicySet;

/// Everything under `~/hydra/runs/`, grouped, plus the worktrees nothing is using.
pub fn all(layout: &Layout) -> Result<String> {
    let home = crate::env::real_home()?;
    all_in(layout, &home)
}

/// [`all`], against the home directory named rather than discovered.
///
/// **The one caller that needs this is a test.** [`crate::env::real_home`]
/// reads the passwd entry and ignores `HOME`, so a suite proving that this
/// command harvests a dead interactive run has to name a scratch directory
/// rather than let the harvest reach into the developer's own
/// `~/.claude/projects/`. `tests/status.rs` is the caller.
pub fn all_in(layout: &Layout, home: &Path) -> Result<String> {
    let policies = PolicySet::discover()?;
    let mut views = crate::live::scan(layout)?;
    if harvest_abandoned_interactive(layout, &views, home) {
        views = crate::live::scan(layout)?;
    }
    let mut out = String::new();

    let live: Vec<&RunView> = views.iter().filter(|v| v.state == State::Live).collect();
    let abandoned: Vec<&RunView> = views
        .iter()
        .filter(|v| v.state == State::Abandoned)
        .collect();
    let ended = views.iter().filter(|v| v.state == State::Ended).count();

    // The root, before any count. A checkout that is not `~/hydra` still reads
    // and writes `~/hydra`, and a newcomer running `status` in a clone was
    // shown 197 runs that were not theirs with nothing saying whose they were.
    out.push_str(&format!("root {}\n", layout.root().display()));
    out.push_str(&format!("policies {}\n", policies.dir().display()));
    out.push_str(&format!("live {}\n", live.len()));
    for view in &live {
        out.push_str(&format!("  {}\n", one_line(view)));
    }

    out.push_str(&format!("abandoned {}\n", abandoned.len()));
    for view in &abandoned {
        // A missing `run_end` is a fact rather than a corruption (contract 6). The
        // pid went into the file before the child started, which is the only
        // reason this state is derivable at all.
        out.push_str(&format!("  {}\n", one_line(view)));
    }

    out.push_str(&format!("ended {ended}\n"));

    let stale = abandoned_worktrees(layout, &views, crate::run::LAUNCH_BUDGET)?;
    out.push_str(&format!("worktrees waiting for a reap {}\n", stale.len()));
    for tree in &stale {
        out.push_str(&format!("  {}\n", tree.display()));
    }

    Ok(out)
}

/// Whether a view is a dead interactive `claude` run this build knows how to
/// harvest: an abandoned pid, a `claude` envelope, `launch_mode: interactive`.
fn is_abandoned_interactive_claude(view: &RunView) -> bool {
    view.state == State::Abandoned
        && view.provider.as_deref() == Some("claude")
        && view.launch_mode.as_deref() == Some("interactive")
}

/// Harvests every dead interactive `claude` run this scan found. Returns
/// whether anything changed, so the caller rescans only when it has to.
fn harvest_abandoned_interactive(layout: &Layout, views: &[RunView], home: &Path) -> bool {
    let mut any = false;
    for view in views {
        if !is_abandoned_interactive_claude(view) {
            continue;
        }
        match crate::interactive::harvest_in(layout, &view.run_id, None, home) {
            Ok(_) => any = true,
            Err(err) => eprintln!("hydra: could not harvest {}: {err:#}", view.run_id),
        }
    }
    any
}

/// One run, as JSON.
///
/// **The projection is the same one `hydra run` returns**, off the same function,
/// on a run this process did not launch. That is the point of contract 17 being a
/// projection of the record rather than a summary of a launch.
pub fn one(layout: &Layout, run_id: &str) -> Result<String> {
    let home = crate::env::real_home()?;
    one_in(layout, run_id, &home)
}

/// [`one`], against the home directory named rather than discovered. See
/// [`all_in`]'s doc comment for why a test needs this.
pub fn one_in(layout: &Layout, run_id: &str, home: &Path) -> Result<String> {
    let report = build(layout, run_id, home)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

/// Contract 17's projection alone, the object this command prints under
/// `result`. `hydra show <id>` prints exactly this, with no `--events`.
pub fn projection(layout: &Layout, run_id: &str) -> Result<serde_json::Value> {
    let home = crate::env::real_home()?;
    let report = build(layout, run_id, &home)?;
    Ok(serde_json::to_value(report.result)?)
}

/// The full report `one` prints, built once so `projection` can take the
/// field it wants rather than parsing the string back apart.
fn build(layout: &Layout, run_id: &str, home: &Path) -> Result<Report> {
    let path = layout.run_file(run_id);
    if !path.is_file() {
        bail!("no run record at {}", path.display());
    }
    let mut view = crate::live::read_view(&path)?;
    if is_abandoned_interactive_claude(&view) {
        match crate::interactive::harvest_in(layout, run_id, None, home) {
            Ok(_) => view = crate::live::read_view(&path)?,
            Err(err) => eprintln!("hydra: could not harvest {run_id}: {err:#}"),
        }
    }

    let worktree_present = view
        .worktree
        .as_ref()
        .map(|w| std::path::Path::new(&w.path).is_dir())
        .unwrap_or(false);

    // Only an ended run has an outcome. Contract 8.6 gives telling a live child
    // from an abandoned one to this command precisely because `derive` cannot: a
    // file with no closing line has no ending to read, and printing a page of
    // nulls beside `state` would invite a reader to take them as measurements.
    //
    // **A record the normalizer chokes on costs the projection and not the
    // command.** `hydra run` already degrades that way, for the reason this
    // module's own scan gives: a status command that refuses to report a damaged
    // record hides the run most worth looking at. `state`, `pid` and `run_file`
    // are off line 0 and are the fields that lead a human to the file.
    let result = match view.state {
        State::Ended => {
            let read = PolicySet::discover()
                .and_then(|policies| crate::normalize::read_file(&path, &policies));
            match read {
                Ok(run) => {
                    // The diff stat needs the tree the seed left, and the sha rode
                    // on a `Worktree` that lived in another process. The seed index
                    // still holds it.
                    let base = view.worktree.as_ref().and_then(|w| {
                        crate::worktree::CheckoutGit::for_run(
                            layout,
                            &view.run_id,
                            std::path::Path::new(&w.path),
                            run.run_start().map(|start| &start.body),
                        )
                        .and_then(|context| {
                            crate::worktree::tree_of_index(
                                &context,
                                &layout.seed_index(&view.run_id),
                            )
                        })
                        .ok()
                    });
                    Some(crate::run::result(
                        &run,
                        &path,
                        view.worktree.as_ref(),
                        base.as_deref(),
                        layout,
                    ))
                }
                Err(err) => {
                    eprintln!("hydra: {} would not read back: {err:#}", path.display());
                    None
                }
            }
        }
        State::Live | State::Abandoned => None,
    };

    let report = Report {
        run_id: view.run_id.clone(),
        state: view.state.as_str(),
        pid: view.pid,
        process_start_time: view
            .process_start_time
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)),
        started_at: view
            .started_at
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
        provider: view.provider.clone(),
        policy: view.policy.clone(),
        branch: view.worktree.as_ref().map(|w| w.branch.clone()),
        worktree: view.worktree.as_ref().map(|w| w.path.clone()),
        worktree_present,
        run_file: path.display().to_string(),
        result,
    };
    Ok(report)
}

/// What `hydra status <run-id>` prints.
#[derive(Serialize)]
struct Report {
    run_id: String,
    /// `live`, `abandoned` or `ended`, derived from contract 5.2's pair.
    state: &'static str,
    pid: Option<u32>,
    process_start_time: Option<String>,
    started_at: Option<String>,
    provider: Option<String>,
    policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree: Option<String>,
    /// Whether the checkout is still on disk. False after a reap, which is the
    /// intended end state rather than a loss: the record is the source of truth
    /// and the `tool_call` events are what survive the branch.
    worktree_present: bool,
    run_file: String,
    /// Contract 17's projection, on an ended run only.
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<crate::outcome::RunResult>,
}

/// Every checkout under `~/hydra/worktrees/` that no live run is using.
///
/// **A worktree outliving its run is the normal case, not a fault.** Nothing is
/// cleaned up automatically (harness spec 10), so this is a list of things
/// `hydra reap` has to do rather than a list of problems. A directory whose run
/// file has gone is on it too: a pruned record leaves a checkout nobody can
/// explain, and that is exactly what a human wants told.
///
/// **A launch in progress is the one case that has to be kept off it.**
/// `hydra run` creates the checkout and seeds it with the parent's uncommitted
/// work before the detached half writes line 0, and on a large repo that is
/// seconds rather than microseconds. In that window the directory exists and
/// nothing that has spoken claims it, which reads exactly like a pruned run. The
/// record may be there and empty, since the detached half opens the file before
/// it spawns the child, and [`crate::live::RunView::launched`] is the test for
/// that. What tells them apart
/// is age, and `grace` is the line: a checkout with no record that is younger than
/// that is one being made right now. The command passes
/// [`crate::run::LAUNCH_BUDGET`], which is the same number the launcher waits for
/// line 0 with, so the two cannot disagree. A filesystem that will not give a
/// creation time keeps the directory on the list, because "report it" is the safe
/// end of a reporting command and reap is a human's decision either way.
pub fn abandoned_worktrees(
    layout: &Layout,
    views: &[RunView],
    grace: std::time::Duration,
) -> Result<Vec<std::path::PathBuf>> {
    let dir = layout.worktrees();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", dir.display())),
    };

    let mut stale: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            match views.iter().find(|view| view.run_id == name) {
                Some(view) if view.state == State::Live => false,
                // A record that has spoken: it carries a pid, or it has ended.
                Some(view) if view.launched() => true,
                // Everything else is judged on age, and a record that exists but
                // says nothing is on that side rather than this one. `hydra run`
                // creates the run file before the child exists, so during a launch
                // there is an empty record reading as abandoned beside a checkout
                // being seeded, and reading the empty file as proof the run is over
                // is how a reaper removes the checkout of a run that is starting.
                _ => !just_created(path, grace),
            }
        })
        .collect();
    stale.sort();
    Ok(stale)
}

/// Whether a directory appeared within `grace`.
pub(crate) fn just_created(path: &std::path::Path, grace: std::time::Duration) -> bool {
    let Ok(created) = path.metadata().and_then(|meta| meta.created()) else {
        return false;
    };
    created.elapsed().is_ok_and(|age| age < grace)
}

fn one_line(view: &RunView) -> String {
    let mut line = view.run_id.clone();
    match view.pid {
        Some(pid) if view.state == State::Live => line.push_str(&format!("  pid {pid}")),
        Some(pid) => line.push_str(&format!("  pid {pid} is gone")),
        None => line.push_str("  no process"),
    }
    if let Some(policy) = &view.policy {
        line.push_str(&format!("  {policy}"));
    }
    if let Some(worktree) = &view.worktree {
        line.push_str(&format!("  {}", worktree.branch));
    }
    if let Some(started) = view.started_at {
        line.push_str(&format!(
            "  started {}",
            started.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ));
    }
    line
}
