# 46 - The user's own hooks add to a run's context, and section 8 says they only subtract

Type: grilling
Status: open

## Question

Harness spec section 8: "The user's own hooks are not an instruction source at all. A hook config
never enters the model's context. It is a second gate with a human owner, which is coherent under
one principal, and it can only ever subtract."

Measured 2026-09-06 on run `01a07886-a450-7a21-92cf-799b26358a05`, `claude` 2.1.263, sequences 1
and 2 of the record: the `UserPromptSubmit` hook in `~/.claude/settings.json` fired inside the run
and returned `additionalContext`, the unslop standing reminder, which the provider appended to the
model's context before the first turn. `HOME` is in the allowlist (D6), so the user's settings load
in every `claude` run Hydra launches, and this hook adds rather than subtracts.

**Is text a user hook adds an instruction or data, and does Hydra keep it out of recorded runs?**

## Settle

- **Which it is under section 8's authorship test.** A human wrote it, Hydra did not write it at
  spawn, and it arrived before the model's first turn. Authorship says data, timing says
  instruction, and the section says the two agree in every case but one. This is a second.
- **Whether Hydra narrows the settings a run loads.** `claude --setting-sources user,project,local`
  chooses which settings files load, and Hydra passes `--settings <gate file>` and nothing about
  sources. Dropping `user` drops this hook, and it drops the user's own guards too, the second gate
  6.6 counts on. That is the trade.
- **Whether the record carries it.** Ticket 38 found `attachment` is run content that entered
  through no tool call. Hook-added context is the same kind, and today it sits in the raw frame
  only.
- **The other two providers.** `~/.cursor/hooks.json` and `agy`'s user-scope hooks reach a run the
  same way, with whatever add-context events each defines.

## Where the evidence is

The record named above. `src/env.rs` for the allowlist. `src/provider/claude.rs` for the argv Hydra
writes. Harness spec 6.6 for why the user's guards are wanted, and section 8 for the claim this
contradicts.
