# 85 - A cockpit for the orchestrating session

Type: research
Status: resolved
Blocked by:
Opened: 2026-09-29, by the session, from the human's notes of the same day.

## Question

The human described what he wants to see while a Master of Puppets session runs, in his words
(dictated, translated):

> The visual is the chat where the orchestrating session runs. Beside it, the table I defined:
> which tasks, which agent each went to, with which model, and the definition of done. Beside
> that, panes like a kanban, one per agent in the table. If I want to go into what an agent is
> doing, put my hands in, ask for a review or whatever, I click the pane and it expands. The
> chance to change policies, under human approval. Include the mascot. Maybe the Hydra grows a
> head per active agent. I do not know whether Herdr limits us there; if it does, we build our
> own TUI.

An assistant answered him with a three-pane layout, a header with the mascot growing heads per
agent, a task matrix, a kanban with drill-down and approval gates, a hot policy editor, and a
comparison of Ratatui against Textual. None of that is measured against what Hydra and Herdr
already do, and the assistant did not know either.

What exists, and what would a first step be that shows the table and the panes without building
a TUI from nothing?

## What to find out

1. **What Hydra already shows.** `integrations/herdr/`: `hy.zsh`, `bin/hydra-mascot` and
   `MASCOTS.md` (the mascot already has states; does it know how many seats are alive?),
   `bin/hydra-bridge`, `bin/hydra-graph`, `docs/status.html`, and `integrations/puppets/puppets.py
   board`, which already prints the table of tasks, assignments and what the human owes. Say for
   each what it renders, from what source (record, goal log, `hydra status`), and how often.
2. **What Herdr can do.** `herdr --help` and every subcommand's help, the plugin manifest
   `integrations/herdr/herdr-plugin.toml`, and whatever documentation Herdr has on this machine
   (`herdr` is on `PATH`; look for a docs or man path in its help). Can a plugin: split and label
   panes, run a command in a pane, draw a table or only run a process, react to a click, show a
   modal, read pane content? Which of the human's wants does Herdr cover as it is, which need a
   process drawing inside a pane, and which it cannot do at all? Measure, do not guess: run the
   help commands and quote them.
3. **The three constraints Hydra puts on any cockpit.** D10, nothing has to be running for the
   harness to exist (`CLAUDE.md`, `docs/rungs-spec.md` near line 790). The gated session cannot
   read `runs/`, so any view runs outside the seatbelt, as Herdr and the bridge do. And protected
   config: a policy editor in a pane would write `policies/` and `config/panels.json`, which only
   the human writes; say what "under human approval" could mean that does not break that.
4. **Ratatui against Textual, with sources.** Mouse events, clickable table cells, modals,
   multiple streaming panes, and what each costs to build for one person. Only claims with a
   source: crate or package docs, or an issue. Say what "spawning a head per agent" costs in each.
5. **The smallest step.** A proposal for what to build first that a human would use next week:
   for example a pane that runs `puppets.py board` on a loop beside the session, and the mascot
   fed by seat count. Cost in sessions, what it reuses, what it does not do. Then the second step.
   No cockpit from scratch in the first step.

## Deliver

`.scratch/hydra/research/85-a-cockpit-for-the-orchestrating-session.md`, in English, plain words,
no em dashes: findings with file and line, quoted help output, sources for every external claim,
and the proposal. Then a draft `## Answer` for this ticket, marked as a draft for the human, with
the decision it recommends and what would change it. Do not edit this ticket file or anything
under `src/`, `integrations/` or `config/`.

## Answer

Resolved 2026-09-29, by the session, under the human's delegation of that day. The human can
reverse it. Research by `claude` sonnet on run 01a0f041-405c under `standard`, about 1.14M tokens,
saved as `research/85-a-cockpit-for-the-orchestrating-session.md`.

**Step 1 only, no TUI from scratch: a pane that loops `puppets.py board <goal>` beside the chat,
launched next to `hy`.** Herdr 0.8.2, measured from its help and bundled schema, runs processes,
splits and zooms panes, and carries up to sixteen tokens and a title per pane. It draws no table
and reacts to no cell click. That covers the table and the per-agent panes: `puppets.py launch`
already opens one Herdr pane per agent, and the mascot already wakes one head per live pane, up
to three, then shows `+N`. A fourth head is new art whichever toolkit draws it.

Policy changes stay out of the cockpit's write path. Protected config stays the human's; a pane
may show a policy and print the command the human types.

Step 2, only if step 1 is used and the lack of clicks hurts: a read-only Textual board. Textual
over Ratatui because the mouse and modals are built in and `integrations/` is already Python. A
policy editor that writes is a map ticket, because it changes what protected config means.

Not confirmed by the research: Herdr zoom on a mouse click, whether a task stores its definition
of done, how a goal id is listed, Textual `DataTable` mouse selection, `popup` in the CLI on 0.8.2.

Build ticket 148 for step 1.
