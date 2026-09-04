# 35 - Confirm the human rejection shape on `claude`, at a keyboard

Type: task
Status: resolved
Blocked by:

## Question

HITL. This is the one thing ticket 34 could not measure, and no agent can close it.

Ticket 34 established the harvest rule for `correction` on `claude` from the binary's own source and
verified the traversal on 35 auto-denials. The rule keeps a pair only when
`toolDenialKind == "user-rejected"` **and** `entrypoint == "cli"`. The corpus holds zero records
matching that conjunction, because every rejection on this machine so far came from a rule, from
automode, or from a script.

Staging one with a script cannot substitute. The SDK path writes `entrypoint: sdk-cli`, which is the
case the rule filters out, so a scripted rejection re-measures what ticket 34 already measured.

## The checklist

1. `cd` to a scratch directory and run `claude` interactively with no `--permission-mode` flag, so
   the permission prompt is live.
2. Ask for a shell command that will prompt, for example `echo T34MARKER`.
3. At the prompt, take the option that rejects **and lets you type a reason**, then type something
   specific such as `use printf instead`.
4. Let it try again, and let the second call through.
5. Repeat once on an `Edit`, so the rejected-diff shape gets an instance. Ticket 34's 35 denials are
   all `Bash` and `Write`, so the diff case is assumed rather than seen.

Then read the new file under `~/.claude/projects/<slug>/` and check four things:

- `toolDenialKind` is `user-rejected`
- `entrypoint` is `cli`
- `userFeedback` holds the typed words
- the `parentUuid` walk reaches a replacement `tool_use` with the same tool name

`.scratch/hydra/verify35.py` checks all four and prints PASS or FAIL per line. It scans every
transcript touched in the last two hours, so it needs no path:

```
python3 .scratch/hydra/verify35.py        # or pass a window in minutes
```

It filters on `entrypoint == "cli"`, so a rejection from a script cannot satisfy it by accident.

A probe directory is set up at `/private/tmp/hydra-t35/`, holding `edit-me.txt` with three lines,
for the rejected-`Edit` half.

## Why it blocks ticket 16

The seam check asks whether every field Ouroboros reads is one the harness writes. `correction` is
the only source `preference.jsonl` has in v1. If `userFeedback` comes back empty, or the chain does
not reach a replacement call, the type is weaker than the contract would claim and section 7.1
changes before it ships. Answer this before the seam check runs, not after.

## Answer

**`correction` has a real producer, and two human preference pairs now exist.** Findings in
[research/35-human-rejection-shape.md](../research/35-human-rejection-shape.md), measured 2026-09-04
on `claude` 2.1.261, which self-updated from 2.1.257 between ticket 34 and this one.

Three of the four checks pass on all three human rejections: `toolDenialKind` is `user-rejected`,
`entrypoint` is `cli`, and `sourceToolAssistantUUID` recovers the rejected call. Both completed pairs
reach a same-tool replacement. The rejected-`Edit` case, which ticket 34 marked assumed, holds: the
diff sits in the `tool_use` input as `old_string` and `new_string` and needs no special handling.

**The fourth check fails, and it falsifies ticket 34.** `userFeedback` was empty on all three. Ticket
34 read `userFeedback: Ot.behavior === "ask" ? …` out of the binary and concluded the field carries
the human's redirection, calling it the richest form of the event. The code only says the field is
gated on the ask path. That the ask path fills it was an inference, and it is wrong on .261.

The words are not lost. They land as an ordinary user turn further down the `parentUuid` chain, so
step 4 of the harvest rule becomes a walk to the first `user` record whose `message.content` is a
plain string rather than a list. That is weaker than a field: nothing marks that turn as a
redirection rather than an unrelated new instruction, and the contract should carry the caveat rather
than pretend the walk is sound.

**Three things nobody asked for.** A denial also emits `[Request interrupted by user for tool use]`,
so a denial contains an interrupt marker and anything keying on that string alone double-counts. The
.261 prompt has no reject-with-feedback option at all: the old "No, and tell Claude what to do
differently" now reads "Yes, and switch to auto mode", and refusing is a bare `4. No`. And a machine
denial through `--permission-prompts none` writes `permission-rule` at `sdk-cli`, a second separator
on top of `entrypoint`, though it does not collapse ticket 34's two-condition test, because the SDK
ask-answered path still writes `user-rejected` for a machine.

The transferable lesson: **a field's presence in a conditional is not evidence that anything writes
it.** Ticket 34 verified its traversal 35 times against real records and verified this one field by
reading a ternary. Only the traversal survived.
