# 271 - The run graph is a cockpit view

Type: build
Status: claimed
Blocked by:

## What

Map 104, D2: the run graph the human uses to study runs, today `integrations/herdr/bin/hydra-graph`
(Python, inside the frozen Herdr plugin), becomes a view inside the cockpit on the selected run.
Read `hydra-graph` and its tests to learn what it draws (tool calls over time, by kind, with
refusals and results) and draw the same from the cockpit's snapshot of that run's normalized
events, with no new reader of the record. A key opens it on the selected run, full page, with the
usual scroll and `Esc`; pick one that is free in `src/cockpit/keys.rs` and add it to the help and
`COCKPIT.md`. The Python graph's output on the plugin's own fixtures is the reference: the test
draws the same runs and checks the same tool calls, refusals and order appear.

## Done when

- Tests in `tests/cockpit.rs` from fixture records; fmt, clippy over the workspace, `cargo test
  --lib`, `tests/cockpit.rs` and `tests/stop.rs` clean. `IMPLEMENTATION.md`: one paragraph.

## Outcome
