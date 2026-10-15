# 324 - A composed plan gets a row of its own, `plan_composed`, chosen and carrying the goal's outcome

Type: build
Status: open
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
