# 05 - Detached spawn, the timeout, and `hydra status`

Type: build
Status: open
Blocked by: 02

## Goal

Children spawn detached and blocking becomes a view over them. `hydra status`
reconciles live pids against open run files. `hydra logs --follow` tails one.

## Where the design says it

- Harness spec 9, every child detached with blocking as a view, `--timeout` defaulting
  to roughly ten minutes, degrading to a handle rather than killing, and a concurrency
  cap that refuses rather than queueing.
- 10, the run lifecycle: process exit is the authoritative end, liveness is derived,
  nothing appends but the owning process, and the worked Ghostty scenario.
- Contract 5.2 for the liveness fields, which are already built and tested, and 6 for
  ownership transferring to whoever proves the previous owner is dead.

## Done when

- A child survives its parent, and closing the terminal orphans nothing that matters.
- `hydra status` derives a dead parent, reports live children by matching pid and start
  time, and reports abandoned worktrees.
- `hydra logs <run-id> --follow` tails the run file and is the only view. No attach, no
  input, no answering a prompt from outside.
- The timeout returns a handle rather than killing the child.
- The concurrency cap refuses with a typed error naming what is in flight and its run
  ids.

## Watch for

**This is blocked on 02 rather than on nothing.** Detaching a child that has no gate is
strictly worse than running it in the foreground, because the human watching is the only
thing between it and the filesystem.

**Liveness answers whether a process exists and nothing about whether it is working.**
There is no stall detector and contract 18.4 says why: `agy` keeps a heartbeat by
accident and the other two would need a flag section 6.7 turns off for reasons that hold
on their own. Do not invent a timeout with a decimal point on it.

**A missing `run_end` is a fact rather than a corruption.** The first slice already
produces one every time it is killed mid-run.

## What ticket 04 left you, added 2026-09-05

**`hydra run` now reads the record back before it returns, and that read is yours to
move.** Contract 17's result is a projection of the file, so `hydra::run::result` takes a
normalized `Run` and hands back the JSON. On a blocking call that read costs a python
process per recorded tool call, measured at about 47ms each, so a 200-call run spends nine
seconds after the child exits before anything prints. Detaching is what fixes it: the
watcher tails the file and derives when it stops, and `hydra status` derives from the same
function on a run it did not launch.

**A run with no `run_end` has no outcome, on purpose, and it is your case.**
`outcome::derive` returns `Option` and `None` means the run has not ended. Contract 8.6
gives telling a live child from an abandoned one to `hydra status`, because it needs 5.2's
pid liveness test rather than the file. So the timeout's handle carries no outcome, which
is the honest answer for a child still running, and `timed_out` has no producer until you
write one.

**`timed_out` is in the enum with nothing making it.** `outcome::Outcome::TimedOut` exists
and `rung_one_learns()` returns true for it, per contract 8's table. Nothing constructs it,
and the watcher giving up is the only thing that should.

**The diff stat's base is on disk in the spawn directory, not only in memory.**
`worktree::seed_tree` snapshots the seeded checkout through a scratch `GIT_INDEX_FILE`,
`Layout::seed_index` and `Layout::stat_index` name the two files, and the tree sha rides on
`Worktree::seed_tree`. A detached run returns before the diff stat can be computed, so
`hydra status` needs the sha: either persist it beside the indexes or recompute
`seed_tree` from `seed-index`, which still holds the seeded tree.
