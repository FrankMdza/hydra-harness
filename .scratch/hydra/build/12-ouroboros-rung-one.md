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

## What ticket 04 left you, added 2026-09-05

**The input exists now. `hydra::outcome::derive` takes a `&Run` and returns
`Option<Derived>`, with `outcome` and `source` on it.** Both of this ticket's filters are
one call each: `Outcome::rung_one_learns()` is contract 8's right-hand column, and a
`None` from `derive` is the abandoned class. The third filter, harness runs, is still
`Run::provider == None` and needs no outcome.

**`rung_one_learns()` is the column, not a judgement, and reading it as "keep" is the
mistake contract 8 spends a paragraph on.** Every run is recorded in full whatever it
says. What this ticket may not do is learn a skill lesson from a run in the lower half.

**Two of the five outcomes it never learns from have no producer yet**, so a corpus walk
today sees `completed`, `gated`, `unknown`-sourced `completed` and `rate_limited` and
nothing else. `skill_collision` and `mcp_unavailable` wait on tickets 07 and 08.

**`gated` is coarser than "a call was refused", and this ticket is why it is.** It fires
only when the gate refused everything the run tried, so a run that had one call denied and
finished the task stays in the corpus as a learnable trajectory with the denial visible per
call. The consequence for this ticket is the reverse one: a run that failed on too narrow a
policy but got one call through comes out `completed`, so keying only on the outcome will
occasionally hand you a policy bug dressed as a skill failure. `Run::permission_decisions`
is where you check, and ticket 27 already says a policy bug is not a skill bug.

**A pre-gate record with a real denial in it derives `completed`.** Three records on this
machine carry `policy: null`, contract 7.7 gives those no `permission_decision`, and one of
them had a `Write` refused. Nothing in the outcome says so. They are the oldest three files
under `~/hydra/runs/` and are worth excluding by the same test `IMPLEMENTATION.md` names, a
null policy, rather than by date.
