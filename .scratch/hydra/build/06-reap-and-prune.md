# 06 - `hydra reap` and `hydra prune`

Type: build
Status: open
Blocked by:

## Goal

The two cleanup commands, both typed by a human and reachable from no run at either
Ouroboros rung. Unblocked today, and it clears the three probe worktrees the first
slice left behind.

## Where the design says it

- Harness spec 10: nothing is cleaned up automatically, reap never removes a worktree
  with uncommitted changes but commits them to the child's own branch first, and prune
  moves run files rather than deleting them unless a flag says otherwise.
- 17 for both rows of the command table.
- Contract 2, `hydra prune` moves because anything deleted is training data destroyed.

## Done when

- `hydra reap [<run-id>]` commits a worktree's uncommitted changes to the child's
  branch, then removes the worktree and the run's spawn directory together.
- `hydra prune` moves run files out of `runs/` and deletes only behind a flag.
- Neither is callable from a run.
- The seven worktrees under `~/hydra/worktrees/` and their `hydra/<run-id>` branches
  are gone, reaped by the command rather than by hand.

## Watch for

**This is the Hermes lesson turned on our own tooling.** Hermes told its model that
archiving was recoverable while the tool underneath called `shutil.rmtree`. A guarantee
written in a prompt is not a guarantee, so reap's commit-first behaviour has to be real
rather than documented.

**Reaping a worktree makes its record's `worktree.path` point at nothing.** That is the
intended end state and contract 5.3 already leans on it: if reading a spawn artifact
were needed to interpret a run, the record would not be the source of truth.

## What ticket 02 left you, added 2026-09-05

**There are seven worktrees now, not three, and only four spawn directories.** Ticket
02's measurement added three and the shell fix that followed added a fourth. The first
slice's three predate the spawn directory entirely, so reap has to cope with a worktree
whose `~/hydra/spawn/<run-id>/` was never created rather than assuming the pair. Section
5 says the two are reaped together, so reap removes both or it has not finished.

**`~/hydra/spawn/` exists and `Layout` already names every path you need**:
`spawns()`, `spawn(run_id)`, `gate_script(run_id)` and `settings_file(run_id)`.

**Reaping a spawn directory removes a file the gate denies writing to**, and that is
fine rather than a contradiction. The denial binds the child; reap is a human command
that no run can reach, which is the same asymmetry harness spec 7.5 relies on when it
says the gate does not stop a run invoking `hydra` itself.

**The three ticket-02 records are the ones worth keeping.** They are the only evidence
on this machine that a compiled deny held under `bypassPermissions`, so if you exercise
`hydra prune` against real files, move them rather than testing the delete flag on them.
