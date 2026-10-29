# 376 - A bubblewrap launch unshares the pid namespace

Type: build
Status: open
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

## Outcome
