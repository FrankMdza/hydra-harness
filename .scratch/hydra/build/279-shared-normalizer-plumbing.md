# 279 - Shared normalizer plumbing

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 4 (finding 13): one `results_by_call(&[Event])`, one `provider_of(text)` for the three dispatchers in `normalize/mod.rs`, one `RecordedStart` each `Walk` builds `Recorded` from. The provider-specific middles of `decide` stay. Net: `tests/normalize.rs`, `tests/outcome.rs`, the fixture walk, `hydra doctor --walk` unchanged.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-04 by run `01a1052f-22c1` (`claude` opus 5.5, `standard`, 1.59M tokens), applied
without changes; 78 lines fewer in `src/`.

Built: `normalize::provider_of`, `RecordedStart` (the six `run_start` fields each walk copied,
with `of` and `recorded`), and a free `results_by_call`, used by `claude` (both walks), `cursor`
and `agy`; `codex` keeps its own, which keeps results with an empty call id where the others drop
them, a difference the review had missed. The provider-specific middles of `decide` are untouched.

Measured by the session: `hydra doctor --walk` over the real corpus, built in master before and
after, prints the same 18 lines; the walk replays every gate and derives every outcome, so the
refactor changes neither.
