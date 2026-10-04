//! A review the cockpit starts, build ticket 183.

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, ExitStatus, Stdio};

use anyhow::{Context, bail};

use super::text::hunk_line;
use super::{App, Effect, Focus};
use crate::cockpit::goal::Goal;

/// A review this cockpit started, build ticket 183: `hydra-puppets review <goal>
/// <task>` in its own process group, both streams in `log`.
pub(super) struct ReviewRun {
    pub(super) goal: String,
    pub(super) task: String,
    log: PathBuf,
    /// How many reviews of the task the goal log held when it started, so
    /// an older verdict is not read as this one's.
    before: usize,
    /// The helper while it lives. [`App::poll_reviews`] reaps it.
    child: Option<Child>,
    /// How it exited, or `None` while it lives or when the wait failed.
    status: Option<ExitStatus>,
    /// The `hunk show ...` line the helper printed to its log, build ticket
    /// 187: `--no-open` prints the command rather than running it.
    open: Option<String>,
}

impl App {
    /// `v`: opens the confirmation for the selected task's review, which
    /// names what it costs.
    pub(super) fn ask_to_review(&mut self) -> Effect {
        if let Some(message) = self.goal_damaged_message() {
            self.message = Some(message);
            return Effect::None;
        }
        let row = match (&self.snapshot.goal, self.focus) {
            (Some(goal), Focus::Table) => goal.rows.get(self.table_selected).map(|row| (goal, row)),
            _ => None,
        };
        let Some((goal, row)) = row else {
            self.message = Some("select a task row first".to_string());
            return Effect::None;
        };
        if row.run_id.is_none() {
            self.message = Some(format!("task {} has no run to review", row.id));
            return Effect::None;
        }
        let asked = (goal.id.clone(), row.id.clone(), row.run_id.clone().unwrap());
        self.poll_reviews();
        if self.review_running(&asked.0, &asked.1) {
            self.message = Some(format!("a review of {} is already running", asked.1));
            return Effect::None;
        }
        self.confirming_review = Some(asked);
        Effect::None
    }

    /// `y` on that confirmation: starts the helper, or says why it did not.
    pub(super) fn start_review(&mut self) -> Effect {
        let Some((goal, task, run_id)) = self.confirming_review.take() else {
            return Effect::None;
        };
        if let Some(message) = self.goal_damaged_message() {
            self.message = Some(message);
            return Effect::None;
        }
        let current_run = self
            .snapshot
            .goal
            .as_ref()
            .filter(|shown| shown.id == goal)
            .and_then(|shown| shown.rows.iter().find(|row| row.id == task))
            .and_then(|row| row.run_id.as_deref());
        if current_run != Some(run_id.as_str()) {
            self.message = Some(format!(
                "task {task} run changed; review confirmation cancelled"
            ));
            return Effect::None;
        }
        self.poll_reviews();
        if self.review_running(&goal, &task) {
            self.message = Some(format!("a review of {task} is already running"));
            return Effect::None;
        }
        match self.spawn_review(&goal, &task) {
            Ok(run) => {
                self.message = Some(format!(
                    "a review of {task} started, its output in {}",
                    run.log.display()
                ));
                self.reviews.retain(|r| !(r.goal == goal && r.task == task));
                self.reviews.push(run);
            }
            Err(err) => {
                self.message = Some(format!("the review of {task} did not start: {err:#}"));
            }
        }
        Effect::None
    }

    /// `hydra-puppets review <goal> <task> --no-open` in a process group of its
    /// own, so a `Ctrl-C` in the cockpit's terminal does not reach it, with
    /// stdin closed and both streams in `$HYDRA_ROOT/cockpit/review-<task>.log`.
    ///
    /// `--no-open`, build ticket 187: the helper has no terminal to open Hunk
    /// in, so it prints the `hunk show ...` command and exits, and the run's
    /// detail shows that line for the human to run. Inside Herdr it starts
    /// the same way and opens no pane.
    fn spawn_review(&self, goal: &str, task: &str) -> anyhow::Result<ReviewRun> {
        if let Some(note) = &self.no_helper {
            bail!("{note}");
        }
        let Some((script, home)) = &self.reviewer else {
            bail!("no review helper is set");
        };
        super::super::check_goal_id(goal)?;
        if !super::super::plain_name(task) {
            bail!("{task:?} cannot name a log file");
        }
        let dir = self.layout.root().join("cockpit");
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let log = dir.join(format!("review-{task}.log"));
        let out =
            std::fs::File::create(&log).with_context(|| format!("creating {}", log.display()))?;
        let err = out.try_clone().context("opening the log twice")?;
        let child = super::super::helper_command(script)?
            .arg("review")
            .arg(goal)
            .arg(task)
            .arg("--no-open")
            .env("HYDRA_ROOT", self.layout.root())
            .env("HYDRA_PUPPETS_HOME", home)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .process_group(0)
            .spawn()
            .with_context(|| format!("starting {}", script.display()))?;
        let before = self
            .snapshot
            .goal
            .as_ref()
            .map_or(0, |g| g.reviews_of(task).0);
        Ok(ReviewRun {
            goal: goal.to_string(),
            task: task.to_string(),
            log,
            before,
            child: Some(child),
            status: None,
            open: None,
        })
    }

    /// Reaps each helper that has exited, `waitpid` with `WNOHANG` through
    /// [`Child::try_wait`], and says so once. [`App::draw`] calls it, and the
    /// loop draws at least every tenth of a second, so `review running` is
    /// gone well inside the four seconds the panel asked for.
    pub fn poll_reviews(&mut self) {
        let mut said = None;
        for run in &mut self.reviews {
            let Some(child) = &mut run.child else {
                continue;
            };
            match child.try_wait() {
                Ok(None) => continue,
                Ok(Some(status)) => run.status = Some(status),
                Err(_) => run.status = None,
            }
            run.child = None;
            if run.status.is_some_and(|status| status.success()) {
                run.open = hunk_line(&run.log);
            }
            said = Some(match run.status {
                Some(status) if status.success() => format!(
                    "the review of {} ended; its verdict shows at the next read",
                    run.task
                ),
                _ => format!("review failed, see {}", run.log.display()),
            });
        }
        if said.is_some() {
            self.message = said;
        }
    }

    /// The `hunk show ...` command the review of `task` this cockpit started
    /// printed, once it ended.
    pub(super) fn review_open(&self, goal: &str, task: &str) -> Option<&str> {
        self.reviews
            .iter()
            .find(|r| r.goal == goal && r.task == task)
            .and_then(|r| r.open.as_deref())
    }

    fn review_running(&self, goal: &str, task: &str) -> bool {
        self.reviews
            .iter()
            .any(|r| r.goal == goal && r.task == task && r.child.is_some())
    }

    /// What the table cell and the detail say about `task`'s review: the one
    /// this cockpit started while it runs or when it failed, and otherwise the
    /// last verdict in the goal log. `None` when there is neither.
    pub(super) fn review_of(&self, goal: &Goal, task: &str) -> Option<(&'static str, String)> {
        let (count, last) = goal.reviews_of(task);
        let ours = self
            .reviews
            .iter()
            .find(|r| r.goal == goal.id && r.task == task);
        if let Some(run) = ours {
            let log = run.log.display();
            if run.child.is_some() {
                return Some(("review running", format!("running, output in {log}")));
            }
            // A helper that failed after it recorded a verdict still gave one.
            if count <= run.before {
                return Some(match run.status {
                    Some(status) if status.success() => (
                        "review ended",
                        format!("ended, no verdict read yet, see {log}"),
                    ),
                    _ => ("review failed", format!("review failed, see {log}")),
                });
            }
        }
        let last = last?;
        let verdict = last.verdict.as_deref().unwrap_or("without a verdict");
        let cell = match verdict {
            "pass" => "review pass",
            "block" => "review block",
            _ => "reviewed",
        };
        let degraded = if last.degraded {
            " by a degraded panel"
        } else {
            ""
        };
        Some((cell, format!("{verdict}{degraded}")))
    }
}
