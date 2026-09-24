# 75 - Master of Puppets: an orchestration mode, a mascot, and a ranking of agents

Type: research
Status: claimed
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
