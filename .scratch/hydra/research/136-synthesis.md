# 136 synthesis: a run's process group dies with the run

Panel of 2026-10-27, decided by the panel because the human said so before leaving. Seats, all under `standard`, read only:

- Opus, `claude-opus-5-5`, run `01a17ecb-475b`: D, which is A plus reap-side guards.
- GPT, codex `gpt-6-luna`, run `01a17ecb-e68d`: A.
- Grok, cursor `grok-4.7-high`, run `01a17ecc-8859`: A.
- Flash, agy `gemini-3.8-flash-high`, run `01a17ecd-34e5`: C, which is A plus `--unshare-pid` on Linux.

## What all four agree on: A

At the end of a recorded, non-interactive run, the group the verified child led gets SIGTERM, a wait of `stop::GRACE`, then SIGKILL. Opus, Grok and Flash each measured it on this Mac with a pty-shaped script under `/tmp`. A descendant that stays in the group and ignores SIGTERM and SIGHUP dies at SIGKILL. A descendant that called `setsid` survives both.

What dies, by every seat's list: MCP stdio servers a provider started, background shells such as `cargo build &`, helpers left in the group. No result depends on them, because the stream and the worktree are fixed when the provider exits. Delegated child runs survive, because `hydra run` detaches its watcher with `setsid`. Interactive sessions stay out of scope, because they share the human's job.

The mechanics, from Grok and Opus. `stop::terminate` returns without signalling once the leader is dead, and here the leader has already been reaped. So save the process group id while the leader lives, and signal the group after `wait` and before `run_end`. Killing after `run_end` would leave reap a 10 s window. The record carries a note on `run_end.error` naming what was killed and whether SIGKILL was needed. The note must not start like `STOP_NOTE` or `SIGNAL_NOTE`, and `derive` must keep the provider's outcome. Grok checked `src/outcome.rs`: with no terminator any error selects `failed`, so the note needs the same treatment `PTY_HELD_NOTE` gets.

## Where they differ: Linux

Flash would add `--unshare-pid` to `compose_bwrap` now, so the kernel kills every member of the namespace, `setsid` escapees included. Opus and Grok would hold it. It has never run, and Opus points out that it would kill delegated child runs, which are descendants inside the namespace. GPT said nothing about it. The session holds it for build 370, which measures bwrap on a real Linux.

Grok found what makes A weaker on Linux than on the Mac. `compose_bwrap` passes `--new-session`, so the provider calls `setsid` before it forks, and the recorded pid is bwrap's. Killing bwrap's group misses the provider's children. On Linux, A needs either the session's members or the namespace. Build 370 measures that before anything is written.

## The `setsid` escapee on macOS

A does not reach it, and it keeps the run's seatbelt with write access to the worktree and the index. Opus's D adds two guards in reap. First, refuse and name any process that still holds a cwd or an open file under the worktree or the private git directory. Second, run host git against a copy of the private directory. No seat argued against either. Build 371, now running, rebuilds the index from `HEAD` and the worktree, which already takes the index out of the run's hands. The session takes the first guard as a build ticket and leaves the copy for after 371 lands.

## Build tickets

1. 372: close-time group kill in `src/watch.rs` with the saved pgid, the note on `run_end.error`, outcome unchanged, and a stub pty test with an ignoring grandchild, plus a `setsid` one that survives. Amend `IMPLEMENTATION.md` line 42: Hydra does not want orphans to outlive a run.
2. 373: reap refuses, naming the process, while anything holds a cwd or open file under the worktree or the private git directory.
3. Linux, `--unshare-pid` and the `--new-session` group: a measurement added to build 370.
