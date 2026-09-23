# 82 - The panel's type carries one answer per provider

Type: build
Status: open
Blocked by:

## What

Study session block (e), question 1. `panel_verdict` at `src/rungs/review.rs:416` checks
`outcomes.len() == 4` and not that the four come from four different providers. Today
`run_judge_panel` guarantees that by construction, and the type does not. Replace the slice with a
`PanelAnswers` whose only constructor takes one `JudgeOutcome` per `Provider` (or an array indexed
by provider), so four answers from one binary cannot be represented. Every caller and every test
that builds a panel moves to it.

## Done when

`panel_verdict` takes the new type; a unit test shows the constructor refusing a duplicate provider
and a missing one; `cargo test --release --test evolve` and the lib tests green; clippy and fmt
clean. Edits `src/rungs/` and `tests/` only.

## Outcome
