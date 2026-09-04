# 35 - Confirm the human rejection shape on `claude`, at a keyboard

Type: task
Status: claimed
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
