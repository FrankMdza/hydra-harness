# 189 - The README describes today's cockpit and commands

Type: build
Status: resolved
Blocked by:

## What

The README's `hydra cockpit` passage (near line 141) and command table were written before builds
166 to 187. Since then: the table's `done` column and column priority, run outcomes in the status
cell, `d` denies, `p`/`e` the policy pane, the header's tokens per minute and window share, the
dragon counting the watched session's seats, tool calls in the detail with `r` for the raw tail
and secrets masked, `v` a review, `m` stepping in on a seat, and `a` on Master of Puppets steps.
The command table gained `stop` and `deny`. A stranger reads this first.

## Do

1. Rewrite the `hydra cockpit` passage in `README.md` so every key and pane it names exists in
   `src/cockpit/app.rs` today (check each against the code, and list them in your report with the
   line that handles each), and nothing that exists is missing. Keep it short: one paragraph for
   what it shows, one line per key. Say what it writes: seat approvals and denials, Master of
   Puppets approvals through the helper, stops; and what it never writes: policy files, goal logs.
2. Check every other README line that names a command or count against `src/cli.rs` and fix what
   is stale. Touch nothing else.
3. Prose follows plain English, no em dashes, no marketing words.

## Done when

- The report lists each key with the code line that handles it, and each README line changed.

## Outcome

Resolved 2026-09-30. Written by `claude` sonnet on run 01a0f3bb-6c3b under `standard`, each key
checked against its handling line in `src/cockpit/app.rs`. The session applied it and added the
chat note to the `m` row. The cockpit passage now names the table's columns and priority, the
lanes with the Puppets steps, the detail's tool calls and masking, the dragon's session count and
the header's tokens and window share, a key table, and what the cockpit writes and never writes.

