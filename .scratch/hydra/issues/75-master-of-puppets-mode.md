# 75 - Master of Puppets: an orchestration mode, a mascot, and a ranking of agents

Type: research
Status: resolved
Blocked by:
Opened: 2026-09-24, by the session, at the human's request.

## Question

The human wants a new way of working with Hydra, in three parts that arrived as one request:

1. One command that opens an interactive, gated, recorded session in a Herdr pane, with a short
   name instead of the release path. `hydra run --interactive --policy standard` exists on `claude`
   only (src/interactive.rs), and nothing puts it in a pane.
2. A mascot: an ASCII hydra that idles, works, flinches on a refused call and breathes fire when a
   run is at full throttle, drawn for Ghostty. D3 and ticket 44 say Hydra builds no TUI and Herdr
   is the interactive layer, so where it lives is the first question.
3. An orchestration loop, "Master of Puppets": three proposers plan, a panel picks or composes,
   the human accepts, tasks get a grade and a class, agents are assigned from a ranking, and the
   panel then orchestrates and reviews with the human approving each run or diff. Behind it, a
   spec sheet per provider and model pair and two rankings, per provider and per task class,
   derived from the corpus.

Which parts are scripts, which are build tickets, and which are decisions the map has to take
first? The map's own item under `## Not yet specified`, a mini agent as a peripheral controller,
is the nearest prior thought.

## Method

A design panel of four proposers, one per provider, each writing
`docs/proposals/2026-09-24-master-of-puppets-<provider>.md` from the same brief. The brief and
the launch scripts are in `~/.claude/handoffs/2026-09-24-puppets-panel/`. The session
consolidates, and the human chooses. The 4-bit local model question that came with the request is
answered by the session in the same consolidation, not by the panel.

## Answer

Resolved 2026-09-24 by the session. The panel ran: `codex` (2.57M tokens), composer-2.5
(0.91M) and `agy` gemini-3.1-pro-high (0.50M) on recorded `standard` runs, and opus as an
Agent-tool subagent (0.18M) because the five-hour window sat at 0.60. The consolidation, with
every kept citation re-checked against cdb9475, is
[research/75](../research/75-master-of-puppets-panel.md). Raw proposals are outside the repo in
`~/.claude/handoffs/2026-09-24-puppets-panel/`.

What the four agree on: the launcher and the mascot are Herdr plugin work; the orchestrator lives
outside every run; a rate-limited task blocks and is never reassigned; state is one file per goal
under `~/hydra/puppets/`, which must join the deny list; cost is window share; the corpus has no
task class and no applied bit, so the per-class ranking starts from the mode's own log.

Where they split, the session chose: a skill plus a Python helper first and Rust after three
goals; approval on the plan, the batch table and each diff, with per-launch approval only where a
gate can fail open; one head per live run, capped at four.

What nobody asked and two seats found: an interactive run's record is thin until harvest, so the
mascot cannot see a refusal beside the very session it was asked for; `hydra-in-pane` cannot carry
an interactive run; the face's enum omits `codex`; rung two's abort never reads a limit's expiry;
four documents still say a panel of three.

A `codex` judge over the consolidation (0.64M tokens under `standard`; under `judge` it was gated on its first read, map ticket 63) said accept with changes and named the codex proposal best; the research document's "The judge" section carries what it corrected. The ten decisions the code cannot take are grilling ticket
[76](76-master-of-puppets-decisions.md). The build order is in the research document; its first
item, the `hy` launcher, is under an hour. The 4-bit local model question is answered at the end
of the research document: Colab is for jobs, not hosting; BitNet 2B4T is the one model that fits
beside the agents on 8 GB; and the grade and class classifier needs no LLM at all.
