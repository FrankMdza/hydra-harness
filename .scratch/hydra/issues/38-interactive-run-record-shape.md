# 38 - What does a record look like on an interactive run?

Type: grilling
Status: resolved
Blocked by:

## Question

The record contract specifies a recorded run. It gives a field-by-field shape for the harness run
(section 5.5) and for a refused spawn (section 8.3), and it never gives one for an interactive run,
which is the third shape and the only one that produces a preference pair.

**The producer column resolves provider and not launch mode.** Section 7.2 is the seam check's main
instrument, and its three columns are `claude`, `cursor-agent` and `agy`. Every cell in it names a
stream-json frame: `assistant` frames, `step_update`, `tool_call` frames at `subtype: started`. An
interactive run has no stream. Section 12 says its body is converted from the vendor's own transcript
after the process exits, and section 7.9 confirms the transcript is a different vocabulary entirely,
reading `user` records, `parentUuid` and `sourceToolAssistantUUID`. So the column describes one of
two launch modes and says so nowhere, and the check "every field Ouroboros reads is one the harness
writes" is a table lookup against a table that covers half the population.

**The one type that only exists interactively is the one export depends on.** `correction` is
harvest-only on `claude` (7.2, 7.9) and it is the sole source of `preference.jsonl` (16.1, 16.3). So
the preference half of D12 runs entirely over the launch mode whose field presence is unstated.

**Three fields have no defined value on an interactive run.** The harness spec's sections 3 and 4 say
an interactive run gets no skill injection, no gate and no preamble.

- `policy` and `policy_sha` (5.1). No gate is compiled, so there is no policy in force. Both fields
  are described as unconditional.
- `skills.store_sha`, `skills.linked`, `skills.resolved` (5.3). Nothing is injected and no snapshot is
  pinned. Rung one reads all three (Ouroboros spec section 6).
- `permission_decision` (7.7) derives by re-evaluating "the policy recorded in `run_start`" against
  the call. With no policy there is nothing to re-evaluate, so the type has no producer interactively,
  and 7.2 lists it for all three providers with no qualification. Rung one's gate-leak check reads it.

Absent, null, or a fourth answer, and the three groups may not want the same one. Section 5.5 set a
precedent worth arguing from: groups absent, scalars null, because "a null `skills.store_sha` invites
a reader to ask which sha the pass pinned."

## What to decide

1. Whether 7.2 gains a launch-mode dimension, or a second table beside it, or a stated scope plus a
   per-type note on the ones that move. Three types are known to move: `correction` appears only
   interactively, `permission_decision` only in a recorded run, and the seven others change their
   source frames.
2. The interactive field-presence table, in the shape of 5.5's.
3. Whether `permission_decision` is absent interactively or present with a `decider` that says the
   run had no Hydra policy. The `external` value in 7.7 is close but not the same thing: it means the
   policy would have allowed and something outside denied, which presumes a policy.

## Scope

**The `claude` column is answerable now.** Tickets 34 and 35 read `~/.claude/projects/` transcripts
against real records and measured the correction shape at a keyboard, so the harvest vocabulary for
the one provider that produces `correction` is known.

**The other two are not, and they stay in fog.** The map's "What ticket 13's harvest can actually
read" patch already holds them: Cursor stores chats in two formats with the newer one carrying
protobuf and a `blobEncryptionKey`, and `agy` omits steps from its own `transcript_full.jsonl`. This
ticket should state the mode question and fill the `claude` column, and hand the rest to that patch
rather than pulling it forward.

## One probe, and it widens the ticket, 2026-09-05

Read from the two transcripts tickets 34 and 35 left at
`~/.claude/projects/-private-tmp-hydra-t35/`, on `claude` 2.1.261. Cheap, and it settles one column
and opens a question the ticket did not have.

**`usage` survives harvest on `claude`.** Three assistant records, three carrying `message.usage`. So
the type has a harvest producer on the one provider that matters here, and section 7.2's `usage` row
is right for the wrong reason: it names `result.usage`, which a transcript has no equivalent of. The
producer differs by launch mode and the answer happens to be non-empty in both.

**`thinking` is unconfirmed rather than absent.** Zero thinking blocks in that transcript, on a short
run that may simply not have thought. Not evidence either way, and worth one more probe on a run that
does before the harvest column claims anything.

**The transcript's record vocabulary is wider than the stream's, and the contract has no home for
it.** That file carries `mode`, `permission-mode`, `attachment`, `file-history-snapshot`, `ai-title`,
`last-prompt`, `bridge-session` and `atis-latch` records alongside `user`, `assistant` and `system`.
Section 3 says a provider line "wraps one provider frame verbatim" and section 13 says nothing is
dropped. Taken together, harvest stores a Claude Code session's internal bookkeeping into the corpus
verbatim, including `attachment` records, which is where a pasted file lands.

That last one is a real question rather than a tidiness complaint, and it reaches export. Section
16.2 computes eligibility from "every path the run touched, extracted through the tool-call adapter".
An `attachment` record is content that entered the run through no tool call, so the adapter never
sees it and the path check never covers it. Either harvest filters the vendor's non-conversational
record types, which puts a hole in never-discard and needs its own justification, or it keeps them and
export needs a second extractor. **Neither document currently says which**, and the ticket should
answer it, because it is the interactive half of the same question section 16.2 already answers for
recorded runs.

## Answer, 2026-09-05

**The contract now specifies three shapes rather than two**, and the producer column says which
population it describes.

**Section 7.2 is scoped to recorded runs and 7.2.1 is new.** Every cell in 7.2 names a stream-json
frame and an interactive run has no stream, so the scope was implicit and load-bearing. Two rows
flip rather than change source: `correction` exists only on a harvested run and `permission_decision`
only on a recorded one. **The two types most often imagined as a pair, a decision and a correction,
can never appear in the same run.** Every other derived row keeps its type and changes where it comes
from.

**Section 5.6 is the interactive run's field-presence table**, in 5.5's shape. `policy` and
`policy_sha` are null because no gate is compiled, `worktree` is null because worktrees are per
recorded run, `skills` is absent, and `mcp` is present on `claude` and absent on the other two,
because ticket 29's command-line asymmetry decides it: `claude`'s config needs no worktree and the
other two read theirs out of one.

**`skills` absent and `policy` null is deliberate asymmetry.** No sha was pinned, so asking which sha
does not apply. But the run really did have a policy question and the answer really is none, and a
reader comparing an interactive run to a recorded one is usually asking exactly that.

**`permission_decision` is absent interactively, not null and not `external`.** `external` means the
run's policy would have allowed and something outside denied, which presumes a policy exists to have
allowed. The gate-leak check goes with it, since a leak is defined against a policy. What still runs
on an interactive session is the user's own hooks, and those were never Hydra's to record a decision
about; a reader wanting them reads the hook frames in `raw`.

### The probe, and what it settled

Two transcripts on `claude` 2.1.261, left by tickets 34 and 35.

**`usage` survives harvest, on `message.usage` on the assistant records.** Three of three. Section
7.2 names `result.usage`, which a transcript has no equivalent of, so the table as scoped would have
had a reader conclude the producer does not exist.

**`thinking` is unconfirmed rather than absent.** Zero blocks across two short runs, which is no
evidence either way, and 7.2.1 says so rather than filling the cell.

**`cursor-agent` and `agy` harvest stays unmeasured and the contract does not guess.** A missing type
on those two is unknown rather than absent. The map's harvest fog patch still holds them.

### The finding the ticket did not start with

**A transcript carries eight record types no stream has**: `mode`, `permission-mode`, `attachment`,
`file-history-snapshot`, `ai-title`, `last-prompt`, `bridge-session` and `atis-latch`. Section 13
keeps them verbatim like everything else and none normalizes to one of the ten types.

`attachment` is the one with consequences. It is where a pasted or dragged file lands, so it is run
content that entered through no tool call, and section 16.2 extracts every path a run touched
*through the tool-call adapter*. The adapter never sees it, so the export allowlist that section 14
says carries the whole weight of export never gets asked about that content.

**So 16.2 gains a fourth question: nothing entered the run except through a tool call.** A run holding
a record type the path extractor cannot resolve is not exportable, exactly as a run holding an
`unclassified` tool call is not. Same rule, second source, same trade: the corpus gets smaller rather
than leakier.

Filtering the vendor's record types at harvest was the alternative and it loses twice. It would cut
the first real hole in never-discard, and an `attachment` is run content rather than vendor
bookkeeping, so dropping it would lose the thing the model actually read.

**The cost is real and lands where the corpus is thinnest.** Interactive `claude` runs are the only
source of `preference.jsonl`, and a session where the human pasted a file now exports nothing until an
extractor exists for that record type. Right side to be wrong on, since the alternative is content the
allowlist cannot see.

### What changed

- **Contract 3** now says a harvested line wraps a transcript record rather than a stream frame, and
  that a reader checks `run_end.body_source` before assuming a shape from 7.2.
- **Contract 5.6** new, the interactive field-presence table.
- **Contract 7.2** scoped, **7.2.1** new.
- **Contract 7.7** opens with "on a recorded run" and says why absent beats `external`.
- **Contract 16.2** gains the no-tool-call rule.
- **Ouroboros 6** marks the gate-leak row recorded-only and the skills row absent-on-interactive, and
  gains two paragraphs on rung one reading two populations that never overlap.
- **Harness 19.6** opened: an interactive run has no worktree, so the fatal Cursor MCP pre-flight
  checks an empty set and `CURSOR_DATA_DIR` points at a directory reaped with a worktree that does not
  exist. Neither breaks anything today and both are first-hour guesses for a builder.
- **Harness 19.2** closed, having been settled by the contract's 8.1 two sessions ago and left open
  in the harness spec.
