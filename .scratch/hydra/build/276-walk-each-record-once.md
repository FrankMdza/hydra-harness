# 276 - Walk each record once

Type: build
Status: claimed
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 1 of the Rust review (finding 1): `hydra doctor --walk` normalizes every record nine times. Memoize `read` in `walk::walk_corpus` (`src/walk.rs:68-84`) with a `HashMap<PathBuf, Result<Run, String>>`, and run `check_normalizing_writes_nothing` first so its snapshot still brackets the first read. Behaviour must not change. Net: `tests/doctor.rs`, `tests/record_contract.rs`. Measure `hydra doctor --walk` wall time on the real corpus before and after is the session's (a run cannot read it).

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
