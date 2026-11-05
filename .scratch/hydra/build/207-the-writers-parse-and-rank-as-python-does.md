# 207 - The writers parse and rank as Python does

Type: build
Status: resolved
Blocked by: 195

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of build 194 (`e151e1b`). It found no
approval the two implementations decide differently apart from item 1, no fence-extraction
mismatch in 100,000 constructed plans, and the same races in both for `verdict`, `tasks`, `assign`
and `reconcile` (each folds before the lock and appends without rechecking; that is the Python's
behaviour and stays). Four findings are left. It waits on 195 only because 195 is editing the
same crate.

## Do

Tests first, as inputs of the differential test or unit tests beside the code, each shown failing
before the change.

1. **`--expect-revision` and `int()`** (`crates/puppets/src/main.rs` near 166 and 173, `py_int`).
   `"\u001c7"`: the Rust strips U+001C, parses 7 and records the approval; Python's argparse
   rejects it with exit 2. `"٧"` (an Arabic-Indic digit): Python accepts it as 7, the Rust
   rejects it. An integer wider than `i128`: Python reaches the revision-mismatch line, the Rust
   reports a parse error. Probe `int()` in `python3` for what it strips and which digits and
   underscores it takes, and follow it; hold the value at any width.
2. **`beta_lower`** (`crates/puppets/src/sheet.rs` near 64). It sums logs where `sheet.py` calls
   `math.lgamma`. With posteriors `(7,3)` and `(4,1)` and cost p50 `1.0` against
   `1.102805621336474`, Python scores `1.9610965296978324` and `...329` and ranks the first; the
   Rust formula gives `...333` and `...329` and ranks the second. Call the C library's `lgamma`
   through `libc`, as CPython does, and add that pair as a test. Say in the report whether
   CPython's `math.lgamma` is the platform's or its own, and port its own if it is.
3. **The goal-id normalization** (`crates/puppets/tests/differential.rs` near 1486, 1516, 1582).
   `is_goal_id` accepts `0b9f2c3e-6a1d-4c7b-0e2f-5d8a7b6c4e31`, whose variant character is not
   `8`, `9`, `a` or `b`. And a writer that prints one valid UUID and creates the log under
   another passes, because stdout and the file name are masked apart. Check the variant, and
   check that the id on stdout names the log that was created before masking either.
4. **Abbreviated options** (`main.rs` near 85). `approve g1 --pl` is `--plan` to argparse and an
   error to clap. Build 193 turned `infer_long_args` on for the read commands; the writers need
   the same. An abbreviation that is ambiguous is an error in both: compare the exit code, and
   list in the report any flag where argparse and clap resolve an abbreviation differently.

## Done when

- Each test failed before the change; fmt and clippy clean over the workspace; `cargo test --lib`
  and `cargo test -p hydra-puppets` pass with the differential test green.

## Outcome

Resolved 2026-10-01. Two runs: `01a0f654` (codex gpt-6.1-sol) probed Python and wrote the first
tests, then stopped at the account's usage limit; the session applied its unfinished diff to
master uncommitted, and `01a0f659-688b` (claude opus 5.5, 4.4M tokens) finished it. Diff applied
to master as written.

Built: `int_literal` follows `int()`: its own whitespace, which leaves out U+001C to U+001F,
decimal digits of any script from a table generated from `python3`, the 4,300-digit cap, and no
width limit below it; `--expect-revision` and `reconcile`'s attempt are a `Value` compared with
`py::eq`. `sheet.rs` has `lgamma`, a port of CPython's `m_lgamma`, and `beta_lower` uses it with
Python's compensated `sum()`. `is_goal_id` checks the variant character, and `new`'s printed id
is tied to the log it created before either is masked.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-puppets` 62
unit tests, 5 in the binary and 18 differential; the write test has 173 cases; the int parser is
compared with `python3` on every code point; `lgamma` matches `python3` 3.14.7 bit for bit on
18,000 arguments; the ignored real-log test passed; `cargo test --lib` 224 passed; the puppets
Python suite passed.

What the ticket assumed that is false:

- Item 2 said to call the C library's `lgamma` through `libc`, as CPython does. CPython's
  `math.lgamma` is its own, and the C library differs from it on 196 of the integers 1 to 399.
  `lgamma` alone was not enough either: this machine's CPython fuses `a * b + c` inside it, and
  `sum()` is compensated; both were needed for Python's floats.
- Item 4: the writers already took abbreviations, since `infer_long_args` on `Cli` reaches every
  subcommand. What was missing was the comparison: the test compared usage errors by stderr. A
  probe of 232 prefixes found no flag argparse and clap resolve differently.
- `beta_lower(0, 1)` raised in the Rust where Python returns `4.336808689942018e-19`. Fixed,
  outside the ticket.

Left open: the fused multiply-add branch is taken on `aarch64` only, and the plain branch has
never run against a real `python3`; `the_ranking_arithmetic_matches_python` would catch it on
such a machine. `m_lgamma` and the compensated sum were written from memory and checked by
measurement. The digit table is Unicode 16.0.

