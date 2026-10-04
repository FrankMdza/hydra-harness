# 280 - One home per small helper

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 5 (findings 5, 14, 15, 16): `Role::as_str` and `ResultStatus::as_str` replace three `role_str` and two `status_str`; `export_event` flattened to one match; the pass branch names written once in `rungs` and `branch_exists` returning `bool`; `live::first_line` the one line-0 reader for `reap`, `resume` and the cockpit; `splitlines` moved into `hydra_pyjson::py`. Behaviour unchanged; the whole suite is the net.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-04 by run `01a10540-0ea7` (`cursor-agent` cursor-grok-4.6-high, `standard`,
4.67M tokens), applied without changes; 93 lines fewer.

Built: `Role::as_str` and `ResultStatus::as_str` replace three `role_str` and two `status_str`;
`export_event` is one match; `rungs::evolve` holds the pass branch names once and `branch_exists`
returns `bool`; `reap` and `resume` read line 0 through `live::first_line`, now `pub(crate)`;
`hydra_pyjson::py::splitlines` replaces three copies in the crates. The cockpit's own line-0
reader stays, since another run held `src/cockpit/` at the time. One edge moved: `reap::line_zero`
treated a readable empty record as missing and stopped; through `first_line` it now also tries
the pruned path, which no case is known to produce.

Measured by the session: `cargo test --workspace`, 1,256 passed and none failed; the export dry
run over the real corpus matches the one before but for runs that ended or started in between.
