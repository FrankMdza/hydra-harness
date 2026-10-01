# 215 - grader in Rust: the monitor reads a prefix of a record

Type: build
Status: resolved
Blocked by: 213

## What

Map ticket 94, the grader's port. This is "plan 213" of `.scratch/hydra/research/94-the-grader-port-plan.md`: read that section whole,
and "What puppets taught" above it. It names the Python files and functions to port, the inputs
the differential test compares on, what must be identical, what is normalized, and the one thing
most likely to make Python and Rust differ.

## Do

1. Port what the plan's section names into `crates/grader` (package `hydra-grader`, binary
   `hydra-grader`), with the Python's arguments, output and exit codes. The crate depends on
   `hydra-pyjson` and not on `hydra-puppets` or `hydra`.
2. Record the Python once: `crates/grader/tests/record.py` runs the Python on every input of this
   ticket from an empty environment, with frozen data files, and writes
   `crates/grader/tests/fixtures/`. The Rust tests compare with those files and start no
   `python3`. Follow `crates/puppets/tests/record.py` and `differential.rs`. Record with the
   `python3` on `PATH`, which is 3.14.7, and write its version into the fixtures.
3. Break the Rust on purpose once (one constant, one word) and say how many comparisons fail.
4. The Python stays. `IMPLEMENTATION.md`: a passage on what this ticket ported and what it
   normalizes.

## Done when

- The report lists each command and input compared, what was normalized and why, and what the
  plan assumed that turned out false. fmt and clippy clean over the workspace; `cargo test --lib`
  and `cargo test -p hydra-grader` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f6f1` (claude opus 5.5, `standard`, 8.6M tokens). The session took
the port on its tests, its deliberate breaks and a live comparison of its own, merged it with
build 217 by hand (three conflicts, each two runs adding a line in the same place), and applied
it to master otherwise as written.

Built: `crates/grader/src/monitor.rs`, `monitor.py` whole, and `monitor_cli.rs`, `hydra-grader
monitor features|train|score`. It calls `baseline::fit`, `slices`, `vector`, `predict_with` and
`report_with` and copies none of them; `baseline.rs` gained `pub(crate)` on what the monitor
calls, a lookup closure in `vector`, and `predict_with` and `report_with` under `predict` and
`report`. No dependency added.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 15 unit
and 12 differential tests; the monitor's test makes 1,960 comparisons over 490 recorded cases (167
feature views, 29 models trained, 103 scores, 60 refusals, 75 tracebacks, 52 wrong command lines,
4 help requests), and build 213's 1,116 still pass. With the dispatch off, 1,218 of 1,904 failed.
The run broke the Rust three ways; two of them passed unnoticed on its first fixtures, and it
added cases until each failed.

The session ran `monitor features` with the real `monitor.py` and with the Rust on thirty real
run records at 0.25, 0.5 and 1.0: ninety outputs, all byte-identical.

Normalized: what build 213 normalizes (a traceback by its class, the last stderr line on exit 2,
help by its exit code), and the checkout's directory in a traceback, recorded as `<repo>`.

What the plan assumed that is false: `call_key` with an input matched at once; the one that
failed was a call with no input, where the Python returns the name itself and not its text, so
tools named `1`, `1.0` and `true` are one key. `harness/...010.jsonl` among the twelve fixture
records is not a run and both refuse it.

Left open: `monitor_cli.rs` repeats about forty lines of `baseline_cli.rs`'s top-level parser.
Recording again rewrites the worktree path inside tracebacks in build 213's `baseline.json`;
`record_baseline.py` wants the `<repo>` replacement `record_monitor.py` has. The fixtures of the
crate are 3.8 MB, 2.0 MB of them this ticket's. `train` and `score` did not run on real data. A
run id that holds a lone surrogate is left out of the join without a look at the file system.

