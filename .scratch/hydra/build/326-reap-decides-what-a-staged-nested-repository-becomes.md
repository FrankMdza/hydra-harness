# 326 - `hydra reap` decides what a staged nested repository becomes: the files committed and the gitlink dropped, or a refusal naming the path

Type: build
Status: open
Blocked by: 325
Opened: 2026-10-15, by the session, from map 123's answer, question 2. The panel put the fix in
reap, outside the seatbelt, where Hydra already rewrites the index before it commits
(`unstage_missing_objects`, `src/worktree.rs` line 743). Two treatments are on the table and the
human has not picked yet: commit the files and drop the gitlink, saying so in reap's output, or
refuse with a message naming the path and leave the checkout for the human. The session's
suggestion is the first, because a refusal leaves the checkout behind as today's exit 128 does.
**This ticket is not takeable until the human's word is written here.**

Decision: pending.

## What

Whichever treatment: before `commit_everything`, reap finds every index entry of mode 160000
and every directory under the checkout that holds a `.git`, and applies the chosen treatment to
each. For "commit the files": remove the gitlink entry from the index, remove or rename the
nested `.git` (rename to `.git.nested` is reversible; say which you did), `git add -A` the
directory so the files land as plain files, and print one line per path in reap's output and
on the run record's reap event if it has one. For "refuse": exit non-zero before any commit with
one line per path, and leave the checkout untouched.

Build 325's first case changes its assertion to the chosen behaviour. 325 also measured the
unstaged case: `commit_everything` still stages a gitlink, then `git worktree remove` exits 128
(`'sub/.git' not recognized as a git repository`) and leaves the checkout, while the nested blob,
tree and commit arrive in the parent unreferenced. Treating only index entries of mode 160000
misses that path. The nested `.git` directories have to be found before `git add -A`, and 325's
second case moves with the first.

## Do

1. After 325 is in master and the decision line above says which.
2. The treatment; 325's case updated; a case for a checkout with two nested repositories.
3. `IMPLEMENTATION.md` beside the `run_object_env` paragraph says what reap does now.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green, and the
decision named in the Outcome.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
