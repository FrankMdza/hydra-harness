//! `hydra stop`, build ticket 169: a human ends a run that is still going.
//!
//! **Typed by a human and reached from nowhere but `main`**, like `reap` and
//! `prune`. The panel of 2026-09-30 chose a command over a watchdog, so D10
//! stands: nothing in Hydra decides on its own that a run has gone on too long.
//! [`command`] also refuses inside a seatbelt, as `hydra approve` does, because
//! a model that can type `hydra stop --all` can end every sibling it has.
//!
//! **The watcher still writes the record.** This module never touches a run
//! file. It writes a marker in the run's spawn directory, then signals the
//! child: SIGTERM to its process group, and SIGKILL after [`GRACE`] if the
//! child is still there. The child exits, the watcher (or, for an interactive
//! session, the harvest) closes the record as it always does, finds the marker
//! and puts [`crate::watch::STOP_NOTE`] first in `run_end.error`. Map ticket 89
//! made that note its own outcome, `stopped`.
//!
//! **A marker rather than a signal to the watcher.** The watcher sits in a
//! blocking tee and has no handler to install, and the record's pid is the
//! child's, not the watcher's, so a signal would need a second pid nobody
//! records. The spawn directory is already the one path the watcher knows, and
//! every policy denies writes under it, so a run cannot mark itself or a
//! sibling. The cost is one race: a child that exits on its own between the
//! marker and the signal is recorded as stopped.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::live::{self, RunView, State};
use crate::outcome::Outcome;
use crate::paths::Layout;
use crate::policy::PolicySet;

/// How long a child has to exit on SIGTERM before it gets SIGKILL.
pub const GRACE: Duration = Duration::from_secs(10);

/// How long the record has to close once the child is gone. The watcher
/// waits up to [`crate::pty::PTY_EOF_DEADLINE`] for the pty and then writes one
/// line, so this is generous on purpose.
pub const CLOSE_BUDGET: Duration = Duration::from_secs(30);

/// `hydra stop` was typed inside a seatbelt.
pub const INSIDE_SEATBELT: &str = "hydra/stop-inside-seatbelt";

const POLL: Duration = Duration::from_millis(100);

/// What to stop.
#[derive(Clone, Copy)]
pub enum Target<'a> {
    /// One run, by full id or unambiguous prefix.
    Run(&'a str),
    /// Every live run.
    All,
    /// A session and every live run whose `parent_run_id` is it.
    Session(&'a str),
}

/// One run `hydra stop` signalled.
#[derive(Debug)]
pub struct Stopped {
    pub run_id: String,
    /// Whether the child outlived [`GRACE`] and got SIGKILL.
    pub killed: bool,
    /// What the record derives now, or `None` when it did not close within
    /// [`CLOSE_BUDGET`] or would not read.
    pub outcome: Option<Outcome>,
}

/// What `hydra stop` does, typed by a human. Refused inside a seatbelt.
pub fn command(layout: &Layout, target: Target<'_>) -> Result<String> {
    if crate::sandbox::inside_seatbelt() {
        bail!(
            "{INSIDE_SEATBELT}: hydra stop runs in a terminal outside every session. This \
             process is inside a seatbelt, which is where a model runs, and no model stops a \
             run. Nothing was signalled."
        );
    }
    let policies = PolicySet::discover()?;
    Ok(report(&stop(layout, &policies, target, GRACE)?))
}

/// Stops what `target` names and waits for each record to close.
///
/// Every marker is written before any signal, so a session's seat launcher
/// refuses new seats from the first moment of the stop, and children are
/// signalled before the runs that launched them.
pub fn stop(
    layout: &Layout,
    policies: &PolicySet,
    target: Target<'_>,
    grace: Duration,
) -> Result<Vec<Stopped>> {
    let views = live::scan(layout)?;
    let mut order: Vec<RunView> = match target {
        Target::Run(prefix) => {
            let run_id = crate::resolver::resolve(layout, prefix)?;
            vec![live_one(&views, &run_id)?]
        }
        Target::All => views.into_iter().filter(RunView::is_live).collect(),
        Target::Session(prefix) => {
            let run_id = crate::resolver::resolve(layout, prefix)?;
            let session = live_one(&views, &run_id)?;
            mark(layout, &session.run_id)?;
            // Scanned again after the marker, so a seat whose record appeared
            // between the first scan and the marker is not left running.
            let mut order = children_of(&live::scan(layout)?, &session.run_id);
            order.push(session);
            order
        }
    };
    if matches!(target, Target::All) {
        // Deepest first, so no parent goes before a run it launched.
        order.sort_by_key(|view| std::cmp::Reverse(depth(view)));
    }

    for view in &order {
        mark(layout, &view.run_id)?;
    }
    let mut stopped = Vec::with_capacity(order.len());
    for view in &order {
        stopped.push(stop_one(layout, policies, view, grace)?);
    }
    Ok(stopped)
}

/// One line per run stopped, with the outcome its record derives.
pub fn report(stopped: &[Stopped]) -> String {
    if stopped.is_empty() {
        return "no live run to stop\n".to_string();
    }
    let mut out = String::new();
    for run in stopped {
        let outcome = match run.outcome {
            Some(outcome) => outcome.as_str().to_string(),
            None => format!(
                "the record did not close within {}s; `hydra status {}` reads it later",
                CLOSE_BUDGET.as_secs(),
                run.run_id
            ),
        };
        let killed = if run.killed {
            format!(" (SIGKILL after {}s)", GRACE.as_secs())
        } else {
            String::new()
        };
        out.push_str(&format!("stopped {}{killed}: {outcome}\n", run.run_id));
    }
    out
}

/// The live run `run_id` names, or the refusal that says why not.
fn live_one(views: &[RunView], run_id: &str) -> Result<RunView> {
    let Some(view) = views.iter().find(|view| view.run_id == run_id) else {
        bail!("no record for run {run_id}, so there is nothing to stop");
    };
    if view.state != State::Live {
        bail!(
            "run {run_id} is {}, not live, so there is nothing to stop",
            view.state.as_str()
        );
    }
    Ok(view.clone())
}

fn children_of(views: &[RunView], parent: &str) -> Vec<RunView> {
    views
        .iter()
        .filter(|view| view.is_live() && view.parent_run_id.as_deref() == Some(parent))
        .cloned()
        .collect()
}

fn depth(view: &RunView) -> u64 {
    live::opening_line(&view.run_file)
        .ok()
        .flatten()
        .and_then(|body| body["depth"].as_u64())
        .unwrap_or(0)
}

/// Writes the marker the watcher and the harvest read when the record closes.
fn mark(layout: &Layout, run_id: &str) -> Result<()> {
    let marker = layout.stop_marker(run_id);
    if let Some(dir) = marker.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(&marker, format!("{}\n", Utc::now().to_rfc3339()))
        .with_context(|| format!("writing {}", marker.display()))
}

fn stop_one(
    layout: &Layout,
    policies: &PolicySet,
    view: &RunView,
    grace: Duration,
) -> Result<Stopped> {
    let mut killed = false;
    if let (Some(pid), Some(start)) = (view.pid, view.process_start_time) {
        signal(pid, start, libc::SIGTERM)?;
        if !wait_for(grace, || Ok(!crate::procinfo::is_alive(pid, start)?))? {
            killed = signal(pid, start, libc::SIGKILL)?;
        }
    }
    let closed = wait_for(CLOSE_BUDGET, || live::has_run_end(&view.run_file))?;
    let outcome = if closed {
        std::fs::read_to_string(&view.run_file)
            .ok()
            .and_then(|text| crate::normalize::read_with(&text, policies, layout).ok())
            .and_then(|run| crate::outcome::derive(&run))
            .map(|derived| derived.outcome)
    } else {
        None
    };
    Ok(Stopped {
        run_id: view.run_id.clone(),
        killed,
        outcome,
    })
}

/// Sends `signal` to the child's process group when it leads one, and to the
/// child alone when it does not. A recorded run's child leads its own session
/// on the pty; an interactive session's `claude` shares the human's job, and
/// signalling that group would reach the shell's other processes.
///
/// Returns whether anything was signalled. The pid is checked against the
/// recorded start time first, so a pid the kernel reused is left alone.
fn signal(pid: u32, start: DateTime<Utc>, signal: libc::c_int) -> Result<bool> {
    if !crate::procinfo::is_alive(pid, start)? {
        return Ok(false);
    }
    let pid = libc::pid_t::try_from(pid).context("a pid out of range")?;
    // SAFETY: getpgid, killpg and kill take plain integers and no pointer.
    let rc = unsafe {
        if libc::getpgid(pid) == pid {
            libc::killpg(pid, signal)
        } else {
            libc::kill(pid, signal)
        }
    };
    if rc == -1 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(false);
        }
        return Err(err).with_context(|| format!("signalling pid {pid}"));
    }
    Ok(true)
}

/// Polls `done` until it answers true or `budget` runs out.
fn wait_for(budget: Duration, mut done: impl FnMut() -> Result<bool>) -> Result<bool> {
    let deadline = Instant::now() + budget;
    loop {
        if done()? {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(POLL);
    }
}
