# Build 360 - `Outcome::Gated` counts `Event::GateRefusal` when no call got through

Status: claimed
Size: haiku

## Goal

In `src/outcome.rs`, `refused_everything` checks whether the gate refused every call the run tried by iterating over `run.permission_decisions()`. On `codex`, a denied non-shell tool call such as `webrun` under `standard` or an unknown tool leaves no stream frame and no `Command blocked by PreToolUse hook` stderr line; since build 339 its only trace on the record is an `Event::GateRefusal` event.

When a run's only tool calls are such refused calls, `run.permission_decisions()` is empty, so `calls == 0` in `refused_everything` and the run derives `Outcome::Completed` instead of `Outcome::Gated`. Meanwhile, if a run also made any allowed tool call, `calls > 0` and `denied < calls`, so a refusal row beside allowed work must not make the run `Outcome::Gated`.

## What to change

1. **`src/outcome.rs`:**
   In `refused_everything`:
   - Check whether `run.events` contains any `Event::GateRefusal(e)` whose `e.body["reason"].as_str()` carries Hydra's voice via `crate::normalize::hydra_voice(reason).is_some()`.
   - When `calls == 0`, return `leaks == 0 && has_hydra_refusal_row`.
   - When `calls > 0`, return `denied == calls && leaks == 0 && (by_hydra || has_hydra_refusal_row)`.

2. **`tests/outcome.rs`:**
   Add two unit tests:
   - A `codex` run that ends cleanly with `turn.completed`, makes no stream tool calls, and carries a `gate_refusal` row with a `hydra/capability:` reason derives `(Outcome::Gated, Source::Derived)`.
   - A `codex` run that has one allowed `command_execution` item alongside a `gate_refusal` row with a `hydra/capability:` reason derives `(Outcome::Completed, Source::Derived)`, not `Outcome::Gated`.

3. **`IMPLEMENTATION.md`:**
   Note under the outcome / `gate_refusal` description that `refused_everything` counts a `gate_refusal` row in Hydra's voice when no stream call was allowed.

## Done when

- `cargo fmt --all --check` passes.
- `cargo clippy --release --workspace --all-targets -- -D warnings` passes.
- `cargo nextest run --release -E 'binary(outcome)'` passes, including the two new tests.
