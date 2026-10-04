# 268 - The cockpit shows this session's tasks, not a stale goal

Type: build
Status: claimed
Blocked by:

## What

Seen by the human on 2026-10-03 in a Master of Puppets session on `~/private-repo`: the cockpit's
tasks pane showed goal `ea69da1f`, a grader goal from 2026-09-24 that finished long ago, because
with no `--goal` it follows the goal log under the puppets home that changed last
(`src/cockpit/snapshot.rs`, `read_goal`). The session itself had launched agents with `hydra run`
and never made a goal, so nothing in the pane was about its work. The pane also printed "panel
verdict composed from unknown" and "split unknown" twice: build 256's panel lines for
`panel_verdict` events written before build 252, which carry no `verdict_source` or seat orders.

1. **A goal is shown only when it is live.** With no `--goal`, the newest goal log is shown when
   it changed in the last 12 hours or still has a task that is not ended or verdicted; otherwise
   no goal is shown. `--goal <id>` still shows that goal whatever its age.
2. **No live goal: the session's tasks.** The pane becomes "tasks of this session": one row per
   run that started after the cockpit started, plus every live run, newest first. Columns: run
   label, status (live or the outcome), provider, model, policy, and a one-line description, the
   first line of the run's task with the preamble Hydra prepends removed (find where `preamble.rs`
   composes it; take the human's task, not the preamble). `Enter` on a row opens the run full page
   like a lane card. Reuse the cards the snapshot already builds; do not read records again.
3. **Old panel events stay quiet.** A `panel_verdict` with no `verdict_source` draws one line,
   "panel verdict <chosen> (recorded before seat details)", and no seat or split lines.
4. Tests in `tests/cockpit.rs`: a goal log older than 12 hours with every task ended is not shown
   and the session's runs are; `--goal` shows it anyway; a run's row carries the task's first line
   without the preamble; an old `panel_verdict` draws one line. Each fails with its change removed;
   say so.

## Done when

- fmt, clippy over the workspace, `cargo test --lib`, `tests/cockpit.rs` and `tests/stop.rs` are
  clean. `COCKPIT.md` and `IMPLEMENTATION.md`: one sentence each.

## Outcome
