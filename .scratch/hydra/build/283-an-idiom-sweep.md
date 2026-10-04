# 283 - An idiom sweep

Type: build
Status: open
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 8 (findings 7, 11, 12, 22): `let ... else` at the three `is_none`/`unwrap` sites; reuse `derived` at `walk.rs:669` and `:775`; drop the dead `|` branch at `call_head.rs:481` and `let _ = width;`; `?` at `run.rs:1331`; a clap `value_parser` for `--provider` (the error text may then list the valid names, which is the one visible change, and is wanted).

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
