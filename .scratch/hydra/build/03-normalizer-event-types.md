# 03 - Normalize a `claude` run into the ten event types

Type: build
Status: open
Blocked by: 02

## Goal

The read-time library that turns stored raw frames into the ten normalized events.
`claude` only. Nothing is written to disk; a normalized field on disk is the one thing
contract 11 forbids.

## Where the design says it

- Contract 7, all of it. 7.1 for the producer column and why it exists, 7.2 for the
  table, 7.3 through 7.9 for the types with disagreements.
- 7.6 for the tool-call adapter and its right to return `unclassified`.
- 7.7 for `permission_decision`, derived by re-evaluating the run's own policy.
- 9 for usage, and the two traps: all three report input net of cache, so store the
  parts and derive the total.
- 11 for stored against derived, and 8.5 for the three `claude` fields that stay in
  `raw` with named paths.

## Done when

- A library function reads a run file and yields events. Nothing normalized is written.
- `permission_decision` is derived from `run_start.policy` and the call, with `decider`
  set from the config Hydra wrote rather than parsed out of a reason string.
- The gate-leak check in 7.7 runs: a call the provider allowed and the policy denies is
  reported, and so is `subagent_stats.spawned` above zero under a policy that denies
  subagents.
- `usage` carries the parts, its `usage_source`, and never a bare null.
- Tests run against the real records in `~/hydra/runs/`, including the pre-gate ones,
  where `permission_decision` must be **absent** rather than empty, because those runs
  have no policy (contract 5.6 and 7.2.1).

## Watch for

**This is blocked on 02 for a reason about evidence, not code.** Written first, every
record it could be tested against has `permission_decision` absent by definition, so
the hardest part of section 7 ships untested. After 02 there are real denials, and
`claude`'s `hook_response` frames are an independent second source. A disagreement
between the two is a finding, not noise.

**Nothing upgrades an `unclassified` result to `ok`.** All three providers close a
failed run reporting success, so a label read off the terminator is a lie.

## What ticket 01 left you, added 2026-09-05

**The gate's refusal reasons carry a stable machine-readable prefix.** Every one starts
`hydra/<code>`: `protected-path`, `capability`, `out-of-scope`, `floor`, `unknown-tool`,
`no-path`, `no-tool-name`, `bad-payload`, and `hydra/allow` when the call goes through.
The reason arrives verbatim as the tool result with `is_error: true`, so a
`permission_decision` event can key on the prefix instead of parsing prose. If you need
a code that does not exist, add it in `src/policy/gate.py` and tell ticket 02.

**7.1's run-time table check is yours.** `src/policy/table.rs` holds `claude` 2.1.261's
twenty-five tools, measured 2026-09-05, and nothing compares it against what the
provider actually offered. The `system`/`init` frame lists every name at no cost, which
is the check 7.1 asks for: a name the table does not know is denied and recorded, so a
self-update that renames a tool breaks the next run loudly. Today the table is a
same-day measurement that ages silently.

## What ticket 02 left you, added 2026-09-05

**There are gated records now.** Three of them, from ticket 02's measurement, and the
first slice's three are the pre-gate ones this ticket's tests still have to treat as
`permission_decision` absent. Tell them apart by a null `policy`, not by
`hydra_version`: all six were written under `0.1.0`, because the version moved to
`0.2.0` after the measurement.

**A gated `claude` run carries the same decision in three places**, and 7.7 still says
the derived event is re-evaluated from `run_start.policy` rather than parsed out of any
of them. What they are worth is corroboration and a check on your re-evaluation:

- A `system`/`hook_response` frame whose `hook_name` reads `PreToolUse:<Tool>` and whose
  `output` is the gate's JSON, so `hookSpecificOutput.permissionDecision` is `allow` or
  `deny` and `permissionDecisionReason` is the `hydra/<code>` string. A matching
  `hook_started` frame precedes it. Both come from `--include-hook-events`, and the
  user's own `UserPromptSubmit` guards produce the same frame shape, so filter on
  `hook_event` rather than assuming every hook frame is Hydra's.
- The tool result: the reason string verbatim, with `is_error: true`.
- `result.permission_denials[]`, carrying `tool_name`, `tool_use_id` and the full
  `tool_input`. Contract 8.5 already names it.

**`decider` is a field Hydra sets from the config it wrote**, and on these records that
is always Hydra, because the gate answered. Do not parse it out of the reason string:
6.6 measured Cursor concatenating every hook's message, and one probe reached the model
quoting the hook that allowed.

**A run that a gate denied still exits 0 and still reports `subtype: success`.** Ticket
04 derives the outcome, and this is the shape it will meet.
