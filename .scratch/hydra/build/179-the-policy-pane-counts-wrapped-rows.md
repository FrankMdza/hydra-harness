# 179 - The policy pane counts wrapped rows

Type: build
Status: resolved
Blocked by: 178

## What

A read-only review by `codex` on 2026-09-30 of builds 174 and 176 found one defect, by row
arithmetic: `src/cockpit/app.rs` near 714 and 719 sizes the policy pane's page as one row per
run. At 80x30 with no mascot and the header live, a run on another sha is a 78-character line in
a 70-column interior and wraps to two rows, so ten runs need twenty rows: later runs are clipped,
no `N more` line shows, and PageDown and the wheel cannot move because the computed last offset
is 0. A resize to that size does the same.

## Do

A test first in `tests/cockpit.rs` at 80x30 with ten runs on another sha, shown failing before
the change. The page size counts each run's wrapped rows at the pane's inner width and keeps one
row for the `more` line; the last run is reachable with PageDown and with the wheel. Or cut each
run to one line instead of wrapping, and say which you chose and why.

## Done when

- The test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f317-da64 under `standard`. The session
applied it as written. The pane counts each run's wrapped rows at its inner width with a new
`wrapped_rows`, checked against what `Paragraph` renders at seven widths; the page holds whole
runs and keeps one row for `N more`; the last page starts where every remaining run fits
(`Hits::policy_last`), and PageDown and the wheel stop there. Runs wrap rather than cut, because
the `differs` sentence is the pane's reason to exist. Two tests, each failing first; fmt and
clippy clean, `--test cockpit` 49, `--lib` 222 on master. Wide characters untested; build 180 cuts
the detail by display width.

