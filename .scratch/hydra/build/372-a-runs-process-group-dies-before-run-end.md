# 372 - A recorded run's process group dies before `run_end`

Type: build
Status: resolved
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

The recorded watcher saves the group led by the verified child before waiting. After reaping
the child it sends SIGTERM to that group, waits up to `stop::GRACE`, then sends SIGKILL.
It waits for members to disappear before writing `run_end`. An empty group adds no note or wait.
The error note names each pid and command read before SIGTERM and identifies SIGKILL recipients.
macOS reads members with `proc_listpids` and `PROC_PGRP_ONLY`. Linux reads `/proc`.
The Linux sandbox boundary still waits on build 370. Interactive sessions are excluded.

The stub descendant ignores SIGTERM and SIGHUP and writes every 100 ms. It is dead when the
closed record is read, its file stops growing, and the note names its pid and command.
The setsid descendant survives and test teardown kills it. The combined fixture took 11.8 s.
The empty-group integration test took 0.45 s. Unit tests also cover an empty group and a
member that exits on SIGTERM.

Cleanup and held-pty notes preserve provider outcomes. Tests cover claude and agy with and
without terminators. The existing held-pty test now expects crashed rather than failed,
as rule 4 requires. IMPLEMENTATION.md and contract section 6 describe the mechanism and limits.

All required checks passed:

- `cargo fmt --all --check`
- `cargo clippy --release --workspace --all-targets -- -D warnings`
- `cargo check --target x86_64-unknown-linux-gnu`
- `cargo nextest run --release -p hydra --test pty_tee --test detached --test outcome --test status`, 86 passed
- `cargo nextest run --release -p hydra --lib stop`, 2 passed

The last command initially selected no tests and exited 4. This ticket's assumption that
stop had library tests was false. Two focused tests now make that check exercise the mechanism.
The cross-check proves compilation only. It does not establish Linux sandbox cleanup.

Applied by the session with two changes. With the group kill on every path, the stop tests in `tests/stop.rs` hung past their 30 s limit in two runs of three, and passed three of three on master without it. A `hydra stop` already signals the group and waits on it, build 173, so the watcher now skips its own termination when the stop marker exists, and the stop file passed four of four after that. The second change: an error from `terminate_group` used to return from `watch` before `run_end`, which leaves a record open for good, so it is now a note on `run_end.error`. The race between the two terminations was not traced further.
