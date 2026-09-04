# 32 - Which transport a recorded run uses

Type: grilling
Status: resolved
Blocked by: 01, 11, 13, 18, 22, 26, 27

## Question

Ticket 01 settled "drive ACP, not print mode". That decision rested on a four-provider set that no
longer exists, and nobody has re-taken it since the set shrank to three.

**What ticket 01 actually measured.** Across the default print modes of `claude`, `cursor-agent`,
`gemini` and `hermes`, only five fields survived all four. Driving `gemini --acp` and `hermes acp`
widened the common set to streamed text, thought chunks, tool calls with arguments and results, and
answerable permissions. Cursor was named the permanent laggard, because it has no ACP.

**Every load-bearing part of that premise has since changed.**

- `gemini` and `hermes` were two of the four, and they were the two the ACP argument was built on.
  Ticket 18 moved Google to `agy`, ticket 22 unrouted hermes.
- Ticket 18 retired ACP for Google explicitly: `agy` is `-p=<prompt> --output-format stream-json`,
  its print mode carries per-step usage with thinking and cache-read tokens separated, and the ACP
  work "does not get built".
- `cursor-agent` never had ACP and still does not.
- Ticket 18's own summary sentence, that ticket 01 "still holds for `claude`, `cursor-agent` and
  `hermes`", is wrong on `cursor-agent` by ticket 01's own finding and stale on `hermes`.

**So ACP now covers exactly one routable provider, `claude`, and no ticket decides whether Hydra
builds an ACP client for one provider or drives all three through print stream-json.**

Two of ACP's three advantages have also been paid for elsewhere. Answerable permissions were its
headline, and tickets 26, 27 and 30 replaced them with a hook gate that works on all three providers
and refuses before the process starts, which is a decision Hydra made rather than a question it
answers mid-run. Bidirectional input exists on `claude` in print mode too, through `--input-format
stream-json`. What ACP would still buy over print mode on `claude` alone is the part to establish.

Settle:

- Does Hydra build an ACP client at all in v1, or is every recorded run print stream-json
- If ACP survives for `claude` only, what it buys that `claude --output-format stream-json` does not,
  stated as fields rather than as protocol quality
- Whether one provider on a different transport is worth the second adapter path, given that ticket
  09 already keeps the raw frame verbatim and normalises at read time, so the adapter is the cost
  and the record shape is not
- What this does to the record contract's `permission_request` event type, which ticket 09 populates
  from ACP and which no routable provider would produce under an all-print-mode answer
- Whether `--input-format stream-json` on `claude` is used for anything in v1, or specced and unbuilt

Blocking: `docs/harness-spec.md` section 6.7 carries a `<transport>` placeholder in the `claude`
command lines, and section 19.1 records this gap. Ticket 31 handed ticket 16 "the exact `claude`
command line" and it cannot be written until this closes.

## Why this is grilling rather than research

The facts are all measured already, across tickets 01, 11, 13, 18, 26, 27 and 30. What is missing is
a decision about what to build, and D1 says the destination is three documents rather than running
code, so the cost of a second transport is a cost the human is deciding to pay or not.

## Correction: the premise was wrong, 2026-09-04

**ACP covers zero routable providers, not one.** This ticket was filed saying ACP now covers `claude`
alone. That is wrong, and it makes the transport question narrower than the ticket assumed.

`claude` 2.1.257 has no ACP. `claude --help` returns zero matches for `acp`, and no ACP subcommand
exists. Checked 2026-09-04.

**Ticket 01 never claimed it did.** Re-read against its findings file: its capability table lists
Claude's bidirectional stdin as `--input-format stream-json` rather than ACP, and it records
reasoning as available from Claude in print mode with full `thinking` blocks and a signature. Its
ACP recommendation named `gemini --acp` and `hermes acp` and nothing else. The sentence "reasoning is
Claude-only on the default path, three of four if Hydra drives ACP" counts Claude on print mode plus
Gemini and Hermes on ACP.

So the four-provider ACP set was always Gemini and Hermes, and both left the routable set. Cursor
never had it. Ticket 18 retired it for Google. **Nothing routable speaks ACP, so the transport is
forced rather than chosen: every recorded run is print mode with `--output-format stream-json`.**

What remains is not "build an ACP client or not". It is what the spec does with ACP's leftovers, and
four `claude` print-mode flags that no ticket has ever considered.

## Answer, 2026-09-04

**Every recorded run is print mode with `--output-format stream-json`, because no routable provider
speaks ACP.** The transport was never a choice by the time this ticket opened. What the ticket
actually decided is what happens to ACP's leftovers and four `claude` flags no ticket had considered.

Settled in a grilling session with the human on 2026-09-04. Versions read at the top of the session:
`claude` 2.1.257, `cursor-agent` 2026.09.02-c22c1a3, `agy` 1.1.26.

### ACP is struck, and one watch replaces two

Not an extension point, not a spec section, one watch condition. The LiteLLM precedent does not carry
here: the raw-model plane is named and unbuilt because a live use case waits on it, the billed Gemini
fallback. ACP has none left. Its headline was answerable permissions and tickets 26, 27 and 30
replaced those with a gate that refuses before the process starts, which is strictly better for a
detached child nobody is watching. Its second was thought chunks, and print mode carries those on two
of three providers.

**The watch is named by capability rather than by protocol: a routable provider gains a channel that
can answer a permission request mid-run.** Ticket 18's watch, "`agy` gains a permission-response input
event", is the same watch in narrower clothes and is folded into this one. A watch on the protocol
would miss `agy` shipping the input event; a watch on the capability catches both.

### The `claude` flags

| Flag | v1 | Why |
|---|---|---|
| `--verbose` | **required** | `--print --output-format=stream-json` errors without it: `When using --print, --output-format=stream-json requires --verbose`. Measured, not read |
| `--include-hook-events` | on | emits `hook_started` and `hook_response` frames naming the hook, with its `exit_code`, `outcome` and stdout |
| `--include-partial-messages` | off | recorded runs are detached and unwatched, and no export format wants intra-message granularity |
| `--forward-subagent-text` | on | costs nothing under the four policies that deny subagents, and is the only record of their work under `permissive` |
| `--input-format stream-json` | specced, not built | ticket 13 made observation read-only and D5 caps the crossings at two |

`cursor-agent` has the exact twin of the third, `--stream-partial-output`, and it stays off for the
same reason. It has no equivalent of the other four. `agy` has neither a partial-output flag nor a
hook-events flag.

**Recorded runs redirect stdin explicitly.** Without it `claude` waits three seconds and warns
`no stdin data received in 3s, proceeding without it`. A detached child should not pay that, and the
warning is noise in the record.

### `--include-hook-events` is corroboration, never the source of truth

Ticket 30 found a provider's own reason text cannot identify which gate fired: on `agy` the first deny
short-circuits so Hydra's hook may never run, and on Cursor every hook's message is concatenated,
which produced `Rejected: t30 CUR-PROJ says allow`, quoting the hook that allowed. Its answer was a
decider field Hydra sets from the config it wrote.

That answer stands. The flag gives `claude` runs a second, independent source that names the hook
directly, and the uniform field is what the record contract keys on, because two of three providers
cannot produce these frames at all.

**It has a cost that gets written down rather than discovered.** The frames carry the hook's full
stdout, and the user's `PreToolUse` guards are 418 lines of Python that can print anything. So every
recorded `claude` run now carries content authored by a script the user edits. Ticket 19's four
redaction classes were written against provider frames and never anticipated this source. No new
class: the export gate scopes by path and hook stdout is not a path, so the allowlist still decides
whether the run leaves, and the write-time prefix sweep already reads every frame. What changes is
ticket 19's residue list, which gains hook stdout, so nobody reads "four classes with a fixed shape"
as coverage of it.

### `permission_request` is dropped: ten event types, not eleven

No routable provider offers a permission channel Hydra can be asked through. ACP is gone, and every
recorded run launches with the permissive flag plus the gate, so the decision is made before the
process starts. `permission_decision` still populates, from the gate, with ticket 30's decider field.

Ticket 09 says the list stays small on purpose and that growing it later is cheap because raw frames
survive. An event type nobody writes is a contract that lies to whoever builds from it.

### `message_delta` stays, behind a producer column

With both partial-output flags off, `message_delta` has no producer either, which is the position
that just cost `permission_request` its slot. The distinction that keeps it is real but does not do
the work on its own: no provider *can* produce a permission request any more, while two providers
ship a flag that would light message deltas tomorrow.

**So the record contract carries a producer column per event type**, and `message_delta` reads "none
in v1, lit by `--include-partial-messages` on `claude` or `--stream-partial-output` on
`cursor-agent`". Without that column the honest call was to drop it to nine types. The column pays
for itself across the rest of the list, since `thinking` has three different answers by provider and
`usage` has two conventions that overlap on three fields, and it is the artifact ticket 16's seam
check needs: "every field Ouroboros reads is one the harness actually writes" becomes a table lookup
rather than a re-read of thirty tickets.

### The flags stay off with a reason, not by neglect

Three consumers want partial messages and all three are deferred. The TUI, since ticket 07 chose
Textual partly for `MarkdownStream`. `hydra logs --follow`, which reads whole lines today. And a
**stall detector**, which nothing on this map has: ticket 13 derives liveness from a pid and a start
time, so a process that is alive and emitting nothing looks exactly like a process working hard, and
ticket 14's five failure classes have no entry for it.

**`agy` alone has a heartbeat in v1**, since it emits `step_update` with `state: ACTIVE` and ships no
flag to stop it. Anyone building the stall detector should not assume uniformity.

### Two fields found while probing, and what they do not change

`claude`'s result frame already carries `permission_denials[]` and a `subagent_stats` block with
`spawned`, `max_depth`, `spawned_by_subagents` and a `refused` breakdown by `depth_limit`,
`concurrency_limit` and `budget`.

**Ticket 27's denial of in-process subagents does not reopen.** Its argument was that a subagent's
work has no record on the trajectory Ouroboros learns from, and a count is not a trajectory. But both
fields go in the record, because they turn the denial into something checkable: `subagent_stats.spawned`
above zero under a policy that denies spawning means the gate leaked, and that is a bug nobody would
otherwise see. It also gives `claude` a native answer for the depth-cap blind spot ticket 27 had to
reason about indirectly.

### Recorded and learned-from are two different questions

Stated here because this ticket is where it came up and no document says it plainly. **A run outcome
that rung one must never learn from is still recorded in full.** D10 and ticket 09's never-discard
rule already require it, and the harness spec's outcome table has a column saying which outcomes rung
one ignores, which a reader could collapse into "discard". Denials, gate refusals, auth failures and
rate limits all land in the record verbatim.

Ticket 27 already names one use for the ignored half: "a run that failed on too narrow a policy is a
policy bug." So gated runs carry a signal about the **policy** rather than about the skill, and
nothing on this map reads it. That is not a blocker, since D5's seam requires every field Ouroboros
reads to be one the harness writes and not the converse. It goes to the fog beside the second-corpus
patch, which is the same shape.

### What this changes elsewhere

- **Ticket 01**: its headline recommendation, "drive ACP, not print mode", is superseded. The
  reasoning was correct for a four-provider set and both ACP speakers left it.
- **Ticket 07**: half of its framework case lapses. `MarkdownStream` was one of the two most
  expensive requirements and nothing streams token by token in v1. The diff viewer carries the choice
  alone. Textual is not reversed.
- **Ticket 09**: ten event types rather than eleven, a producer column per type, and two new `claude`
  fields.
- **Ticket 14**: gains a named absence, the stall detector, and a use for the outcomes rung one
  ignores.
- **Ticket 18**: its permission-response watch folds into this ticket's capability watch.
- **Ticket 27**: gains `subagent_stats` as a check on its own denial.
- **Ticket 16**: section 6.7's `<transport>` placeholder resolves, section 19.1 closes, and the
  producer column becomes the seam check's main instrument.
