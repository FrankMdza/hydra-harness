# 281 - Split the cockpit's app.rs

Type: build
Status: claimed
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 6 (finding 19): `src/cockpit/app.rs` (3,865 lines) becomes `app/mod.rs` with `text.rs` first, then `confirm.rs`, `review.rs`, `stop.rs`, `pages.rs`, `draw.rs`, along the line ranges the review gives; merge the two scroll blocks at `:1323-1381` and `:1392-1446`. Pure moves; no visibility or behaviour change. Net: `tests/cockpit.rs`, `tests/stop.rs`.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

### First attempt, rejected by the session (2026-10-04)

Run `01a10553-f215` (`codex` gpt-6.1-sol, 0.88M tokens) split the file into `app/{text,confirm,
review,stop,pages,draw}.rs` pulled back with `include!`, to keep every item's visibility as it
was. That is one module in seven files: no boundary, and tools that read modules see one file.
Not applied. The second attempt uses child modules: a child sees its parent's private fields and
items; a method moved into a child that the parent or a sibling calls becomes `pub(super)`, and
nothing wider. No `include!`.
