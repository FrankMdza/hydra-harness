# 303 - A plan proposal row carries whether it was chosen and what became of its tasks

Type: build
Status: open
Blocked by:
Opened: 2026-10-13, by the session, from the map 118 panel (`118-synthesis.md`, question 1),
where the GPT seat read `labels.rs` and found that a plan proposer's row is a role label and
nothing else, so no training row for a planner can be counted today. All three seats agreed the
join is the first thing to build and that nothing trains until the rows can be counted.

## What

`crates/grader/src/labels.rs::labels_for_goal` gives every run a label row. A task's builder run
gets `task_id`, `class`, `grade`, `human_verdict` and the task text. A plan proposer's run gets
`class: "plan_proposer"` with the goal text and nulls everywhere else, whether its plan was the
one chosen or one of the rejected drafts, and whatever its tasks later did. The goal log holds
both facts: `panel_verdict` carries `chosen` (`crates/puppets/src/write.rs` line 605), and each
task's `human_result` is `applied`, `fixed` or `rejected` after the verdict event. The join is
not written, so the one row the orchestrator-model question needs, "this proposal, chosen or
not, and these outcomes", does not exist.

## Do

1. Read `crates/puppets/src/write.rs` for how a plan is recorded (`plan`, `panel_verdict`,
   `chosen`) and how a task reaches `human_result`; read `labels_for_goal` whole; read
   `crates/puppets/tests/grader_labels.rs`, the test that keeps `labels.rs` equal to the Python
   it was ported from over 147 recorded logs, and say in the Outcome how you kept that test
   meaningful (a golden file regenerated and diffed by hand, or an equality over the fields
   that existed before, not a deleted assertion).
2. Extend the `plan_proposer` row with two fields, kept apart as the panel asked:
   `chosen`, `true` for the proposal the panel or the human picked, `false` for the others,
   `null` when no verdict was recorded; and `outcome`, `null` unless `chosen` is `true`, then a
   dict with the counts `applied`, `fixed`, `rejected` and `pending` over the plan's tasks, read
   from the task fold. A preference label and an outcome label on the same row, never merged.
3. `hydra-grader labels count` (or the nearest existing subcommand; read `main.rs` and the
   `*_cli.rs` files before adding one): over every goal log under the puppets home, print one
   line per `class` with the number of rows, and for `plan_proposer` the split chosen, not
   chosen, no verdict. This is the number the human asked for and nobody could produce.
4. Run it on this machine's real puppets home and put the printed counts in the Outcome. Two
   goals and eleven tasks were visible on 2026-10-13 (build 301's Outcome); the count will be
   small, and small is the finding.
5. Amend `IMPLEMENTATION.md` where it describes `labels.rs` (around line 4091) in the same
   change.

## Done means

A plan proposer's label row says whether it was chosen and, when it was, how its tasks ended;
`hydra-grader` prints rows per class; the equivalence test still guards the fields it guarded;
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and
`cargo test -p hydra-grader -p hydra-puppets` green.

## Outcome
