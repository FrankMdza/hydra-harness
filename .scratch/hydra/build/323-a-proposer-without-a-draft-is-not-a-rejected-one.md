# 323 - A proposer that never produced a plan gets `chosen: null`, not `false`

Type: build
Status: resolved
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

## Outcome

Built draft-aware proposer labels: `has_draft` records whether the folded plan has a
nonempty path, and a proposer without one has `chosen: null` and `outcome: null` even
when a verdict exists. The count puts these rows in `no draft`, ahead of `no verdict`,
and outside the chosen/not-chosen preference split. Updated build 303's implementation
paragraph and both `before_303` comparisons to strip and check the new proposer-only field.

Added a three-proposer goal fixture and a regression test through the real Puppets fold,
label extraction and count command. It covers a path verdict, provider-only choice and
no verdict; the last case counts two draft rows as no verdict and one row as no draft.
The existing count assertion now includes the new zero-valued bucket.

Measured the actual release CLI with `HYDRA_PUPPETS_HOME` set to the synthetic fixture
now at `crates/grader/tests/fixtures/no-draft`, without reading any external goal logs:

```text
before: plan_proposer  3  chosen 1  not chosen 2  no verdict 0
after:  plan_proposer  3  chosen 1  not chosen 1  no verdict 0  no draft 1
```

No behavioral assumption in the ticket proved false. The new `has_draft` field is needed
because `chosen: null` alone cannot distinguish no draft from no verdict. The gate refused
`clock.sleep` as `hydra/unknown-tool` (`clocksleep` absent from the capability table).
It was not retried; verification used the running process's normal wait operation.
Changes are uncommitted.

All four required checks are green:

```text
cargo fmt --all --check: exit 0, silent
cargo clippy --workspace --all-targets -- -D warnings:
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 49.81s
cargo nextest run --release --workspace:
     Summary [  44.578s] 1392 tests run: 1392 passed, 4 skipped
cargo test --release --workspace --doc:
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
