# 175 - A stopped codex run, and a bounded rescan

Type: build
Status: resolved
Blocked by: 173

## What

A read-only review by `codex` on 2026-09-30 of build 173 (`28224bd`) confirmed its three fixes
and found one defect and three risks, from reading the code:

1. **Defect.** `src/walk.rs`: the `stopped` exemption sat after the `codex` `turn.failed` rule, so
   a `codex` run stopped as it wrote a non-quota `turn.failed` got "expected failed, got stopped"
   from `hydra doctor --walk`.
2. **Risk.** `src/stop.rs` `all_marked`: the mark-and-rescan loop had no bound; a launcher that
   keeps starting new root runs could hold `--all` in its marking phase.
3. **Risk, not fixed.** A group id reused between two polls of the grace could take the SIGKILL.
   The loop stops as soon as the group is empty, so the window is one 100 ms poll.
4. **Risk, not fixed.** A new `hydra run` whose parent is an old, stopped run is killed at once by
   the leftover marker. Sessions mint fresh ids, and a delegate's parent is live by
   construction, so no path the design uses reaches it.

## Do

The session does 1 and 2 itself: the exemption moves to right after the outcome is derived, and
`all_marked` makes at most 10 passes. A doctor test for 1 on a synthetic `codex` record.

## Outcome

Resolved 2026-09-30 by the session. The `stopped` exemption in `walk.rs` now runs right after the
outcome is derived, ahead of every rule that reads a terminator; `walk_passes_a_stopped_codex_run_that_ended_on_turn_failed`
failed before the move and passes after. `stop::all_marked` makes at most `MAX_PASSES` (10)
mark-and-rescan passes, then signals what it found. Risks 3 and 4 stay as written above. fmt and
clippy clean, `--lib` 220, `doctor` 5, `stop` 6, `record_contract` 20.

