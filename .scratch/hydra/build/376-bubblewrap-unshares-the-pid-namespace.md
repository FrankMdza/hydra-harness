# 376 - A bubblewrap launch unshares the pid namespace

Type: build
Status: resolved
Blocked by: 370

## Objective

Map 136 gave a recorded run's process group a SIGTERM and then a SIGKILL before `run_end`, and build 372 built it. On Linux the map left two questions for build 370: whether the provider stays in the group of the pid Hydra records, given bwrap's `--new-session`, and whether `--unshare-pid` kills what a run leaves behind. Build 370 measured both in Docker with bubblewrap 0.8.0:

- Under `--new-session` the sandboxed child leads a new session and process group, apart from the launcher's. Build 372's kill targets bwrap's group, which does not hold the provider's descendants.
- With `--unshare-pid`, a `setsid` descendant died when bwrap's command exited. Without it, the descendant survived.
- Delegation from inside a Linux run is refused through `inside_seatbelt()`'s mount-namespace witness, so `--unshare-pid` kills no delegated child run.

So the answer the map waited for is in: add `--unshare-pid` to the argv `compose_bwrap` writes. Keep `--new-session`, which build 318b added against `TIOCSTI`.

## Scope

- `src/sandbox.rs`: `--unshare-pid` in the bwrap argv, beside `--proc /proc` and `--new-session`. The `sandbox::tests` that pin the argv change with it.
- Check what reads pids from inside a run. The record's `pid` is the host pid Hydra spawned, `procinfo::holders` runs on the host, and the gate script runs inside. Anything inside that compares its own pid with a host pid would break under a new pid namespace. Name what you checked in the Outcome.
- `SECURITY.md`'s `setsid` bullet and `IMPLEMENTATION.md`'s Linux paragraph: on Linux the pid namespace ends what a run leaves behind when bwrap's command exits, and build 372's group kill reaches bwrap itself.
- The builder cannot run Docker from inside a run. The session measures the change in the container after applying it: a stand-in `claude` that leaves a `setsid` loop behind, then a check from the host that the loop is gone after `run_end`. Leave a line in the Outcome for that measurement.

## Checks

`cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --release --target x86_64-unknown-linux-gnu`, and `cargo nextest run --release --workspace --no-fail-fast`.

## Outcome

`compose_bwrap_for_launch` now emits `--unshare-pid` before `--proc /proc` and keeps
`--new-session`. The pid namespace ends the command's remaining processes when the command
exits, including descendants that called `setsid`. Build 372's group kill reaches bwrap itself.
`SECURITY.md` and `IMPLEMENTATION.md` describe this Linux path and the macOS group limit.

The existing argv contract test now requires the added flag across every policy and provider,
with and without a parent git directory. It failed before the code change. All 15 sandbox
unit tests passed after the change. The fixture needed no change.

All four required checks passed: `cargo fmt --all --check`,
`cargo clippy --release --workspace --all-targets -- -D warnings`,
`cargo check --release --target x86_64-unknown-linux-gnu` and
`cargo nextest run --release --workspace --no-fail-fast`. The workspace suite ran 1541 tests,
all passed, with 4 skipped, in 67.444 seconds. The workspace release build and doc-test command
also passed. The doc-test command found no doc tests. The corpus failure mentioned by the
self-edit skill did not occur.

Pid review: `pty::spawn` returns the host child pid from `child.process_id`, and `watch` records
that pid and reads its start time on the host. `procinfo::holders` scans the host `/proc` and
compares its own pid only with entries from that same namespace. `src/policy/gate.py` has no
pid lookup or pid comparison. `inside_seatbelt` reads `/proc/self/ns/mnt` and mountinfo and
compares the mount namespace with `launcher.mnt`. The pid namespace does not change that
delegation witness. No inside-run comparison with a host pid was found in these paths.

Measured by the session on 2026-10-28 in Docker, bubblewrap 0.8.0. A stand-in `claude` left a
`setsid` loop that wrote a heartbeat file every 0.2 s. The loop ran as pid 3 in the run's own
pid namespace. After `run_end` no such process existed on the host and the heartbeat had stopped.
`run_end.error` carried no group note, because the group was already empty.

Assumed and false: the container of build 370 was not enough. With `--unshare-pid`, bwrap
exited with "Can't mount proc on /newroot/proc: Operation not permitted" and the run derived
`crashed`. Docker masks paths under `/proc` with tmpfs mounts, and the kernel refuses a fresh
`/proc` in a new pid namespace while those masks stand. `--unshare-pid` without `--proc` and
`--proc` without `--unshare-pid` both worked. The container needs
`--security-opt systempaths=unconfined` as well, and `docs/install.md` says so. A Linux host
without those masks does not need it.
