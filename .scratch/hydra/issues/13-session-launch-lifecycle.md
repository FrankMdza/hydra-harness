# 13 - Session launch and pane lifecycle

Type: grilling
Status: resolved
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

## Answer

**Hydra owns the pty, every run file is self-describing, and nothing cleans up without being told
to.** The launch path was already mostly settled by tickets 02, 01, 11 and 18. What this ticket
adds is the wreckage handling, plus one collision nobody had noticed between D3 and ticket 01.

### The collision, and the split it forces

D3 says no custom TUI in v1 and the vendor CLIs keep their own. Ticket 01 says drive ACP, because
print mode leaves only five fields common to all four providers. Both cannot hold in one process.
ACP makes the CLI a server and the client draws the interface, so a CLI in ACP mode has no TUI to
keep, and `stream-json` print mode is the same story.

So interactive and recorded are two launch modes, not one. An interactive run lets the vendor own
the terminal and records thin. A delegated or scripted run goes through ACP or print mode and
records in full. D4's correction already named this axis, the recorded path against the interactive
path, so the concept is not new, only its consequence.

The consequence is that Ouroboros learns from run records, and thin records teach it nothing.

### Harvest closes the gap

Every provider already writes its own transcript. `claude` keeps append-only `.jsonl` under
`~/.claude/projects/<slug>/`, and it writes them live: the file for a running session grows during
the session, confirmed 2026-09-02 against this repo's own session. `agy` keeps
`conversation_summaries.db` and a `brain` directory under `~/.gemini/antigravity-cli/`.
`cursor-agent` keeps per-chat directories under `~/.cursor/chats`.

When an interactive run exits, Hydra converts the vendor's transcript into the run record. That
keeps the run record as the single contract D5 requires, and an interactive session is recorded
late rather than not at all.

**Harvest at exit is the contract. Live tailing is a per-provider optimisation.** Only `claude` is
confirmed to write an append-only log during the run, so only `claude` interactive runs can stream.
A contract three providers cannot meet is not a contract. The spec states the streaming answer per
provider rather than promising one it breaks three times in four.

**Harvest is idempotent and any process can do it.** The run envelope names the provider and its
session id, so harvest is a pure function of two files on disk and does not care who runs it or
when. `hydra status` finds a dead run with an unharvested body and completes it. This is what
closes the scenario at the bottom of this ticket, and it keeps D10 true down to the recorder itself.

### Liveness is derived, never announced

The pid and the process start time go in the run file's opening envelope, which Hydra writes at
launch anyway. A run is alive when a process matching both still exists. The start time is not
optional, because macOS recycles pids and a bare pid can point at something unrelated.

No sidecar, no lockfile, no second source of truth to contradict D10. And the rule that falls out
of it: **nothing appends to a run file except the process that owns it.** Death is a conclusion a
reader draws, never an event a bystander writes. Harvest does not break this, because Hydra owns
the run file throughout and the child never touches it.

### Session identity

Hydra mints the run id, ticket 09 already fixed it as UUIDv7, and records the provider's own session
id beside it as a field. `agy` gives a `conversation_id`, `claude` has its own, and `hydra resume
<run-id>` reads that field and hands the provider its own resume flag. Hydra never reconstructs
conversation state itself.

A resumed run gets a fresh run id linked to the old one, so one file per run survives intact. Where
a provider has no resume concept, resume fails loudly instead of starting a fresh conversation
dressed up as a continuation.

### Watching, capping, reaping

**Observation is read-only.** Children run detached and unwatched per ticket 11, and D5 forbids a
second channel, so `hydra logs <run-id> --follow` tailing the run file is the only view. No attach,
no input, no answering a permission prompt from outside. Ticket 11 made permissions policy-driven
and ticket 18 found `agy` cannot be asked anything mid-run, so this is the only coherent shape
rather than a compromise.

**The concurrency cap refuses.** It returns a typed error naming what is in flight and its run ids.
Queueing needs a scheduler and a live process to drain it, which D10 rules out. Blocking parks the
orchestrator, and ticket 11 says delegation exists to save its context. Refusing lets the caller
decide, and an agent can act on the error instead of hanging.

**Nothing is cleaned up automatically.** `hydra prune` and `hydra reap` are commands you type.
Prune moves run files rather than deleting them, unless a flag says otherwise, because D12 keeps
lossless export as a standing constraint and anything deleted is training data destroyed. Reap
never removes a worktree with uncommitted changes: it commits them to the child's own branch first.
Ouroboros cannot call either at either rung.

This is ticket 05's lesson turned on our own tooling. Hermes told its model that archiving was
recoverable while the tool underneath called `shutil.rmtree`, so a guarantee written in a prompt is
not a guarantee.

### The scenario

You close the Ghostty window on a session with two children mid-flight. Hydra dies. The children
are detached, so they keep running and keep writing their own run files. The interactive session's
run file holds an envelope with a pid nothing matches any more, and the vendor transcript sits on
disk unharvested.

The next `hydra status` reconciles all three. It derives the parent as dead, harvests its body from
the vendor transcript, and reports the two children as alive by matching pid and start time. If
they later die badly, their worktrees show up as abandoned and wait for a `hydra reap` you type.
Nothing was lost and nothing needed to be running.


## Amended by ticket 10, 2026-09-02

**`agy` must be launched with `--add-dir <worktree>`.** It has a workspace concept detached from the
process working directory, so a directory it is not told about is not its workspace. Ticket 10
proved it on the skill root: a probe placed in the launch directory was invisible twice, as a real
directory and as a symlink, and appeared as soon as the directory arrived as `--add-dir`, from an
unrelated cwd too. This is consistent with ticket 18 finding that
`--dangerously-skip-permissions` resolved a relative path into `~/.gemini/antigravity-cli/scratch/`
rather than the launch cwd.

For this ticket that means `cd` into the worktree is not the launch contract for Google, the flag
is. Anything else Hydra expects `agy` to read out of the run's directory needs the same treatment,
and ticket 23 checks whether MCP config is one of them.

## Amended by ticket 29, 2026-09-03

**The two launch modes get different MCP on `claude`.** A recorded run is launched with
`--mcp-config` plus `--strict-mcp-config`, so its server set is exactly what Hydra passed. An
interactive run gets `--mcp-config` without the strict flag, so the human's own servers survive and
Hydra's are added. This is D3's split showing up in one more place: the exact set is worth having
where the record depends on it, and stripping a human's MCP servers out of a session they are sitting
in front of is the harness overriding them in the mode D3 hands to the vendor.

**Harvest is unaffected by the Cursor redirect.** `CURSOR_DATA_DIR` moves `projects/<slug>/` and
leaves `chats/` under `CURSOR_CONFIG_DIR`, which Hydra does not set, so the transcript this ticket
harvests stays where it is.

**A launch can now be refused before any process starts.** Ticket 29's launch check compares the
servers the run's policy admits against the servers the launch can deliver. On Cursor a free
`mcp list` runs after the approvals and before the spawn, in both launch modes, and an injected
server that is not ready fails the run. Interactively it fails before the TUI opens.
