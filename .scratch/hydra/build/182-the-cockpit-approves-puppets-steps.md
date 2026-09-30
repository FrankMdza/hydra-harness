# 182 - The cockpit approves Master of Puppets steps

Type: build
Status: resolved
Blocked by: 181, 184

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

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f346-2bed under `standard`. The session
applied it, resolving one conflict in `set_snapshot` with build 186 (both lines kept). The reader
runs `pending <goal> --json` at most every 10 s and when the goal changes, in its own process group
with a 2 s budget (`bounded()` kills the group if late), and refuses any item whose argv does not
end in `--expect-revision` of its own revision. Steps list in the waiting lane; `a` shows the exact
argv, `y` runs it with a 5 s wait and shows the helper's last line; exit 2 reads "the goal moved
on; nothing approved"; a helper still running at 5 s is reaped on a thread, not killed. The
cockpit never writes a goal log. Seven tests; fmt and clippy clean, `--test cockpit` 68, `--test
stop` 12, `--lib` 224 on master.

What the ticket assumed that is false: an approval does not move the goal's revision (only `task`
and `assignment` events do), so a second `a`/`y` within one 10 s `pending` window would record the
same approval twice. The cockpit hides a step it approved until a newer `pending` read; the helper
side is build 185.

