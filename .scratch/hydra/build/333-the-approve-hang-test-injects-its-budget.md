# 333 - `a_helper_that_hangs_on_approve_is_waited_on_five_seconds` injects its budget instead of waiting five seconds on the wall clock

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 130's answer, question 9, where all four seats ranked it
first among the tickets worth opening. `tests/cockpit.rs` near line 3865 asserts
`5 s <= waited < 7 s` around `app.handle` of the confirm key, against the production
`APPROVE_BUDGET` of five seconds (`src/cockpit/app/mod.rs`). It is the same wall-clock shape build
330 removed from two other cockpit tests after one of them went red twice under load. It has not
gone red yet; it will, on the first loaded CI run strangers watch.

## What

Read build 330's Outcome for the style. Inject the approve budget the way `Reader::with_pending_budget`
injects the pending one: a constructor or setter the test sets to 100 ms, the production constant
unchanged and pinned by one assertion (`APPROVE_BUDGET == 5 s`). The test asserts the mechanism: the
give-up fires, the wait took at least the budget and less than half the stub's hang, the approval
is left unknown and the cockpit draws. It must still fail if the give-up never fires: disable it
once, watch the red, restore.

## Do

1. The injection point; the test rewritten; the pin.
2. The test under eight `yes > /dev/null &` workers five times in a row, timings in the Outcome.
3. `IMPLEMENTATION.md` where the cockpit's approve wait is described, if it is.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green, the five loaded
runs green.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
