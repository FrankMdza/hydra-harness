# 127 - The walk assertions in `src/walk.rs` keep the comments that said why

Type: build
Status: resolved
Blocked by:

## What

Build ticket 126 moved nine live corpus walks from `tests/record_contract.rs`,
`tests/normalize.rs` and `tests/outcome.rs` into `src/walk.rs`, and dropped most of their
comments on the way: 105 comment lines left the test files, and `src/walk.rs` has 20. Those
comments carried the reasons, each naming the record, ticket or measurement behind an
assertion: the one silent gate on 2026-09-07 and why a `StalePolicy` is also an acceptable
reading of it, why `Ungated` is worth checking on this machine, why every old record rebuilds the
scopes once, the `agy` timeout and the codex seat behind the `failed` rule, and so on.

## Do

1. `git show d58c003:tests/normalize.rs`, `git show d58c003:tests/outcome.rs` and
   `git show d58c003:tests/record_contract.rs` hold the originals. For each function in
   `src/walk.rs`, find the test body it came from and carry its comments over to the matching
   lines, unchanged unless a comment names something that moved (a test name that is now a
   function, "this test" that is now "this check"). Doc comments on the removed live tests become
   doc comments on the matching `check_*` function.
2. Change no code. `git diff -w --stat` should show comment lines only; say so in your report,
   and list any comment you dropped with the reason (for example, it described the `#[test]`
   harness itself).

## Done when

- Every comment removed from those three files by build 126 is either in `src/walk.rs` or
  listed in your report with a reason.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib --test
  doctor --test normalize --test outcome --test record_contract` pass.

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d587-e3e1 under `standard`, about 0.59M
tokens. The session applied the diff as written.

`src/walk.rs` gains 94 comment lines, and `git diff -w` shows no code change. Six short doc
comments build 126 wrote were replaced by the originals. Each `check_*` function now carries the
doc comment of the test it came from, and the inline reasons are back at their assertions:
contract 5.5 and 7.8 in `check_normalizes`, ticket 48's silent gate, the `Ungated` fail-open
note and build 15's rebuilt scopes in `check_gate_leaks`, contract 11 in
`check_normalizing_writes_nothing`, and the eight per-outcome comments in
`check_gated_write_denials`. Dropped on purpose: the doc comments on the old `Reader` alias and
`read_live`, which described test wiring that no longer exists. fmt and clippy are clean, and
doctor 2 and normalize 87 pass on master.
