# 127 - The walk assertions in `src/walk.rs` keep the comments that said why

Type: build
Status: open
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
