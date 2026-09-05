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
