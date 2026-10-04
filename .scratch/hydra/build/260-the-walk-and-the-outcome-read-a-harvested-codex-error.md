# 260 - The walk and the outcome read a harvested `codex` error

Type: build
Status: resolved
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

Resolved 2026-10-04 by run `01a10578-7d60` (`cursor-agent` cursor-grok-4.6-high, `standard`,
3.07M tokens), applied without changes.

Built: `codex_turn_failed` treats a harvested `task_complete` with a non-null `error` as a failed
turn, so a last turn on `other` or `cyber_policy` derives `failed`, and `codex_limit_refusal`
reads the limit from `payload.error` on that shape; the walk's failed-turn rule knows the same
shape, so a harvested limit is expected `rate_limited`. Two tests, each failing with its file
reverted to HEAD.

Measured by the session: `hydra doctor --walk` over the real corpus, built in master before and
after, prints the same 12 lines; no harvested `codex` record on this machine ended on such an
error yet, so the change is pinned by the synthetic tests.
