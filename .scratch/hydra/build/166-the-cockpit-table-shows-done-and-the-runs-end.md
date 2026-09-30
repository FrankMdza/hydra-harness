# 166 - The cockpit table shows done and the run's end

Type: build
Status: claimed
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
