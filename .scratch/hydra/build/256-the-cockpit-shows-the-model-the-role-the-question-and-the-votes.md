# 256 - The cockpit shows the model, the role, the question and the votes

Type: build
Status: claimed
Blocked by:

## What

Map 102 D7, decided by the human on 2026-10-03, the long form. He wants to see not only that
`codex` is running but which model, and for a panel, how it stands.

1. **Every run's row**: provider, exact model, policy, elapsed time, tokens so far. The model is
   the one the record carries (`run_start.model`, or the model the provider's own lines report
   when the launch passed none, as `codex` does); write `default` only when neither says.
2. **A run's detail**: the task text first, before the tool calls.
3. **A panel seat**: when a run is a seat of a Puppets panel (its id is in a goal log's
   `judge_run_ids` or `tie_break_run_ids`), the row adds its role and the detail adds the question
   it was asked. Read the role and the order from the goal log through `hydra-puppets`' own
   reader or the files it writes; do not re-derive them. Build 252 added `tie_break_*`,
   `label_map`, `seat_orders` and `verdict_source` to `panel_verdict`.
4. **A panel**: in the Puppets view, a goal's panel shows who sat (provider and model per seat
   and per tie-break), whether the three split, each seat's vote mapped back to the provider, and
   where the verdict came from.

Reuse what exists: `status::projection`, `resolver`, `live`, and the cockpit's snapshot. Do not
compute an outcome or usage a second way.

## Done when

- Tests in `tests/cockpit.rs` style draw a frame from a scratch layout: a row with model, policy
  and tokens; a seat row with its role; a panel with a split and a tie-break verdict. Each fails
  with its part removed; say so. fmt, clippy over the workspace, `cargo test --lib` and the
  cockpit tests are clean. `COCKPIT.md`: one paragraph. `IMPLEMENTATION.md`: one paragraph.
- The row still fits an 80-column terminal: say what it drops first when narrow.

## Outcome
