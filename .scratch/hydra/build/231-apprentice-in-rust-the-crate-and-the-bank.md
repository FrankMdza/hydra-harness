# 231 - apprentice in Rust: the crate and the bank

Type: build
Status: resolved
Blocked by: 224

## What

Map ticket 95. This is "A1" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`: read that section whole, and the plan's
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

## Outcome

Built 2026-10-04 by an Opus run (`01a10626`), reviewed and applied by the session.

`crates/apprentice` (package and binary `hydra-apprentice`, on `anyhow`, `hydra-pyjson`,
`hydra-records` and `sha2`): `bank.rs` ports `apprentice.py` lines 61 to 319 (`cut`, `safe`, the
facts, the 7 verifiable and 4 open templates, `_pick`, `bank build`), `dump.rs` is
`json.dumps(sort_keys=True, ensure_ascii=False)` with a lone surrogate written raw, `cli.rs` the
argparse of `bank build`; `attempt`, `score` and `pairs` exit 2 naming builds 232 and 233.
`tests/record_bank.py` recorded 63 cases (772 KB) from the Python once, and `tests/bank.rs`
replays them with no `python3`. Broken on purpose: an excerpt budget of 2,999 fails 4 of 252
comparisons, the surrogate escape 3 of 252.

**Measured by the session on the real corpus:** `bank build --n 200` from Python and from Rust,
each into its own scratch home, wrote byte-identical `bank.jsonl` files of 400 tasks. The Rust
took 9.2 s against the Python's 7.3 s, so the port is not faster yet; the likely cost is the
code-point `PyStr` work per record, not measured.

Assumed and false: CPython 3.14.7; the machine has 3.14.8 and the fixture records it. Kept on
purpose: a seat count past 128 bits is an `OverflowError` here where Python carries on; no record
reaches it. `cargo test -p hydra-apprentice` 4 of 4, fmt and clippy clean.
