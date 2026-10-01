# 216 - grader in Rust: triage names why a run did not complete

Type: build
Status: claimed
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
