# 136 - Does a run's process group die with the run?

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-27, by the session, from the review of `3f5e6a8..95e01a9` (`research/panel-review-435f156.md`). Grok `01a17eb4-20d0` and Flash `01a17eb4-c167` both named a race: `sweep_private` and the host git after it take no lock, and a descendant the run left behind can write the private git directory, or the worktree, after `run_end` and before or during reap. `IMPLEMENTATION.md` line 42 already says "whether Hydra wants orphans to outlive a run on Linux is undecided". The human, leaving on 2026-10-27, said the panel decides.

## What happens today

`src/watch.rs` around line 300: when the provider exits, the watcher writes `run_end` and notes `PTY_HELD_NOTE` if a descendant still holds the pty. It signals the child's process group only when `hydra stop` asked it to, through `stop::GRACE`: SIGTERM, then SIGKILL after 10 s, with the pid checked against its start time. A descendant that ignores SIGHUP, or that left the group with `setsid`, keeps running with the run's sandbox: the seatbelt profile on macOS, the bwrap namespace on Linux. It can keep writing everything the run could write, including the index and the worktree, while reap commits and while `hydra status` reads.

## Question

At the end of a recorded, non-interactive run:

- **A.** Kill the group the verified child led, as `hydra stop` does: SIGTERM, wait `stop::GRACE` on the group, SIGKILL. Record what was killed.
- **B.** Leave it, as today, and record it. Reap refuses while anything still holds the pty or the worktree, and names it.
- **C.** A, and on Linux run the provider with `--die-with-parent` semantics extended to the whole namespace, so a `setsid` escapee dies too. bwrap's `--die-with-parent` already ends the sandbox's init when the launcher dies; say whether that covers it.
- **D.** Another answer.

Say what each breaks: a provider that leaves a legitimate server running across turns, MCP servers a provider spawns, a run that starts `cargo build` in the background and exits. An interactive session shares the human's job and is out of scope; say if that is wrong.

## Answer

Resolved 2026-10-27 by a four-seat panel, which the human delegated the decision to. Reasoning, votes and measurements are in `research/136-synthesis.md`.

**A, on which all four seats agree.** At the end of a recorded, non-interactive run, the group the verified child led gets SIGTERM, `stop::GRACE`, then SIGKILL. The process group id is saved while the leader lives, the kill happens before `run_end`, and a note on `run_end.error` names what died without changing the outcome. Hydra does not want orphans to outlive a run. A `setsid` escapee survives on macOS, so reap refuses while any process holds the worktree or the private git directory. On Linux, `--unshare-pid` and the provider's own session under `--new-session` wait for build 370's measurement. Build tickets 372 and 373.
