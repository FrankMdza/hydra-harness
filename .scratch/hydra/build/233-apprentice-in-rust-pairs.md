# 233 - apprentice in Rust: pairs

Type: build
Status: claimed
Blocked by: 231

## What

Map ticket 95. This is "A3" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`: read that section whole, and the plan's
answers under it. It names the files and functions to port, what the differential test compares
on, what must be identical, what is normalized, and the one thing most likely to differ.

## Do

1. Port what the plan's section names into `crates/apprentice` (package and binary `hydra-apprentice`), with the
   Python's arguments, output and exit codes. The crate depends on `hydra-pyjson` and
   `hydra-records` as it needs, never on `hydra`, `hydra-puppets` or `hydra-grader`.
2. Record the Python once, as `crates/grader/tests/record_collect.py` does: an empty environment,
   frozen data, shell stubs for every program the Python calls, the checkout written as `<repo>`.
   The Rust tests compare with the fixtures and start no `python3`. Fixtures under 1 MB.
3. Break the Rust on purpose once and say how many comparisons fail.
4. The Python stays until the piece's cutover. `IMPLEMENTATION.md`: a passage on what was ported.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.
