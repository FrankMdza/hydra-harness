# 288 - The set of original diffs

Type: build
Status: claimed
Blocked by:

## What

Map 109, first step (`.scratch/hydra/research/109-a-system-one-of-our-own.md`): every human verdict in build 143's queue joined with the diff exactly
as the agent returned it, read from the run's branch `hydra/<run-id>`, written to a local jsonl.
It is the bottleneck of every System 1 that follows. A command (where it lives is part of the
work: `hydra-grader` is the likely home). Tests on synthetic repositories; a run cannot read
`~/hydra/runs`.

Pointers, added by the session on 2026-10-04:

- The verdicts are build 143's queue in `crates/grader/src/queue.rs`: `ticket-verdicts.json`,
  rows with `ticket`, `run_id`, `provider`, `model`, `outcome` and `verdict` (`applied`, `fixed`,
  `unknown`). Read it through the code that already reads it; do not parse it a second way.
- "The diff exactly as the agent returned it" is the run branch's tip tree against the run
  record's `diff_stat.base_tree` (reap commits the worktree onto `hydra/<run-id>` before the
  checkout goes). A run whose branch or base tree is gone gets a line with a `missing` reason,
  never a silent skip, so the count of verdicts and the count of lines match.
- The home is a `hydra-grader` subcommand (`crates/grader/src/*_cli.rs` shows the pattern).
  One line per verdict: `run_id`, `ticket`, `verdict`, `provider`, `model`, `diff` (unified, the
  same `git diff` the session applies with), `files_changed`. Output is a path the human passes;
  nothing leaves the machine.

## Outcome
