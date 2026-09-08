# 20 - A run's sequence as a graph in the terminal

Type: build
Status: resolved
Blocked by: 18

## Goal

Draw one run as the sequence it was: every event in order, what tool it called with what
arguments, what the gate said, what it cost, and what came next. The record has all of it and
nothing draws it. Build ticket 18 gives `hydra show <run-id> --events` as JSON, and this ticket
is the renderer over that JSON, on Herdr's side rather than in Hydra, so D3 holds: Hydra draws
nothing and knows nothing about the pane it is drawn in.

## Where the design says it

- Harness spec 9 and 17 on `hydra show`, build ticket 18 for the JSON shape.
- Map tickets 44 and 45: the viewer lives beside the bridge, in `integrations/herdr/`.

## The JSON it reads

One object: `run_id`, `projection` (contract 17's nine fields), `events` (an array), `findings`.
Each event has `seq`, `type`, and that type's fields: `message` has `role`, `text`, `block`;
`thinking` has `text`, `signed`, `block`; `tool_call` has `call_id`, `name`, `arguments`,
`capability`; `tool_result` has `call_id`, `status`, `output`, `truncated`;
`permission_decision` has `call_id`, `decision`, `decider`, `reason`; `usage` has `source` and
`counts`; `run_start` and `run_end` carry the record's body, and `run_start.ts` and `run_end.ts`
are timestamps.

## The slices

**`integrations/herdr/bin/hydra-graph`.** Python 3, standard library only. Reads that JSON from a
file argument or stdin. Prints one line per event in `seq` order: a fixed-width column for `seq`,
one for the type, then a one-line summary. A `tool_call` shows its name and a 60-character cut of
its arguments; its `tool_result` is joined by `call_id` and shown indented under it with the
status and the first line of output; a `permission_decision` shows the decision, the decider and
the reason on the line of the call it decided; `thinking` shows its length and whether it had
text; `message` shows the role and its first line; `usage` shows total tokens. A header line
carries the run id, provider, policy, outcome and total tokens from `projection`. A tree glyph
in the left margin joins a call to its result and its decision. No colour. Under 200 lines.

**`--follow <run-id>`.** Re-runs `hydra show <run-id> --events` every two seconds and prints
only events with a `seq` it has not printed, so the graph grows in a pane while the run works.

**The test.** `integrations/herdr/tests/test_graph.py` with one fixture JSON written by hand,
six events, asserting the header, the join of a call to its result and decision, and that a
second render of a longer fixture prints only the new lines.

## Done when

- `hydra show <id> --events | hydra-graph` draws a real run from `runs/`.
- `hydra-graph --follow <id>` grows while a run is live.
- The Python test passes with `python3 -m unittest discover -s integrations/herdr/tests`.

## Outcome

Resolved 2026-09-07. Written by `cursor-agent` on Composer 2.5, run 01a07f02-36a9 under
`standard`, against the JSON shape this ticket specified and before ticket 18 landed: 160 lines,
four tests, 628k tokens with 573k cache reads, one pass. Applied to master and then adjusted by
hand once the real `hydra show` output existed: `run_start` and `run_end` fields sit under
`body`, `usage.counts` has no total so the renderer sums the five counts, and a result printed
under its call was printing a second time when the loop reached it. Renders run 01a07f08 in
five lines and the 839-frame Sonnet run that built ticket 18 with every call joined to its
result and its gate decision.
