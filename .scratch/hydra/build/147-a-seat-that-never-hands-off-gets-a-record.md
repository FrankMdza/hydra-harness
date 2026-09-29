# 147 - A seat that never hands off gets a record

Type: build
Status: open
Blocked by:

## What

Map ticket 84, resolved 2026-09-29. Read its `## Answer` first.

When a gated interactive session ends, `Served::stop` in `src/seats.rs` waits up to
`HANDOFF_BUDGET`, 60 seconds, for a seat still inside `run::run` to hand off to its detached
watcher. A seat still launching after that dies with the process and is named on stderr. Its run id
is in the `runs` of the session's `seat_request` line, and no record holds what happened to it.

The answer: that seat gets its own record with outcome `never_started`, the outcome a launch that
fails after its worktree exists already writes (`record_never_started`, `src/run.rs` near 936).
The session's record gets no new line type.

## Do

**First, the check the answer asks for, before any code.** Read `docs/run-record-contract.md` and
`src/record/` for what a `never_started` record must carry. If the contract does not admit one for
a run whose worktree may not exist, or whose line 0 was never written, stop and say so in your
report. That is a decision for the map and not yours.

If it does:

1. Tests first in `tests/seats.rs`, on the stub provider the file already uses, each shown failing
   before the change. A seat that does not hand off inside the budget leaves a record under its
   run id, with outcome `never_started`, `parent_run_id` the session's, and a reason that says the
   session ended before the handoff. The record passes the contract's validator.
2. A seat that hands off inside the budget is untouched: one record, written by its own watcher,
   and no second one.
3. A seat that had already written its line 0 when the session ended does not get a second line 0.
   Say in the report which of the two the code does for that seat, and why.
4. The line on stderr stays, since a human at the terminal still wants it.
5. `IMPLEMENTATION.md`: amend the passage on the launcher in the same change. `grep -n` for
   `HANDOFF_BUDGET` or `seat_request`, and read at most 80 lines around it.

## Done when

- The tests above exist, each failed before the change, and `cargo test --test seats` passes with
  `cargo test --lib`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
- `hydra doctor --walk` accepts the record the test wrote, or the report says why the walk could
  not be run inside the worktree.
- The report says what stays untested on stubs and needs a live launch.
