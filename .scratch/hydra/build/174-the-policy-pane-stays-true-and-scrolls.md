# 174 - The policy pane stays true and scrolls

Type: build
Status: resolved
Blocked by: 172

## What

A read-only review by `codex` on 2026-09-30 of build 171 (`b01ef00`) found two defects in the
cockpit's policy pane, from reading the code:

1. **A stale `current`.** `src/cockpit/app.rs` near line 292, compared near line 654: the pane
   reloads the policy set only after its own `e`. A policy edited outside the cockpit while the
   pane is open leaves every run on the old sha marked `current`, and a broken outside save leaves
   the old valid sha on screen.
2. **Runs cut off.** Near line 675: at 120x24 with no mascot the pane has 19 rows; headings and
   the five policies take ten, so with ten live runs some runs and their marks never show, and
   nothing scrolls the pane.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. On every snapshot refresh while the pane is open, recompute the policy set's sha from the
   files (cheap: five small files), and show the loader's error if it no longer loads. A test
   edits a policy file between two refreshes and expects `differs`.
2. The run list in the pane scrolls: PageUp/PageDown or the wheel over the pane, and a line
   `N more` when runs are cut. A test with ten live runs at 120x24 reaches the last one.
3. `IMPLEMENTATION.md`, the policy pane paragraph.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f307-7744 under `standard`. The session
applied it as written. `set_snapshot` reloads the policy set while the pane is up, so an outside
edit shows as `differs` (or the loader's error) at the next refresh. The run list pages with
PageUp and PageDown, moves one run with the wheel over the pane, and shows `N more, A above and B
below` when cut; the offset clamps to the last page. Two tests, each failing first; fmt and clippy
clean, `--test cockpit` 35, `--lib` 221 on master. Since build 172 the pane has 18 rows inside its
border at 120x24 when a run is live, not 19.

