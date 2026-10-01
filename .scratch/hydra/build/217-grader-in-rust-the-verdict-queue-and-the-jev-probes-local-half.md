# 217 - grader in Rust: the verdict queue and the Jev probe's local half

Type: build
Status: resolved
Blocked by: 212

## What

Map ticket 94, the grader's port. This is "plan 215" of `.scratch/hydra/research/94-the-grader-port-plan.md`: read that section whole,
and "What puppets taught" above it. It names the Python files and functions to port, the inputs
the differential test compares on, what must be identical, what is normalized, and the one thing
most likely to make Python and Rust differ.

Map 94: `jev_probe ask` is not ported and no TLS dependency is added. Port `build`, `patches`
and `score`. Its recorded results stay where they are.

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

Resolved 2026-10-01, in two runs. `01a0f6d9-b63c` (cursor-agent composer-2.5, 17.3M tokens)
wrote `queue.rs`, `jev.rs` and 23 recorded cases, added `clap`, `regex` and `csv` to the crate
and rewrote `main.rs`; the session did not apply it, put it in master's tree uncommitted, and
`01a0f6f2` (codex gpt-6.1-sol, 8.7M tokens) finished it from there. That diff was applied to
master as written.

Built: `hydra-grader baseline label-queue|label-import` and `hydra-grader jev
build|patches|score`. `queue.rs` reads and writes CSV by hand as Python's `csv` module does
(`\r\n` line ends, minimal quoting) and cuts the `## Outcome` section without a regular
expression; `jev.rs` and `jev_cli.rs` are the probe's local half; `ask` is not ported (map 94).
`crates/grader/Cargo.toml` and `Cargo.lock` are as they were: no new dependency.
`hydra-pyjson` gained `dumps_unicode_default`.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 11
unit and 11 differential tests, the two new ones 180 comparisons over 45 queue cases and 164 over
41 probe cases; `hydra-pyjson` and `hydra-puppets` unchanged. Against the dispatch before the
change 144 of 180 and 123 of 164 differed. The run broke the Rust three ways: LF-only CSV line
ends failed 15 comparisons, another accepted verdict 47, no credit for a tie in the AUC 1.

The session ran `label-queue` with the real `baseline.py` and with the Rust over the real build
tickets and a read-only copy of the real dataset: the CSV is byte-identical (250 rows, 115,957
bytes), and so are stdout and stderr.

What the first run got wrong beyond the dependencies, found by the second: integer probabilities
in `score` and Python's newline conversion on read.

Left open: nothing ran `label-import` or the probe's commands on real data. The one unit test
build 213 had for "not ported yet" is gone with the sentence it checked.

