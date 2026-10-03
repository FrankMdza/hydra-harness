# 256 - The cockpit shows the model, the role, the question and the votes

Type: build
Status: resolved
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
   The goal log does not record roles today (found by run `01a103af-c365`, which stopped on it):
   they live in `PANEL` and `TIE_BREAK` in `crates/puppets/src/panel.rs`. Add `judge_roles` and
   `tie_break_roles` to every `panel_verdict` event `panel.rs` writes, in the same order as the
   run ids, and read them from there. An older event without them shows no role. The question a
   seat was asked is its run's task, `run_start.task` in its record; read it from there.
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

Resolved 2026-10-03 by run `01a103b0-ec0d` (`codex` gpt-6.1-sol, `standard`, 8.25M tokens), after a
first run (`01a103af-c365`) stopped because the goal log carried no roles. One fix by the session.

Built: a run row carries id, provider (outside the per-agent view), model, what it said, its goal
task, its role, then policy, tokens and elapsed time; a narrow pane drops elapsed, then tokens,
then policy. The model is the envelope's, else the `init` frame's on `claude`, `cursor-agent` and
`agy`, else `default`. A run's detail opens with its task, labelled `question` for a panel seat,
then role and elapsed. The Puppets view lists each panel: verdict and source, each seat and
tie-break with provider, model, role, vote mapped through `label_map` and reading order, and
`unanimous`, `split` or `split unknown`. `panel.rs` writes `judge_roles` and `tie_break_roles` on
every `panel_verdict`; an older event shows no role. At 80 columns or fewer the three panes stack.
Tests: four in `tests/cockpit.rs`, one in `panel.rs`; the agent removed each part and saw it fail.

What the session changed: `provider_model` read the whole record to find the `init` frame,
whenever the envelope had no model and the file had grown, which on a live `codex` run (no model,
no `init` frame) meant rereading megabytes every tick for nothing. It now reads the first 64 lines
and only for the three providers that write one.

Left as is: the votes shown are parsed again from each seat's answer text by `panel_vote`, since
the event keeps the raw answers but not the parsed choices.

Corrected 2026-10-03: the new row put the model between provider and state, and three assertions
in `tests/stop.rs` still looked for `01996d3c claude stopping`, `live` and `stopped`. The agent ran
only `tests/cockpit.rs`; the session's full suite caught it. They now read `claude stub-1 ...`.
