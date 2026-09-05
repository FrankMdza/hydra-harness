# 12 - Ouroboros rung one

Type: build
Status: open
Blocked by: 04

## Goal

The curator: read the corpus, find failures worth learning from, propose skill edits.
Invoked rather than scheduled, writing only to the skill store, proposing rather than
landing.

## Where the design says it

- `docs/ouroboros-spec.md`, rung one in full. Read it before this ticket; the harness
  spec and the contract only say what crosses.
- Harness spec 17 for `hydra curate`, an ordinary recorded run under the `curate` policy.
- Contract 5.5, why a curate pass gets no record of its own, and 8.6, why rung one skips
  harness runs as a kind rather than by outcome.
- Contract 8's rung-one column: five outcomes it never learns from, and why that is not
  the same as throwing them away.

## Done when

- `hydra curate` runs as an ordinary recorded run under `curate` and proposes rather
  than lands.
- The five outcomes rung one never learns from are skipped, keyed on the outcome rather
  than on a guess.
- Harness runs are skipped, keyed on `provider: null`.

## Watch for

**Blocked on 04 because rung one keys on outcomes, and until they derive it has no
input.** Skills and MCP are not blockers: a curator that proposes edits to a store with
few skills in it is still a curator.

**A run that failed on too narrow a policy is a policy bug rather than a skill bug**, and
nothing in v1 reads that signal. Do not let rung one read it as a skill failure.

**Ouroboros cannot call `hydra reap`, `hydra prune` or `hydra export` at either rung**,
and the gate does not stop it, because those are a binary reached through `shell`. What
holds it is that no Ouroboros code path calls them, which is a property of Hydra's own
source. Naming the right mechanism matters more here than having two.

## What ticket 03 left you, added 2026-09-05

**Read the corpus through `hydra::normalize`, never by matching on `raw`.** `read_file`
takes a path and a `PolicySet` and returns a `Run`. The whole point of contract 11 is
that a curator reading normalized events gets last month's records right after this
month's adapter fix, and a curator that greps `raw` does not.

**Two filters rung one needs are already fields.** 8.6 skips a harness run, which is
`Run::provider == None`. A run with no `run_end` is the abandoned class, which is
`Run::run_end() == None`.

**`Run::findings` is where a run says it broke its own policy**, and it is not part of
any trajectory. A `GateLeak`, a `SubagentLeak` or a `Disagreement` means the record and
the policy disagree about what was allowed. Learning from such a run without reading its
findings would teach the curator that a refused call succeeded.

**One cost worth knowing before you walk the whole corpus.** Deriving
`permission_decision` runs the real gate script, one `/usr/bin/python3` process per tool
call. Nine records cost a quarter of a second; a corpus of thousands will not. The
policy compiles once per run, so the cost scales with calls rather than with records, and
if it bites, the fix is a batch mode on the gate rather than a second implementation of
the rules in Rust.
