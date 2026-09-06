# 45 - Install Herdr and Hunk and probe what ticket 44 assumed

Type: task
Status: resolved

## Question

Ticket 44 decided Herdr is the interactive layer and Hunk the diff viewer, with a bridge on their
side reading `~/hydra/runs/`. Every claim about their APIs was read off their sites on 2026-09-06,
and neither binary is on this machine. Install both, drive them, and write down what holds. Nothing
is decided here; a claim that fails goes back to ticket 44 as a note.

## Probe

1. A Herdr plugin or pane command runs
   `hydra run --provider claude --policy read-only --model haiku --task "say hi"`. Check the
   record: `run_start.pid` is Hydra's child, the stream is in the record, and the run derives an
   outcome. Hydra must still own the pty.
2. Herdr's socket accepts a state label from outside the pane, through `herdr pane
   report-metadata` or the newline-delimited JSON API. Which fields, and what the pane shows when
   `HERDR_AGENT` is unset.
3. What Herdr puts in the pane process's environment and what `TERM` it sets. `src/env.rs`
   allowlists six names, so anything Herdr relies on beyond those is stripped before the provider
   starts.
4. On one run, Herdr's screen-scraped `blocked` against the record's `gated`. Same run, both
   readings.
5. Hunk on branch `hydra/<run-id>` against the record's `diff_stat.base_tree`. Then Hunk's agent
   annotation input: what shape it takes, and whether a record's `message` events can be turned
   into it without a Hydra change.
6. Versions and dates for all of it, in this ticket's `## Answer`, because both tools will move.

## Where the evidence is

Map ticket 44's `## Answer`. The map's `## Not yet specified` entry on a mini agent as a peripheral
controller, which already assumed a Herdr plugin tailing the record. `src/env.rs` for the allowlist,
`src/watch.rs` for who owns the pty.

## Answer

Both tools exist, both installed, both drove. Four of ticket 44's claims hold, two are wrong, and one
of the wrong ones is a decision rather than a correction. Nothing is decided here.

### Versions and dates

Probed 2026-09-06, 18:00 to 18:16 local (UTC-05). Installed with one `brew install herdr hunk`, both
from `homebrew/core`.

| | version | notes |
|---|---|---|
| herdr | 0.8.2 | socket protocol 20, Apache-2.0, 19,094 installs in 30 days |
| hunk | 0.21.1 | MIT. `brew info` advertised 0.20.1 and the bottle manifest resolved 0.21.1 |
| claude | 2.1.263 | |
| hydra | 0.2.0 | |

Homebrew is the install route. Neither has a crates.io crate; `npm view herdr` is a 0.0.0 placeholder
and `npm view hunk` an unrelated package at 1.0.6.

### 1. A run launched from a Herdr pane

Two runs went through `herdr pane run`, both `--provider claude --policy read-only --model haiku`.
Both came back `rate_limited` at 0 tokens: the five-hour window was already at utilization 1 and
rejected before a request left the machine. So the launch chain is measured end to end and the
completed-run half is not. What held:

- **`run_start.pid` is the provider on the pty.** Not `hydra run`, not the watcher. `watch.rs:168`
  takes it from `pty::spawn`, and `process_start_time` sits beside it. Run 01a07900 recorded 8101.
- **The stream is in the record.** Seven raw frames between line 0 and `run_end`: two gate hook
  frames, `system/init`, the `rate_limit_event`, a synthetic assistant turn, and the trailing
  `\x1b[?25h` with no newline.
- **The outcome is derived.** `rate_limited`, `outcome_source: derived`.
- **Hydra still owns the pty.** `watch.rs` calls `setsid()` and opens the pty inside that new
  session, so the pane's pty is never the provider's.
- `env_allowlist` on both records is exactly `HOME PATH SHELL TERM TMPDIR USER`.

One thing the ticket did not anticipate: **the pane's cwd decides whether the run gets a checkout at
all.** The first run went from the pane's default cwd, `/Users/USER`, which is not a repo, and
recorded `worktree: null` with no branch and no `diff_stat`. A Herdr plugin has to pass `--cwd` on
the split or `cd` first, or it hands Hunk nothing to open.

### 2. State from outside the pane

`herdr pane report-agent` and `herdr pane report-metadata` both exist and both work from a process
outside the pane. Against a pane running nothing at all, one `report-agent --source hydra-bridge
--agent claude --state working` made `pane get` return `agent: claude, agent_status: working` and put
the pane in `agent list`. No process, no detection.

The constraint the map missed: **`--state` takes `idle`, `working`, `blocked`, `unknown` and nothing
else.** `--state gated` is refused with `invalid pane agent state: gated (expected idle, working,
blocked, or unknown)`. Hydra's three words are not states Herdr will hold.

They are display labels, through a second call. `report-metadata --state-label blocked=gated
--state-label idle=completed --state-label "unknown=timed out"` was accepted and `pane get` read the
map back. The same call takes `--title`, `--display-agent`, `--token k=v` (returned as a `tokens`
map, where `run_id` and `policy` both landed) and `--ttl-ms`, so labels expire.

**`HERDR_AGENT` is not a Herdr variable.** It is absent from 0.8.2's pane environment, its CLI and
its API schema. Nothing shows when it is unset because nothing reads it.

The newline-delimited JSON API is real and documented. `herdr api schema` writes 255 KB of JSON
Schema at protocol 20: 150 request methods, 26 subscription events. `pane.report_agent`,
`pane.report_metadata`, `pane.release_agent` and `pane.clear_agent_authority` are methods;
`pane_agent_status_changed` and `pane_output_changed` are events. A bridge can speak the socket
instead of shelling out.

### 3. The pane environment and TERM

Herdr sets seven names and forces one, read out of a live pane with `env`:

```
HERDR_ENV=1  HERDR_PANE_ID  HERDR_TAB_ID  HERDR_WORKSPACE_ID
HERDR_SESSION  HERDR_SOCKET_PATH  HERDR_BIN_PATH
```

**TERM is forced, not inherited.** I started a second server under `TERM=vt100` and its pane came
back `TERM=xterm-256color`. No shell rc on this machine sets TERM, so Herdr set it. In the same test
`HYDRA_PROBE_MARKER` passed straight through, so everything outside those seven names and TERM is
inherited from the server process.

The consequence is the point. `env.rs` passes `HOME USER PATH TERM` to every provider and `SHELL
TMPDIR` to `claude`, plus six proxy names and `NODE_EXTRA_CA_CERTS` when set. All seven `HERDR_*`
names are off the list, confirmed twice on the records' own `env_allowlist`. So `herdr` inside a
Hydra-launched provider has no socket path and no pane id, and would fall back to the default
session's socket, a different server. D6 needs no change.

It also means Herdr's own `claude` integration is inert inside a Hydra run. `herdr integration
status` says it installs as `~/.claude/hooks/herdr-agent-state.sh` and reports state through that
socket, which the allowlist strips. I did not install it: it writes outside this repo, and that is
the human's call.

### 4. Herdr's blocked against the record's gated

**There is nothing to compare, and that is the finding.** Ticket 44 says Herdr's `blocked` is
screen-scraped and the record corrects it. Herdr produces no reading at all for a Hydra run.

Through run 01a07900, polled every five seconds, `agent_status` stayed `unknown` and `agent list`
stayed empty. Then I ruled out the confound: I replayed a real record's frames into a pane with
`hydra logs 01a07218-...`, a run whose stream carries a `permission_denied` frame, and `agent_status`
stayed `unknown` for the whole replay. Herdr's detection reads the pane's bottom buffer for an
agent's interactive UI. A Hydra run puts Hydra's tail in the pane and the provider's UI in another
session's pty, so there is no UI to classify.

The bridge is not a corrector, it is the only source. That sentence in 44 needs rewriting.

### 5. Hunk on the branch, and annotations

Hunk drove clean, inside a Herdr pane, on real Hydra branches. `hunk show
hydra/01a07340-67a3-7c41-8c69-8d1e28bb58a7` opened the reaped run's changeset and `hunk session
review --json` reported three files at 1, 62 and 72 insertions against 21 deletions, matching `git
diff --shortstat` to the line. `hunk diff` inside the unreaped worktree `01a078bc-...` opened that
run's dirty files. All 21 `hydra/*` branches here are openable.

**The record does not carry `diff_stat.base_tree`.** Ticket 44 says it does. None of the 24 records
on this machine holds it. `diff_stat` is a field on the derived outcome (`outcome.rs:413`), computed
in `run.rs:806` from the worktree and the seed tree and printed by `hydra run` on exit.
`run_start.worktree` carries `{path, branch}` and nothing more.

Worse, the tree it names is reachable from nothing. Run 01a07900's base tree is 9dfc3ef6, a real
12-entry tree object that `git fsck` reports as an `unreachable tree`, because Hydra synthesizes it
from the parent's uncommitted work rather than committing it. It is prunable, and one `git gc
--prune` takes the base for every past run with it.

It is recoverable before a reap. `worktree.rs:109` says `~/hydra/spawn/<run-id>/seed-index` is the
index the seed staged into and that `write-tree` against it answers the same sha, and that file is on
disk for all three spawn directories here. A local guard hook stopped me running `write-tree` by
hand, so that one rests on the code and the file rather than my own measurement. Reap removes the
spawn directory, so after a reap the route is gone too.

Hunk does not need it. `hunk show hydra/<run-id>` needs only the branch, which the record does carry,
and for a reaped run it shows the reap commit against its parent, the seed commit. The caveat: reap
does `git add -A` (`reap.rs:215`), so that commit holds the seed's uncommitted work alongside the
run's own edits, and `base_tree` is the only thing separating them. **If the viewer has to separate
them, that is a Hydra change and a decision, not an implementation.**

Annotations work, and the shape is concrete. Two inputs:

- `--agent-context <path>` on `hunk diff` and `hunk show`, a JSON sidecar at launch. I found no
  schema for it, in the help or the bundled skills.
- `hunk session comment apply --stdin` against a live session through Hunk's daemon, documented in
  the skill Hunk ships at `/opt/homebrew/Cellar/hunk/0.21.1/libexec/skills/hunk-review/SKILL.md`.

I drove the second. Built from run 01a07340's record, this landed:

```json
{"comments":[{"filePath":"probe-shell.txt","hunkNumber":1,
  "summary":"...","rationale":"...","author":"hydra-bridge"}]}
```

Hunk answered with `commentId`, `fileId`, `hunkIndex: 0`, `side: "new"`, `line: 1`, and `comment list
--json` read it back with author and rationale intact.

Can a record's messages be turned into that without a Hydra change? Yes, and the bridge has real work
to do. An item needs `filePath`, `summary`, and one of `hunk`, `hunkNumber`, `oldLine` or `newLine`.
A record's tool calls are nested inside `assistant.message.content[]` rather than sitting as
top-level frames, they never carry line numbers, and they carry a file path only when the tool was
Write or Edit. In run 01a07340 every write went through Bash `echo HELLO > probe-shell.txt`, so there
was no `file_path` field and I read the path out of the shell string. `hunkNumber` is the cheapest
target because it needs no line number, but the bridge still has to map a path to a hunk index by
reading the diff. No Hydra change, and no straight field copy either.

### What could not be measured

A completed run, so also a run deriving `gated`. The five-hour window was exhausted before I started
and resets 20:50 local. Both runs cost 0 tokens. The gate's hook frames fired on both, so that path
is exercised; what is missing is a refused tool call and the `gated` outcome derived from it. Probe 4
already shows Herdr has no competing reading whatever the outcome turns out to be, so this looks
unlikely to change the answer.

### Two things the probe turned up unasked

`herdr agent start --kind` lists 22 kinds, and `claude`, `cursor`, `agy` and `hermes` are four of
them. Herdr knows all three routed providers and the unrouted fourth by name.

Line 2 of both records is a `UserPromptSubmit` `hook_response` injecting `additionalContext` out of
`~/.claude/CLAUDE.md`. A user-level hook reached into a Hydra run and added to it. That is ticket
46's subject, 46 is a grilling, and this is evidence for the human rather than an answer.

### Artifacts left behind

- Two records, `runs/01a078f9-...` and `runs/01a07900-...`, both `rate_limited`.
- One worktree and branch, `worktrees/01a07900-...` on `hydra/01a07900-...`, and its spawn directory.
  `hydra status` now shows two worktrees waiting for a reap. Nothing was reaped or pruned.
- herdr and hunk installed through Homebrew. Both probe servers stopped, their session directories
  left under `~/.config/herdr/sessions/`. The default Herdr server was never started, and no agent
  integration was installed.
