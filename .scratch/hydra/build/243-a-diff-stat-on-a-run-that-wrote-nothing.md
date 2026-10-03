# 243 - `diff_stat` counts 75 deletions on a run that wrote nothing

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-03 by run `01a101a7-89a9` (`claude` opus 5.5, `standard`, 1.51M tokens),
applied by the session unchanged.

The file and the two trees: `.claude/skills/puppets/SKILL.md`, tracked, 75 lines. The base is
the seed tree, staged before skill injection. The head is the closing tree, staged by
`diff_stat` into a fresh empty index after injection wrote `.claude/skills/.gitignore` holding
`*`. In an empty index every file is untracked, `add -A` skips untracked files an ignore rule
matches, so the tracked skill file was left out and `diff-tree` reported it deleted. The
session's own `git diff --numstat` used the worktree's real index, where the file is tracked, so
it could not see this. The provider wrote nothing; the count was wrong, and every `claude` run in
this repository carried the same false deletion.

Built: `seed_tree` (`src/worktree.rs`) runs `git read-tree HEAD` into its scratch index before
`add -A`, so a file tracked in the checkout's `HEAD` stays tracked whatever an ignore rule says.
Both snapshots and `rungs/evolve.rs` go through it. Test:
`a_run_that_writes_nothing_changes_no_files_even_where_injection_ignores_a_tracked_one` in
`tests/worktree.rs`, which runs the real `skills::inject` for `claude`.

Measured by the session on master: with the `read-tree` line removed the test fails with
`Stat { files_changed: 1, insertions: 0, deletions: 75 }`; restored, it passes. `--test worktree`
9, `--test evolve` 42, `--lib` 234; fmt and clippy clean.

Not done: why `run_start` on `01a0fe73-5a9c` carries no `base_tree` (the run reads it as the
launch-time seed snapshot failing, then `hydra show` rebuilding the base from the seed index,
`src/status.rs:188`). Not checked: whether any reader stored the false count; the grader and
Rungs read diff stats through `hydra show`, which computes them, so a record holds none.
