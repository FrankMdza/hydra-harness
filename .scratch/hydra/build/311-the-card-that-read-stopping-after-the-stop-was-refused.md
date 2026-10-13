# 311 - The card that read `stopping` after the stop was refused: one frame late in the cockpit

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from build 306's Outcome, which fixed the test and left the
product finding: "a real cockpit with a chat can show that same transient `stopping` frame
after it internally consumes a refusal in its post-paint poll. The next draw paints `live` and
the refusal message."

## What

`App::draw` (`src/cockpit/app/draw.rs` line 28) polls the stop threads first and then paints
the cards, so a card is painted with what the poll found. `Host::draw` (`src/cockpit/host.rs`)
calls `self.app.draw(...)` and then `settle_stops` (line 197), which polls again and consumes
a refusal that arrived during the paint. That second poll sets the message and clears the
in-flight stop, but the frame on screen was painted before it: the card says `stopping` and
no message shows, until the next tick redraws. On a loaded machine the refusal lands in that
window often enough that 306 saw it once in seven suites. A user who pressed `y` and reads
`stopping` for one tick is misled for one tick, which is small, and the frame is not what the
state says, which is the kind of thing this cockpit is supposed to never do.

## Do

1. Read `Host::draw`, `Host::settle_stops`, `App::draw` and `App::poll_stops` and say in the
   Outcome why the second poll exists after the paint (what it does that the first does not).
2. Make the frame match the state: settle before painting, or poll once per draw and have
   `settle_stops` read what that poll found rather than poll again. Keep whatever the second
   poll was for (the chat note, the message) and keep its ordering with respect to the chat
   side, which the tests in `tests/stop.rs` pin.
3. `tests/stop.rs`: build 306 changed `reported` so the test waits for the refusal before it
   captures a screen, which hides this. Add one test that does not wait: it sends the refusal
   so that it arrives during the draw (the fixture controls the stub watcher), draws once, and
   asserts the captured frame reads `live` with the message set. Under the old code it must
   fail; say that you saw it fail.
4. `cargo nextest run --release --workspace` and `cargo test --release --workspace --doc`
   green, and the stop tests fifty times under eight `yes` workers, as 306 and 308 did,
   zero failures.

## Done means

One draw, one state: a refused stop never paints `stopping`. The new test, the counts, and
the Outcome's answer to step 1. Amend `IMPLEMENTATION.md` where the cockpit's stop path is
described, and `COCKPIT.md` if it describes the draw order.

## Outcome

Resolved 2026-10-13. The chat-backed host now settles stops before its final
paint. The original post-paint poll is retained: unlike the first poll in
`App::draw`, it collects answers that arrived while the panels were being
painted and delivers every queued note to the chat in the same turn, in start
order. The first poll updates card labels and messages and queues drafts; it
does not paste those drafts or transfer the keyboard to the chat.

`App::poll_stops` now reports whether it consumed an answer. `Host::settle_stops`
returns that fact after draining drafts. If that poll consumed an answer, or a
draft changed the keyboard side, the host clears and repaints both panels from
the settled state using `App::paint`, which does not poll stops again. There is
no unbounded redraw loop: at most one additional paint per draw. The final card,
message and focus agree; a reply arriving during that final paint waits for the
next draw without changing the state behind the displayed frame. Notes remain
unsent and ordered, refusals paste nothing, and standalone drawing is unchanged.
Clearing the panels also prevents trailing text from the previous paint surviving
when a label gets shorter.

Added `tests/stop.rs::a_refusal_during_draw_paints_live_and_its_message_in_that_frame`.
The fixture rendezvous with the stop's record reader through a FIFO, holds the
reader before drawing, then replaces the path with an invalid stub record and
releases the held reader after 10 ms. A deliberately busy snapshot of 10,000
cards keeps painting across that release. It captures exactly one draw, without
`reported` or a completion wait before capture, and asserts that the draw consumed
the refusal, the card is live with its refusal message in the captured frame,
no stopping label remains, the keys stay on the cockpit and the chat got no bytes.
The delay schedules the stub response, rather than substituting a sleep for
waiting on the state being asserted. The fixture fails loudly if its answer was
not consumed during the draw.

Observed failure on the original source, restored temporarily for the regression
check: the new test passed its settled-state and refusal-message assertions but
failed its live-card screen assertion. Its captured frame showed `stopping` and
`stopping 01996d3c`, while the app's message already said `was not stopped`.
The fixed source passes. No existing test or assertion was changed.

Amended the stop-path description in `IMPLEMENTATION.md` and the seat-stop
explanation in `COCKPIT.md`. No new dependency.

Validation on the final source:

- `cargo nextest run --release --workspace`: 1,385 passed, 4 skipped.
- `cargo test --release --workspace --doc`: passed.
- `cargo build --release --workspace`: passed.
- Fifty invocations of `cargo nextest run --release --test stop` with eight
  additional `yes` workers held throughout: **0 failing suites / 50**,
  **700 passed test executions / 700**, including all 50 regression executions.
  The shell exit trap terminated and waited for all eight workers afterward.
- `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`: clean.

The seeded changes in unrelated tickets and research were left intact.
