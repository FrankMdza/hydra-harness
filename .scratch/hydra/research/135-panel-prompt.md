# Decision panel for map ticket 135

You are one seat on a four-seat decision panel. Read only: do not edit, create or delete a file in the repository, and do not commit. Scratch experiments under `/tmp` are fine. Your seat name is in your task line.

The human is away and said the panel decides. Your answer counts as a vote, so commit to one option per question.

## Read

1. `.scratch/hydra/issues/135-what-bwrap-does-with-paths-that-do-not-exist-and-the-private-git-dir.md`, the whole ticket.
2. `src/sandbox.rs`: `rules_of`, `compose_bwrap_for_launch`, `translate_rule_to_bwrap`, `write_bwrap_profile_for_launch`.
3. `src/worktree.rs`: `clear_private_logs`, `commit_everything`, and every place host-side git runs on a run's checkout. `src/reap.rs` and `src/interactive.rs` for harvest.
4. `src/policy/mod.rs`: `own_write_paths`, `write_denies` and the helpers it calls.

## Answer

For question 1, pick A, B, C or D and say why. For each attack the ticket lists, say whether your choice stops it, and how. Name every host-side git call your choice has to cover, with file and line. If you pick B, say what changes on macOS and which tests break.

For question 2, give one rule per kind of path: protected config, layout directory, worktree skill or plugin root, worktree settings file. For each rule, say what it breaks. If a claim depends on what bwrap or git does, say whether you ran it or read it. One claim to check: does `bwrap` create a missing mount point on the host filesystem under `--dev-bind / /`?

Then list the build tickets your answer needs, one line each, smallest first.

At most 900 words. End with `## Pending from me`, `## Changed` and `## Found`.
