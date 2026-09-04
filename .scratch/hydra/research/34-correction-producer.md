# 34 - Does any vendor transcript record a correction as a pair?

Ticket: [34 - Does any vendor transcript record a correction as a pair?](../issues/34-correction-producer.md)
Type: research. This file reports measurements. It proposes mechanisms but decides nothing.

Measured 2026-09-04 on this machine, 13:10 to 15:48 local. The method is corpus mining rather than
fresh probes: these three stores hold months of real use, and a rejection that already happened is
better evidence than one staged for the ticket. Where the corpus ran out, the `claude` binary itself
answered, and what neither could reach is marked unmeasured at the end.

## Versions under test

Read at 15:45 local and again at 15:48 after the last measurement. Identical both times.

```
claude        2.1.257 (Claude Code)
cursor-agent  2026.09.02-c22c1a3
agy           1.1.26
```

Same three builds the map recorded at 11:40. Nothing self-updated during this ticket, which is worth
saying out loud given `agy` has moved five times in four days.

## Probe hygiene

Everything created for this ticket lives in
`/private/tmp/claude-501/-Users-USER/63edcca5-a9f3-4f9c-ba02-4bf0272e02c9/scratchpad/t34/` and is
scan scripts only. No provider was launched. No user-scope config was written:
`~/.claude/settings.json` still has its 2026-08-31 22:57 mtime and sha
`7fd1e3d3637f3f62dc77d77268afebba015ae4a5`, the same sha ticket 30 recorded, and neither
`~/.cursor/hooks.json` nor `~/.gemini/config/hooks.json` exists.

The user's own transcripts were read and never written. Quotations below are the shortest excerpt
each finding needs. Nothing secret turned up in the records that were read, so nothing was redacted.

## The short version

**One provider of three can produce the pair, and it is `claude`.** It is also the only one that
separates a human's rejection from an auto-deny with a field rather than a guess.

| | rejected content recoverable | chosen content recoverable | human vs auto separable | human's words kept |
|---|---|---|---|---|
| `claude` | yes, 35 of 35 | yes, by chain walk | yes, `toolDenialKind` | yes, `userFeedback` |
| `cursor-agent` | yes, by `toolCallId` | yes, by chain order | no | no |
| `agy` | yes, the call survives | yes, next step | no | no |

The catch is real and it needs saying plainly. **The corpus contains no human interactive rejection
on any provider.** All 35 denials on `claude` are auto-denies or SDK-callback denials, and every
Cursor and `agy` denial traces back to Hydra's own ticket 26 and ticket 30 probes. So the pairing
mechanism is verified 35 times over, the discriminator is confirmed from the binary's own source,
and the specific conjunction the contract names has zero observed instances. That last gap needs a
human at a keyboard and nothing else will close it. The checklist is at the end.

My read: `correction` should stay on the list with `claude` as its only producer, and the contract
should say so rather than implying three. But that is ticket 16's call, not this file's.

## `claude`

### The denial record

A denial writes a `user` record carrying three fields that ordinary records do not have:
`toolDenialKind`, `userFeedback`, and `sourceToolAssistantUUID`. The last is a back-pointer to the
assistant record that made the call. Recovering the rejected content is then a two-step lookup:
follow `sourceToolAssistantUUID`, then match the `tool_use` block whose `id` equals the denial's
`tool_result.tool_use_id`.

**That lookup succeeded on 35 of 35 denials in the corpus.** No misses, no ambiguity, across five
project directories and both entrypoints.

### The census

Every record carrying `toolDenialKind`, across all 16 project directories in `~/.claude/projects/`:

```
kind              entrypoint   next assistant act    count
permission-rule   sdk-cli      text                    12
permission-rule   cli          tool_use, same tool      8
automode-blocked  cli          tool_use, same tool      5
user-rejected     sdk-cli      text                     3
permission-rule   cli          text                     2
permission-rule   cli          tool_use, other tool     2
permission-rule   sdk-cli      tool_use, same tool      1
automode-blocked  cli          nothing, end of file     1
automode-blocked  cli          text                     1
                                                  total 35
```

Sixteen of the 35 are followed by a replacement tool call, and 14 of those retry the same tool with
different input. That is the pair shape preference export wants: same tool, rejected input against
chosen input, in one trajectory.

`userFeedback` is null in all 35. Nobody typed a redirection, because nobody was asked.

### The discriminator, from source

The ticket worried that a human rejection and an auto-deny carry the same label. In the
`tool_result` text they do. In the record they do not, and the mapping is readable in the binary:

```js
function qLn(e){if(e.behavior==="ask")return"user-rejected";
  let n=e.decisionReason;
  if(n.type==="classifier"&&n.classifier==="auto-mode"){
    if(n.reason===tse)return"automode-unavailable";
    if(n.noVerdict)return"automode-parsing-error";
    return"automode-blocked"
  ...
```

`user-rejected` means the decision took the **ask** path. It does not mean a human. Under
`entrypoint: sdk-cli` the thing answering the ask is the SDK's permission callback, which is code,
and that is exactly what the three `user-rejected` records in the corpus are: ticket 30's own probe
harness saying no. The field name is misleading and a harvester that trusts it alone will label a
scripted deny as human preference.

`entrypoint` is the second half of the test. `cli` is a human at a terminal; `sdk-cli` is a program.
So the human rejection is the conjunction, not either field alone:

```
toolDenialKind == "user-rejected"  AND  entrypoint == "cli"
```

The same ask path gates the human's words:

```js
userFeedback: Ot.behavior === "ask" ? Ot.userFeedback : void 0
```

`userFeedback` is populated only where a human could have typed something, which is good news for
the record: the field exists precisely to carry the redirection, and it is absent by construction
everywhere a machine decided. A denial with `userFeedback` set is a human who both refused and said
what to do instead, which is the richest form of the event the contract could hope for.

### The candidate harvest rule

Stated as a traversal, with the field names a harvester needs:

1. Find `user` records where `toolDenialKind` is present.
2. Human filter: keep those where `toolDenialKind == "user-rejected"` and `entrypoint == "cli"`.
3. Rejected content: follow `sourceToolAssistantUUID` to the assistant record, take the `tool_use`
   block whose `id` matches the denial's `tool_result.tool_use_id`. Its `input` is the rejected
   content, its `name` the tool.
4. Human's words: `userFeedback`, when set.
5. Chosen content: walk children by `parentUuid` from the denial record, skipping `attachment` and
   other non-message types, to the first `assistant` record holding a `tool_use` block. Its `input`
   is the chosen content.
6. Emit a pair only when step 5 found a `tool_use` and its `name` matches step 3's. A `text` reply
   means the agent argued instead of retrying, which is a rejection without a correction.

Step 6 is where the yield goes. On the auto-deny population it keeps 14 of 35. There is no reason to
expect the human population to behave the same way, and no data either way.

### What a rejected edit holds

Unmeasured, and worth flagging. Every one of the 35 denials in the corpus is `Bash` or `Write`. On a
rejected `Edit` the rejected content is a diff sitting in the `tool_use` input as `old_string` and
`new_string`, and the record shape gives no reason to think it is treated differently. But no
rejected `Edit` exists in the corpus to confirm it.

## `cursor-agent`

Chats live under `~/.cursor/chats/<workspace>/<agent-id>/store.db`, a SQLite file with two tables:
`blobs` (content-addressed) and `meta`. Thirty-three stores exist on this machine.

**Two storage formats are in play.** Older chats hold plain JSON messages. Newer ones hold protobuf,
and `meta` carries a `blobEncryptionKey`. A harvester has to handle both, and the protobuf half has
no schema Hydra owns. Ordering is not implicit either: blobs are content-addressed, so the sequence
comes from `latestRootBlobId` in `meta`, whose blob holds the ordered list of child hashes.

The message shape across the JSON corpus is flat OpenAI-style:

```
490 role, 490 content, 383 id, 90 providerOptions
roles: assistant 211, tool 172, user 74, system 33
blocks: assistant/tool-call 172, tool/tool-result 172, assistant/text 115,
        user/text 45, assistant/redacted-reasoning 42, assistant/reasoning 7
```

Pairing is available: `tool-call` carries `toolCallId` and the matching `tool-result` echoes it.

A refusal appears as the tool result's `result` string, prefixed `Rejected: `:

```json
{"type":"tool-result","toolCallId":"tool_7ee1c470-...","toolName":"Shell",
 "result":"Rejected: Command execution was blocked by a hook: t30 CUR-PROJ says deny\n\nTo view or modify configured hooks, go to Cursor Settings > Hooks.\n\nAgent note: Do not suggest workarounds to the blocked tool."}
```

**There is no denial field anywhere.** The top-level keys are `role`, `content`, `id` and
`providerOptions`, and nothing else. Everything a harvester could learn about why the call died is
inside a prose string, which lines up with what ticket 30 already found: the reason text does not
identify which gate fired.

Two results carry a bare `"Rejected: "` with an empty reason, which looked at first like it might be
the human path. It is not. In chat `ca196f9f` the model's own next line explains it:

> The shell command 'exit 3' was rejected, likely due to sandbox restrictions.

That chat is ticket 14's `p-cursor` probe. The empty reason is the sandbox refusing, not a person.
So on Cursor the discriminator between human and auto is the presence or absence of hook prose in a
free-text field, which is an accident of how each gate phrases itself rather than a marker. A
harvester cannot rely on it.

Verdict: rejected and chosen content are both recoverable, and human versus auto is not.

## `agy`

Transcripts are at
`~/.gemini/antigravity-cli/brain/<id>/.system_generated/logs/transcript_full.jsonl`, 30 of them.
Record keys across the corpus: `step_index`, `source`, `type`, `status`, `created_at`, `content`,
`tool_calls`, `thinking`. Types: `PLANNER_RESPONSE`, `USER_INPUT`, `GENERIC`, `SYSTEM_MESSAGE`.

The interesting part is what a denied call looks like. Here is a whole transcript from ticket 30's
hook probe, complete, nothing elided:

```
rec 1  step 0  USER_EXPLICIT  USER_INPUT        content: "<USER_REQUEST>\nRun the shell command: echo T30MARKER..."
rec 2  step 1  MODEL          PLANNER_RESPONSE  tool_calls: [{"name":"run_command","args":{"CommandLine":"echo T30MARKER",...}}]
rec 3  step 3  MODEL          PLANNER_RESPONSE  content: "The command was blocked (`tool call denied by pre-tool hook: t30 AGY-B says deny`). Stopping immediately as requested..."
```

**Step 2 is missing.** `step_index` jumps from 1 to 3, and the denial that occupied that step is not
in the file. `agy` drops the record rather than writing it, so the only structural trace a rejection
leaves is a discontinuity in `step_index`, and the only description of it is the model's own prose
in the following step. `status` reads `DONE` on the blocked step, the same as any other.

The rejected content does survive, in `tool_calls` on the preceding record. But `tool_calls` carries
`name` and `args` and no id, so there is nothing to pair on except adjacency, and adjacency is
exactly what the missing step breaks.

Verdict: the rejected call survives, the rejection does not, and human versus auto is not separable.
Of the three this is the weakest, and inferring a rejection from a gap in an index is not something
I would build a training corpus on.

`conversation_summaries.db` was checked and holds summaries rather than turn-level records, so it
adds nothing here.

## What this means for the contract

Stated as findings, since a research ticket decides nothing.

- `correction` has exactly one credible producer, `claude`, harvested by the six-step rule above.
  Section 7.1's producer column would carry one provider rather than three.
- Two providers would sit in that column as gaps with named reasons: Cursor cannot separate a human
  from a hook, and `agy` does not record the denial at all.
- The `usage` precedent from ticket 16 applies: a type whose behaviour differs this sharply by
  provider belongs in the column beside the type rather than in prose.
- Ticket 19 ruled policy denials out of preference export in writing. Every pair in this corpus is
  precisely that excluded class, so the 14 same-tool pairs measured here prove the traversal works
  and are themselves unexportable. The mechanism is verified on data the contract forbids.
- `preference.jsonl` therefore has a source in v1 only if a human ever rejects a tool call in an
  interactive `claude` run. That is a plausible thing to happen and it has not happened yet.

## Unmeasured, and the checklist to close it

One thing is missing and it cannot be simulated: **a human rejecting a tool call at an interactive
`claude` prompt**. Generating one from a script produces `entrypoint: sdk-cli`, which is the very
case step 2 of the harvest rule filters out, so a staged rejection would not test the rule. It would
only re-measure what the 35 records already show.

Four minutes at a keyboard closes it:

1. `cd` to a scratch directory and run `claude` interactively, with no `--permission-mode` flag, so
   the permission prompt is live.
2. Ask it to run a shell command that will prompt, for example `echo T34MARKER`.
3. At the prompt, choose the option that rejects **and lets you type a reason**, and type something
   specific like `use printf instead`.
4. Let it try again, and let the second call go through.
5. Repeat once on an `Edit`, so the rejected-diff shape gets an instance too.

Then read the new file in `~/.claude/projects/<slug>/` and confirm four things: `toolDenialKind` is
`user-rejected`, `entrypoint` is `cli`, `userFeedback` holds the typed words, and the `parentUuid`
walk reaches a replacement `tool_use`. `t34/pair.py` in the scratch directory prints all four in one
line if you point it at the file.

If that comes back as expected, `correction` keeps its producer and the harvest rule is measured
rather than derived. If `userFeedback` turns out empty, or the chain does not reach a replacement
call, the type is weaker than this file suggests and ticket 16 should hear about it before the
contract ships.

## Possible fog

Two things surfaced that are not this ticket's to settle.

- **Cursor's two transcript formats.** Ticket 13's harvest reads the vendor's own transcript at exit.
  This file found JSON in older chats and protobuf plus a `blobEncryptionKey` in newer ones, with
  ordering carried by a root blob's hash list. Whether harvest can read both, and what it does when
  it cannot, is unexamined. This touches ticket 13's harvest rather than the correction event.
- **`agy` drops steps from its own transcript.** The `step_index` gap is a general property, not
  something about denials. If `agy` omits other step kinds too, ticket 13's harvest is reading a
  transcript with holes in it on one provider of three.
