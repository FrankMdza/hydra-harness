# 47 - Snapshot creation races when multiple runs launch on the same skill store sha

Type: build
Status: resolved
Blocked by:

Filed as 44 by another session on 2026-09-12 at 11:37, one minute before the orchestrator's 44 on
the hook config was committed; renumbered to 47 the same day so the index stays one ticket per
number. Nothing else changed.

## What

Concurrent runs launched on the same skill store sha can race to create
`~/hydra/skills-snapshots/<sha>`. `src/skills/mod.rs:137-166` in the `snapshot()` function
calls `git worktree add --detach -q <dir> <sha>` to check out the store at that sha. When
three runs launch in the same second on the same sha (observed 2026-09-12 13:52:10 to
13:52:22), one wins and creates the snapshot. The other two fail: one with "fatal: '<dir>'
already exists" and another with "Unable to create '.../skills/.git/worktrees/<sha>1/index.lock':
File exists".

The losing runs do not clean up. They leave orphaned worktrees in `~/hydra/worktrees/`
without run records in `~/hydra/runs/`, and `hydra status` lists them as "waiting for a reap".
The snapshot stays registered in the store as prunable with metadata in `.git/worktrees/<sha>1`,
and every subsequent launch on any sha fails with "already exists" because the store thinks
the snapshot still exists.

Manual recovery required: `hydra reap` of the orphaned worktrees, `rm -rf
skills-snapshots/<sha>`, `git -C skills worktree prune`, then the next launch succeeds.
Runs on a sha that already has a snapshot do not race (prior concurrent launches on sha
`911f896` spawned three at a time without collision).

Fix three things. First, make snapshot creation idempotent and serialized: use a
`.lock` file at `skills-snapshots/<sha>.lock` or create in a temporary directory and
rename on success. If the snapshot directory exists and is registered in the store, reuse
it. If it exists but is unregistered or marked prunable, repair it instead of failing.

Second, ensure a launch that fails before writing the run record cleans up the worktree
it left in `~/hydra/worktrees/` — either prevent leaving one, or remove it on failure.

Third, add a test in `tests/` that spawns two concurrent snapshots of the same sha and
waits for both to complete with the snapshot valid. Both must succeed or the snapshot must
be valid for both to use.

Snapshot creation lives at `src/skills/mod.rs:145-155` in the `git worktree add` call.

## Outcome

Resolved 2026-09-12. `Store::snapshot` in `src/skills/mod.rs` now checks the store out into
`skills-snapshots/<sha>.tmp-<pid>-<n>` and asks `git worktree move` to put it at `<sha>`. The
move is one `rename(2)`, a directory cannot be renamed onto a non-empty one, so one launch wins
and a loser removes its own staging checkout through git and takes the winner's. Any existing
directory is verified before use: `HEAD` in it must be the sha, and the store must list it. A
directory the store lost track of gets `git worktree repair`. One at another sha is refused
with an error naming what to remove by hand, because deleting it under a run that may be
reading it is not the launch's call. A launch that dies mid-checkout leaves a staging
directory, never a half-made `<sha>`, and the reap sweep removes it once it is past the grace
and unreferenced, like any other snapshot.

**The rename was not enough, and the ticket's lock file turned out to be the sound half.**
The first race test, four threads on four unique staging paths, failed one time in four with
`fatal: failed to read .git/worktrees/<other staging>/commondir: Undefined error: 0`. Every
`git worktree` command begins by reading every entry under the store's `.git/worktrees/`, and
an entry another launch is in the middle of making has no `commondir` yet. Unique paths do
nothing about that. So the git calls run under `flock(2)` on `skills-snapshots/.lock`, held
by `snapshot` for its few calls and by `remove_snapshot` for its one. The kernel releases the
lock with the descriptor, so a launch that dies holding it wedges nobody, there is no stale
state to clear, and the file is never read. It is `libc::flock` rather than
`std::fs::File::lock` because `Cargo.toml` pins `rust-version = "1.88"` and the std call
arrived in 1.89. The rename stays because it is what keeps a crash from leaving a broken
`<sha>` for the next launch to refuse.

The ticket's "already exists on every subsequent launch" and the `index.lock` collision are
both the same git behaviour seen from two sides: two `worktree add`s picking the same admin
name and reading each other's half-made entries. Under the lock neither can happen.

The ticket's second ask, cleaning up a checkout under `worktrees/` when a launch fails before
line 0, is not in this change. `hydra run` creates the worktree before it calls `snapshot`,
so a snapshot failure still leaves one. With the race gone the failure no longer happens on
a healthy machine, and the checkout a failed launch leaves is what `hydra reap` already
reports as waiting for a reap. A change to what `run.rs` does on its failure paths is a
ticket of its own.

Tests, `tests/skills.rs`: `launches_racing_on_a_fresh_sha_all_end_with_the_one_snapshot`
races four threads on one fresh sha behind a barrier and checks that all four get the one
directory, it is at the sha, the store's `git worktree list --porcelain` shows exactly it
beside the store, and no staging directory survives.
`a_snapshot_the_store_lost_track_of_is_repaired_and_one_at_the_wrong_sha_is_refused` covers
the two verification branches. The sequential `two_runs_on_one_sha_share_one_snapshot` is
unchanged and passes.

`cargo fmt`, `cargo clippy -- -D warnings` clean. `cargo test --test skills` run five times
in a row: `23 passed; 0 failed` each time. The full suite and the release build were not
run, on the caller's instruction, since two other agents shared the machine.

`IMPLEMENTATION.md`, "The skill store, the snapshot and what a run is linked", carries the
mechanism.
