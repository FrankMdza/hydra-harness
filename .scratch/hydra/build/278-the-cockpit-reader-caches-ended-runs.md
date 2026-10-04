# 278 - The cockpit reader caches ended runs

Type: build
Status: open
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 3 (finding 3, and finding 7's clone of `Ended`): skip `read_view` for an ended run whose length has not changed, read line 0 once per record and hand it to `opening_of`, cache goal folds by length and mtime. No change in what the cockpit shows. Net: `tests/cockpit.rs`, `tests/stop.rs`, the snapshot tests.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
