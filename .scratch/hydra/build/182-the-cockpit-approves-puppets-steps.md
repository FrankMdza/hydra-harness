# 182 - The cockpit approves Master of Puppets steps

Type: build
Status: open
Blocked by: 181, 180

## What

The panel of 2026-09-30 (unanimous B, decision log): the cockpit shows what a goal waits for and,
after `y`, runs the approval as a child process, so `puppets.py` stays the only writer of its
log. Build 181 gave the helper `pending <goal> --json` and `approve ... --expect-revision N`.

## Do

Tests first in `tests/cockpit.rs` with a stub `puppets.py` script (a scratch file that prints a
fixed `pending` JSON and records its argv), each shown failing before the change.

1. The snapshot runs `pending <goal> --json` on the goal the table shows, at most once per 10
   seconds, with a 2 s timeout, and shows the items in the `waiting approval` lane beside seat
   requests, labeled by kind and ref.
2. `a` on such an item opens the confirmation with the exact argv it will run; `y` runs it as a
   child (the argv from `pending`, `--expect-revision` included), waits at most 5 s, and shows the
   helper's exit line; exit 2 reads "the goal moved on; nothing approved".
3. The cockpit never writes the goal log. Inside a seatbelt `a` on these refuses, as for seats.
4. `IMPLEMENTATION.md`, the cockpit's writes paragraph.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.
