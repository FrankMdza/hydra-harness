# 269 - The dragon wakes for the orchestrator's agents

Type: build
Status: claimed
Blocked by:

## What

Seen by the human on 2026-10-03: with three runs live in a Master of Puppets session, the
cockpit's dragon kept one head awake ("1 live seat(s), two_asleep"). With `--session`, a seat is
the session's own run or a run whose `parent_run_id` is it (`src/cockpit/mascot.rs` near line
314, the same test in `src/cockpit/app.rs` near lines 525 and 2496). The orchestrator launches
its agents from its own shell with `hydra run`, which records no parent, so none of them counts.

With `--session`, a seat is also any live run that started after the session's run started
(build 268 already keeps start times in the snapshot's record info; reuse them). That set drives
the dragon's heads and flashes, the header's seat count, and the `k` lane-per-agent view's
"only the session and its seats". It does **not** change what `m` may stop (`app.rs` near line
1770): stopping stays limited to runs whose recorded parent is the session.

## Done when

- Tests in `tests/cockpit.rs`: with a session and three live runs started after it with no
  parent, three heads are awake and the header says 3; a live run that started before the
  session is not a seat; `m` on one of the unparented runs is still refused. Each fails with its
  change removed (remove it and run; say so). fmt, clippy over the workspace, `cargo test --lib`,
  `tests/cockpit.rs` and `tests/stop.rs` are clean. `COCKPIT.md`: one sentence.

## Outcome
