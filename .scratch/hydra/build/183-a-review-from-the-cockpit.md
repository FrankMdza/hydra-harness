# 183 - A review from the cockpit

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f320-dd35 under `standard`. The session
applied it as written. `v` on a task row with a run asks with the ticket's quota text; `y` starts
`puppets.py review <goal> <task>` in its own process group, stdin null, both streams to
`$HYDRA_ROOT/cockpit/review-<task>.log`, with `HYDRA_ROOT` and `HYDRA_PUPPETS_HOME` set. Every draw
polls it with `try_wait`; the row reads `review running`, then the verdict the log recorded, or
`review failed` with the log path in the detail. One review per task. The goal fold reads `review`
events. Four tests, each failing first; fmt and clippy clean, `--test cockpit` 57, `--lib` 222.

What the ticket assumed that is false: `puppets.py review` ends by `execvp` of `hunk show`, which
without a terminal will likely fail or hang, leaving the row on `review running` or showing a
verdict with a failed exit. Build 185 gives the helper `--no-open` and the cockpit passes it.
Nothing under `integrations/herdr/` opens a pane; `puppets.py` splits its own in Herdr. A review
started before a cockpit restart shows nothing until its verdict lands.

