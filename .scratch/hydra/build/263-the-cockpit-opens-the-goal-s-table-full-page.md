# 263 - The cockpit opens the goal's table full page

Type: build
Status: resolved
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

Resolved 2026-10-03 by run `01a1044a-13b1-7580` (`cursor-agent` composer-2.5, `standard`, 3.58M
tokens), with three fixes by the session.

Built: `t` opens the shown goal's table as a full page (`Esc`, `t` or `Left` return; the usual
scroll keys), with the goal, the revision, whether the plan and the table are approved, the
panel lines of build 256, and per task its text, class, grade, provider, model, policy, deps,
status, run and launch approval. `cockpit/goal.rs` folds task text, deps, policy and the
`approval` events. Three tests in `tests/cockpit.rs`.

What the session changed: a `task` event without `text` made the fold drop the whole event, so a
task from an older log would have vanished from the table; it now shows with no text. A task
whose launch had run read "needed, not approved", because the run consumes the approval; a
consumed approval now counts as given. The agent did not check its tests against a reverted
change; the session did for one: with the policy read removed,
`ticket263_goal_table_page_shows_task_policy_and_deps` fails, and restored it passes.

Left for build 264: the fold of approvals (voided by a later revision, consumed by a run) is a
second copy of `hydra-puppets`' own, which `hydra-puppets show <goal>` already prints.
