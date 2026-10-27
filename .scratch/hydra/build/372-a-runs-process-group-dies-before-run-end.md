# 372 - A recorded run's process group dies before `run_end`

Type: build
Status: claimed
Blocked by:

## Objective

Carry out map ticket 136, `.scratch/hydra/issues/136-does-a-runs-process-group-die-with-the-run.md`, with the panel's reasoning and measurements in `.scratch/hydra/research/136-synthesis.md`. Read both first.

Today, when the provider exits, `watch` in `src/watch.rs` writes `run_end` and leaves alive any descendant in the provider's group that ignored SIGHUP. That descendant keeps the run's sandbox and can write the worktree and the private git directory while reap commits. The panel decided: Hydra does not want orphans to outlive a run.

## Rules

1. On a recorded, non-interactive run, save the process group id the verified child leads while the child is alive, around `src/watch.rs` line 214. `stop::terminate` at `src/stop.rs` line 275 returns early once the leader is dead, and here the leader has been reaped by `spawned.wait()` at line 252, so the group needs its own path. Factor a group-only terminate out of `stop.rs`: SIGTERM to the group, wait up to `stop::GRACE` while `group_remains`, then SIGKILL. A pgid cannot be reused while a member lives, which is the argument `stop.rs` already makes around line 271.
2. Run it after `wait` and before `writer.hydra_line(&RunEnd::...)`, so reap never sees a closed record with a live group. An empty group costs nothing: no wait and no note.
3. When members were signalled, append a note to the error `run_end` carries, the way `PTY_HELD_NOTE` is appended around line 300. Name each member by pid and command name, read before SIGTERM: `proc_listpids` with `PROC_PGRP_ONLY` on macOS, `/proc` on Linux. Say which needed SIGKILL. The note must not start with `STOP_NOTE` or `SIGNAL_NOTE`.
4. The note never changes the outcome. `derive` in `src/outcome.rs` around line 259 sends any error on a run without a terminator to `Failed`. Make this note, and `PTY_HELD_NOTE`, informational there, so a `claude` or `agy` run without a terminator stays `Crashed` and a run with one keeps the provider's word. Test both.
5. Interactive sessions are out of scope: they share the human's job, and the child does not lead its own group.
6. Linux is not finished here. Under bwrap's `--new-session` the provider leads a session of its own, so the group of the pid Hydra records may not hold the provider's children. Build 370 measures it. Write the Linux path the same way, add a comment naming 370, and do not claim it works.

## Do

1. Tests first in `tests/pty_tee.rs` or `tests/detached.rs`, whichever drives a stub provider through `watch`. One stub leaves a grandchild that ignores SIGTERM and SIGHUP and touches a file every 100 ms. The grandchild is dead before `run_end` is on disk, the file stops growing, and the note names it. A second stub leaves a `setsid` grandchild, which survives: assert that and kill it in the test's teardown. A third leaves nothing: no note, and no 10 s wait, measured by elapsed time.
2. Amend `IMPLEMENTATION.md` line 42, which says it is undecided whether Hydra wants orphans to outlive a run: it does not, on both platforms, with the Linux half waiting on 370. Amend the paragraph near line 488 about `hydra stop`'s group signalling. `docs/run-record-contract.md` section 6, where it lists what `run_end.error` can carry, gains the note.
3. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, `cargo nextest run --release -p hydra --test pty_tee --test detached --test outcome --test status`, and `cargo nextest run --release -p hydra --lib stop`.

Build 373 runs beside you in `src/reap.rs`. Do not touch it.

## Outcome
