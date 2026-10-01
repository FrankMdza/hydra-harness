# 212 - grader in Rust: the crate, the record scan and the redactor

Type: build
Status: resolved
Blocked by: 210

## What

Map ticket 94, the grader's port. This is "plan 210" of `.scratch/hydra/research/94-the-grader-port-plan.md`: read that section whole,
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

Resolved 2026-10-01. Run `01a0f6c7-3395` (claude opus 5.5, `standard`, 5.4M tokens). The session
took the port on its tests and its deliberate breaks, did not read the 1,100 lines of Rust line
by line, and applied it to master as written.

Built: `crates/grader` (package and binary `hydra-grader`; depends on `anyhow`, `hydra-pyjson`
and `sha2`). `records.rs` is `records.py` without `refresh_show_answer` and `iter_records`: the
record scan, `committed_lines`, `record_row`, `duration_seconds`, the JSON and JSONL readers and
writers, the preamble functions and the two homes. `redact.rs` is `redact.py` whole. `isotime.rs`
is `datetime.fromisoformat` and the subtraction, following 3.14's parser and its looseness (week
dates, colon-less times, any separator character, offsets like `+00:99`). `hydra-pyjson` gained
`loads_bytes_explained`, Python's text for a `UnicodeDecodeError`. `crates/grader/tests/record.py`
restarts itself with an empty environment and writes the fixtures (0.9 MB); two runs give the
same bytes. The binary prints one line and exits 2: nothing ported here has a command line.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 6 unit
and 8 differential tests, 6,316 comparisons (350 for the redactor, 1,256 for the record scan over
127 records, 4,458 for rows and durations, 164 for files written and read, 88 for the preamble and
the homes); `hydra-pyjson` 22 and the math probe; `hydra-puppets` 45, 5 and 18; `cargo test
--lib` 224. The run broke the Rust six ways, one at a time, and between 2 and 18 comparisons
failed each time; the session changed one secret prefix and the redactor test failed, then
restored it.

Normalized, and only this: where Python raises, the Rust error starts with the same class and
the message is not compared; a home is compared by path components.

What the ticket and the plan assumed that is false: `records.py` and `redact.py` have no command
line, so the comparison is function by function. `duration_seconds` does not always return `None`
for unreadable input: one timestamp with an offset and one without raises `TypeError`, and the
Rust returns that error. `chrono` could never have matched `fromisoformat`.

Kept as the Python does it, and worth a decision before the cutover: `committed_lines` stops at a
lone carriage return as if it were a torn tail, the reading the helper gave up in build 199.

Left open: the fixtures are CPython 3.14.7 on arm64 macOS only. `isotime.rs` was written from
memory of the C source and checked by observation (2,197 recorded pairs, and 150,000 generated
ones that are not kept). The twelve fixture records are copied into `scan.json` as bytes, and
nothing flags the copy going stale. Nothing ran on the real grader home.

