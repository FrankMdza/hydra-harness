# 82 - The panel's type carries one answer per provider

Type: build
Status: resolved
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

`PanelAnswers::new(claude, cursor_agent, agy, codex)` stores one outcome per `Provider` in a fixed
order and writes `Provider::binary()` into each; `run_judge_panel` returns it directly, so the
production path cannot build a bad panel and cannot fail on it between harness start and end.
`PanelAnswers::from_outcomes` is the list-shaped path and refuses a duplicate, missing or unknown
provider; `verdict_from_outcomes` turns that refusal into `PanelVerdict::Blocked`. `panel_verdict`
borrows the panel. `tests/evolve.rs`'s critical-finding test now gives the finding to one judge only.
The type guarantees one slot per provider label, not which binary wrote each answer; the doc comment
says so. Measured on master: evolve 25, curate 20, lib 188 passed; clippy and fmt clean. Built by
`claude` on Haiku 4.5 under `standard`, run 01a0cc2b-10d6, 4.0M tokens. Haiku's version compared
strings and turned a short panel into an error that left the pass record open; an Opus review found
both and an Opus subagent rebuilt it on `Provider`.
