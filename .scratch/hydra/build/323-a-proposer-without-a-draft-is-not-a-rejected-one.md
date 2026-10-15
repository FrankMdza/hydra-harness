# 323 - A proposer that never produced a plan gets `chosen: null`, not `false`

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 122's answer, question 3. `crates/puppets/src/fold.rs`
creates a plan entry on `plan_proposed` (line 180) and gives it its `path` on `plan_ready` (line
196); `crates/grader/src/labels.rs` labels every plan entry `chosen` whenever a verdict exists,
path or not. A proposer that never produced a plan, such as codex rate-limited on `ea69da1f` with
0 tokens, is labelled `chosen: false` as if the panel had read and rejected a draft. Build 303's
"not chosen 6" includes such rows, and a preference pair built from one teaches nothing.

## What

In `labels.rs`, a `plan_proposer` row whose entry has no `path` (no `plan_ready`) gets
`chosen: null` and is left out of the preference split. `hydra-grader labels count` prints "no
draft" as its own line beside "chosen", "not chosen" and "no verdict". The equivalence tests of
build 303 (`before_303`) are extended the way 303 extended them, so the old rows still compare
equal after stripping the field.

## Do

1. Read `labels.rs` where 303 added `picked`, `tally`, `chosen` and `outcome`, and
   `labels_cli.rs`.
2. The null, the count line, the tests: one goal with a proposer that has `plan_proposed` and no
   `plan_ready`, asserting null and the count.
3. Run the real count on the goal logs you can read (`~/hydra/puppets/` is outside your
   worktree and refused; use the fixtures under `crates/grader/tests/` or whatever synthetic
   logs the tests build) and paste the before and after in the Outcome. The session runs the
   real one on master.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the labels are described (build 303's paragraph).

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
