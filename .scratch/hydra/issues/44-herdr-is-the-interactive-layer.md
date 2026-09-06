# 44 - Herdr is the interactive layer, not a TUI of Hydra's own

Type: grilling
Status: resolved

## Question

D3 deferred the TUI and ticket 07 chose Textual to build it, on two requirements: a diff viewer,
which ratatui priced at two to three weeks, and making ACP the interactive path. Ticket 32 turned
streaming off and section 18 made ACP a non-goal, so the second requirement is gone and the first
is the whole case. Meanwhile Herdr (herdr.dev) exists: a terminal multiplexer that owns the panes
coding agents run in, with a socket API and a plugin format, looked at on 2026-09-06 in the fog
paragraph this ticket graduates from. Hunk (hunk.dev) exists too: a review-first terminal diff
viewer that takes a repo, a commit, a working tree or a patch on stdin, and renders agent
annotations above the hunk they explain.

**Does Hydra build a TUI, or is Herdr the interactive layer and Hunk the diff viewer, both driven
from outside?** And if the second, what crosses between Hydra and them?

## Answer

Resolved 2026-09-06 with the human, in the session that closed ticket 39, at the human's request
and against the one-ticket-per-session rule. Two questions, both answered.

**Herdr is the interactive layer, Hunk is the diff viewer, and Hydra builds no TUI.** D3 is amended
and section 18's TUI entry rewritten. Ticket 07's framework choice lapses with its requirements:
nothing streams and the diff viewer is someone else's.

**Nothing crosses from Hydra to either.** D5 does not apply, since neither is the harness nor
Ouroboros. D10 does: nothing has to be running for Hydra to work, so Hydra cannot depend on Herdr's
daemon. The shape is one-directional and on their side:

- A Herdr plugin launches `hydra run` in a pane. Hydra stays the parent of the provider's pty and
  the record is written as on any run.
- A bridge on Herdr's side tails `~/hydra/runs/` and labels the pane from the record: `gated`,
  `completed`, `timed out`. Herdr's own `blocked` state is screen-scraped; the record knows the
  truth and the bridge tells it.
- Hunk opens the run's branch, `hydra/<run-id>`, against the `diff_stat.base_tree` the record
  carries. If Hunk's annotation input can be fed, a bridge turns the record's messages into
  annotations beside the hunks they explain.
- Hydra's core knows neither tool exists.

**What does not change.** Interactive runs still record thin and are harvested at exit (ticket 13);
a pane buffer is a screen, not a record. The env allowlist stays as D6 has it, so whatever Herdr
puts in a pane's environment beyond `HOME`, `USER`, `PATH`, `TERM`, `SHELL` and `TMPDIR` never
reaches the provider.

**Before any build, ticket 45.** Neither Herdr nor Hunk is installed on this machine as of
2026-09-06, and everything the map knows about their APIs comes from reading, which the plan's own
note says is not measuring. Ticket 45 installs both and probes the claims this answer rests on.
Build tickets come after it, not before.
