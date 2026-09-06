# 06 - `hydra reap` and `hydra prune`

Type: build
Status: resolved
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

## What ticket 04 left you, added 2026-09-05

**A spawn directory now holds four files, not two.** `gate`, `settings.json` and two git
index files: `Layout::seed_index` and `Layout::stat_index`, which the diff stat stages
through so the child's own index is never touched. Reap removes the directory whole, so
this changes nothing about what to delete; it is here so a reader of `~/hydra/spawn/`
does not wonder what the indexes are.

**Reaping a worktree destroys the diff stat's base.** Harness spec 10's reap commits
uncommitted work to the child's branch before removing anything, and the tree the stat is
measured against is only reachable while the objects live. That is the same reasoning
contract 17 gives for the diff stat not being a record field: the `tool_call` events are
the version that survives the branch.

**Recounted after ticket 04: ten worktrees, ten `hydra/*` branches and seven spawn
directories.** Ticket 04's own measurement run added the tenth,
`01a073ef-701c-7522-b5bb-56d5ecba0f57`. The three oldest still have no spawn directory, so
the pair reap assumes is not there for them. It is the only
rate-limited record on this machine and the only evidence here of contract 8's nastiest
shape on a live run, so keep it if you exercise `hydra prune` against real files.

## What ticket 05 left you, added 2026-09-05

**`hydra status` already prints the list you have to act on.** It reports every checkout
under `~/hydra/worktrees/` that no live run is using, which after any finished run is
most of them, and it does it from the pid in each record rather than from a directory
listing. `hydra::status::abandoned_worktrees` is that function and reap should read it
rather than walk the directory again, because the one thing reap must never touch is a
worktree whose run is still going.

**It takes a grace period as an argument, and reap has to choose one deliberately.** A
checkout is created and seeded before its run file exists, so for seconds on a large repo
a launch in progress has no record and looks exactly like a run whose record was pruned.
`hydra status` passes `hydra::run::LAUNCH_BUDGET`, ten seconds, which is what the launcher
itself waits for line 0. A reaper that passes zero will eventually remove the checkout of
a run that was about to start.

**Recounted after ticket 05: thirteen worktrees, thirteen records and ten spawn
directories.** Three of the thirteen are ticket 05's own measurement runs,
`01a07470-e247-7283-8aae-62d72aa86eda`, `01a07471-0a3e-74d2-b576-153ac0e7427b` and
`01a07484-1667-7500-b299-2889144b84c1`. The three oldest still have no spawn directory, so
the pair reap assumes is not there for them.

**A spawn directory now holds six files.** `gate`, `settings.json`, the two git index
files, and `launch.json` and `watcher.log` from the detached spawn. Reap removes the
directory whole, so this changes nothing about what to delete. `launch.json` carries the
run's composed task, which is already on line 0 of the record, so nothing is lost with it.

**Reap has to cope with a run that is still live.** Runs are detached now, so a human can
type `hydra reap` while one is writing. Contract 5.2's test is the guard and
`hydra::live::read_view` is it: a worktree whose run reads `live` is not reap's to touch,
and removing it would pull the checkout out from under a running child.

## Outcome

Both commands are built, `src/reap.rs` and `src/prune.rs`, wired into `main` and nowhere
else. `tests/cleanup.rs` is 20 tests against a throwaway repo, and the full suite is 207.

**Reap's commit-first rule is enforced by git and not by our own code, which is the whole
point.** `git commit` leaves the checkout clean, and only a clean checkout is one
`git worktree remove` will take without `--force`, which reap never passes. A commit that
did not happen therefore stops the removal instead of preceding it. That is the shape the
Hermes lesson asks for: the guarantee is a mechanism rather than a sentence.

**The ticket assumed the branch goes with the checkout, and it cannot.** "Done when" asks
for the thirteen `hydra/<run-id>` branches to be gone. Harness spec 10 also requires reap
to commit uncommitted work to the branch first, and every one of the thirteen checkouts was
dirty, so the commit makes each branch unmerged and deleting it would throw away the commit
reap had just made. Committing and then deleting is exactly the theatre section 10 is about.

So reap deletes a branch only when `git branch -d` will take it, meaning nothing would be
lost, and `--delete-branch` is `git branch -D` for a human who has decided otherwise. It is
the same shape prune gives deletion: recoverable by default, destructive behind a flag.
`docs/ouroboros-spec.md` 4.11 already expects a leftover branch to go by `git branch -D`.

**Measured on this machine, 2026-09-05.** `hydra reap` with no flag, thirteen checkouts:

- 13 checkouts removed, 24 MB, `~/hydra/worktrees/` now empty.
- 10 spawn directories removed, 408 KB, `~/hydra/spawn/` now empty. The three oldest runs
  had none, which is the case ticket 02 warned about and reap handles.
- 13 commits made, one per branch, printed with their shas. All 13 branches kept, because
  each now holds a commit. The two probe files ticket 02 produced, `probe.txt` and
  `probe-shell.txt`, are reachable at `git show hydra/<run-id>:probe-shell.txt`.
- 13 run records untouched. `hydra status` reports `worktrees waiting for a reap 0`.
- `git worktree list` is back to one entry and `.git/worktrees/` is gone.

The human was asked before the sweep ran and chose to keep the branches. Every checkout
held a snapshot of this repo at a commit already on master plus that moment's uncommitted
work, all of it since committed; the only bytes master had never seen were the two probe
files, each containing `HELLO`.

**Prune was exercised against the real corpus only as `--dry-run`,** which reported all 13
and created nothing, not even `~/hydra/pruned/`. The reason is a finding worth carrying:
**`hydra prune` with no arguments would empty the corpus `tests/record_contract.rs` walks.**
That test reads every `.jsonl` under `~/hydra/runs/` and returns quietly when there are
none, so a sweep costs the check silently rather than failing it. Prune by name.

**Prune moves to `~/hydra/pruned/<run-id>.jsonl`, and the destination is this build's
choice.** Harness spec 5's table says prune moves and names no destination, the same gap
19.1 leaves for the spawn directory's filenames. A sibling of `runs/` keeps the move a
rename on one filesystem and keeps the run id on the file, so putting a record back is a
`mv`. A destination already holding that run id is a refusal rather than an overwrite.
`pruned/` is in `.gitignore` beside `runs/`.

**Two guards the ticket did not name, both found while writing the tests.**

- A checkout on a detached HEAD is refused rather than reaped. A commit on one is reachable
  from nothing once the checkout is gone, which is the single shape where committing first
  saves nothing.
- Only the branch Hydra named for the run is ever deleted. A child that checked out one of
  the human's own branches gets its work committed there and the branch left alone.

**Reap takes the launch grace period as an argument, the way `status::abandoned_worktrees`
does,** and `main` passes `run::LAUNCH_BUDGET`. Naming a run id skips it: the grace exists
to protect a launch in progress, and a human who typed the id has answered that question.
A sweep also walks `spawn/` for a directory with no checkout beside it, because a run
outside a repo has the pair the other way round.

**Reap, then prune.** Reap derives liveness from the pid in each run record, so pruning
first leaves it reading records that are no longer there. `IMPLEMENTATION.md` says so too.

### What the code review found

`/code-review high`, five findings, all real, all fixed. Three carry a test that fails
against the old code; the other two are on paths a test cannot reach cheaply. 207 tests.

**Two of them were the same bug wearing different clothes, and it is a bug in the launch
window rather than in either command.** `watch` calls `Writer::create` before
`pty::spawn` and writes line 0 after, so for up to `LAUNCH_BUDGET` there is a run file
that exists and is empty. `read_view` can only call that abandoned with a null pid, which
is exactly what a run that died looks like.

- `status::abandoned_worktrees` applied its grace only when no record existed at all. A
  record that existed and said nothing took the other arm, so the age was skipped and the
  checkout of a starting run came back as reapable. Reap would then commit the seeded tree
  and `git worktree remove` the directory the child is about to work in.
- `prune` had no grace at all. The sweep moved the empty record; the writer's descriptor
  follows a rename, so the whole record would land in `pruned/` while the launcher waited
  ten seconds for a line 0 it could no longer see, then reported a running run as one that
  wrote nothing. `--delete` destroyed it outright.

`RunView::launched` is the shared test now: the record carries a pid, or it has ended.
Anything else is judged on age. A refused spawn passes because it has a closing line,
which is what makes contract 5.2's null pid a claim rather than a silence.

**Prune addressed records by rebuilding a path from `view.run_id`.** That id comes off line
0 and only falls back to the filename, so a record whose envelope disagrees with its name
had the sweep move a file that was not there, and the named form find no view and skip the
liveness check entirely, which is the half that loses a live run's record. It works on
`view.run_file` now and matches the named lookup by path.

**The copy-and-remove fallback could leave the record in both places.** A copy that lands
followed by a remove that fails left `pruned/` holding a duplicate, and the overwrite guard
then refused every retry forever. The copy is taken back out on that path.

**Reap collapsed every `git branch -d` refusal into "it holds commits".** A branch checked
out in another worktree is refused for a different reason that `--delete-branch` cannot fix
either, so the human got the wrong reason and a useless flag. `Done::branch_kept` carries
git's own first line now.
