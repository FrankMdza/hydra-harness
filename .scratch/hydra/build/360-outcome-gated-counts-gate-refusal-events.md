# Build 360 - `Outcome::Gated` counts `Event::GateRefusal` when no call got through

Status: resolved
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

2. **`src/walk.rs`:**
   In `check_gated_write_denials`:
   - Mirror the same `has_hydra_refusal_row` check over `run.events` (`Event::GateRefusal` whose `body["reason"].as_str()` satisfies `crate::normalize::hydra_voice(reason).is_some()`).
   - Compute `expected` as `Outcome::Gated` and increment `gated` when `(calls == 0 && has_hydra_refusal_row) || (calls > 0 && refused == calls)`.

3. **`tests/outcome.rs`:**
   - Add two unit tests:
     - A `codex` run that ends cleanly with `turn.completed`, makes no stream tool calls, and carries a `gate_refusal` row with a `hydra/capability:` reason derives `(Outcome::Gated, Source::Derived)`.
     - A `codex` run that has one allowed `command_execution` item alongside a `gate_refusal` row with a `hydra/capability:` reason derives `(Outcome::Completed, Source::Derived)`, not `Outcome::Gated`.
   - In `the_fixture_write_denial_is_the_only_gated_fixture_on_the_stream`, update `assert_eq!(gated, 1, ...)` to `assert_eq!(gated, 2, "the fixtures hold one read-only Write denial and fixture 321's judge Bash refusal row")`, because fixture `tests/fixtures/records/claude@2.1.261/00000000-0000-7000-8000-000000000321.jsonl` has a refused `Bash` `gate_refusal` row under `judge` with no stream call.

4. **`IMPLEMENTATION.md`:**
   Note under the outcome / `gate_refusal` description that `refused_everything` and `walk::check_gated_write_denials` count a `gate_refusal` row in Hydra's voice when no stream call was allowed.

## Done when

- `cargo fmt --all --check` passes.
- `cargo clippy --release --workspace --all-targets -- -D warnings` passes.
- `cargo nextest run --release -E 'binary(outcome)'` passes, all 48 tests green.

## Outcome

Run `01a17ccf-c6a7-7dc3-9e8e-a67cee2e7198` on `claude` Haiku 5.5, 1,834,981 tokens, after an earlier pass `01a17cb2-738d-73e2-964f-0aa31b6f95e9` (1,634,078 tokens) identified the companion `walk::check_gated_write_denials` check.

Built as specified. `refused_everything` now returns true for a run with no permission
decisions when a `gate_refusal` row carries Hydra's voice and no leak exists. A row beside an
allowed call still leaves the run `completed`. The walk's `check_gated_write_denials` counts the
same rows through a shared helper, `outcome::has_hydra_refusal_row`, so the two cannot drift.

Measured: the new codex test without the fix derived `(Completed, Derived)` and failed on that
assertion. With the fix, all 48 outcome tests pass, `binary(doctor)` (13) passes, and
`binary(record_contract) | binary(normalize) | binary(show) | binary(harvest)` (157) passes.
`cargo fmt --all --check` and `cargo clippy --release --workspace --all-targets -- -D warnings`
both exit 0.

Assumption that held: fixture 321's `judge` `Bash` row derives `Gated` with no stream call, so the
fixture count moves from 1 to 2 as the ticket says.

The ticket's intro says four checks, but its `Done when` list names three. The fourth run here is
`binary(doctor)`, which drives the walk this change edits.
