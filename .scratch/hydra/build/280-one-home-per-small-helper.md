# 280 - One home per small helper

Type: build
Status: open
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 5 (findings 5, 14, 15, 16): `Role::as_str` and `ResultStatus::as_str` replace three `role_str` and two `status_str`; `export_event` flattened to one match; the pass branch names written once in `rungs` and `branch_exists` returning `bool`; `live::first_line` the one line-0 reader for `reap`, `resume` and the cockpit; `splitlines` moved into `hydra_pyjson::py`. Behaviour unchanged; the whole suite is the net.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome
