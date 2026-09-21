# 62 - The self-improvement module is called rungs

Type: build
Status: resolved
Blocked by: 61

## What

razzant/ouroboros, 1,342 stars and a paper, wins the search for the name, and the README already
speaks of "rung one" and "rung two". The human chose `rungs` on 2026-09-21. This ticket is the
mechanical half, in the code; the documents are the orchestrating session's.

1. `src/ouroboros/` becomes `src/rungs/`, with `git mv` so history follows. `pub mod ouroboros`
   in `src/lib.rs` and every `hydra::ouroboros::` path in `src/main.rs` and `tests/` follow.
2. Every identifier, doc comment and string inside `src/` and `tests/` that says `ouroboros`
   or `Ouroboros` says `rungs` or "the rungs", except two things that are on disk and stay:
   the branch names `ouroboros/<pass-id>` and `ouroboros/blocked/<pass-id>` that existing
   passes wrote, and the `docs/ouroboros-spec.md` path, which the session renames in the same
   change and updates every reference to. Where a doc comment cites "ouroboros spec 4.7", it
   cites "rungs spec 4.7". The branch prefix for new passes becomes `rungs/`; a test pins that
   an old `ouroboros/` branch is still recognised by whatever reads the prefix.
3. `integrations/herdr/` if it names the module anywhere.

Out of scope: `docs/`, `README.md`, `CLAUDE.md`, `IMPLEMENTATION.md`, `CHANGELOG.md`, `.scratch/`.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check`, `cargo test --release --test evolve --test curate --test gate`, and `grep -rn -i
ouroboros src tests` showing only the two things item 2 keeps. Do not run the whole suite.
## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c4a6-cf6d under `standard`: 18 files,
five of them `git mv`, 155 insertions and 116 deletions, clippy and fmt clean, evolve, curate,
gate and cleanup green in the worktree and on master. `pass_run_id_from_branch` reads both
prefixes with a test on each. The orchestrating session moved `docs/ouroboros-spec.md` to
`docs/rungs-spec.md` and renamed the subsystem across the documents; the run records, the
dated reviews, the ticket history and the branches of earlier passes keep the old word.
Nothing the ticket assumed turned out false.
