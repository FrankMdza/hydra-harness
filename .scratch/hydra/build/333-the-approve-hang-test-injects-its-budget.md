# 333 - `a_helper_that_hangs_on_approve_is_waited_on_five_seconds` injects its budget instead of waiting five seconds on the wall clock

Type: build
Status: resolved
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

## Outcome

Built on 2026-10-15. `App::with_approve_budget` injects the wait, defaulting to the unchanged
five-second `APPROVE_BUDGET`. The late-helper message formats the duration with `{:?}` so the
injected 100 ms reads `100ms`, while production still reads `5s`.

Rewrote and renamed the existing test to
`a_helper_that_hangs_on_approve_is_given_up_at_the_budget`; no separate test was added. It pins
`APPROVE_BUDGET == 5s`, uses 100 ms against the shared 30-second stub hang, and checks elapsed
time is at least the budget and less than half the hang. It also checks the refresh effect,
the still-running message, the dismissed confirmation, both steps remaining waiting rather than
being hidden as approved, and drawing the step and give-up message. `IMPLEMENTATION.md` describes
the injection beside the production approve wait.

Measured:

- With the give-up deferred to 60 seconds, the test failed at the elapsed assertion:
  `app.handle` waited 30.024793917 s; nextest reported 30.302 s for the test. Restored the
  injected budget immediately afterwards.
- Five consecutive runs under eight `yes` workers all passed, in 0.384, 0.427, 0.356, 0.374,
  and 0.357 s per test (nextest timings, including setup). All eight workers were stopped.
- Required checks all green: `cargo fmt --all --check` exited 0 with no output;
  `cargo clippy --workspace --all-targets -- -D warnings` finished the dev profile in 26.59 s;
  `cargo nextest run --release --workspace` reported `Summary [  68.745s] 1419 tests run:
  1419 passed, 4 skipped`; `cargo test --release --workspace --doc` ended with
  `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.

No ticket assumption turned out false. The original production budget and elapsed assertion
were as described. A diagnostic `ps` call was refused with `operation not permitted`; it was
not retried and no process inspection was needed.
