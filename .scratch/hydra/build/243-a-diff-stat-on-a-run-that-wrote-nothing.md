# 243 - `diff_stat` counts 75 deletions on a run that wrote nothing

Type: build
Status: open
Blocked by:

## What

Usability probe of 2026-10-02. Run `01a0fe73-5a9c-7c81-aff3-9726beb4ea60` (`claude` sonnet,
`standard`) was refused its one `Write` and ended `gated`. `hydra show` says
`files_changed: 1, insertions: 0, deletions: 75` against `base_tree`
`fe17debb1afac63c0294fc352394a4db8081c803`.

The orchestrating session could not explain the number. `git diff --numstat <base_tree>` in the
worktree shows only the three untracked files the worktree was seeded with from the parent
(`pendintes_hydra.md`, 12 lines; two files under `.scratch/hydra/labeling/2026-09-29/`, 176
lines each), and no sum of those is 75. `run_start` on that record carries no `base_sha` and no
`base_tree`; the `base_tree` is on the normalized answer only. The worktree and its branch are
kept for this ticket: do not reap `01a0fe73-5a9c`.

## Do

1. Reproduce: which file does `diff_tree_stat` (`src/worktree.rs:1191`) count, and which two
   trees does it compare on this run. Write the answer into the Outcome before changing code.
2. If the count is wrong, fix it and add a test in `tests/worktree.rs`: a run that writes nothing
   in a worktree seeded with untracked files reports zero files changed.
3. If the count is right and names something the provider wrote (a settings file, a lock), say
   what, and decide whether `covers` should name it. That part may be a decision: if it is, stop
   and say so.

## Done when

- The Outcome names the file and the two trees. fmt, clippy and `cargo test` pass.
