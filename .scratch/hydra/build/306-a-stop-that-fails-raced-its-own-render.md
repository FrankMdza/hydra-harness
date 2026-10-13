# 306 - A stop that fails raced its own render: the cockpit test that read `stopping` for `live`

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session, from build 305's Outcome: under load 34 the first
`cargo test --release --workspace` of that run failed `tests/stop.rs::a_stop_that_fails_pastes_nothing`
at line 689 and the next six suites, three per runner, passed it.

## What

`a_stop_that_fails_pastes_nothing` presses F2, `m`, `y` with no watcher behind the seat, so the
stop refuses, and asserts the chat said "was not stopped" and that the rendered card reads
`01996d3c claude stub-1 live`. The first assertion held and the second saw `stopping`.
`reported` pumps, renders, then checks whether the stop is still in flight; on a loaded machine
the refusal landed after the render, so the text captured was the card mid-stop. The builder
called it an inferred race. Nothing in the suite has shown it twice, and that is the problem:
a test that fails once in seven under load will fail CI on the day it matters.

## Do

1. Read `tests/stop.rs` whole around the helpers `reported`, `recording_chat`, `pump` and
   `hang_up`, and `src/cockpit/app/` for how a stop's refusal reaches the snapshot and the
   message. Say in the Outcome where the window is: between which two calls the state can
   change.
2. Reproduce it on purpose: put the machine under load (`cargo build` of another worktree,
   or a `yes > /dev/null` per core) and run the one test fifty times with
   `cargo nextest run --release --test stop -- a_stop_that_fails_pastes_nothing` or the cargo
   equivalent. Record the failure count. If it does not fail in fifty, say so and widen the
   search: the same window may exist in the sibling tests of the same file.
3. Close the window in the test, not the product: `reported` or the test itself waits for the
   condition it asserts (the message set and the stop no longer in flight) before rendering,
   with a bounded wait that fails loudly on timeout rather than a sleep. Keep every assertion.
   If the window is in `src/cockpit/app/` and a real user could see a card read `stopping`
   after the refusal, that is a product finding: say so in the Outcome and do not fix it here.
4. Run the fifty again under the same load; the count must be zero.

## Done means

The Outcome names the window, both fifty-run counts, and the change. `cargo nextest run
--release --workspace` and `cargo test --release --workspace --doc` green.

## Outcome
