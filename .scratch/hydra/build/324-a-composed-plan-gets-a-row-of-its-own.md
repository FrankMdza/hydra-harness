# 324 - A composed plan gets a row of its own, `plan_composed`, chosen and carrying the goal's outcome

Type: build
Status: resolved
Blocked by: 323
Opened: 2026-10-15, by the session, from map 122's answer, question 1. When the panel's verdict
is `composed`, every proposer row is `chosen: false` and the goal's task verdicts land on no row:
on `ea69da1f`, six verdicts (three applied, three fixed) credited to nobody. The human decided
that no proposer is credited and that the composed plan, the one that actually ran, gets its own
row.

## What

In `crates/grader/src/labels.rs`, a goal whose verdict is `composed` yields one extra row,
`class: "plan_composed"`, `chosen: true`, with the goal text, the composed plan's JSON (read from
the path the verdict recorded, `plans/composed.md` or whatever `fold.rs` holds), the
`judge_run_ids` of the verdict (empty when the session chose a file, as on `ea69da1f`), and the
same `tally` and `outcome` fields 303 gives a chosen proposer. Proposer rows of that goal keep
`chosen: false` (or `null` after 323). `labels count` prints "composed" as its own line.

The row says who composed: `by: "panel"` when `judge_run_ids` is non-empty, `by: "session"`
when it is empty, so a reader of the corpus can keep the hand-written case apart. The 118 panel
noted the composed plan's author read the proposals, so its input was richer than a proposer's;
that is what the `by` field is for.

## Do

1. After 323 is in master (this ticket touches the same file).
2. The row, the count line, the equivalence tests extended as 303 and 323 did.
3. A synthetic goal log in the tests with a `composed` verdict and task outcomes; assert the row
   and that the proposers stay unchosen.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the labels are described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Built on 2026-10-15 by a Hydra run.

`labels_for_goal` gives a goal whose verdict chose `composed` one more row, `class:
"plan_composed"`, after the proposer rows. Labels are keyed by run id and no run wrote a composed
plan, so the key is `composed:<goal id>`. `collect` joins labels to the runs it scans by run id,
so the row never reaches the dataset, and it shows up in `labels count` and in nothing else. Its
fields are the goal text, `by`, `chosen: true`, the verdict's `judge_run_ids` and `path`, `plan`
and `outcome`, the same tally a chosen proposer carries. `plan` is the first `json` fence of the
file at `path`, which is how `panel.rs` writes `plans/composed.md`, or the whole file when it is
bare JSON. It is `null` when the file is gone or neither parses. A missing file is no error,
because the recorded corpus names temporary homes that no longer exist and one unreadable plan
should not stop `labels count` for every goal. Proposer rows are untouched and stay `false`.
`labels count` prints `plan_composed\tN\tby panel P\tby session S`.

Tests: a unit test in `labels.rs` (no row without `composed`, `plan` null for a missing file,
bare JSON read, `by` from `judge_run_ids`), and
`a_composed_plan_gets_a_chosen_row_and_the_proposers_stay_unchosen` in
`crates/puppets/tests/grader_labels.rs`, which writes a goal log and a `composed.md` into a
scratch home and folds them through the real `hydra-puppets show`. Two proposers, three tasks
(applied, fixed, no verdict), once with judge run ids and once without. It asserts the whole row,
both proposers `false` with `outcome` null, and the `labels count` output, including the
`plan_judge` line that only the panel case has. Both equivalence tests drop `plan_composed` rows
before the byte comparison with the Python, as 303 and 323 dropped fields. `grader_labels.rs`
asserts every such row is chosen and that at least one exists. The grader's differential test
asserts the key starts with `composed:`. The recorded corpus now prints:

    labels: 147 goal logs equal with 293 labels, 10 of them with a human verdict; 17 logs refused by both; proposers 4 chosen and 6 not; 2 composed

293 was 291: h-009 (a panel, three judges, its `composed.md` in a temporary directory long gone,
so `plan` is null) and h-015 (`choose composed` typed as a provider, `path` null, `by:
session`).

What the ticket assumed that is false. A verdict chosen by the session is not always a file: on
h-015 the human typed `composed` as a provider name, so `path` is null and there is no plan to
read. The row still exists with `plan: null`. The ticket also names "the same `tally` and
`outcome` fields 303 gives a chosen proposer", but 303 gives only `outcome`, which holds the
tally, so the row has `outcome` and no `tally`. And build 323's Outcome says it amended
`IMPLEMENTATION.md`, but `has_draft` appeared nowhere in that file; this change adds a paragraph
for it beside 324's.

Not measured: `labels count` against this machine's real puppets home, which is outside the
worktree this run may read. On `ea69da1f` the expected line is `plan_composed 1 by panel 0 by
session 1` with outcome applied 3, fixed 3, as the ticket describes; the session can confirm it.

The gate refused two calls, and neither was retried the same way: an inline `python3 -I -c` to
read two fixtures (`hydra/floor`, could not parse), replaced by `grep`, and a `Read` of a
background task's output under `/private/tmp` (`hydra/out-of-scope`), replaced by running the
checks in the foreground. One nextest run failed `cockpit::ticket264_a_late_show_leaves_the_approvals_unknown`
while a second cargo run competed for the CPU. It passed alone, and so did the next full run.

    cargo fmt --all --check: exit 0, silent
    cargo clippy --workspace --all-targets -- -D warnings: Finished `dev` profile in 0.79s
    cargo nextest run --release --workspace: Summary [85.031s] 1408 tests run: 1408 passed, 4 skipped
    cargo test --release --workspace --doc: test result: ok. 0 passed; 0 failed; 0 ignored

The session ran `hydra-grader labels count` on the real puppets home after 323 and 324 landed:
`plan_proposer 7, chosen 1, not chosen 5, no verdict 0, no draft 1` and `plan_composed 1, by panel 0,
by session 1`, which is what the Outcome above predicted for `ea69da1f`.
