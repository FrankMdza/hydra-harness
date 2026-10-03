# 260 - The walk and the outcome read a harvested `codex` error

Type: build
Status: open
Blocked by:

## What

Build 259 left two ends, both on records no interactive `codex` has written yet:

1. `src/walk.rs`: a harvested `codex` record that derives `rate_limited` carries a non-null
   `run.result` shaped as an `event_msg` `task_complete`, and the walk's rule for `turn.failed`
   does not know that shape, so `hydra doctor --walk` would report "expected completed, got
   rate_limited". Teach the rule the session shape.
2. `src/outcome.rs` `codex_turn_failed`: a harvested session whose last turn ended on a
   non-limit error (`codex_error_info` `other`, `cyber_policy`) reads `completed` even when the
   TUI exited nonzero. It should read `failed`, as the streamed `turn.failed` does.

## Done when

- Tests on synthetic session lines for both; each fails with its change reverted.
  `hydra doctor --walk` on master stays clean.

## Outcome
