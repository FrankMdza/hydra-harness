# 283 - An idiom sweep

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 8 (findings 7, 11, 12, 22): `let ... else` at the three `is_none`/`unwrap` sites; reuse `derived` at `walk.rs:669` and `:775`; drop the dead `|` branch at `call_head.rs:481` and `let _ = width;`; `?` at `run.rs:1331`; a clap `value_parser` for `--provider` (the error text may then list the valid names, which is the one visible change, and is wanted).

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-04 by run `01a10583-ffb6` (`cursor-agent` cursor-grok-4.6-high, `standard`),
applied without changes.

Built: `--provider` is a clap value parser returning `Provider`, so a bad name is refused with the
four valid ones (one test, which failed before); three `is_none`/`unwrap` pairs became `let ...
else` or `if let`; the walk reuses `derived` twice; `agy_gate_mounted` keeps its error chain with
`?`. Two items were already gone: the dead `|` branch (build 284) and `let _ = width;` (281).

Measured by the session: `cargo test --workspace`, 1,265 passed and none failed; clippy clean.
That closes the Rust review's eight refactor tickets (276 to 283) and its fixes (284).
