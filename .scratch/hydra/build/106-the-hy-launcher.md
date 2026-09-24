# 106 - `hy`: a gated interactive session in a Herdr pane, in one word

Type: build
Status: claimed
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
