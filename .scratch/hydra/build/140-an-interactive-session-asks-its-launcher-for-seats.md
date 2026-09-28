# 140 - A gated interactive session asks its own launcher for seats

Type: build
Status: open
Blocked by: 138, 139

## What

Map 82, answers 1, 2 and 5, which amend map 64. In a gated interactive launch
(`src/interactive.rs`, `run_gated`), `hydra run --interactive` waits outside the seatbelt while
`claude` runs inside it. The face (`src/face.rs`, `call`, near 234) refuses today with
`hydra/seatbelt-delegation`. After this ticket, for an interactive session only, the face hands
the request to that waiting launcher over one channel, the launcher checks it against
`config/panels.json`, launches each seat as a child run with the session as `parent_run_id`, and
returns the results marked as foreign (`marked`, near 255), exactly as the face does outside the
seatbelt.

The channel is the second channel D5 rules on, admitted by 82 for this one use: per run, from
that run's face to that run's launcher, carrying a launch request and returning a result, nothing
else. Pick the mechanism (a unix socket in the run's spawn directory, created by the launcher
before exec, is the obvious one) and the seatbelt rule that lets only the face reach it. If the
seatbelt cannot scope it to one socket path, stop and say so.

A request that matches no shape is not launched in this ticket: the face answers
`hydra/approval-required` with a request id and records the request. 141 builds the approval.

## Do

Tests first, each shown failing before the code, all on stubs, no paid provider:

1. A launcher test: a stub interactive session's face sends a request matching a fixture shape;
   the launcher starts the seats as child runs of the session (their `run_start` carries the
   parent), respects the shape's cap per hour, and the face returns each child's text inside the
   boundary.
2. A request for a shape the session's policy may not ask for, or a seat above the session's
   policy, is refused by name and launches nothing.
3. A request matching no shape comes back `hydra/approval-required` with an id, and the request is
   on the session's record.
4. From inside the seatbelt, the socket of another run is unreachable (kernel refusal), and a
   non-interactive run's face still answers `hydra/seatbelt-delegation`.
5. The session's policy gains `mcp_call` scoped to `mcp__hydra__*` only. Say whether the capability
   table supports that scope today; if not, build it with a gate test that `mcp__claude-in-chrome__*`
   stays refused.

Harness spec 9 (the face) and 10 (interactive mode) and D5's note, with `IMPLEMENTATION.md`.

## Outcome
