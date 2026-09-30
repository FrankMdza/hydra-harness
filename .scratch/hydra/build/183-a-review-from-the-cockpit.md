# 183 - A review from the cockpit

Type: build
Status: claimed
Blocked by: 180

## What

The panel of 2026-09-30 (unanimous B): after `y`, the cockpit starts `puppets.py review <goal>
<task>` as a detached child and shows that a review runs for the task. Conditions: the
confirmation says it launches three judge runs against quota; "running" follows the real child
and is gone within 4 s of its exit (`cursor-agent`'s reversal measure); a failed launch says so.

## Do

Tests first in `tests/cockpit.rs` with a stub `puppets.py`, each shown failing before the change.

1. `v` on a task row with a run opens the confirmation: "review <task>: three judge runs, each
   counts against quota. y starts it". `y` spawns the child detached (own process group, stdout
   and stderr to `$HYDRA_ROOT/cockpit/review-<task>.log`), and inside Herdr, if
   `integrations/herdr/` offers a way to open a pane, uses it instead; say which you found.
2. The row shows `review running` while the child lives (polled with `waitpid(WNOHANG)` or the
   start-time check `stop.rs` uses), then the verdict from the goal log once the helper records
   it, or `review failed, see <log>` on a non-zero exit.
3. One review per task at a time; `v` again says it is running.
4. `IMPLEMENTATION.md`.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.
