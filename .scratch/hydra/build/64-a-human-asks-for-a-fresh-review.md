# 64 - A human asks for a fresh full-panel review of a blocked patch

Type: build
Status: resolved
Blocked by: 63

## What

Map ticket 57, resolved 2026-09-21: a block is final for its pass, and the appeal is a fresh
review of the same patch, not an override. `hydra evolve --review <pass-id> --evidence <file>`
takes a blocked pass, reads its patch off `rungs/blocked/<pass-id>`, and runs the panel again
under a new pass id with the evidence file's text in every judge's prompt under a heading that
says a human supplied it and why. The new pass's commit message names the pass it reviews. The
old block stays on its branch untouched. `ApprovedPatch` is still built only by `panel_verdict`;
nothing about the override path changes because there is none.

Tests in `tests/evolve.rs` against a scratch repo with a synthetic blocked branch: the review
pass names the old one, the evidence reaches the prompt, a pass id that is not blocked is
refused. Launch no provider. Do not edit `docs/`, `IMPLEMENTATION.md` or `.scratch/`.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` and `cargo test --release --test evolve`. Do not run the whole suite.

## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c624-12c5 under `standard`, 2.1M
tokens: `hydra evolve --review <pass-id> --evidence <file>`, each flag requiring the other and
`--review` refusing `--task`; `ensure_reviewable` refuses a pass that is not on a blocked branch
or that also has an offerable one; `read_blocked_patch` takes the diff off the blocked branch
against the proposer's `base_tree` and the proposer id and task off the pass commit and record;
`judge_task` gains the evidence under `HUMAN_EVIDENCE_HEADING`; `run_judge_panel` is the loop
both paths share; `commit_review_branch` lands the new pass through `commit-tree` and leaves the
old branch untouched; `ApprovedPatch` is still built by `panel_verdict` alone. Four tests in
`tests/evolve.rs`, 24 of 24. The worktree was seeded after ticket 63 and before 68, so the
session took its four files and restored 68's `sandbox: None` on the harness record by hand.
Nothing the ticket assumed turned out false.
