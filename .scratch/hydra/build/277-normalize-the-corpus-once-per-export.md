# 277 - Normalize the corpus once per export

Type: build
Status: open
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 2 (finding 2): `hydra export` normalizes the corpus once, once more per allowlisted repo, and twice more per eligible run. Build one `BTreeMap<String, Run>` in `export_with_signoff` and thread it through `audit`, `collect_pairs` and `read_judge_pairs`. Output must be byte-identical. Net: `tests/export.rs`.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
