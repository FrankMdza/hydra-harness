# 89 - Rung two offers the tree its checks and judges read, and no other

Type: build
Status: claimed
Blocked by:

## What

Finding G42 of the second hiring panel, 2026-09-23, raised by `codex`. A rung-two pass reads the
proposer's patch once, in `read_proposer_patch` (`src/rungs/evolve.rs` ~397). The size cap
(`patch_over_cap`, ~143) and the protected-path check (`protected_path_hit`, ~164) read that patch,
and so do the judges (`judge_task(&patch, ...)`, ~253). Between the read and the commit,
`admission_gate` (~229) runs `cargo build` and `cargo test` in the same worktree under the proposer's
seatbelt profile, which lets them write there. Then `commit_pass_branch` (~968) runs
`commit_everything` on the worktree as it stands now and points the pass branch at that commit.
`ApprovedPatch` (`src/rungs/review.rs` ~14) holds a pass id and nothing that names a tree.

So a build script or test in the patch can edit files during admission, a protected path such as
`src/policy/` included, after both checks ran. The offered branch `rungs/<pass>` then carries
changes no check and no judge saw, under a commit message that carries the panel's verdict. A
process the proposer left running can do the same.

`read_proposer_patch` already builds the tree it diffs: `worktree::diff_stat` (`src/worktree.rs`
~333) calls `seed_tree(worktree, index)`, which returns a tree sha. Carry that sha on
`ProposerPatch`, and make the pass branch's commit have exactly that tree whatever the worktree
holds afterwards. Two shapes that work, your call, say which and why in the Outcome:

- build the verdict commit with `git commit-tree <read tree> -p <checkout HEAD> -m <message>`, so
  the worktree's later state never reaches the branch; or
- recompute the tree right before the commit, and if it differs from the read one, send the pass to
  `rungs/blocked/<pass>` with the reason "the worktree changed after its patch was read", naming the
  paths that changed.

Either way `ApprovedPatch` gains the tree sha, and the commit message names it.

## Done when

- A test in `tests/evolve.rs` with a patch whose admission step edits a file in the worktree (a
  `build.rs` that writes, or a test that writes, whatever `tests/evolve.rs` can drive without a
  provider) shows the offered or blocked branch does not carry the edit. Run it without your change
  and say in the Outcome that it failed.
- A pass whose worktree did not change after the read behaves exactly as before; the existing
  `tests/evolve.rs` cases pass unchanged.
- `IMPLEMENTATION.md`'s rung-two section says the branch carries the read tree. `docs/` is the
  session's.

## Notes for the run

Tests take a scratch `Layout`, never `Layout::discover()`. Ignore
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`, which fails inside every
worktree. Run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` before you stop.
