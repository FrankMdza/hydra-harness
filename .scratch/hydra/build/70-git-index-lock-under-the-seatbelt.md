# 70 - A run under the seatbelt cannot take git's index lock

Type: build
Status: open
Blocked by:

## What

Build ticket 68 put the seatbelt around every recorded launch and denied `~/hydra/.git` to
writes, because the parent repository's `.git` holds every run's branch and, on this machine,
Hydra's own history. A worktree's private git directory lives inside it, at
`~/hydra/.git/worktrees/<run-id>/`, and that is where `index`, `HEAD` and `logs/` for the run's
checkout are. Measured 2026-09-22 on a real profile: inside a run `git add`, `git commit` and
`git stash` fail with `Unable to create '.../index.lock': Operation not permitted`, and `git
status` fails when it wants to refresh the index and works when it does not. `git diff`, `git
log` and `git rev-parse` work. Two recorded runs from 2026-09-21 hit it, 01a0c5b5-b8e6 on
`codex` and 01a0c5d8-af17 on `claude`, and each said so in its report. `SECURITY.md` names it.

## Done when

The run's own private git directory is an own write path in `policy::own_write_paths`, so the
profile re-allows it after the deny on the parent, the way the worktree is re-allowed, and the
gate's carve-out agrees. `git add` and `git status` work inside a run; `git commit` still cannot
write objects into the shared `~/hydra/.git/objects`, and whether a run may commit at all is a
decision this ticket does not make: `hydra reap` commits from outside and the record's diff is
against the seeded tree either way, so say in `IMPLEMENTATION.md` that `commit` stays outside.
`tests/spawn.rs` drives `git add` under the real profile. One pong per provider completes.
