# 03 - Normalize a `claude` run into the ten event types

Type: build
Status: resolved
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

## Outcome

`src/normalize/` reads a stored `claude` run into contract 7's events, `read_file` from a
path and `read_with` from text against a layout the caller names. Nothing is written.
`tests/normalize.rs` has 22 tests: synthetic records for the frame shapes and the two
findings this machine has never produced, and four that walk the nine records under
`~/hydra/runs/`.

**Eight of the ten types have a producer, and the two dark ones are dark for the reasons
the contract gives.** `message_delta` needs `--include-partial-messages` (7.4) and
`correction` needs an interactive harvest (7.9). Both are in the enum with a doc comment
saying so, because a type the reader cannot name is a type nobody knows is missing.

**`permission_decision` re-runs the run's own policy through the real gate script**
rather than through a second implementation of the rules in Rust. `src/policy/replay.rs`
compiles the policy once per run and feeds the gate one payload per call on stdin, with
`-c` so nothing is written to disk. The trade is in `IMPLEMENTATION.md`: a
`/usr/bin/python3` process per tool call, and that interpreter is now a read-time
dependency. A Rust copy of the seven path denials, the scope resolution and a floor that
tokenizes shell with `shlex` would drift the day either side is edited, and a copy that
said `allow` where the gate said `deny` would report a gate leak that never happened.

**`decider` reads the `hydra/` prefix and never the prose after it.** Every reason the
gate writes carries the code ticket 01 made stable, so a `PreToolUse` deny with no prefix
is somebody else's. That is what makes `external` derivable at all, and it is not the
thing 7.7 forbids: the prefix is a code Hydra writes, the reason text is what the Cursor
probe in 6.6 was about.

**What the nine records answer, checked per call rather than in aggregate.** Three
pre-gate runs produce no `permission_decision` at all, which is 7.2.1 and 5.6. Six gated
runs produce exactly one per call. The three `read-only` `Write` calls re-derive to
`deny` under `hydra/capability`, and ticket 06's two `Bash` calls split: the one naming
`~/hydra/runs` re-derives to `deny` under `hydra/protected-path`, the one writing inside
its own worktree to `allow`. No record reports a gate leak, a subagent leak or a
disagreement.

**What the ticket assumed that turned out false.**

- **`is_error` is not always present on a `claude` tool result.** The `standard` run's
  successful `Write` came back with the field absent, so the adapter answers
  `unclassified` there. 7.6 forbids reading an absent field as success, and this is the
  first measured case of it on this provider rather than on `agy`.
- **A hook frame carries no `tool_use_id`**, so corroboration is joined by tool name and
  order off `hook_name`. Ticket 02's run fired four `PreToolUse:Bash` responses for one
  call, three of them with an empty `output`, so filtering on a parseable
  `permissionDecision` picks Hydra's out. Two calls to one tool inside a single assistant
  frame would be joined in hook order, which is an assumption. It reaches only the
  disagreement check, never the decision.
- **`policy_sha` does not cover the gate script.** It hashes `policies/`, and the rules
  live in `src/policy/gate.py`. A record can re-evaluate against gate code that changed
  while its sha matched, and a disagreement is what reports that, not `StalePolicy`. The
  nine records happen to be clean, so the skew is latent rather than observed.
- **The floor-escape gap now has a reader.** The normalizer has to pass
  `floor_lifted: false`, so a run where a human typed `--allow-floor-escape` re-derives
  to a `hydra/floor` deny it never got, and then reports a gate leak on top. Inventing a
  field is a contract change, so the gap stayed where ticket 02 left it.

**`Run::result` keeps the terminator frame verbatim, and it is not a normalized
field.** 8.5 names three `claude` fields that stay in `raw` and two of them,
`permission_denials` and `subagent_stats`, live on that frame; the leak checks read them
there. Contract 8's outcome derivation reads three more off the same frame, so keeping it
is what stops ticket 04 re-opening the file. The third of 8.5's fields,
`rate_limit_event`, is its own frame type with no reader until rung two's early abort, so
nothing keeps it.

**7.1's run-time table check is done and quiet.** It reads the `system`/`init` frame,
which lists every tool at no cost. `claude` 2.1.261 offered exactly the twenty-five names
`table.rs` holds, so a rename now breaks the first record written after it.

## What the code review changed, added 2026-09-05

Eight findings, all real, all fixed with a test each. Two were correctness bugs on shapes
the records already have.

**The hook join had no upper bound, so one call could absorb another's refusal.** The
join runs on tool name and order because the frame carries no `tool_use_id`, and it
searched forward without stopping. A `standard` run with two `Bash` calls where the user's
guard denies only the second gave call one `deny`/`external` carrying call two's reason,
and call two an `allow`. Both decisions wrong, effectively swapped. The window now closes
at the next call that came later, and
`an_external_deny_belongs_to_the_call_it_answered` fails without it. The same bound
covers the gate answer, where the symptom was a fabricated disagreement.

**`Finding::Ungated` was declared and never constructed.** It is wired now, and the check
is honest on this build because `src/provider/claude.rs` passes `--include-hook-events`
on every recorded run: a call with no gate answer and no refusal in Hydra's own voice is
the fail-open case 6.6 measured. All six gated records carry a gate answer for every call,
so the corpus test asserts the finding stays quiet.

**A torn line no longer fails the whole record.** The writer syncs per line, so a killed
run can leave half a line at the tail, and contract 6 calls that an abandoned run rather
than a corrupt file. The reader disagreed and returned `Err` for the file. It now costs
its own frames and a `Limit`.

**Four smaller ones.** A `usage` object holding none of section 9's counts reported
zeroes from `source: provider`; it says `unavailable` now. A record naming a policy with
no `policy_sha` derived a full set of decisions and said nothing about answering for
today's `policies/`; it emits a `Limit`. Decisions were keyed by `call_id`, so two blocks
with no id collided and one call lost its decision; they are spliced positionally now.
And two text blocks in one tool result were concatenated with no separator, which put the
seam inside a word for every reader downstream.

**Findings are not events**, and there are seven: `GateLeak`, `SubagentLeak`,
`Disagreement`, `Ungated`, `UnknownTool`, `StalePolicy` and a bare `Limit` for anything
the derivation could not do.

`Limit` has five producers. A policy or a gate that would not answer. A file with no
`run_start`, which otherwise reads exactly like a pre-gate run, because both carry a null
policy and no decisions. A line that would not parse. A record that names a policy and
carries no `policy_sha`. And a call the record stops after, which was going to be a false
gate leak: a run Hydra was killed during can end between a `tool_use` block and its
result, and a policy that denies that call has no evidence either way, so the finding says
unknown and the decision still follows the policy.

None of the ten event types derives `Serialize`, so a caller who wants to put one in a
file has to say so out loud first. That is contract 11's rule expressed in the type
system.
