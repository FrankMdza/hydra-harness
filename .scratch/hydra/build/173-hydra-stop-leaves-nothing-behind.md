# 173 - hydra stop leaves nothing behind

Type: build
Status: claimed
Blocked by: 171

## What

A read-only review by `codex` on 2026-09-30 of build 169 (`3bc6327`) found three defects in
`hydra stop`, each derived from the code and not yet reproduced. The review is in the human's
notes; the three are restated here in full.

1. **A descendant can survive.** `src/stop.rs` near line 201: when the recorded pid exits on
   SIGTERM, the grace check sees it gone and skips SIGKILL, and `signal()` rechecks that pid, so a
   descendant in the run's process group that ignores SIGTERM and SIGHUP keeps running while the
   record closes `stopped`.
2. **A seat launching during the stop can escape it.** `src/stop.rs` near line 107 and
   `src/seats.rs` near line 735: a request passes the launcher's `stopped()` check, pauses before
   its record appears, the stop writes the session marker and scans, and the seat then launches
   outside the stop list. `--all` scans even earlier, so a parent can go before this child.
3. **`hydra doctor --walk` rejects a valid stopped record.** `src/walk.rs` near lines 636 and 717:
   the checker skips outcomes rung one does not learn from only when there is no terminator. A
   provider that catches SIGTERM and writes a success terminator gives a record that derives
   `stopped` and then fails the completed/gated comparison.

## Do

Tests first, each shown failing before the change, on the stub provider `tests/stop.rs` already
uses. For 1, a stub whose shell spawns a child that traps TERM and HUP. For 2, drive the launcher's
admission and the stop's scan in the order above with the hooks the code gives you; if the race
cannot be driven deterministically, say so and test the invariant you add instead. For 3, a
synthetic record in the walk tests.

1. After the recorded pid is gone, the stop still sends SIGKILL to the run's process group when
   the child led one and any member remains, checked by process group, not by the recorded pid.
   Never signal a group the run did not own.
2. The launcher and the stop agree through the marker: the launcher checks the marker again after
   the seat's record exists and stops that seat itself if the marker appeared meanwhile, or
   another way you find in the code. Say which and why.
3. `walk.rs` exempts `stopped` from the completed/gated comparison with or without a terminator.
4. `IMPLEMENTATION.md`: amend the `hydra stop` section's gaps.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, and `--test stop`,
  `--test seats`, `--test record_contract`, `--test doctor` pass.
