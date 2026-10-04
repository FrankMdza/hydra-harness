# 263 - The cockpit opens the goal's table full page

Type: build
Status: claimed
Blocked by:

## What

Asked by the human on 2026-10-03: the assignment table a Master of Puppets goal is approved on
should be readable in the cockpit, as a popup or its own tab. Today the Puppets view draws the
goal's rows in a narrow pane (`src/cockpit/app.rs`, `Column`: id, status, provider, model, run,
grade, class, done) beside the lanes, and lacks what the human approves on.

1. A key, `t`, opens the shown goal's table as a full page, the way `Enter` opens a run (build
   246's full-page view: `Esc` closes, `PageUp`/`PageDown`/`g`/`G` scroll, `Left` goes back).
   `t` again or `Esc` returns. Without a goal it says so in the status line and opens nothing.
2. The page shows, per task: id, the task's text (wrapped), class, grade, provider, model,
   policy, deps, status, run id, and whether its launch needed and got `approve --launch`. Above
   the rows: the goal text, the table's revision, whether `approve --table` covers that revision,
   whether the plan was approved, and the panel verdict line build 256 draws. Read all of it from
   the goal log through `cockpit/goal.rs`'s fold; add fields there, never a second reader.
3. The help overlay (`?`) and `COCKPIT.md`'s key table gain the `t` row.
4. Tests in `tests/cockpit.rs` style from a scratch goal log: the page shows a task's policy and
   deps; an edit after `approve --table` shows the table as not approved at the new revision;
   `t` with no goal opens nothing. Each fails with its change removed; say so.

## Done when

- fmt, clippy over the workspace, `cargo test --lib`, `tests/cockpit.rs` and `tests/stop.rs` are
  clean (build 256 broke `stop.rs` by running only the cockpit tests). `IMPLEMENTATION.md`: one
  paragraph.

## Outcome
