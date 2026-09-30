# 168 - The monitor counts repeated calls

Type: build
Status: resolved
Blocked by: 162

## What

The human's "Cortana" idea asks for a watch on runs that loop on the same tool call. Map 85 and
the plan closed Cortana as a process, but a loop is a feature the prefix monitor of build 162 can
read without stopping anything. `integrations/grader/monitor.py` counts tool calls, refusals and
distinct tools in a prefix; it does not see the same call made again and again.

## Do

Tests first in `integrations/grader/tests/test_monitor.py`, on synthetic records, each shown
failing before the change. Stdlib only.

1. `Prefix` keeps, per tool call, a key of the tool name plus a stable hash of its input (the
   `input` of a `claude` `tool_use`, the args of a `cursor-agent` `tool_call`, the `command` of a
   `codex` `command_execution`, the `tool_info` args of an `agy` tool step; read the shapes
   `monitor.py` already reads, do not invent new ones). A call whose input is missing keys on the
   name alone.
2. Two new features: `repeated_calls_so_far`, the number of calls whose key was already seen in
   the prefix, and `longest_repeat_run`, the longest streak of consecutive calls with the same
   key. Both are scaled like the other counts, and `features` prints them.
3. `integrations/grader/README.md`: two lines in `## The monitor`.

## Done when

- Each new test failed before the change; the grader suite passes.
- The report gives the train commands for the real home, since this run cannot read it.

## Outcome

Resolved 2026-09-30. Built by `claude` sonnet on run 01a0f2c9-9881 under `standard`. `Prefix`
keys each call on its tool name and a sha256 prefix of its input (`claude` `tool_use.input`,
`cursor-agent` `*ToolCall.args`, `codex` `command_execution.command`, `agy`
`tool_info.parameters`, which the session confirmed on a real `agy` record), and counts
`repeated_calls_so_far` and `longest_repeat_run`. Nine tests.

The session measured on the real dataset, 576 rows, test from 2026-09-23. Repeats are common:
27 of 233 `claude` runs, 28 of 199 `cursor-agent`, 34 of 71 `agy`, 10 of 73 `codex`; the longest
streak is 9 identical `claude` calls. In the fitted vector they made the monitor worse: outcome
AUC 0.838, 0.860, 0.880 at 0.1, 0.25, 0.5 against 0.856, 0.867, 0.881 without, and verdict lost
to the provider and model bar at 0.1 and 0.25. So the session kept them in `features` output and
took them out of `raw_features` and `SCALE_NAMES`; the models in `grader/models/monitor/` were
retrained without them. The grader suite passes, 116 tests.

The counts are what a loop view would show; as a failure predictor they add nothing yet.

