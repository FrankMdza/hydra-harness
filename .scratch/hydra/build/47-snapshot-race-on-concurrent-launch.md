# 47 - Snapshot creation races when multiple runs launch on the same skill store sha

Type: build
Status: open
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
