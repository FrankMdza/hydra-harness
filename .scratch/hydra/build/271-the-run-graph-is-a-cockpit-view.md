# 271 - The run graph is a cockpit view

Type: build
Status: resolved
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

Built 2026-10-04 by an Opus run (`01a106b4-a233-74f1`), reviewed and applied by the session.

`src/cockpit/graph.rs` draws what `hydra-graph` drew: a row per event in record order with its
`seq` and type, each call's result on the next row under it, refused calls marked, flagged
lines hiding their input and output. `snapshot.rs` makes the detail's calls and the graph in one
pass (`open_run_of`), so the record still has one reader. `c` toggles it on the selected run, a
full page with the usual scroll and `Esc`; help and `COCKPIT.md` have the row.

Three tests in `tests/cockpit.rs` compare rows, nesting, tools, results and roles with the
Python's output on fixtures 2, 8, 11 and 12, recorded once.

**Assumed and false:** the plugin has no fixture records, so the reference is the Python run on
`hydra show` of the repo's own fixtures. The Python prints the replayed decision
(`allow hydra_policy ...`) where the cockpit, which has no policy set, reads the refusal off the
result, so fixture 2 shows `refused protected-path`. The header's provider and policy come from
the run's card, empty in the Python. Not tried in a real terminal; the graph has no cap on events.
