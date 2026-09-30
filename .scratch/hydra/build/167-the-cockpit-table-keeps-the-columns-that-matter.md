# 167 - The cockpit table keeps the columns that matter

Type: build
Status: resolved
Blocked by: 166

## What

Found by build 166. `src/cockpit/app.rs` `row_text` lays out seven fixed-width columns (id 8,
class 20, grade 6, provider 12, model 12, status 15, run 8) and then `done` in what is left. With
the chat beside the cockpit at 120 columns the table panel is 52 wide, so only id, class, grade
and part of provider show: never the status, never `done`. The human's redesign puts status,
assignment and definition of done first.

## Do

Tests first in `tests/cockpit.rs`, drawing into a `Buffer` as the other table tests do, each shown
failing before the change.

1. Columns get a priority: id, status, done, provider, model, run, grade, class. A column is shown
   only when every column of higher priority fits; the lowest ones drop first. The display order
   stays readable: id, status, provider, model, run, grade, class, then `done` last taking what
   is left, with a floor of 12 characters for `done` when it is shown.
2. `class` shrinks to 12 before it is dropped.
3. At 52 inner columns the table shows at least id, status and `done`. At the full width it shows
   all eight as today. Test both widths, and one in between.
4. The header follows the same columns as the rows.
5. `IMPLEMENTATION.md`: one sentence in the cockpit passage.

## Done when

- Each new test failed before the change; fmt, clippy, `cargo test --lib` and `cargo test --test
  cockpit` pass.
- The report shows the table at 52, 80 and 150 inner columns.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f2c6-c137 under `standard`. The session
applied it as written. A `Column` enum and `columns(width)` replace the fixed widths: priority id,
status, `done` (floor 12), provider, model, run, grade, class (20, then 12); the first column that
fits at no width ends the list, and the rest keep display order. Cells no longer overflow their
width. Four tests; fmt and clippy clean, `--test cockpit` 24 and `--lib` 220 on master.

At 52 inner columns the table shows id, status, provider and `done`; at 80 it drops class; at 150
all eight. Build 166's paragraph said 50 inside at 120 and the ticket said 52: same four columns
at either. Nobody has seen it in a real terminal yet.

