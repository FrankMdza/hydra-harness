# 311 - The card that read `stopping` after the stop was refused: one frame late in the cockpit

Type: build
Status: claimed
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
