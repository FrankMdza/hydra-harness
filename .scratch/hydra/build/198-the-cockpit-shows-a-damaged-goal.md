# 198 - The cockpit shows a damaged goal

Type: build
Status: open
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
