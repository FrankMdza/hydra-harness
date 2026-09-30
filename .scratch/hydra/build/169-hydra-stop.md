# 169 - hydra stop

Type: build
Status: claimed
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

**Check first, before code.** The panel set one condition: the record must say a human stopped
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
