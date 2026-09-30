# 166 - The cockpit table shows done and the run's end

Type: build
Status: resolved
Blocked by:

## What

First step of the human's cockpit redesign (30 sep 2026): the task table should show each
task's definition of done and whether its run failed. `puppets.py` stores `done` on each task
(`integrations/puppets/puppets.py:415`, and `:1711` where `board` prints it), but
`src/cockpit/goal.rs` `TaskRow` does not read it. The status column today is `fold`'s
(`pending`, `assigned`, `running`, `ended`, or the human's verdict) and never says a run failed,
though `src/cockpit/snapshot.rs` already derives each run's outcome for the kanban.

## Do

Tests first in `tests/cockpit.rs` (or the unit tests in `goal.rs`), each shown failing before the
change.

1. `TaskRow` gains `done: Option<String>`, folded from the task event's `done` field by the same
   rule `fold` uses (a later `task` event for the same id replaces it).
2. The table (`src/cockpit/app.rs`, near line 559) gains a `done` column, cut to the width left,
   and the detail view shows it whole.
3. When a row's status is `ended` and the snapshot knows its run, the status cell reads the run's
   outcome instead (`completed`, `failed`, `gated`, `rate_limited`, ...). A human verdict still
   wins over the outcome, as today.
4. Nothing new is written anywhere. The cockpit stays read-only apart from the one approval it
   already makes.
5. `IMPLEMENTATION.md`: amend the cockpit passage (grep `cockpit`) with the two columns.

## Done when

- Each new test failed before the change; `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test --lib` and `cargo test --test cockpit` pass.
- The report says how the table looks at 120 columns with a long `done`.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f2c1-7090 under `standard`. The session
applied it as written. `TaskRow.done` folds from the task event; `status_of` shows the run's
outcome for an `ended` row the snapshot knows, a human verdict still wins; the status column
widened to 15 for `skill_collision`; a `done` column cut to the panel; the run's detail shows
`task` and `done` in full. One new test, one extended; fmt and clippy clean, `--test cockpit` 20,
`--lib` 220 on master. `IMPLEMENTATION.md` amended.

What the ticket assumed that turned out false: with the chat beside it, the table panel is 52
columns wide at 120, and the row ends one character into `model`, so neither the status nor
`done` shows. That was already true of status before this ticket. Build 167 orders the columns by
priority. A row with no run still has no detail view, so its `done` is only in the table.

