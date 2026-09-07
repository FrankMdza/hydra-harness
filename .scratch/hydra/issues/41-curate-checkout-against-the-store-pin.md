# 41 - The curate checkout happens before the launch commits the store

Type: task
Status: open
Blocked by: 40

## Question

Two build tickets landed in the same hour and their orderings do not compose. Neither
decided it, both wrote it down, and it is live on `master`.

`hydra curate` checks the skill store out at `HEAD` and then launches (build ticket 12).
A launch commits a dirty store before pinning it, because harness spec 6.4 says launching
from `HEAD` and ignoring the working tree would mean editing a skill, running, and
watching nothing happen (build ticket 07).

**So a curate pass started with an uncommitted skill in the store reads a checkout its own
launch then moves past.** The curator proposes against yesterday's store while the run it
is part of pins today's.

## The two options, and what each costs

**Commit the store before the curate checkout.** `hydra curate` calls `Store::pin` itself,
then checks out the sha it got back. Costs: the pin's commit trailer says
`Hydra-Pinned-By: <run-id>` for a run that has not started yet, which is true but earlier
than the trailer's wording implies. `run::run` then pins a store that is already clean and
gets the same sha, so nothing is done twice.

**Move the checkout after the launch.** The gate is compiled before the child starts and
the `skill_store` scope points at the checkout, so the gate would name a directory that
does not exist yet. `src/policy/mod.rs` resolves a path whose tail is missing on purpose,
so this works, but it means a `curate` run's write scope is a promise rather than a
directory for the first moments of the run.

The first is the smaller change and the second is the one that keeps `hydra curate` out of
the store's git history. Pick one and say why in the answer, because the next person will
wonder.

## Done when

- One of the two is implemented and `IMPLEMENTATION.md`'s note "One ordering the skill
  store broke, and it is live now" is replaced by what was done.
- A test covers a curate pass launched on a dirty store, since that is the only shape where
  the two orderings differ.

## Where the evidence is

`IMPLEMENTATION.md`, the paragraph named above, and build ticket 12's `## Outcome`, which
flagged this before ticket 07 wired the pinning that made it real.

## What ticket 40 settled, added 2026-09-06

**The checkout stays, so both your orderings stay live.** Ticket 40 closed with the run's own
checkout as the design and `~/hydra/skills/` as a path no policy writes. Nothing here dissolves.

**The objection to the second ordering was measured and it does not hold.** A gate compiled while
`skill-worktrees/` does not exist yet produces a byte-identical `scopes` block to one compiled after
the checkout is there, and both allow a write inside it. So "the gate names a directory that does not
exist yet" costs nothing at compile time. The one caveat is `resolved()`'s: it canonicalizes the
longest existing ancestor and appends the rest, so a symlinked component appearing only after
compilation would leave the two strings apart. `Layout::discover` reads the passwd home and
`/Users/USER/hydra` has no such component.

**The scope you are writing against has a different name now.** `file_write` on `curate` moves from
`skill_store` to a new `skill_checkout` scope, and `skill_store` goes back to meaning
`~/hydra/skills/` under every policy. `policies/curate.json` is protected config #1 and a human
types that line. Build ticket 14 carries the change.

**One thing that bears on your first option.** `~/hydra/skills/` is now denied to every policy at the
gate, so a curate pass calling `Store::pin` itself is Hydra doing the commit rather than the child,
which is the only writer the store has left besides a human. That makes the first ordering the one
that composes with the deny list rather than around it.
