# 205 - One pass for the mask, and a review bound to its run

Type: build
Status: claimed
Blocked by:

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of builds 203 (`ba906ef`) and 202
(`0cfa47f`). It found no panic in the new scans, no URL bypass, and `repair_removed_count` equal
to both helpers. Two findings are mechanical and are here. The rest are about what the mask
should cover at all, which is map ticket 92; do not widen or narrow the patterns in this ticket.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. **Quadratic masking** (`src/cockpit/snapshot.rs` near 1024, 835, 1127). `enclosing_quote`
   scans from the start of the text for every match, and the text is masked before it is cut to
   400 characters; the whole-arguments path has no bound at all. A 262,140-character row of
   `KEY=x ` costs about 5.7 billion steps, a 1 MB arguments object about 50 billion, and the
   snapshot reader stalls. Track the quote state in the one forward pass `masked` already makes,
   and bound the input the mask reads to what can reach the screen. The test times a 1 MB input
   and fails over a budget you state.
2. **A review confirmation follows the task to another run** (`src/cockpit/app.rs` near 324,
   1311, 1334). The confirmation holds `(goal, task)`. Open it for task `t` on run `A`, let a
   snapshot give `t` run `B` with the goal id unchanged, press `y`: `puppets.py review` reads the
   task again and reviews `B`. Hold the run id in the confirmation, show it, and at `y` refuse
   with a message when the task's run is no longer that one.
3. `IMPLEMENTATION.md` near 3387 says the raw view behind `r` masks nothing, and
   `src/cockpit/snapshot.rs` near 1127 masks each raw line. Read both and make the sentence say
   what the code does.

## Done when

- Each test failed before the change; fmt, clippy over the workspace, `cargo test --lib` and
  `cargo test --test cockpit` pass.
