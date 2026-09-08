# 49 - What the Herdr bridge reports, and whether the record carries `base_tree`

Type: grilling
Status: resolved
Blocked by:

## Question

Ticket 44 put the interactive layer in Herdr and ticket 45 measured what holds. Two things it
found are decisions, and the bridge cannot be built around either without someone making them.
Both are small. Both are the human's.

**Which Herdr state `timed out` sits on.** Herdr holds four pane states, `idle`, `working`,
`blocked` and `unknown`, and refuses any other name. Hydra's words go on as display labels over
one of those four through `report-metadata --state-label`. `completed` and `failed` sit on `idle`
and `gated` sits on `blocked`, and nobody has argued about those. `timed_out` is the odd one: the
watcher gave up and the child may still be running, so the pane is neither idle nor known to be
working. The bridge can read `hydra status` and tell a live child from a dead one, which would
split the word over two states, or it can pick one and say so.

**Whether the record carries `base_tree`.** `diff_stat.base_tree` is computed at read time from
the seed index in the spawn directory and printed by `hydra run` on exit. It is on no record. The
tree it names is reachable from no ref, so `git gc --prune` deletes the base of every past run, and
`hydra reap` removes the spawn directory that could recompute it. Hunk opens `hydra/<run-id>`
without it. What it cannot do is separate the run's own edits from the seed, because reap commits
both in one commit with `git add -A`. Writing the sha into `run_start.worktree` is a contract
change under section 15, one field, and it would also let a reader tell the two apart on a reaped
run. Not writing it means the diff stat on the record's projection is the only place the number
ever existed.

## What a session can do without the answers

Build the bridge with `timed_out` on `unknown`, which is the state Herdr itself reserves for
"present but unclassified", and leave `base_tree` off the record. Both are one-line changes when
the human answers differently, and the ticket exists so that the bridge does not get built as if
the answers were settled.

## Where the evidence is

Ticket 45's `## Answer`, sections 2 and 5. Ticket 44's `## Note from ticket 45`. `outcome.rs` for
where `diff_stat` is computed and `worktree.rs` for the seed index.

## Answer

Resolved 2026-09-07 with the human, two questions, both answered in one round.

**`timed_out` sits on `unknown`, and the pane says more than the state.** Herdr's `unknown` is
the state it reserves for "present but unclassified", which is what a watcher giving up means.
The bridge keeps `working` while `hydra status` says the child is live, so `timed out` is only
ever displayed over a process that died without a `run_end`, and an abandoned run sits there
too. The human's addition: since Herdr cannot classify a Hydra run, the pane carries what a
human needs to understand it without Herdr's help, as tokens and title from the record:
`outcome`, `state` as Hydra reports it, whether the pid is alive, the last tool the run called,
and elapsed time. Build ticket 24 carries that.

**`base_tree` goes on the record, in `run_start.worktree`.** Contract 5's `worktree` row gains
`base_tree`, the sha `git write-tree` answers against the seed index the moment the worktree is
seeded, before the provider starts. The tree it names stays unreachable from any ref, so the
field is a name and not a guarantee that the object survives a `git gc`, and the contract says
so. What it buys: a reader of a reaped run can separate the seed from the run's own edits by
diffing `base_tree` against the reap commit, which today conflates the two; Hunk can open a
run against it; and the corpus stops attributing the human's uncommitted work to the model.
`hydra reap` stays at one commit. Build ticket 23 carries the field, and a record written
before it has no `base_tree` and re-derives as it does today, from the seed index if the spawn
directory still exists and as absent otherwise.
