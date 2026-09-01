# 02 - Ghostty pane control from an external process

Type: research
Status: resolved

## Question

How does a process that is not Ghostty open a pane or window in the running Ghostty instance, give it a command, and learn when it exits?

Ghostty has no scripting interface in the shape tmux offers. Establish what actually exists today: the `ghostty` binary's own flags, any IPC or socket, macOS-specific paths, and what the config at `~/Flava/ghostty/config` already sets up.

Then judge the alternatives against the same job:

- tmux running inside Ghostty, with Hydra driving tmux instead of Ghostty
- zellij in the same role
- Hydra managing its own pty per provider and rendering nothing, so panes stop being its problem

The answer should say which one Hydra targets and what that costs. A dead end is a valid finding: if nothing drives Ghostty panes cleanly, say so and the harness spec changes shape.

## Answer

Ghostty cannot supervise a process, and that is unlikely to change soon.

Ghostty 1.3.1 does open a window, tab or split and hand it a command, through the AppleScript dictionary added in 1.3.0 (March 2026), using a `surface configuration` record carrying command, cwd, env and initial input. But the `terminal` class exposes no PID and no exit status. Issue #11592 asked for `pid` and `tty`; PR #11922 merged in April 2026 but targets an unreleased 1.4.0, and even then it is a property you poll, not an event you receive. There is no socket, no URL scheme and no macOS equivalent of D-Bus, and the maintainer has said in discussion #2353 that there will be no unified IPC, only platform-native paths. AppleScript itself is still marked preview.

tmux 3.6a and zellij 0.44.3 both solve the whole job natively. tmux through `pane_dead_status`, `pane-exited` hooks and control mode; zellij through `run --block-until-exit*` and `list-panes --json`.

The recommendation is neither. Hydra should own the pty per provider directly, because it already has to be the parent process to tee output into the run record. Routing through a terminal multiplexer purely to recover an exit code adds a layer it does not need. That splits one question into two: process supervision, decided here, and live rendering, which becomes the TUI's problem and stays deferred.

Cost: one dependency in the Rust core (`portable-pty`), and no live multi-pane view until a TUI exists.

Detail: [research/02-ghostty-pane-control.md](../research/02-ghostty-pane-control.md)
