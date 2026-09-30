# 169 - hydra stop

Type: build
Status: resolved
Blocked by:

## What

Hydra has no command that stops a run; `reap` and `prune` clean up runs that already ended. The
panel of 2026-09-30 (three vendors, unanimous, recorded in the human's decision log) chose option
B: a command the human types, never a watchdog (D10 stands). `hydra stop <run>`, `hydra stop
--all`, and `hydra stop --session <run>`, which stops the session and every live run whose
`parent_run_id` is it.

Each run is a detached watcher (`src/watch.rs`) that owns the provider's pty and writes the
record. A child that dies on a signal the watcher sees gets `run_end.error` starting
`SIGNAL_NOTE` ("the child died on signal "), which `src/outcome.rs` reads as `crashed`.

## Do

Read `src/watch.rs`, `src/live.rs`, `src/status.rs`, `src/resolver.rs`, `src/reap.rs` and the
`run_end` section of `docs/run-record-contract.md` before any code.

**Map 89 answered the check (2026-09-30): add a tenth outcome, `stopped`.** Contract 8 gets one
row, harness spec 12's outcome list one entry, `Outcome::Stopped` in `src/outcome.rs` with
`rung_one_learns` false, and the derivation reads a fixed stop-note prefix in `run_end.error`
before `SIGNAL_NOTE`. This ticket may amend `docs/run-record-contract.md` section 8 and harness
spec 12 for exactly that, nothing else. Every `match` the compiler flags gets the new arm.

The text below was the check as first written; it is done. **Old check:** The panel set one condition: the record must say a human stopped
the run, not only `crashed`. Find whether that fits the contract as it stands, for example as a
`run_end.error` text the watcher writes when it was asked to stop, with the outcome derived as
something other than `crashed`. If it needs a new field or a new outcome in the contract, that is
a decision: stop there, and write in your report the smallest contract change you would propose
and why. Do not change `docs/`.

If it fits:

1. `hydra stop <prefix>` resolves the run with `resolver.rs` (a full id or an unambiguous
   prefix), refuses a run that is not live, and asks the run's watcher to stop the child: SIGTERM
   to the provider's process group, then SIGKILL after a grace period (say 10 s, a constant), so
   the watcher still writes `run_end`. Say how the watcher learns it was a stop (a signal to the
   watcher, a marker file in the run's spawn directory, or another way you find in the code) and
   why that one.
2. `--all` stops every live run; `--session <prefix>` stops that run and every live run whose
   record's line 0 carries it as `parent_run_id`, children first, and the session's seat launcher
   refuses new seats once the stop began.
3. `hydra stop` prints each run it stopped and the outcome its record now derives.
4. It is typed by a human and reached from nowhere but `main`, like `reap` and `prune`.
5. `IMPLEMENTATION.md` and the command list in `README.md`: the new command.

Tests in `tests/` on the stub provider `pty_tee.rs` and `detached.rs` already use, launching no
real provider, each shown failing before the change: a stopped run's record ends with `run_end`
and derives the stop, not `crashed`; `--session` stops the child before the parent; a run that is
not live is refused; a prefix that names two runs is refused.

## Done when

- The check above is in the report, whichever way it went.
- If built: each test failed before the change; fmt, clippy, `cargo test --lib` and the test
  files you touched pass.

## Outcome so far

2026-09-30, run 01a0f2cc-004a (`claude` opus): stopped at the check, as asked. The stop note fits
`run_end.error`, but no outcome of contract 8 means a human stop: today it would derive `crashed`
(which rung one learns from) or `failed` (which contract 8.6 rejects for an interrupted pass).
Map ticket 89 decides the outcome; this ticket waits for it and is otherwise unchanged.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f2d9-2497 under `standard`, after map 89.
The session applied it as written. `hydra stop <run>`, `--all`, `--session <run>` (`src/stop.rs`):
a `stop` marker in each run's spawn directory, all markers before any signal, then SIGTERM to the
child's process group (to the child alone when it leads none), checked against the recorded start
time, SIGKILL after 10 s, and a wait of up to 30 s for `run_end`. The watcher, or the harvest for
an interactive session, finds the marker and puts `STOP_NOTE` first in `run_end.error`; `derive`
reads it as `Outcome::Stopped`, which rung one never learns from. A marked session's launcher
refuses new seats with `hydra/session-stopped`. Contract 8 and harness spec 12 gained the row.
Refused inside a seatbelt. Five tests; on master fmt and clippy clean, `--lib` 220, `stop` 4,
`seats` 31, `outcome` 44, `harvest` 12, `detached` 9, `export` 23, `record_contract` 20,
`cockpit` 25, `normalize` 87.

What map 89 assumed that turned out false: the compiler flagged no match; every consumer of
`Outcome` goes through `rung_one_learns` or compares named variants. Gaps written in
`IMPLEMENTATION.md`: a child that exits on its own between marker and signal reads `stopped`;
`--session` stops direct seats, not their children; a seat whose record appears after the second
scan is missed. Never run against a live provider or a live interactive session.

The session added `stopped` to the Herdr bridge's outcome map in the same commit.

