//! The detached half of a run, harness spec 9 and 10.
//!
//! **Every child is spawned detached, and blocking is a view over it.** So the
//! process that owns the pty, tees the frames and writes the closing line is not
//! the process the human is talking to. `hydra run` composes the launch, writes it
//! to the spawn directory, starts this half in a session of its own and then tails
//! the run file like any other reader. Closing the terminal takes the view and
//! leaves the run.
//!
//! **The watcher is a dumb executor and that is the point.** Everything
//! provider-specific happens before it starts: the argv, the gate, the worktree and
//! the whole of line 0 are composed by `hydra run` and land in the spec. What is
//! left here is "run this argv on a pty, tee it into this file, close it when the
//! process exits", which is why a test can drive it with `/bin/sh` and spend no
//! subscription.
//!
//! **The spec holds no inherited environment values.** The watcher rebuilds the
//! child's environment from the allowlist out of what it inherited, because
//! `HTTPS_PROXY` can carry a password and contract 5.3 keeps values out of the
//! record for the same reason. Names go in the record; values never touch the
//! disk. The one exception is `hydra_env`, values Hydra computed itself. What
//! the watcher inherits is already the allowlist, because [`detach`] starts it
//! from the child's environment and not from the launcher's.
//!
//! **Reading a spec is not reading a record.** Harness spec 5 requires that
//! interpreting a run never need a spawn artifact, and nothing here interprets:
//! the spec is consumed at launch, before the record exists, and everything it
//! carries is copied onto line 0.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, bail};
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};

use crate::env::ChildEnv;
use crate::provider::Provider;
use crate::record::{RunEnd, Writer};

/// What `hydra run` hands the detached half.
#[derive(Serialize, Deserialize)]
pub struct Launch {
    pub run_id: String,
    /// The run that asked for this one, contract 4. It goes on every line of the
    /// child's record and is never reconciled against what the child says about
    /// itself. `None` on a run a human started.
    #[serde(default)]
    pub parent_run_id: Option<String>,
    /// The provider name, for the envelope and for [`Provider::observe`].
    pub provider: String,
    /// The binary to exec. Separate from `provider` so the suite can drive this
    /// half with a stub and never spend a subscription.
    pub binary: String,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub run_file: PathBuf,
    /// What Hydra passed on `--model`, which the envelope carries until the
    /// provider says which model actually answered.
    pub model: Option<String>,
    /// Variables Hydra sets for its own purposes, 6.2's second category, by
    /// name and value. **Never an inherited value**: the rule that keeps
    /// secrets off this file is about what the launcher's own environment
    /// carries, and these are values Hydra computed: the containment marker
    /// `HYDRA_RUN_ID` on every run, and `CURSOR_DATA_DIR` on `cursor-agent`.
    /// Anything the launcher sets on the child and leaves off this map never
    /// reaches the child, because [`watch`] rebuilds the allowlist.
    #[serde(default)]
    pub hydra_env: std::collections::BTreeMap<String, String>,
    /// The whole of line 0's body, composed by `hydra run`, with `pid` and
    /// `process_start_time` null. Those two are the only fields this half fills
    /// in, because they do not exist until the process does.
    pub run_start: serde_json::Value,
    /// The seatbelt profile on macOS, consumed at spawn and not copied to the
    /// record beyond the sha on line 0.
    #[serde(default)]
    pub sandbox_profile: Option<PathBuf>,
    /// Where the pinned snapshot lives, for comparing a `codex` skill copy at
    /// exit. Absent on every other provider and on runs with no injected skills.
    #[serde(default)]
    pub skill_snapshot: Option<PathBuf>,
}

pub fn write_spec(path: &Path, launch: &Launch) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(launch)?)
        .with_context(|| format!("writing {}", path.display()))
}

/// Starts the detached half in a session of its own.
///
/// **`setsid` is what makes the run outlive the terminal.** Closing a Ghostty
/// window sends `SIGHUP` to the foreground process group of its tty; a process in
/// its own session has no controlling terminal to be hung up on. Harness spec 10's
/// worked scenario is exactly this: Hydra dies, the children keep running and keep
/// writing their own run files.
///
/// The three standard descriptors go to the log rather than to the tty, for the
/// same reason. A watcher holding the terminal open would keep the shell's job
/// control waiting on a run nobody is watching any more.
/// `exe` is the Hydra binary to re-exec, which `hydra run` reads off itself.
/// It is a parameter rather than a `current_exe` call inside, because "this
/// process is hydra" is true of the launcher and not of every caller: the suite
/// drives this from a test binary.
///
/// **The watcher starts from `env` and nothing else.** It lives outside the
/// seatbelt for the whole run, and on macOS any same-uid process reads its
/// environment with `ps -E`, so a watcher that inherited the launching shell
/// held `GEMINI_API_KEY` for as long as the child ran. The third hiring panel
/// measured it, H3. `env` is the child's own environment, the allowlist plus
/// what Hydra set, because that is the least the watcher can rebuild the
/// child's from, and it needs nothing else: every path it reads is on the spec.
pub fn detach(exe: &Path, spec: &Path, log: &Path, env: &ChildEnv) -> Result<Child> {
    use std::os::unix::process::CommandExt;

    let out = std::fs::File::create(log).with_context(|| format!("creating {}", log.display()))?;
    let err = out.try_clone().context("cloning the watcher log handle")?;

    let mut command = Command::new(exe);
    command
        .arg("watch")
        .arg(spec)
        // Nothing here reads a relative path, and a watcher holding the human's
        // directory would keep a removed worktree alive on the filesystem.
        .current_dir(std::path::Path::new("/"))
        .env_clear()
        .envs(env.vars())
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err);

    // SAFETY: setsid is async-signal-safe and is the whole of what runs between
    // fork and exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    command
        .spawn()
        .context("starting the detached half of the run")
}

/// The detached half: spawn the child, tee it, close the record.
///
/// This is what `hydra run` used to do inline. The only thing it decides is what
/// goes on the closing line.
pub fn watch(spec: &Path) -> Result<()> {
    let text =
        std::fs::read_to_string(spec).with_context(|| format!("reading {}", spec.display()))?;
    let launch: Launch =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", spec.display()))?;

    let provider: Provider = launch.provider.parse()?;
    let mut child_env = ChildEnv::build(provider)?;
    for (name, value) in &launch.hydra_env {
        child_env.set(name, value.clone());
    }

    let mut writer = Writer::create(
        &launch.run_file,
        &launch.run_id,
        launch.parent_run_id.as_deref(),
        Some(&launch.provider),
        launch.model.as_deref(),
    )?;

    let spawned = crate::pty::spawn(
        &launch.binary,
        &launch.argv,
        &launch.cwd,
        child_env
            .vars()
            .map(|(k, v)| (k.to_string(), v.to_string())),
        launch.sandbox_profile.as_deref(),
    );

    let mut spawned = match spawned {
        Ok(spawned) => spawned,
        Err(err) => {
            // A run that never had a process. Contract 5.2: pid null, and a reader
            // must not read that as dead-and-unharvested and go looking for a
            // vendor transcript that does not exist.
            writer.hydra_line(&launch.run_start)?;
            writer.hydra_line(&RunEnd::streamed(None, Some(format!("{err:#}"))))?;
            return Err(err);
        }
    };

    let linked_skills = if provider.reads_skills_by_shell() {
        launch
            .run_start
            .get("skills")
            .and_then(|skills| skills.get("linked"))
            .and_then(|linked| linked.as_array())
            .map(|linked| {
                linked
                    .iter()
                    .filter_map(|name| name.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let pid = spawned.pid;
    let process_start = crate::procinfo::start_time(pid)?;
    let mut start = launch.run_start;
    start["pid"] = serde_json::json!(pid);
    start["process_start_time"] = process_start
        .map(|t| serde_json::json!(t.to_rfc3339_opts(SecondsFormat::Micros, true)))
        .unwrap_or(serde_json::Value::Null);
    writer.hydra_line(&start)?;

    let mut tee_error: Option<String> = None;

    // The session id is observed and not kept. Contract 17 leaves it off the
    // result and 5.1 has a reader take it from `raw` on the init frame, so a
    // second copy in memory would have no consumer.
    let teed = spawned.tee(|line| {
        if let Some(model) = provider.observe(line).model {
            writer.set_model(model);
        }
        writer.provider_line(line)?;
        Ok(())
    });
    if let Err(err) = teed {
        tee_error = Some(format!("{err:#}"));
    }

    let exit_code = spawned.wait()?;

    // A signal death is the one ending the detached half witnesses and the
    // stream cannot show, and the pty reports it as exit code 1. `run_end.error`
    // is contract 6's field for text Hydra captured that is not on the stream,
    // and the outcome derivation reads this note first.
    if let Some(signal) = &spawned.signal {
        let note = format!("{SIGNAL_NOTE}{signal}");
        tee_error = Some(match tee_error {
            Some(existing) => format!("{note}; {existing}"),
            None => note,
        });
    }

    if spawned.lossy_lines > 0 {
        let note = format!(
            "{} line(s) the child wrote were not valid UTF-8 and were stored lossily",
            spawned.lossy_lines
        );
        tee_error = Some(match tee_error {
            Some(existing) => format!("{existing}; {note}"),
            None => note,
        });
    }

    if spawned.capped_lines > 0 {
        let note = crate::pty::capped_line_note(spawned.capped_lines);
        tee_error = Some(match tee_error {
            Some(existing) => format!("{existing}; {note}"),
            None => note,
        });
    }

    if spawned.pty_still_held {
        tee_error = Some(match tee_error {
            Some(existing) => format!("{existing}; {PTY_HELD_NOTE}"),
            None => PTY_HELD_NOTE.to_string(),
        });
    }

    if provider.reads_skills_by_shell()
        && let Some(snapshot) = &launch.skill_snapshot
        && !linked_skills.is_empty()
    {
        let copy_root = launch.cwd.join(provider.skill_root());
        if let Some(note) = crate::skills::copy_mismatch(&copy_root, snapshot, &linked_skills) {
            tee_error = Some(match tee_error {
                Some(existing) => format!("{existing}; {note}"),
                None => note,
            });
        }
    }

    writer.hydra_line(&RunEnd::streamed(exit_code, tee_error))?;
    Ok(())
}

/// How `run_end.error` starts when the child died on a signal. The outcome
/// derivation matches on this prefix, and the signal's name follows it.
pub const SIGNAL_NOTE: &str = "the child died on signal ";

/// `run_end.error` when the child exited but a descendant still held the slave
/// when the tee gave up. The outcome derivation reads this as Hydra stopping
/// rather than the process dying.
pub const PTY_HELD_NOTE: &str = "the pty was still held by a descendant when the record closed";

/// What the blocking view saw before it stopped watching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waited {
    /// The closing line is on disk.
    Ended,
    /// The call's budget ran out first. The child was not touched.
    TimedOut,
    /// The detached half exited without closing the record, so nothing is going
    /// to write the rest of this run.
    WriterGone,
}

/// How often the view looks at the file it is tailing.
const POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// Waits for line 0, so a handle always carries a pid.
///
/// A watcher that dies before writing anything is the one failure the view cannot
/// read off the record, because there is no record. It is our own child until we
/// stop waiting on it, so `try_wait` answers.
pub fn wait_for_start(
    run_file: &Path,
    watcher: &mut Child,
    budget: std::time::Duration,
) -> Result<()> {
    let deadline = deadline(budget);
    loop {
        if run_file.exists() && crate::live::read_view(run_file)?.pid.is_some() {
            return Ok(());
        }
        if let Some(status) = watcher.try_wait()? {
            // One last look: it may have written line 0 and exited in the same
            // tick, which is what a provider binary that is not installed does.
            if run_file.exists() {
                return Ok(());
            }
            bail!("the detached half of the run exited ({status}) before it wrote anything");
        }
        if expired(deadline) {
            bail!(
                "the detached half of the run wrote nothing to {} within {:?}",
                run_file.display(),
                budget
            );
        }
        std::thread::sleep(POLL);
    }
}

/// Tails a run file until it closes, the budget runs out, or the writer dies.
///
/// **Giving up only means it stops watching** (harness spec 9). Nothing here
/// signals the child, removes anything or writes to the record: the run is
/// somebody else's, and the only thing this call owns is the caller's patience.
pub fn wait_for_end(
    run_file: &Path,
    watcher: &mut Child,
    budget: std::time::Duration,
) -> Result<Waited> {
    let deadline = deadline(budget);
    loop {
        if crate::live::has_run_end(run_file)? {
            return Ok(Waited::Ended);
        }
        if watcher.try_wait()?.is_some() {
            // The writer is gone. Look once more, because it closes the record
            // and then exits, and this can land between the two.
            std::thread::sleep(POLL);
            if crate::live::has_run_end(run_file)? {
                return Ok(Waited::Ended);
            }
            return Ok(Waited::WriterGone);
        }
        if expired(deadline) {
            return Ok(Waited::TimedOut);
        }
        std::thread::sleep(POLL);
    }
}

/// When a budget runs out, or `None` for one the clock cannot reach.
///
/// **A budget the machine cannot count to is not an error.** `Instant + Duration`
/// panics on overflow, and `--timeout` takes any `u64` of seconds, so adding
/// blindly would abort the launcher after the worktree, the gate and the child
/// already exist and leave the run behind a stack trace. Somebody who typed a
/// number that large meant "do not give up", and `None` is that.
fn deadline(budget: std::time::Duration) -> Option<std::time::Instant> {
    std::time::Instant::now().checked_add(budget)
}

fn expired(deadline: Option<std::time::Instant>) -> bool {
    deadline.is_some_and(|at| std::time::Instant::now() >= at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_budget_the_clock_cannot_reach_is_not_a_panic() {
        // `--timeout` takes any u64 of seconds and `Instant + Duration` panics on
        // overflow, so adding blindly would abort the launcher after the worktree,
        // the gate and the child already exist.
        assert_eq!(deadline(Duration::from_secs(u64::MAX)), None);
        assert!(!expired(None));
    }

    #[test]
    fn a_zero_budget_is_already_expired() {
        assert!(expired(deadline(Duration::ZERO)));
        assert!(!expired(deadline(Duration::from_secs(60))));
    }
}
