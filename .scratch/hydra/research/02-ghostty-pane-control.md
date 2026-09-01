# 02 - Ghostty pane control from an external process

Researched 2026-09-01 against the versions on this machine: Ghostty 1.3.1 (stable channel), tmux 3.6a, zellij 0.44.3, macOS 15 (Darwin 25.2.0). Ghostty ships fast and its scripting surface is under a year old, so a claim here that isn't pinned to a version number should be treated as already stale.

## The short answer

Ghostty has no way today to tell an external process when a command it launched has exited. It can open a window, tab or split and hand it a command through AppleScript, but the scripting dictionary stops at creation. There is no exit code, no PID (until an unreleased 1.4.0), no hook, no callback. tmux and zellij both solve the full job natively and have for months. Hydra should not route process supervision through Ghostty, tmux, or zellij at all: it should own the pty for each provider directly, and treat "what shows this pane on screen" as a separate, swappable decision.

## What the `ghostty` binary itself offers

`ghostty --help` on this machine confirms the CLI is action-only on macOS:

```
On macOS, launching the terminal emulator from the CLI is not
supported and only actions are supported. Use `open -na Ghostty.app`
instead, or `open -na ghostty.app --args --foo=bar --baz=quz` to pass
arguments.
```

`ghostty +new-window --help` returns `+new-window is not supported on this platform.` (exit code 1, tested directly). That action is real, but it's the Linux path: Ghostty's own maintainer describes it as backed by D-Bus/GTK action activation, which macOS has no equivalent for ([discussion #2353](https://github.com/ghostty-org/ghostty/discussions/2353)).

`open -na Ghostty.app --args ...` does something different from what the ticket is asking: it starts a brand new Ghostty process every time, not a pane inside the one already running. Fine for "launch a fresh terminal," useless for "hand a pane to the instance the user is already looking at."

No custom URL scheme exists (`CFBundleURLTypes` is absent from `Info.plist`), and the two Finder Services Ghostty registers (`openTab`, `openWindow`) only accept a folder path, not a command, and give no completion signal.

## AppleScript: the real mechanism, and where it stops

Ghostty 1.3.0, released March 9, 2026, added a built-in AppleScript dictionary, on by default (`macos-applescript = true`; the user's config at `~/Flava/ghostty/config` doesn't touch this key, so the default holds). The dictionary lives in the app bundle at `Contents/Resources/Ghostty.sdef` and is worth reading directly rather than trusting a summary, since it's the actual contract.

The object graph is `application → window → tab → terminal`. A `surface configuration` record can be attached to `new window`, `new tab`, or `split`, and carries exactly the fields Hydra would want to hand off a provider run:

- `command` (what to execute instead of the default shell)
- `initial working directory`
- `environment variables`
- `initial input`
- `wait after command` (boolean: keep the pane open once the command exits, or let it close)

So the create-a-pane-and-run-a-command half of the job works, confirmed live: `osascript -e 'tell application "Ghostty" to get name of every window'` returned the actual title of a running window on this machine. `input text` and `send key` also let a controller keep talking to a pane after launch (paste-style text, or individual keystrokes with modifiers), which covers interactive follow-up turns, not just a one-shot command.

The learn-when-it-exits half doesn't exist. The `terminal` class exposes only `id`, `name` (title) and `working directory`. No PID, no exit status, no "on close" handler a script can register for. Someone asked for exactly this gap to be closed: [issue #11592](https://github.com/ghostty-org/ghostty/issues/11592) requested `pid` and `tty` properties on the terminal class "to reliably map processes to specific terminals." It's closed, implemented by [PR #11922](https://github.com/ghostty-org/ghostty/pull/11922) (merged April 20, 2026), targeted at the 1.4.0 milestone, which had not shipped as of this research (the machine's stable install is still 1.3.1). Even once it lands, the PR only adds a queryable property at read time, "not monitoring lifecycle events" per its own summary. It gets you a PID to poll with `kill -0`, not a push notification, and it isn't in the version installed here yet regardless.

Two more things worth ruling out explicitly:

- `notify-on-command-finish` (added 1.3.0, still `never` in this config) fires a bell or a macOS notification when a shell using OSC 133 markers finishes a command. It proves Ghostty can already parse command boundaries internally, but the signal is user-facing (Notification Center, audible bell), not exposed to another process, and it depends on shell integration being active in that pane's shell in the first place.
- No Unix domain socket exists for the running instance (checked with `lsof -c ghostty`, nothing). A fork, [ghostty-automator](https://github.com/hyperb1iss/ghostty-automator), bolts on a length-prefixed JSON socket protocol with a dozen actions, and a separate project called cmux builds its own agent-control socket on top of Ghostty's rendering engine. Both are real but neither is upstream Ghostty; building on either means tracking someone else's fork of a fast-moving project, which is the kind of dependency the map already steers away from for Hermes (D13) for the same underlying reason.

Ghostty's own maintainer has been explicit about the roadmap shape in [discussion #2353](https://github.com/ghostty-org/ghostty/discussions/2353): no unified cross-platform scripting protocol, deliberately. macOS gets AppleScript and App Intents, Linux gets D-Bus over the existing GTK action API, Windows has nothing yet. AppleScript itself is still labeled a preview as of 1.3.0, with maintainers stating outright that they expect breaking changes before 1.4. That's not a surface to build a harness's process supervision on, independent of the missing exit signal.

**Verdict for Ghostty specifically: dead end for the job as asked.** It can create a pane and start a command. It cannot tell you when that command is done. The only way to close that gap today is for Hydra to build its own out-of-band signal (wrap the command so it drops an exit code into a file or FIFO Hydra is already watching), at which point Ghostty has contributed nothing to the supervision problem, only the window.

## tmux, if Hydra drove it instead

tmux (3.6a here) solves the whole job natively, and has for a long time. `new-window`, `split-window`, and `send-keys` create panes and inject commands from any external process against a session on its default or a named socket. Exit is a first-class concept, confirmed directly from `man tmux` on this machine:

- `pane_dead_status` : "Exit status of process in dead pane" (queryable via `list-panes -F` or `display-message -p`)
- `remain-on-exit [on | off | failed]` : keeps a pane alive after its command exits instead of closing it, so there's something to query
- `pane-exited` hook: "Run when the program running in a pane exits." (`set-hook -g pane-exited 'run-shell "..."'`, arbitrary shell command, event-driven, no polling required)
- Control mode (`tmux -C`): a textual protocol on stdin/stdout with defined notifications including `%exit`, used by things like iTerm2's tmux integration

This is the most complete and most battle-tested of the three options against the literal job description. The cost is an extra hop (Hydra → tmux → whatever terminal displays it), tmux's own session/socket lifecycle to manage, and prefix-key UX friction if a human ever reaches into a pane by hand.

## zellij, in the same role

zellij (0.44.3 here) is younger at this specific job but a closer match to how Hydra would actually want to call it. `zellij run <command>` "Returns: Created pane ID (format: `terminal_<id>`)" per its own `--help`, and ships blocking flags built for exactly this pattern: `--blocking`, `--block-until-exit`, `--block-until-exit-success`, `--block-until-exit-failure`. Per the [official docs](https://zellij.dev/documentation/programmatic-control.html), a command launched with `new-pane -- <command>` has its exit status "reflected in the pane metadata," queryable with `zellij action list-panes --json`, and the blocking flags propagate that exit status through the calling process's own exit code, explicitly designed to support shell chaining like `zellij run --block-until-exit-success -- cargo test && zellij run --blocking -- cargo build --release`. `zellij subscribe` adds push-based pane update monitoring on top. The docs are explicit that none of this needs a socket, HTTP API, or library binding: it's all subprocess invocation and JSON on stdout.

This automation surface shipped in 0.44.0 on March 23, 2026 ([release announcement](https://zellij.dev/news/remote-sessions-windows-cli/)), about five months before this research, and is live in the 0.44.3 build installed here today, not aspirational. Headless operation matches tmux: `zellij attach --create-background <name>` starts a session with no attached client.

## Hydra owns the pty per provider, renders nothing

This sidesteps the entire question instead of answering it, which is the point. Hydra spawns each provider CLI as its own direct child process, with its own pty if the provider needs real tty semantics (most coding-agent CLIs check `isatty` and change behavior without one). The exit code comes from the OS to Hydra directly, the same `wait()` call any process supervisor uses, no multiplexer, no AppleScript dictionary, no version pin on Ghostty, tmux, or zellij to track.

This also happens to match what's already settled: the map's own domain description has Hydra teeing each provider's structured output to a trajectory log, which means Hydra is already the thing reading that process's stdout stream, whether or not any terminal multiplexer is in the loop. D3 leaves the vendor CLIs to keep their own TUI, meaning Hydra's job was never to render, only to supervise. D10 makes the JSONL run record the source of truth, independent of whether anything is currently running or displaying it.

The cost is real: nothing shows up live in a pane by default. A session invoked in the foreground shows its output the ordinary way, because that's what a child process attached to a controlling terminal does. Watching a session that isn't your foreground one, or watching several at once, has no answer in this shape until issue 07's TUI lands, or until someone manually opens a pane and tails the log themselves.

## Recommendation

Hydra should own the pty per provider for process supervision, not delegate that role to Ghostty, tmux, or zellij. The reasoning holds regardless of which multiplexer looks most capable, because Hydra already has to be the direct parent of the provider process to tee its output into the trajectory log. Routing through a terminal multiplexer just to get an exit code back through a side channel adds a translation layer for something Hydra gets for free by being the parent in the first place.

Keep "what renders it live" as a separate decision, downstream of the harness spec, not a blocker to it:

- Ghostty is fine as inert window chrome around whatever shell is attached to it. It should never be the thing Hydra scripts to start and supervise a provider run, because as of 1.3.1 it has no way to report back.
- If simultaneous live viewing of multiple sessions matters before the TUI (issue 07) ships, zellij's `--block-until-exit*` flags and `list-panes --json` are the closer fit to a call-and-wait harness than tmux's older hook-and-poll model, and both are viable if that need turns out to be real. Neither is required for the harness to function, per D10.
- The cost of this path is one new dependency in the Rust core: a pty-owning subprocess layer (the `portable-pty` crate is the standard choice here, and notably what WezTerm itself is built on, the terminal this config replaced).

This is the dead-end finding the ticket asked me to flag plainly if it turned out to be true: nothing drives Ghostty panes cleanly enough to supervise a process through it today. The harness spec should treat pane display and process supervision as two different concerns and stop looking for one tool that does both.
