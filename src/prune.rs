//! `hydra prune`, harness spec 10 and 17, contract 2.
//!
//! It moves run files out of `~/hydra/runs/` and deletes only behind a flag,
//! because D12 keeps lossless export a standing constraint and anything deleted is
//! training data destroyed.
//!
//! **Moved where is this build's choice.** Harness spec 5's table says prune
//! "moves rather than deletes" and names no destination, the same gap section 19.1
//! leaves for the filenames inside the spawn directory. `~/hydra/pruned/<run-id>.jsonl`
//! keeps the file under a path Hydra created (D9) and keeps the run id on it, so a
//! record put back is a `mv` and nothing else. A destination that already holds
//! that run id is a refusal rather than an overwrite: a second prune of a run
//! already pruned would otherwise destroy the copy the first one saved.
//!
//! **A live run is never pruned, and neither is a launch in progress.** Contract 6
//! gives a run file one writer, and a rename does not take the file away from an
//! open descriptor: the writer would keep appending, into a path nothing looks in.
//! Contract 5.2's liveness test catches the run that has written line 0. The one it
//! cannot catch is the run whose record exists and is still empty, because the
//! detached half opens the file before it spawns the child, and an empty record
//! reads as abandoned. Moving that file makes the launcher time out waiting for a
//! line 0 that is being written somewhere else; `--delete` destroys a running run's
//! record outright. `RunView::launched` and the launch grace period are the guard,
//! the same pair `hydra reap` uses.
//!
//! **Prune leaves the checkout and the spawn directory alone.** Those are
//! `hydra reap`'s, and the order the two commands are typed in matters: reap reads
//! the pid in each record to know what is still going, so pruning first leaves it
//! deriving liveness from records that are no longer there. Reap, then prune.
//!
//! **Nothing in this crate calls into here except `main`,** for the reason
//! `crate::reap` gives at length.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::live::State;
use crate::paths::Layout;

/// What one run's prune did.
pub enum Pruned {
    Moved {
        run_id: String,
        to: PathBuf,
    },
    Deleted {
        run_id: String,
    },
    /// What would have happened, under `--dry-run`.
    Would {
        run_id: String,
        what: String,
    },
    /// Nothing was touched on purpose. A live run is the ordinary reason.
    Left {
        run_id: String,
        why: String,
    },
    Failed {
        run_id: String,
        why: String,
    },
}

/// Prunes the named runs, or every run that is not live.
///
/// **An empty list means all of them, and that is safe because the default is a
/// move.** The command that destroys something needs a flag; the command that
/// tidies does not need a list. A run named explicitly that turns out to be live
/// is reported rather than skipped quietly, because the human asked about that one.
pub fn prune(
    layout: &Layout,
    run_ids: &[String],
    delete: bool,
    dry_run: bool,
    grace: std::time::Duration,
) -> Result<Vec<Pruned>> {
    let views = crate::live::scan(layout)?;

    let mut out = Vec::new();
    if run_ids.is_empty() {
        for view in &views {
            match busy(view, grace) {
                Some(_) => continue,
                None => out.push(one(layout, &view.run_file, delete, dry_run)),
            }
        }
        return Ok(out);
    }

    for run_id in run_ids {
        let path = layout.run_file(run_id);
        if !path.is_file() {
            out.push(Pruned::Failed {
                run_id: run_id.clone(),
                why: format!("no run record at {}", path.display()),
            });
            continue;
        }
        // Matched on the path rather than on `run_id`, because `read_view` takes
        // the id off line 0 and a record whose envelope disagrees with its filename
        // would otherwise find no view at all and skip the liveness check entirely.
        match views.iter().find(|view| view.run_file == path) {
            // A run named explicitly is reported rather than skipped quietly,
            // because the human asked about that one.
            Some(view) => match busy(view, grace) {
                Some(why) => out.push(Pruned::Left {
                    run_id: run_id.clone(),
                    why,
                }),
                None => out.push(one(layout, &path, delete, dry_run)),
            },
            None => out.push(one(layout, &path, delete, dry_run)),
        }
    }
    Ok(out)
}

/// Why this record is not prune's to move, or `None`.
///
/// Two reasons and they are different. A live run has written line 0 and its pid
/// still matches. A launch in progress has written nothing at all, and only the
/// file's age says so.
fn busy(view: &crate::live::RunView, grace: std::time::Duration) -> Option<String> {
    if view.state == State::Live {
        return Some(match view.pid {
            Some(pid) => format!("the run is still writing it, pid {pid}"),
            None => "the run is still writing it".to_string(),
        });
    }
    if !view.launched() && crate::status::just_created(&view.run_file, grace) {
        return Some("the run is still starting and has not written line 0".to_string());
    }
    None
}

/// Moves or deletes one record, named by the path it was found at.
///
/// The destination keeps the source's own filename rather than being rebuilt from
/// a run id, so a record whose line 0 disagrees with its name still lands under the
/// name it had. The label in the report is that filename too.
fn one(layout: &Layout, from: &std::path::Path, delete: bool, dry_run: bool) -> Pruned {
    let run_id = from
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let run_id = run_id.as_str();

    if delete {
        if dry_run {
            return Pruned::Would {
                run_id: run_id.to_string(),
                what: format!("delete {}", from.display()),
            };
        }
        return match std::fs::remove_file(from) {
            Ok(()) => Pruned::Deleted {
                run_id: run_id.to_string(),
            },
            Err(err) => Pruned::Failed {
                run_id: run_id.to_string(),
                why: format!("deleting {}: {err}", from.display()),
            },
        };
    }

    let to = layout.pruned().join(from.file_name().unwrap_or_default());
    if to.exists() {
        return Pruned::Failed {
            run_id: run_id.to_string(),
            why: format!(
                "{} is already there, so this would overwrite it",
                to.display()
            ),
        };
    }
    if dry_run {
        return Pruned::Would {
            run_id: run_id.to_string(),
            what: format!("move {} to {}", from.display(), to.display()),
        };
    }

    match move_file(from, &to) {
        Ok(()) => Pruned::Moved {
            run_id: run_id.to_string(),
            to,
        },
        Err(err) => Pruned::Failed {
            run_id: run_id.to_string(),
            why: format!("{err:#}"),
        },
    }
}

/// Rename, falling back to copy and remove.
///
/// `~/hydra/pruned/` is a sibling of `runs/`, so the rename is what happens. The
/// fallback is for the home directory that spans two mounts, where a rename fails
/// with `EXDEV` and losing the record to it would be the one outcome this command
/// exists to prevent.
///
/// **A half-done move takes the copy back out.** Leaving the record in both places
/// would be harmless once, and then the destination check above would refuse every
/// retry as an overwrite, so a transient failure would become a permanent one.
/// Whatever this returns, the record is in exactly one place.
fn move_file(from: &std::path::Path, to: &PathBuf) -> Result<()> {
    let parent = to.parent().expect("a run file has a parent directory");
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::fs::copy(from, to)
                .with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
            std::fs::remove_file(from).map_err(|err| {
                let _ = std::fs::remove_file(to);
                anyhow::anyhow!("removing {} after the copy: {err}", from.display())
            })
        }
    }
}

/// What a human reads back. The destination is on every moved line, because a
/// command that moves your evidence has to say where it put it.
pub fn report(pruned: &[Pruned]) -> String {
    if pruned.is_empty() {
        return "nothing to prune\n".to_string();
    }
    let mut out = String::new();
    for item in pruned {
        match item {
            Pruned::Moved { run_id, to } => {
                out.push_str(&format!("moved {run_id} to {}\n", to.display()));
            }
            Pruned::Deleted { run_id } => out.push_str(&format!("deleted {run_id}\n")),
            Pruned::Would { run_id, what } => out.push_str(&format!("would {what} ({run_id})\n")),
            Pruned::Left { run_id, why } => out.push_str(&format!("left {run_id}: {why}\n")),
            Pruned::Failed { run_id, why } => out.push_str(&format!("failed {run_id}: {why}\n")),
        }
    }
    out
}

/// How many entries were tried and would not go through.
pub fn failures(pruned: &[Pruned]) -> usize {
    pruned
        .iter()
        .filter(|item| matches!(item, Pruned::Failed { .. }))
        .count()
}
