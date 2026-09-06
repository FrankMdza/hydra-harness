# 45 - Install Herdr and Hunk and probe what ticket 44 assumed

Type: task
Status: open

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
