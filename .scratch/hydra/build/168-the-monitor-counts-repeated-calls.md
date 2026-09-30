# 168 - The monitor counts repeated calls

Type: build
Status: claimed
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
