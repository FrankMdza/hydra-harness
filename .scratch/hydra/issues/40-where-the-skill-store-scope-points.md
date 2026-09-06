# 40 - Where the `skill_store` scope points, now that two documents disagree

Type: grilling
Status: open

## Question

Graduated from `## Not yet specified`, "The curator's own worktree of the skill store",
which said the open detail was where that worktree lives and that it was sharp enough to
confirm rather than decide. Build ticket 12 built it and the confirming never happened, so
the code now answers a question the documents answer two ways.

**Is the `skill_store` scope `~/hydra/skills/` literally, or the run's own checkout of
it?** Harness spec 7.2 says the first: "`curate` writes to `~/hydra/skills/`, outside its
own worktree". `docs/ouroboros-spec.md` 3.6 says the second, and 9.1 flags it as a
refinement to confirm rather than assume.

What the code does, as of 2026-09-06: `Layout::skill_store_scope(run_id, policy)` returns
`~/hydra/skill-worktrees/<run-id>/` for `curate` and `~/hydra/skills/` for everything else.
Build ticket 12 chose that and had a reason the map should hear, because it is a better
reason than either document gives:

**The scope has to be a function both sides can compute.** Contract 7.7 re-derives every
`permission_decision` by running the run's policy again at read time. No field on the
record carries a scope. So a scope that lived only in the compiled gate would make every
curate run read back as a run whose gate leaked on its own writes, which is exactly what
happened before the fix: ten `Finding::GateLeak` on one pass, one per file the curator
touched in its own checkout. That is ticket 42's problem seen from one instance.

## Settle

- **Which document moves.** One of 7.2 and 3.6 is wrong and they cannot both stand. 7.2 is
  a table row in the harness spec and 3.6 is a paragraph in the Ouroboros spec, so this is
  a one-line edit in whichever loses.
- **Whether the answer keys on the policy name.** `skill_store_scope` branches on the
  string `curate`, which makes a policy name load-bearing in the layout. The alternative is
  a field on the policy, which is a policy schema change, or a per-run argument, which is
  what ticket 12 removed to make the replay work.
- **`~/hydra/skill-worktrees/` is a sixth place and harness spec 5 names four.** Ticket 12
  put it beside `worktrees/` rather than in the spawn directory on the grounds that `hydra
  reap` removes a spawn directory whole and a pass's uncommitted proposal is work. That
  reasoning is sound and the table still does not have the row.
- **Who removes one.** Nothing does. `hydra reap` sweeps worktrees, spawn directories and
  now skills snapshots; a curate checkout survives every command in the surface.

## Blocks

Ticket 41 needs this answered first: the order of the curate checkout and the store pin
depends on where the checkout is.

## Where the evidence is

`src/paths.rs`, `Layout::skill_store_scope`. `IMPLEMENTATION.md` on `hydra curate` and on
the skill store. Build tickets 07 and 12, both `## Outcome`.
