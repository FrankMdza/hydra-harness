# 13 - Session launch and pane lifecycle

Type: grilling
Status: claimed
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

## What ticket 11 settled, and what it leaves here

Two of this ticket's questions are now answered elsewhere. Children are spawned **detached** and run **unwatched**, with permissions answered from a policy rather than by a human, so "does a delegated child get a visible pane" is settled as no. Nothing escalates to a person by default.

What that hands back to this ticket is the reaping problem, which is now the bigger half of it:

- `hydra status` reconciling live pids against open run files, and what it does with a run file whose process is gone
- Cleaning up worktrees left behind by children that crashed, and who decides a branch is abandoned
- What the concurrency cap does when it is hit: queue, refuse, or block
- How you observe an unwatched child while it runs, given the run file is the only channel and D5 forbids inventing a second one

The launch path itself is simpler than it was. Ticket 02 put Hydra on the pty as the parent, ticket 11 made every child detached, and ticket 01 made process exit the authoritative end of a run. So launch and teardown are ordinary process work, and the interesting questions left are all about what happens to the wreckage.

## What ticket 18 found

**Launch flags that are not optional.** `gemini` refuses to run headless in an untrusted directory
and needs `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`. `agy` takes its prompt attached to
the flag, `-p=<prompt>`, because a bare `-p` swallows the next argument as the prompt and silently
drops the real one.

**Session identity is solved for Google.** `agy` mints a `conversation_id` in its `init` event and
`--conversation <id>` resumes it, so the provider's own session concept is stable and addressable.

**Relative paths do not land in the launch cwd.** With `--dangerously-skip-permissions`, a probe
that wrote `probe.txt` put it in `~/.gemini/antigravity-cli/scratch/`, not the directory Hydra
launched from. Any assumption that the child writes where the parent stands is wrong for `agy`.

**Permission mode is a launch decision, not a runtime one.** `agy` print mode reports
`permission_mode: request-review` and auto-denies acting tools mid-run, reporting the refusal as a
tool `ERROR`. Reading tools still work. There is no way to answer a prompt once the process is
running, so the mode has to be chosen at spawn.
