# 373 - Reap refuses while a process holds the run's checkout or its private git directory

Type: build
Status: resolved
Blocked by:

## Objective

Map ticket 136, `.scratch/hydra/issues/136-does-a-runs-process-group-die-with-the-run.md`, with its synthesis in `.scratch/hydra/research/136-synthesis.md`. Build 372 kills the run's group at run end, but a descendant that called `setsid` is outside the group and survives. Three panel seats measured that on this Mac. It keeps the run's seatbelt profile, so it can write the worktree and the private git directory while reap sweeps and commits. Reap should not race a writer it can see.

## Rule

Before `sweep_private` runs for reap, and before reap's first git command on a run checkout, reap lists every process that has its cwd, or an open file, under the worktree or under `<git>/worktrees/<run-id>/`. If any exist, reap refuses that run and names each process by pid and command, and says `hydra stop` or `kill` is the way out. Reap goes on with the other runs it was asked to reap. Hydra's own process is excluded.

Use the platform's own interface, not a subprocess where one can be avoided. On macOS that is `proc_listpids` with `proc_pidinfo` and `PROC_PIDVNODEPATHINFO` for the cwd and `PROC_PIDLISTFDS` with `proc_pidfdinfo` for open vnodes. On Linux, `/proc/<pid>/cwd` and `/proc/<pid>/fd`. If `lsof` turns out simpler and correct, say why in the Outcome. A process that cannot be inspected for lack of permission belongs to another user, and the run's processes all run as this user, so skip it.

A live run is already refused by reap through `src/live.rs`. This covers an ended run whose descendants did not end.

## Do

1. Tests first in `tests/cleanup.rs`: an ended run whose checkout has a `setsid` process sleeping with its cwd in the worktree is refused by name; the same with the process holding a file open under the private git directory; once the process is killed, reap goes through. Clean up every spawned process in the test.
2. Do not touch `src/watch.rs`, `src/stop.rs` or `src/outcome.rs`. Build 372 runs beside you there.
3. Amend `IMPLEMENTATION.md` where it describes reap, and `SECURITY.md` where it describes what survives a run.
4. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, and `cargo nextest run --release -p hydra --test cleanup --test status`.

## Outcome

Reap lists holders before it sweeps a private git directory or runs git on a checkout. `procinfo::holders` walks this uid's processes through `proc_listpids` on macOS and `/proc` on Linux, reads cwd and open vnodes, and skips Hydra's own pid and any pid that cannot be inspected. A match is `Reaped::Failed` naming each pid and command and saying `hydra stop` or `kill` is the way out. Other runs in the same call still reap.

Tests in `tests/cleanup.rs` spawn a `setsid` python with its cwd in the worktree, and another holding `HEAD` open under the private git directory. Both are refused by pid. Killing the holder lets reap commit. A second ended run in the same sweep still reaps. Drop of the helper kills the child.

`lsof` was not simpler. libc already exposes `proc_vnodepathinfo`, `proc_fdinfo`, `proc_pidinfo` and `proc_pidfdinfo`. The missing wrapper is `vnode_fdinfowithpath`, defined next to those calls.

Measured on this Mac: `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, and `cargo nextest run --release -p hydra --test cleanup --test status`, 86 passed. The Linux path compiled and did not run.

Nothing the ticket assumed turned out false.
