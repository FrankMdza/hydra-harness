# 282 - Split run.rs and the claude harvest

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 7 (findings 20, 21): `src/run.rs` into `run/{lineage,refuse,record}.rs` along the seams the review names, and the inline `mod harvest` of `src/normalize/claude.rs` into its own file. Pure moves. Net: the whole suite.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-04 by run `01a1056a-8b37` (`codex` gpt-6.1-sol, `standard`, 0.63M tokens),
applied without changes. Pure moves into child modules, no `include!`.

Built: `src/run/mod.rs` (1,661 lines, the launch sequence and its tests), `run/lineage.rs` (203),
`run/refuse.rs` (349), `run/record.rs` (341), and `src/normalize/claude/harvest.rs` (463, the
interactive transcript reader). Every item the new files expose as `pub` was already `pub` in
`run.rs`, checked by the session one by one, and `run` re-exports them, so no caller changed.

Measured by the session: `cargo test --workspace`, 1,262 passed and none failed; clippy clean.
