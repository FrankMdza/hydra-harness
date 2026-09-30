# 178 - The detail shows tool calls

Type: build
Status: open
Blocked by: 177

## What

The human's cockpit redesign asks that expanding an agent shows what it is doing. The detail
pane shows provider, model, policy, outcome, tokens, `result_text` and the last 20 raw record
lines (`src/cockpit/snapshot.rs`, `TAIL_LINES`). Raw lines are provider frames, hard to read.
`src/normalize/` already turns a record into events, `Event::ToolCall` among them
(`src/normalize/mod.rs` near 37 and 127), and since build 172 each normalizer has a `walk(text)`
that runs without the gate replay.

## Do

Tests first in `tests/cockpit.rs` on the fixture records under `tests/fixtures/records/`, each
shown failing before the change.

1. The detail shows the run's last 15 tool calls from `walk` (never `finish`, which replays the
   gate): tool name, a one-line summary of its input cut to the pane (a path, a command, a
   pattern; whatever the `ToolCall` carries), and the decision when the record has one (allowed,
   refused, with the refusal code). Newest last.
2. A key, `r`, toggles back to today's raw tail, so nothing is lost.
3. Live runs reread only when the file grew, as the snapshot already does for usage.
4. `IMPLEMENTATION.md`, the detail paragraph.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` and
  `--test normalize` pass.
