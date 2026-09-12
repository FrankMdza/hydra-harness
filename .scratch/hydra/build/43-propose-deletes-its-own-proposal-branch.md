# 43 - `propose()` tries to delete the proposal branch it just made

Type: build
Status: resolved
Blocked by:

## What

`src/ouroboros/curate.rs:254-264`: after removing the curate checkout, `propose()` runs
`git branch -d <proposal branch>`. The branch is the output of the pass and is never merged
when this runs, so the delete always fails and every real pass ends with a `left` entry
saying so; the first real curate run, `01a08d66-76fd`, reported it. The delete exists for the
no-proposal case, where the branch equals the pinned sha and is noise. Delete only when the
branch points at the sha it started from; keep it otherwise. Test both in `tests/curate.rs`.
Map ticket 59.

## Outcome

Resolved 2026-09-12. `propose()` reads `git rev-parse HEAD` in the checkout before it
commits, which is the sha `check_out` cut the branch from. After `git worktree remove`
succeeds it reads `git rev-parse <branch>` and runs `git branch -d` only when the two match.
A branch that moved is the proposal and nothing asks git about it, so `left` stays empty on
a real pass. The `Proposal::branch_removed` doc comment no longer credits `git branch -d`
with the decision.

Both cases live in the two `tests/curate.rs` tests that already covered the commit and the
empty pass. The commit test now asserts `left` is empty and that the branch's tip is the
commit the pass made; the empty test asserts the checkout started at `main`'s sha before
the pass deletes the branch. `cargo test --release --test curate`: 18 passed.

The ticket said the delete "always fails" on a real pass. It always failed and was always
reported, and the branch was always kept, so the outcome was right and only the report was
wrong. Nothing about the first real pass needs redoing.
