# 284 - The cockpit's tokenizer reads characters, and its scroll clamps

Type: build
Status: open
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Fixes that change behaviour, findings 8 and 10: `src/cockpit/call_head.rs:445` and `:498` iterate bytes as `char`, so an accented path is shown as mojibake and `à` (C3 A0) splits a word; iterate `char_indices()`. And the scroll limits at `app.rs:1331` and `:1400` cast to `u16`, wrapping past 65,535 rows, with `+` that panics in debug; use `u16::try_from(..).unwrap_or(u16::MAX)` and `saturating_add`, and clamp the wheel at `:1063`. A test each, failing before.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
