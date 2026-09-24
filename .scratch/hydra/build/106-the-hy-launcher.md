# 106 - `hy`: a gated interactive session in a Herdr pane, in one word

Type: build
Status: resolved
Blocked by:

## What

The first build item of map ticket 75's panel. `hydra run --interactive --provider claude
--policy standard` exists, and nothing puts it in a Herdr pane with the run id on the pane, because
`bin/hydra-in-pane` sends stdout to `/dev/null`, waits for the command to return and then tails the
log, which is right for a detached recorded run and wrong for a vendor TUI. Add a sibling,
`bin/hydra-tui-in-pane`, that runs the interactive launch with the terminal inherited, tees stderr
to catch the `hydra: run <id>` line, and makes the same two `herdr pane report-*` calls, so the
bridge labels the pane as it does any run. Add it as an action in the manifest, and add `hy.zsh`, a
shell function the human sources, that splits a pane beside the current one and runs it.

Nothing under `src/` changes. The interactive run's record stays thin until the harvest at exit,
so the bridge shows the pane alive and working with no tool names until then; that is map ticket
76's decision G, not this ticket.

## Done when

Sourcing `hy.zsh` and typing `hy` inside a Herdr pane opens a new pane running a gated `claude`
session, the pane carries `run_id`, `provider`, `policy` and `launch_mode=interactive` tokens
within a few seconds of launch, and the record is harvested when the session exits. The script's
mechanics are checked with a stub binary; the one thing this ticket cannot check without a human at
the keyboard is the vendor's TUI under the seatbelt, which `IMPLEMENTATION.md` has asked a human to
confirm since build ticket 73.

## Outcome

Built 2026-09-24 by the session, not on an agent run: three files under `integrations/herdr/`
and one manifest entry, no Rust. `bin/hydra-tui-in-pane` runs `hydra run --interactive "$@"`
with stdin and stdout on the pane's pty and stderr through `tee`, and a background subshell reads
the `hydra: run <id>` line, calls `hydra status <id>` for provider and policy, and puts `run_id`,
`provider`, `policy` and `launch_mode=interactive` on the pane through `herdr pane
report-metadata`, then `report-agent working`. `hy.zsh` defines `hy [model] [policy]`, which
splits a pane to the right with `HYDRA_BIN` in its environment and runs the script there. The
manifest gains an `interactive` action.

Measured with a stub binary in a real Herdr pane (w3:pB, w3:pC): the tokens landed within four
seconds, the stub's stdout reached the pane, `EXIT=0` passed through, and `hy` from inside a pane
split a second pane that carried the same tokens. Not measured, and the reason the ticket stays
short of its own "done when": a real `claude` session under the seatbelt with a human at the
keyboard, which `IMPLEMENTATION.md` has asked for since build ticket 73. The panel's judge added
one thing to look at in that session: `tee` puts a pipe on the provider's fd 2, so stderr is not
the terminal. If `claude` misbehaves on a non-tty stderr, the fix is `script`-style pty
passthrough or a launch receipt on a separate descriptor, which codex's proposal preferred.

Also in this change, as a prototype from map ticket 75 and not part of this ticket's "done when":
`bin/hydra-mascot`, a stdlib-only renderer that crops and shrinks a reference image with macOS
`sips`, prints it as half-block truecolor cells with dark pixels transparent, and under `--demo`
cycles idle, working, refused and fire. The three-headed reference reads as a dragon at 96
columns and not at 70. It reads no record yet; wiring it to the pane tokens is the panel's
build item 3.
