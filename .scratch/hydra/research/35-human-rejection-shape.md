# 35 - Confirm the human rejection shape on `claude`, at a keyboard

Ticket: [35 - Confirm the human rejection shape on `claude`, at a keyboard](../issues/35-confirm-human-rejection-shape.md)
Type: task. Manual work with the human driving. This file reports what the transcript held.

Measured 2026-09-04, 16:50 to 17:20 local. The human ran the sessions and pressed the keys. Nothing
here was scripted, which is the point: every previous denial on this machine was a machine refusing.

## Versions under test

```
claude  2.1.261 (Claude Code)
```

**The binary self-updated mid-ticket**, 2.1.257 to 2.1.261, between ticket 34's measurements and
this one. Ticket 34 read `qLn` out of the 2.1.257 binary and this ticket exercised .261, so the two
findings below that disagree could in principle be a version difference rather than a mistake. One
of them is a mistake regardless, and the section says why.

## Probe hygiene

Probe directory `/private/tmp/hydra-t35/`, holding `edit-me.txt` and `t35probe.txt`, plus
`control/` for the machine-denial control. No user-scope config was written. The sessions ran with
`--permission-mode manual`, which the record writes as `permissionMode: "default"`: the CLI name and
the recorded name differ, and they are the same mode.

Three transcripts under `~/.claude/projects/-private-tmp-hydra-t35/`, one under
`-private-tmp-hydra-t35-control/`.

## The short version

**`correction` has a real producer, and two complete pairs now exist.** They are the first human
preference pairs recorded on this machine.

**Ticket 34 got one thing wrong, and it is the field it called the richest part of the event.**
`userFeedback` was empty on all three human rejections. The human's words survive, as an ordinary
user turn further down the `parentUuid` chain, so the event is recoverable. The harvest rule's step
4 has to change.

Three other things fell out that nobody asked for:

- A denial also emits `[Request interrupted by user for tool use]`. Ticket 34 listed interrupts and
  denials as possibly-distinct shapes. A denial **contains** an interrupt marker, so anything keying
  on that string alone double-counts.
- On .261 the Bash permission prompt has **no reject-with-feedback option**. The old
  "No, and tell Claude what to do differently" is gone, replaced by "Yes, and switch to auto mode".
  Refusing is a bare `4. No`.
- A machine denying through `--permission-prompts none` writes `permission-rule`, not
  `user-rejected`, which is a second independent separator on top of `entrypoint`.

## What the human did

Two rejections, each followed by a redirection and an approved retry.

| | rejected | said | chosen |
|---|---|---|---|
| Bash | `touch t35probe.txt && ls -l …` | "use printf to write the word hello into it instead" | `printf 'hello' > t35probe.txt && ls -l …` |
| Edit | `edit-me.txt`, `beta` to `delta` | "change gamma instead, leave beta alone" | `edit-me.txt`, `gamma` to `delta` |

A third rejection exists in an earlier session, a `touch` refused with no follow-up, because that
session was abandoned. It is useful as the negative case: a rejection with no chosen side.

The Edit pair is the better of the two for preference export. Same file, same `new_string`, one
`old_string` refused and another chosen. The difference between the two records is exactly the thing
the human decided and nothing else.

## The record, line by line

From `-private-tmp-hydra-t35/d0575754-8a7c-4ae9-b70a-4f9452a5ea10.jsonl`, the Bash pair:

```
16  assistant   tool_use Bash {"command":"touch /private/tmp/hydra-t35/t35probe.txt && ls -l …"}
17  user        tool_result "The user doesn't want to proceed with this tool use…"
                toolDenialKind: "user-rejected"   sourceToolAssistantUUID -> line 16
18  user        text "[Request interrupted by user for tool use]"
19  system
20  file-history-snapshot
21  user        "use printf to write the word hello into it instead"
24  assistant   tool_use Bash {"command":"printf 'hello' > /private/tmp/hydra-t35/t35probe.txt …"}
```

The `parentUuid` chain is unbroken from 16 through 24: `ac38439c` to `c3f0654f` to `6fb076a0` to
`86cd4ca0` to `6ed3aa06` to `94d8d5ee` to `e6f1215e`. Two record types sit in the middle that a
naive walk would trip on, a `system` record and a `file-history-snapshot`, and the snapshot carries
no uuid at all.

The Edit pair, same file, lines 42 to 45 and onward, has the identical shape. The rejected diff is
fully present in the `tool_use` input:

```json
{"replace_all": false, "file_path": "…/edit-me.txt", "old_string": "beta", "new_string": "delta"}
```

Ticket 34 flagged the rejected-`Edit` case as assumed rather than seen, because all 35 denials in
its corpus were `Bash` and `Write`. It is now seen, and the assumption was right: a rejected diff
needs no special handling, since `old_string` and `new_string` are ordinary fields of the input.

## The four checks

Run against all three human rejections with `.scratch/hydra/verify35.py`.

| check | result |
|---|---|
| `toolDenialKind == "user-rejected"` | PASS, 3 of 3 |
| `entrypoint == "cli"` | PASS, 3 of 3 |
| rejected call recovered via `sourceToolAssistantUUID` | PASS, 3 of 3 |
| `userFeedback` holds the typed words | **FAIL, 0 of 3** |
| chain reaches a same-tool replacement | PASS on both completed pairs |

## Where ticket 34 went wrong

Ticket 34 read this out of the 2.1.257 binary:

```js
userFeedback: Ot.behavior === "ask" ? Ot.userFeedback : void 0
```

and concluded that `userFeedback` carries the human's redirection, calling a denial with it set "the
richest form of the event the contract could hope for". That conclusion does not survive contact.

What the code says is narrow and true: `userFeedback` is populated **only** on the ask path. What
ticket 34 added, and should not have, is that the ask path populates it. On .261 the human's text
does not go there. The field was read as a promise when it was only a gate.

Two candidate explanations, and the ticket cannot separate them:

- The .261 prompt has no reject-with-feedback option at all, so the path that once filled the field
  is gone from the UI. The old option 3 read "No, and tell Claude what to do differently" and now
  reads "Yes, and switch to auto mode".
- The field was never filled by this path and ticket 34's inference was simply wrong on .257 too.

The corpus cannot decide it, because ticket 34 established there is no human rejection anywhere in
it to check against. The distinction does not change the harvest rule either way, so I am leaving it
open rather than guessing.

The lesson is the cheaper finding: **a field's presence in a conditional is not evidence that
anything writes it.** Ticket 34 verified its traversal 35 times on real records and verified this one
field by reading a ternary. Only the traversal survived.

## The corrected harvest rule

Steps 1 to 3 and 5 to 6 from ticket 34 stand as written. Step 4 changes.

1. Find `user` records where `toolDenialKind` is present.
2. Human filter: `toolDenialKind == "user-rejected"` and `entrypoint == "cli"`.
3. Rejected content: follow `sourceToolAssistantUUID`, take the `tool_use` block whose `id` matches
   the denial's `tool_result.tool_use_id`.
4. **Human's words: walk children by `parentUuid` from the denial to the first `user` record whose
   `message.content` is a plain string rather than a list of blocks.** That is a typed turn. A
   `tool_result` is a list, so the two do not collide. Skip the `[Request interrupted by user for
   tool use]` record, which is a `user` record carrying a text block, and skip `system` and
   `file-history-snapshot` records, the latter having no uuid.
5. Chosen content: continue the same walk to the first `assistant` record holding a `tool_use`.
6. Emit a pair only when step 5's tool name matches step 3's.

Step 4 is weaker than reading a field. Nothing marks that user turn as a redirection rather than an
unrelated new instruction, so a human who rejects a call and then asks for something else entirely
produces a record indistinguishable from a correction. Both pairs here are genuine because the human
was following a script. In the wild that is an assumption, and the contract should say so rather
than pretend the walk is sound.

## The control

To check whether `user-rejected` still separates a human from a machine on .261, one run with
`--permission-prompts none`, which the help describes as "nobody: anything that would prompt is
denied automatically":

```
line 12: toolDenialKind='permission-rule'  entrypoint='sdk-cli'  userFeedback=None
line 16: toolDenialKind='permission-rule'  entrypoint='sdk-cli'  userFeedback=None
```

A machine auto-denial writes `permission-rule`, not `user-rejected`, and `sdk-cli`, not `cli`. Two
separators, both pointing the same way.

**This does not collapse ticket 34's conjunction into a single field.** Its corpus holds three
`user-rejected` records at `sdk-cli`, produced by ticket 30's SDK harness answering the ask path in
code. That path writes `user-rejected` for a machine. `--permission-prompts none` is a different
mechanism, an auto-deny rather than an ask answered, so this control adds a data point without
covering the case that made the conjunction necessary. Keep both conditions.

## What this leaves for the contract

- `correction` keeps its producer on `claude` and stays on the list. Ten event types, unchanged.
- Section 7.1's producer column carries one provider, with `cursor-agent` and `agy` as gaps for the
  reasons ticket 34 recorded.
- The event's `text` field, whatever it ends up called, comes from a chain walk rather than a record
  field, and carries the caveat in step 4.
- `preference.jsonl` has a real source in v1. Two pairs exist today.

## Unmeasured

- **Whether .257 ever filled `userFeedback` on this path.** Both explanations above fit the
  evidence and the corpus has nothing to check against.
- **What the reject-with-feedback option did before .261.** Its removal is visible in the prompt
  text; whether it wrote a different record is not recoverable now.
- **Whether a redirection that changes the subject is distinguishable from a correction.** Step 4
  cannot tell them apart and no record here tests it, because the human stayed on task both times.
