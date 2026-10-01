# 198 - The cockpit shows a damaged goal

Type: build
Status: resolved
Blocked by: 197

## What

Map ticket 91: nobody folds around a bad line in a goal log. Build 197 made the helper's readers
and writers stop on one. The cockpit has its own fold, `src/cockpit/goal.rs`, which skips a bad
line, counts it in `Goal::skipped`, and prints `(N line(s) skipped)` in the goal heading
(`src/cockpit/app.rs` near 1734). It also has no `repair` event in its revision count.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. A newline-terminated line that is blank or not JSON marks the goal damaged. The goal's
   heading says `damaged at line <n>: run puppets.py repair <id>` (or `hydra-puppets repair`
   once 196 has landed, whichever the tree has), in the warning style, and the cockpit shows no
   task table and offers no `a`, `v` or `m` for that goal: the helper would refuse them anyway.
   A line that is JSON but lacks a field the fold reads keeps today's skip count.
2. `apply` handles the `repair` event as the helper's fold does: `revision += removed + 1`.
3. `IMPLEMENTATION.md`: the cockpit passage that describes the skip count.

## Done when

- The tests failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f5d9` (cursor-agent composer-2.5, `standard`, 2.3M tokens), diff
read whole by the session, applied to master and corrected there.

Built: the cockpit's fold stops at the first newline-terminated line that is blank or is not JSON
and records its number; the goal pane shows `damaged at line <n>: run puppets.py repair <id>` in
yellow and no table; the reader skips `pending` for a damaged goal; `v`, and `a` on a Master of
Puppets step, answer with that message. JSON that lacks a field still counts as skipped.

The ticket was wrong in one place and the session corrected the code against it: it asked for `a`
and `m` to be refused outright. `a` on a seat request runs `hydra approve`, and `m` runs `hydra
stop`; neither touches the goal log, so both still work on a damaged goal. Two more corrections:
`Goal::revision` counted only `repair` events while its comment said it was the helper's
revision, and it now counts `task` and `assignment` too; and a line with a BOM, which the helper
reads, was damage here.

Measured on master: fmt and clippy clean over the workspace, `cargo test --lib`, `cargo test
--test cockpit`.

Left open: `serde_json` refuses `NaN` and `Infinity`, which Python's `json.loads` accepts, so a
log line holding one is damage in the cockpit and good in the helper. The run did not see its
tests fail at run time, only fail to compile.

