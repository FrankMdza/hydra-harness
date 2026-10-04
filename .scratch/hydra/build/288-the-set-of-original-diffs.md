# 288 - The set of original diffs

Type: build
Status: resolved
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

Built 2026-10-04 by a Grok 4.6 run (`01a105e9`), reviewed and applied by the session.

`hydra-grader originals [--home] [--dataset] [--tickets] [--repo] [--runs] out`
(`crates/grader/src/originals.rs`, `originals_cli.rs`): one line per verdict in
`ticket-verdicts.json`, with ticket from the label queue, provider and model from
`dataset.jsonl`, and the unified diff of `refs/heads/hydra/<run-id>` against the record's
`worktree.base_tree`, plus `files_changed`. Git runs with the inherited git path variables
removed and no global or system config. Three tests on a synthetic repository and queue, each
failing before the change; `cargo test -p hydra-grader` 77 of 77, fmt and clippy clean.

**Two pointers the session wrote were wrong.** `ticket-verdicts.json` is `{run_id: verdict}`,
not six-column rows; the run joins the queue and the dataset for the rest. The record's field is
`worktree.base_tree`; `diff_stat.base_tree` is only what `hydra show` prints.

**Measured on the real queue: 70 verdicts, 70 lines, 7 with a diff.** 62 runs' branches are
gone and 1 run has no base tree. `git fsck --unreachable` finds no commit for them either, since
each run commits into its own object directory and a deleted branch takes the commit with it.
So the set of original diffs is 7 today. It grows only forward, from runs reaped without
`--delete-branch`. The session's own applied patches under `~/.claude/handoffs/*/diff-*.patch`
cover some of the lost runs, but they are cut to the ticket's files, so they are not the diff
as returned and the command does not read them.
