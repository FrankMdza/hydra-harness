# 216 - grader in Rust: triage names why a run did not complete

Type: build
Status: resolved
Blocked by: 215

## What

Map ticket 94, the grader's port. This is "plan 214" of `.scratch/hydra/research/94-the-grader-port-plan.md`: read that section whole,
and "What puppets taught" above it. It names the Python files and functions to port, the inputs
the differential test compares on, what must be identical, what is normalized, and the one thing
most likely to make Python and Rust differ.

Map 94: `sample` does not have to draw what Python's `random.sample` draws. Use a small seeded
generator of your own (say which), make the sample reproducible for a seed, and compare everything
else byte for byte. Say in `IMPLEMENTATION.md` that samples drawn before this ticket cannot be
redrawn from their seed.

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

Resolved 2026-10-01. Run `01a0f70c` (codex gpt-6.1-sol, `standard`, 7.1M tokens). The session
took the port on its tests and a live comparison of its own, and applied it to master as
written.

Built: `crates/grader/src/triage.rs` (the cause rules, the evidence, the timestamps, the table,
the sample) and `triage_cli.rs`, `hydra-grader triage run|table|sample`. It reads a record's
prefix through `monitor.rs`, whose walker became visible inside the crate. No dependency added.
`sample` seeds with FNV-1a, draws with SplitMix64 and a partial Fisher-Yates; samples drawn by
the Python before this ticket cannot be redrawn from their seed, as map 94 accepted.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 15 unit
and 13 differential tests; the triage test makes 912 comparisons over 93 recorded cases. Before
the implementation 165 of 661 failed; with `unknown` renamed 18 of 819 did.

The session ran `triage run` and `triage table` with the real `triage.py` and with the Rust over
the real runs directory and a copy of the real dataset: 78 rows, identical but for `triaged_at`,
and the table byte-identical.

Normalized: `triaged_at`; help and usage; a traceback by its exception class. The sample is
checked by its output blocks, its size, uniqueness and that it repeats for a seed.

Left open: nothing else.

