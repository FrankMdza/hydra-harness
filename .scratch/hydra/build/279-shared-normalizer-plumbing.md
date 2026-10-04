# 279 - Shared normalizer plumbing

Type: build
Status: claimed
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 4 (finding 13): one `results_by_call(&[Event])`, one `provider_of(text)` for the three dispatchers in `normalize/mod.rs`, one `RecordedStart` each `Walk` builds `Recorded` from. The provider-specific middles of `decide` stay. Net: `tests/normalize.rs`, `tests/outcome.rs`, the fixture walk, `hydra doctor --walk` unchanged.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
