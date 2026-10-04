# 278 - The cockpit reader caches ended runs

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 3 (finding 3, and finding 7's clone of `Ended`): skip `read_view` for an ended run whose length has not changed, read line 0 once per record and hand it to `opening_of`, cache goal folds by length and mtime. No change in what the cockpit shows. Net: `tests/cockpit.rs`, `tests/stop.rs`, the snapshot tests.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-04 by run `01a1052a-6599` (`codex` gpt-6.1-sol, `standard`, 0.92M tokens),
applied without changes.

Built: the reader keeps an ended run's card keyed by its file length and skips `read_view`, line
0 and the ended projection while the length holds; goal logs are folded once per length and
mtime and shared between the table and the seat roles; line 0 is parsed once per read; `ended`
returns a reference instead of cloning the projection. Per tick an unchanged ended run saves three
file opens and three JSON parses. Not done: a single line-0 read shared with `live::read_view`,
which would change `read_view` itself.

Measured by the session: `cargo test --workspace` with it applied, 1,254 passed and none failed;
clippy clean.
