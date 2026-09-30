# 189 - The README describes today's cockpit and commands

Type: build
Status: claimed
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
