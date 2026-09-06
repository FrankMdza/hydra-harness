# 05 - Detached spawn, the timeout, and `hydra status`

Type: build
Status: resolved
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

## Outcome

Built, and measured on `claude` 2.1.261 on 2026-09-05. `src/watch.rs` is the detached
half, `src/run.rs` is the view over it, `src/live.rs` is contract 5.2's test, and
`src/status.rs` and `src/logs.rs` are the two commands that read it. 187 tests pass,
`tests/detached.rs` and `tests/status.rs` are new, and `IMPLEMENTATION.md` has a section
of its own for this.

**`hydra run` re-execs itself as a hidden `hydra watch <spec>` under `setsid`.** The
launcher composes the environment check, the worktree, the gate, `claude`'s argv and the
whole of line 0 except `pid` and `process_start_time`, writes all of it to
`~/hydra/spawn/<run-id>/launch.json`, and forks. The detached half opens the pty, fills in
the two liveness fields, tees and closes the record. `tests/detached.rs` asserts the
session id differs from the launcher's, which is the mechanism rather than a symptom of
it: a Ghostty window closing hangs up the foreground process group of its tty, and a
process in its own session has no controlling terminal to be hung up on.

**Handing the detached half a binary and an argv rather than a provider is what made the
whole path testable.** The suite drives it with `/bin/sh` and spends no subscription, and
that fell out of the design rather than being bolted on: everything provider-specific had
to happen before the fork anyway, so what is left on the other side is "run this argv,
tee it into this file".

**The environment is rebuilt on the far side rather than carried in the spec.**
`HTTPS_PROXY` can hold a password and contract 5.3 keeps values off the record for that
reason, so the same reasoning keeps them out of the spawn directory. Names reach the
record; values never touch disk.

**Measured, the timeout leaves the child alone.** Run
`01a07471-0a3e-74d2-b576-153ac0e7427b`, `read-only`, `--timeout 4s`: the call returned a
handle with `outcome: timed_out`, `alive: true` and pid 49537, the child carried on with
nobody watching, `hydra status` showed it live, and it closed its own record two minutes
later. That record derives `completed`.

**`hydra status` reconciles the real corpus.** On this machine, twelve records: eleven
ended, one live during the measurement, and twelve worktrees waiting for a reap. Under
10ms for the listing pass, which reads line 0 and the tail of each file and never
normalizes. `hydra status <run-id>` on `01a073ef-701c-7522-b5bb-56d5ecba0f57`, launched by
ticket 04 and read by a later binary, returned contract 17's projection off the same
function `hydra run` uses, base tree `7c222fb9d996872a39801ad14ef7db1d835280fb` recovered
from the run's `seed-index`. So the ticket's second option was the right one: nothing is
persisted beside the indexes, because `git write-tree` against an index nothing writes to
again answers with the sha it answered with at launch.

**The concurrency cap is eight and it is a build choice.** Section 9 asks for a cap and
names no number. It refuses with `live::TooManyLive`, a type carrying the run ids, and
writes no record: contract 8.3 makes a refused spawn a record when the refusal is one of
contract 6's four codes, and the cap is not one of them.

## What this ticket assumed that turned out otherwise

**The ticket's two sentences about `timed_out` point opposite ways, and one of them had to
give.** It says "the timeout's handle carries no outcome, which is the honest answer for a
child still running" and, in the same paragraph, "`timed_out` has no producer until you
write one" with "the watcher giving up is the only thing that should". Both cannot hold:
the only moment a watcher gives up is the moment the record has no ending, and nothing can
write to that record anyway, because the child's own writer owns it and contract 6 gives a
file one writer at a time.

What is built takes the producer sentence. `Outcome::TimedOut` is constructed on the
handle, which is the call's answer rather than the record's, and the record keeps its own.
Contract 8's row reads "the watcher gave up; the child may still be running", and that
second clause is only ever true of a call, never of a `run_end`.

The alternative worth naming, because it looks right and is not: have the detached half
note on `run_end` that the call's budget expired, so the record derives `timed_out` later.
That would file `01a07471-0a3e-74d2-b576-153ac0e7427b` in the corpus as a timeout when it
completed perfectly, which teaches rung one a lesson about the caller's patience while
looking like one about the task. Killing the child at the deadline instead is what harness
spec 9 rules out in as many words.

**This is a decision rather than an implementation, and it belongs on the map.** It is
recorded in `IMPLEMENTATION.md` under the detached-spawn section and it is one line in
contract 8's table plus one value in 8.2's enum: either `timed_out` is a call-level
outcome the record never carries, which is what is built, or `outcome_source` gains a
fourth value for an outcome nobody read from a file. The build did not stop for it, because
the other four "done when" items are pure implementation and hold either way, and because
the shape that is built is reversible: it constructs the enum in one place.

## What the code review found

Seven, all real, all fixed with a test each where a test could reach them.

**The one that would have bitten a user first: `hydra logs --follow` gave up one poll too
early.** The detached half reaps its child and then writes the closing line, so every
ordinary run passes through a window where the pid is gone and the record has not ended. A
follow landing in it printed "this run has no closing line and its process is gone" and
returned without the `run_end`. It is an `fsync` wide against a 100ms poll, which is a
percentage of every followed run rather than a theory. `wait_for_end` already guarded the
same race and `logs` did not.

**A launch that failed before the fork wrote no record at all.** Moving the `Writer` into
the detached half took the record with it, so `write_spec`, `current_exe` or `detach`
failing left a worktree, a branch and a compiled gate on disk with nothing in
`~/hydra/runs/` to explain them. `run::never_started` writes it from the foreground now,
which is safe precisely because the failure is what stopped the second writer existing.

**`live::scan` was all-or-nothing and now gates every launch.** One unreadable entry under
`~/hydra/runs/`, a directory named `*.jsonl` included, stopped `hydra run` and `hydra
status` alike. It skips with a note.

**The cap's refusal told the user to run `hydra reap`, which this binary does not have and
which would not have helped.** The count is of live runs, so reaping an ended one frees
nothing.

**A `--timeout` of `u64::MAX` panicked rather than erroring**, after the worktree, the gate
and the child already existed. The deadline uses `checked_add` and `None` means the budget
never expires, which is what somebody typing that number meant. Measured after the fix, run
`01a07484-1667-7500-b299-2889144b84c1`: the same flag launches, waits with no deadline and
returns `completed`.

**A launch in progress was reported as a worktree waiting for a reap.** The checkout is
created and seeded before line 0 exists, so for seconds on a large repo the directory has
no record and reads like a pruned run. Age tells them apart, and
`status::abandoned_worktrees` takes the grace period as an argument so ticket 06's reaper
picks it at the call site.

**`hydra status <run-id>` hard-failed where `hydra run` degrades**, on a record the
normalizer chokes on. It drops the projection and keeps `state`, `pid` and `run_file`.

An eighth was found and fixed while the review ran: `parse_duration` split the string one
byte from the end, so `--timeout 10é` panicked on a character boundary.

## What this leaves for later

**Nothing in this build can stop a run.** Ctrl-C on `hydra run` stops the view and leaves
the child, which is what section 9 asks for and is also a way to leak a run you meant to
stop. `hydra status` finds it and `hydra reap` is ticket 06.

**`~/hydra/spawn/` is now a live hazard rather than a latent one.** 7.5 denies each run
its own gate script and nothing else in that directory, and eight runs can be in flight at
once now, so a `permissive` one among them can rewrite another's live script.
`IMPLEMENTATION.md` already carried this as latent; ticket 05 is what made it reachable.

**Harvest is the half of `hydra status` that is not built.** Section 10 has it complete a
dead interactive run from the vendor transcript, and no command in this build launches an
interactive run, so there is nothing on this machine to harvest. It arrives with the
interactive path rather than here.
