# 282 - Split run.rs and the claude harvest

Type: build
Status: open
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 7 (findings 20, 21): `src/run.rs` into `run/{lineage,refuse,record}.rs` along the seams the review names, and the inline `mod harvest` of `src/normalize/claude.rs` into its own file. Pure moves. Net: the whole suite.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
