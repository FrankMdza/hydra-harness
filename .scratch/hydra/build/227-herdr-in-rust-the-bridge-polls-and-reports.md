# 227 - herdr in Rust: the bridge polls and reports

Type: build
Status: resolved
Blocked by: 226

## What

Map ticket 95. This is "H3" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`: read that section whole, and the plan's
answers under it. It names the files and functions to port, what the differential test compares
on, what must be identical, what is normalized, and the one thing most likely to differ.

## Do

1. Port what the plan's section names into `crates/herdr` (package and binary `hydra-herdr`), with the
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

## Outcome

Dropped 2026-10-03 by map ticket 104 (D1, B): Herdr is frozen and the cockpit is the one interface, so the plugin is not ported to Rust. The run graph moves into the cockpit as build 271.
