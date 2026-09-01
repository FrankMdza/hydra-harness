# 13 - Session launch and pane lifecycle

Type: grilling
Status: open
Blocked by: 02, 11

## Question

What happens between typing `hydra` and having a working agent in front of you, and what happens to that session afterwards.

Settle:

- The launch path, given whatever ticket 02 found about driving panes
- Where the tee sits, so that the pane stays interactive while every event also reaches the run file. This is the mechanical heart of the harness and it is easy to get wrong
- Session identity: who mints the id, how it survives a resume, and how it maps to each provider's own session concept
- Detach and reattach, if that is even possible per provider
- What happens on crash: the provider CLI dies, Hydra dies, the pane closes, the machine sleeps
- Whether a delegated child gets a visible pane or runs unseen, and how you watch one that is unseen
- Cleanup: when a run file closes, and what prunes `~/hydra/runs/`

Scenario: you close the Ghostty window on a session that has two children mid-flight.

## What the research changed

**Ticket 02 split this ticket in two.** Ghostty opens a window, tab or split and hands it a command through AppleScript, but exposes no PID and no exit status, and its maintainer has ruled out a unified IPC. So Hydra cannot supervise a process through Ghostty.

The recommendation it landed on: Hydra owns the pty per provider directly, with `portable-pty` in the Rust core. It already has to be the parent process to tee output into the run record, so routing through Ghostty, tmux or zellij purely to recover an exit code adds a layer for nothing.

That makes process supervision a settled mechanism and live rendering a separate question that stays deferred with the TUI. Decide here whether you accept that split, and what the loss of a live multi-pane view costs you until a TUI exists.

## What ticket 01 added

**Process exit is the authoritative end of a run.** Claude always writes exactly one `result` frame, Gemini usually does, Cursor only on success, Hermes never. So the lifecycle hangs off `wait()`, which is consistent with ticket 02's finding that Hydra owns the pty and is therefore the parent.

Exit codes are useless as a common signal. Gemini has nine meaningful ones, Claude two, Cursor three, Hermes two, and Hermes returns 0 even when the agent turn failed. A shared status has to be derived per provider, and for Hermes it can only ever say "completed" or "crashed", never "the agent failed".
