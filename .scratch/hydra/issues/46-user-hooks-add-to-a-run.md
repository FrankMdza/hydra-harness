# 46 - The user's own hooks add to a run's context, and section 8 says they only subtract

Type: grilling
Status: resolved

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

## Answer

Resolved 2026-09-06 with the human, one round. Four decisions.

**Measured first, free, on 2.1.263.** The record is right: sequences 1 and 2 are `hook_started` and
`hook_response` for `UserPromptSubmit`, carrying the unslop reminder as `additionalContext`. The
user's settings hold one add-context hook and four `PreToolUse` guards. `--setting-sources
project,local` beside `--settings <Hydra's file>` dropped the user's hook while a marker hook in
Hydra's file still fired, so the gate survives isolation. No `--restricted` probe carried the
reminder, so `judge` never sees it. No `~/.cursor/hooks.json` and no user-scope `agy` hook config
exist here, so on this machine only `claude` does it.

**1. It is data.** A human wrote it and Hydra did not write it at spawn, which is section 8's test
as written. The instruction list stays the three things the record can vouch for. Section 8's "can
only ever subtract" was true of the guards on this machine and not of the mechanism, and is
rewritten: a user hook can add text, the text is data, the hook's denials still only subtract, and
the MCP exception becomes "every case but two".

**2. Hydra keeps loading the user's settings.** The isolation is one flag away and it is not taken,
because the four guards live in the same file and 6.6 already calls them protection Hydra never
specced. 6.6 now names the price.

**3. The text stays in `raw`.** It is on every record already as a `hook_response` frame under
`--include-hook-events`. Contract 7.7's `claude` row and section 13 say so. No eleventh event type
for one hook on one machine.

**4. The `cursor-agent` case is stated as unmeasured** in 6.6, beside the `agy` asymmetry, and build
ticket 09's drift note picks it up on the next paid Cursor run.

**What moved.** Harness spec 6.6 and 8; contract 7.7 and 13; build ticket 09. No source changed, no
build ticket opened.
