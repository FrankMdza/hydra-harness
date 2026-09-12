# 43 - `propose()` tries to delete the proposal branch it just made

Type: build
Status: open
Blocked by:

## What

`src/ouroboros/curate.rs:254-264`: after removing the curate checkout, `propose()` runs
`git branch -d <proposal branch>`. The branch is the output of the pass and is never merged
when this runs, so the delete always fails and every real pass ends with a `left` entry
saying so; the first real curate run, `01a08d66-76fd`, reported it. The delete exists for the
no-proposal case, where the branch equals the pinned sha and is noise. Delete only when the
branch points at the sha it started from; keep it otherwise. Test both in `tests/curate.rs`.
Map ticket 59.
