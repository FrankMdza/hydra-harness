# 35 - Confirm the human rejection shape on `claude`, at a keyboard

Type: task
Status: open
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

`pair.py` in
`/private/tmp/claude-501/-Users-USER/63edcca5-a9f3-4f9c-ba02-4bf0272e02c9/scratchpad/t34/` prints
all four from a transcript path. It is in a scratch directory and will be reaped, so copy it into
the repo if this ticket outlives the machine's temp.

## Why it blocks ticket 16

The seam check asks whether every field Ouroboros reads is one the harness writes. `correction` is
the only source `preference.jsonl` has in v1. If `userFeedback` comes back empty, or the chain does
not reach a replacement call, the type is weaker than the contract would claim and section 7.1
changes before it ships. Answer this before the seam check runs, not after.
