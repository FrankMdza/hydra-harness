# 269 - The dragon wakes for the orchestrator's agents

Type: build
Status: resolved
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

Resolved 2026-10-03 by run `01a104ec-9311` (`cursor-agent` cursor-grok-4.6-high, the first build
on Grok 4.6 after the human retired composer, `standard`, 4.84M tokens), applied without changes.

Built: `Snapshot::is_session_seat` counts the session, a run whose recorded parent is it, and a
run whose start (line 0's `ts`, now on `RunInfo.started`) is after the session's. The dragon's
heads, flashes and seat count and the `k` lanes use it; `m` still needs a recorded parent. Three
tests in `tests/cockpit.rs`; the agent removed or inverted each branch and saw them fail.

Measured by the session: the whole workspace with this applied, 1,238 passed; the two failures
were the puppets differential test catching build 270's files applied while the suite ran, and
rerun on the settled tree it passes 18 of 18. With the orchestrator live and three agents, the
header reads 4, which the three heads draw all awake. This commit also carries build 270's one
sentence in `IMPLEMENTATION.md`.
