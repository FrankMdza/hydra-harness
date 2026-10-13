# 303 - A plan proposal row carries whether it was chosen and what became of its tasks

Type: build
Status: resolved
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

Built on 2026-10-13 by a Hydra run.

`labels_for_goal` now gives a `plan_proposer` row two more fields, after `text`: `chosen` and
`outcome`. No other class gets either. It reads `chosen_plan` from the state `show` prints, which
the fold sets from the last `panel_verdict`. `chosen` is `null` with no verdict, and also with a
verdict whose `chosen` is null (the panel failed to agree). Otherwise `true` or `false`.
`write.rs::choose` and `panel.rs::choose_judge` record a verdict three ways, so the join matches
three ways. With a `path`, the proposal whose own `path` is that file is chosen: the panel copies
the proposer's `path`, and `choose <file>` records the file's absolute path. With no path, from
`choose <provider>`, the proposal is matched by provider. A composed plan's path is
`plans/composed.md`, which no proposer wrote, so every proposer of a composed goal is `false`.
`outcome` is a dict `applied`, `fixed`, `rejected`, `pending` over the goal's tasks, deduplicated
by id as the fold keeps them, and `pending` is any `human_result` that is not one of the three.

`hydra-grader labels count` is new, in `labels_cli.rs`, and runs on the deep stack like `judge`.
No existing subcommand counted labels. `collect_cli.rs` owns the four `grader.py` commands and
mirrors the Python's argparse, so I gave it its own file.

Keeping the equivalence test meaningful: neither the 147 recorded logs nor `labels.json` were
regenerated. Both tests (`crates/puppets/tests/grader_labels.rs` and the grader's
`differential/collect.rs`, which this ticket did not name and which failed first) now remove
`chosen` and `outcome` from each row and compare the rest byte for byte with the Python's pairs,
so every field the test guarded before is still guarded. Each test also asserts that the two
fields are on every proposer row and on no other row. `grader_labels.rs` counts the corpus's
split and asserts at least one chosen and one not, so the join cannot go dead on the recorded
logs without the test failing. It printed:

    labels: 147 goal logs equal with 291 labels, 10 of them with a human verdict; 17 logs refused by both; proposers 4 chosen and 8 not

Two unit tests in `labels.rs` cover the file match with its tally, no verdict, a failed verdict,
`choose <provider>` and a composed plan.

`./target/release/hydra-grader labels count` on this machine, puppets home `~/hydra/puppets`
(`HYDRA_ROOT` unset, so `$HOME/hydra`), two goal logs:

    rust_off_launch_path	2
    plan_proposer	7	chosen 1	not chosen 6	no verdict 0
    plan_judge	3
    batch_scripting	5
    security_review	1

Seven proposer rows, one labelled chosen, and that one's tasks have no outcome yet. Goal
`a598914f` chose claude's proposal: `outcome` is applied 0, fixed 0, rejected 0, pending 5.
Goal `ea69da1f` chose `composed`, so its three proposers are all `false`, and its six tasks
with verdicts (3 applied, 3 fixed) land on no proposer row. Today there is one usable "chosen
and how it ended" row on this machine, and it is all pending.

What the ticket assumed that is false: that the chosen plan is always one proposer's. On the one
goal with outcomes, the panel composed a plan. Whether a composed plan's outcome is credited to
the proposals it drew from, and how, is a decision and not an implementation. It belongs on the
map, and this ticket does not decide it.

`cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean,
`cargo test -p hydra-grader -p hydra-puppets` green: grader 41 unit, cutover 7, differential 17,
launchd 8, originals 3, system_one 4; puppets 61 and 5 unit, builders 3, differential 18 (1
ignored), frontier 2, grader_labels 1, index 1, sheet_build 1, task_graph 4.
