# 306 - A stop that fails raced its own render: the cockpit test that read `stopping` for `live`

Type: build
Status: resolved
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

Resolved 2026-10-13. Only `tests/stop.rs::reported` changed; every assertion is kept.

The window is inside the draw that `reported` used to capture before checking
`host.app.stopping()`: `App::draw` polls stops and paints the live card, then
`Host::draw` calls `Host::settle_stops` after `self.app.draw(cockpit_area, buf)`.
If the stop thread sends its refusal after the first poll but before that second
poll, `settle_stops` consumes it. `App::poll_stops` removes the pending stop and
`stop_reported` sets `message` to "01996d3c was not stopped", leaving the snapshot's
live card unchanged. The old helper then sees no stop in flight and returns the
buffer already painted as `stopping`. The state does not change asynchronously
between the returned draw and `stopping()`; the second poll within `Host::draw`
is what makes the captured screen older than the checked state.

Product finding, left unfixed as requested: a real cockpit with a chat can show
that same transient `stopping` frame after it internally consumes a refusal in
its post-paint poll. The next draw paints `live` and the refusal message. No
product code changed, so `IMPLEMENTATION.md` is unchanged.

Measurement used eight additional `yes` workers, one per logical core, while the
other Hydra run already shared the machine. The same workers stayed alive
continuously through the before, expanded search and after batches; they were
stopped afterward. Observed load averages included 38.97 / 16.49 / 10.53 before
the expanded search and 30.52 / 26.02 / 16.55 during the after batch.

- Before: 50 invocations of `cargo nextest run --release --test stop -- a_stop_that_fails_pastes_nothing`, **0 failures / 50**. The race did not reproduce.
- Expanded search before editing: 50 invocations of `cargo nextest run --release --test stop`, **0 failing runs / 50**, all 650 test executions passed. Reviewed every sibling using `reported`: chat-backed siblings share the post-paint poll; the standalone host has no second stop poll after painting. No additional failing sibling was observed.
- After: the same 50 invocations of the single-test command under the same added load, **0 failures / 50**.

`reported` now pumps and checks that the message is set and no stop is in flight
before drawing the screen it returns. Pending iterations still pump and draw,
with the existing 20 ms polling interval and the existing 45-second deadline
(`GRACE + CLOSE_BUDGET + 5 s`); expiry still fails loudly with the captured screen.
There is no fixed delay substituted for completion and no assertion removed.

Validation: `cargo nextest run --release --workspace` passed all 1,381 active tests
with 4 skipped; `cargo test --release --workspace --doc` passed;
`cargo build --release --workspace` passed; `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets -- -D warnings` were clean. No new dependency.
