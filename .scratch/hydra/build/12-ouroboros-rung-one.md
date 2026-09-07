# 12 - Ouroboros rung one

Type: build
Status: resolved
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

## What ticket 06 left you, added 2026-09-05

**The two commands now exist, so the constraint above has real code to violate.**
`hydra::reap::reap` and `hydra::prune::prune` are public on the library and `main` is the
only caller. When you add a rung-one code path, that stays true or the
`evidence_is_read_only` checklist item is already broken. There is no gate to fall back
on, for the reason harness spec 7.5 gives.

**Prune with no arguments moves every run that is not live.** A curator pass that shells
out to `hydra prune` for any reason would take the corpus it learns from with it.

## Outcome

Resolved 2026-09-06. `hydra curate` is built, `src/ouroboros/` holds all of it, and
`tests/curate.rs` covers both halves without launching a provider.

**What it does.** `corpus.rs` walks `~/hydra/runs/` through `hydra::normalize`,
splits it into what rung one may learn from and what it may not, and counts the five
classes. `curate.rs` turns that into the task string, checks the skill store out at
`~/hydra/skill-worktrees/<run-id>/` on `ouroboros/curate/<run-id>`, launches an
ordinary recorded run under `curate` through `hydra::run::run`, and commits whatever
the run left in that checkout onto the branch. A human merges it or does not.

**The filters, all keyed on fields.** Harness runs on `provider: null` (8.6), the
five never-learn outcomes on `Outcome::rung_one_learns` (contract 8), live runs on
the pid liveness test before the file is opened, and the curator's own children on
`parent_run_id`. `hydra::normalize::Run` gained `parent_run_id`, lifted off the
envelope, because contract 4 puts it there and nothing else could see it.

**A fourth filter the ticket did not ask for, and the corpus is why.** A recorded
run with a null policy predates the gate, derives `completed` whatever the gate did,
and one such record here had a `Write` refused with nothing saying so. The test is a
null policy **beside `launch_mode: recorded`**: an interactive run also carries a
null policy, legitimately, and it is the only population a human correction can come
from, so keying on the null alone would drop half the corpus and the whole of class
two with it. That correction is the ticket's own note from 04 made precise.

**Measured, 2026-09-06.** Thirteen records on this machine: 6 learnable, 7 skipped
as 3 pre-gate, 3 `gated`, 1 `rate_limited`. The walk costs 0.55 s end to end,
process start included. One full pass ran against a real `claude`, run
`01a07513-baa3-7cf0-b20a-ded11ede6a38`: `completed`/`unknown`, 489,440 tokens, and
it proposed nothing. It declined to propose a skill edit for the one denied `Bash`
call, ranked the one `unclassified` result without diagnosing from it, and said the
store held no skill to edit. The empty branch was deleted by `git branch -d`, which
is the mechanism working rather than a pass that failed.

**The pass above found a bug in the first version of this ticket's own work, and
the corpus test is what surfaced it.** Contract 7.7 re-evaluates a run's policy at
read time and rebuilds the scopes rather than reading them off the record, so a
curate run whose store scope was passed in at launch came back with ten
`Finding::GateLeak`, one per file the curator wrote or probed inside its own
checkout. `Layout::skill_store_scope(run_id, policy)` is the fix: one rule, read by
the launch when it compiles the gate and by the reader when it replays it. No field
on the record carries a scope and inventing one is a contract change.
`tests/normalize.rs`'s corpus-wide leak check caught it and `tests/curate.rs` pins
the round trip now.

**What the ticket assumed that turned out otherwise.**

- **The store arrived mid-ticket.** The plan was to refuse when `~/hydra/skills/` is
  not there and name ticket 07. That refusal is built and tested, and then ticket 07
  created the store in the main checkout while this ran, so the pass above went end
  to end instead. Its checkout was empty because the store's skills were not
  committed yet, which is the next point.
- **A curate checkout of the store's `HEAD` races the launch that commits a dirty
  store.** Ouroboros spec 3.6 has a launch commit the store before pinning it, and
  this pass checks out before it launches, so once ticket 07 wires that commit the
  curator reads the store as it was before its own launch tidied it. Whoever wires
  the pinning owns the fix and it is an ordering decision rather than a line of
  code: commit the store before the curate checkout, or move the checkout after the
  launch and compile the gate against a directory that does not exist yet. Map
  ticket 41 took the first on 2026-09-06: the pass pins, then checks out at that sha.
- **Section 9.1's suggestion that the checkout could live in the spawn directory
  does not hold.** `hydra reap` removes that directory whole, and a pass whose run
  outlived its call leaves an uncommitted proposal in the checkout. It lives at
  `~/hydra/skill-worktrees/<run-id>/` instead, a sixth place `IMPLEMENTATION.md`
  records as this build's choice.
- **`hydra run` could not mint the run id for this caller.** The checkout the gate
  scopes to is named after the run, so `Request` gained `run_id`. It is `None` for
  `hydra run` and nothing else about the launch changed.
- **Ticket 07's first records broke a corpus test in `tests/outcome.rs`.** It
  expected every record with no `api_error_status` to be `gated` or `completed`,
  and a refused launch is neither: contract 8.2 makes that outcome a fact Hydra
  wrote rather than a reading of a stream. The loop now skips
  `Source::Refusal` and asserts only that rung one never learns from one. Expect a
  conflict with 07 there if it fixed the same test its own way.

**The read-only property is a test now.** `evidence_is_read_only` is the only
mechanism keeping Ouroboros away from `hydra reap`, `hydra prune` and `hydra export`,
so `tests/curate.rs` reads every file under `src/ouroboros/`, skips comment lines and
fails on `reap::`, `prune::`, `export::` or a spawn of `hydra`. It was checked against
a deliberate violation and it caught it.

**Left for later, and each is a producer rather than a gap in this ticket.** Human
rejection needs an interactive run and harvest (ticket 12 of the design's fog, and
this build launches none). The retry loop needs delegation, ticket 11. Section 3.8's
before-and-after measurement is a second pass reading the same counts, which is one
`hydra curate` after another rather than code. Section 3.11's stall class is left
unbuilt on the design's own argument.
