# Hydra cockpit, for the person at the keyboard

This file is for a model in the chat to read and answer questions in plain language.
Nothing loads it automatically. It is not `AGENTS.md`.

## The two ways to run the cockpit

For a gated chat, use `hydra run --interactive --provider <p> --policy <name>` on the command line.

**Ungated copilot: `hydra cockpit --session claude` or `--session codex` with no `--policy`.**
The left panel is an ordinary chat of that provider with the person's own permissions and no gate. It launches agents
from a shell with `hydra run`, waits for them with `hydra wait`, and reads them with
`hydra show`. Each agent it launches is gated and recorded; the copilot is not. It has no
`panel` or `delegate` tool. On `claude` its `Edit`, `Write` and `NotebookEdit` tools are
disallowed so file changes go through `hydra run`; `codex` has no such switch. It is told this in one line when it starts.

**Master of Puppets: the copilot with `HYDRA_COPILOT_MODE=puppets`.** The same ungated copilot,
started from `~/.hydra-puppets-copilot` with the `puppets` skill installed and its loop in the
chat's instructions, so a goal goes through plan, table, launches and diffs on the repository
`hydra-copilot` was given; the goal and its panel show in this cockpit.

**Viewer: `hydra cockpit` with no `--session`.** No chat. The cockpit shows runs, seat
requests and Puppets steps on disk, and you can still approve, deny, review and stop from the
keyboard. The `a` key of the help view is not offered, since there is no chat to type into.

`F2` moves the keys between the chat and the cockpit with the copilot.

In both modes the cockpit may start `hydra-puppets` for pending steps and reviews, and without a recent or unfinished goal its tasks pane lists live runs and runs started since the cockpit opened, with Enter opening the selected run. That helper
is the only writer of a goal log. The cockpit does not write policy files; `e` in the policy
pane opens a file in your editor and the cockpit reloads the set.

## Keys (same table as the `?` help overlay)

Moving:

| Keys | Does |
|---|---|
| arrows, Tab, Shift-Tab | move within a panel, or between panels |

Opening:

| Keys | Does |
|---|---|
| Enter, Esc | open the selected run as a full page, or a request or step in the detail pane; Esc closes or leaves the page |
| Left, Backspace | on a full-page run, back one step; Esc returns to the cockpit |
| t | open the shown goal's task table as a full page; t again or Esc returns |

Scrolling the detail or a full-page run:

| Keys | Does |
|---|---|
| PageUp, PageDown, Space, b | scroll by a screen |
| Home, g, End, G | scroll to either end |

Writes that wait for `y`:

| Keys | Does |
|---|---|
| a, y | approve the selected seat request, or run the argv a step listed |
| d, y | deny the selected seat request; a step has no deny |
| v, y | start hydra-puppets review on the selected task |
| m, y | stop the selected live seat |

Views and other cockpit keys:

| Keys | Does |
|---|---|
| k | swap the four lanes by state for one lane per agent |
| r | swap tool calls for the raw last twenty record lines |
| i | reveal the whole input of the selected call or line |
| p | policy pane: the set sha and live runs; e opens the file in $EDITOR |
| q | quit |
| ? | open or close the help overlay |

Help overlay only, with `--session`:

| Keys | Does |
|---|---|
| a | in this help view, type a question about COCKPIT.md into the chat, unsent |

Policy pane (after `p`):

| Keys | Does |
|---|---|
| Up, Down | select a policy file |
| PageUp, PageDown | scroll live runs in the policy pane |
| e | open the selected policy file in $EDITOR |
| p, Esc | close the policy pane |

With `--session`:

| Keys | Does |
|---|---|
| F2 | move the keys between the chat and the cockpit |
| Ctrl-q | quit once the chat has exited; while it runs, ask then y hangs it up |
| (with keys on the chat) | every other key goes to the chat |

While the help overlay is open, `Esc` or `?` closes it. With `--session`, press `a` in the
help view to paste this into the chat, unsent, with the cursor at the end so you can finish
the sentence: `Read COCKPIT.md in this repository and tell me how to `.

## How to scroll, stop a seat, and read a run

Use `PageUp`, `PageDown`, `Space`, `b`, `Home`, `g`, `End`, or `G` in the detail pane or on a
full-page run. `Enter` on a lane card opens that run full screen; `Left` or `Backspace` steps
back through pages you opened. `m` then `y` on a live card stops that seat the same way
`hydra stop` does. With `--session`, only runs launched as seats of this chat's session can be
stopped from `m`; the cockpit leaves an unsent note in the chat when a stop succeeds. To read
what a run did, open it and use `r` to switch between summarized tool calls and the raw tail of
the record, and `i` to reveal a hidden input when the policy allows. Run rows show provider,
exact recorded model (or the provider's init model, then `default`), policy, tokens so far and
elapsed seconds. At 80 columns the panes stack; narrower rows drop elapsed first, tokens next
and policy last before clipping the identifying text. The run detail begins with its task,
or its question for a panel seat. Panel seats add their logged roles when available. The
Puppets table shows each panel and tie-break seat's provider, model, vote mapped through the
logged proposal labels, proposal order, split or unanimity, and the recorded verdict source.
An older panel event without roles shows none; missing votes show unknown.

## Gate refusals you meet first

A refused tool call or shell command comes back with a line that starts with `hydra/` and a
code. These four show up first for newcomers.

**`hydra/out-of-scope`.** The path is outside what this policy lets the tool touch. Under
`standard` a file tool reads and writes inside the worktree and reads the skill snapshot. Stay
inside them. A shell command is not checked this way, but in a gated session the sandbox still
refuses a write under the home directory outside the worktree.

**`hydra/protected-path`.** The target is one of Hydra's own paths: its run records, its
policies and configuration, the gate script, the skills it linked. No policy lets a run touch
them. A person changes those files by hand.

**`hydra/capability`.** The policy does not allow that kind of action at all, for example
`file_write` under `read-only`, or `subagent_spawn` (a provider's own subagents) under
`standard`. The refusal says what to do instead: ask for a `panel` in a gated session, or ask
the person to launch an agent with `hydra run`.

**`hydra/seatbelt-delegation`.** A run inside the macOS sandbox asked the `delegate` tool for a
child. That child would be Hydra itself under the parent's sandbox, which cannot reach Hydra's
own data, so it is always refused. The way out is the person, or the ungated copilot, launching
it with `hydra run`.

The refusal text is the source of truth for what went wrong. This file only orients you; it does
not replace the gate.
