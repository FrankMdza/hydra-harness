# 298 - The cockpit shows the task text `hydra-puppets` composes, not a copy of its wording

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from the review of build 293.

## What

Build 293's task detail shows, for a task with no run yet, "the full task text the agent will be
given". The builder rebuilt that text in `src/cockpit/app/pages.rs` from the plan's `text` and
`done` plus the launch prompt's own sentences ("Done means", the "Not confirmed" paragraph),
which live in `crates/puppets/src/launch.rs::worker_task_text`. The hydra crate cannot import
the puppets crate, so there are two copies of one string and the cockpit's will drift the first
time the launch prompt changes. The cockpit already asks `hydra-puppets show <goal>` every ten
seconds (build 264); that is where the composed text belongs.

## Do

1. `hydra-puppets show` prints, per task, a `task_text` field: exactly what
   `worker_task_text` would hand the builder for that task today, composed by the same function.
   A task that already has a run keeps printing what it has; the cockpit prefers the record's
   `task` when a run exists, as 293 built it.
2. `src/cockpit/goal.rs` reads `task_text` into `TaskRow`; the detail in `pages.rs` shows it and
   the rebuilt string goes away. An older `show` without the field leaves the detail saying
   "task text not available from this hydra-puppets", never a guess.
3. One sentence in `COCKPIT.md` and in `IMPLEMENTATION.md` where 293's paragraph says the text is
   rebuilt.

## Done when

- A test in `crates/puppets/` that `show` prints `task_text` equal to `worker_task_text`'s
  output for a fixture goal; a test in `tests/cockpit.rs` that the detail shows it and that a
  `show` without the field shows the "not available" line; no string from `launch.rs` remains in
  `src/cockpit/`. fmt, clippy, `cargo test -p hydra-puppets`, `cargo test --lib`,
  `cargo test --test cockpit`.

## Outcome

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra run `01a13433-9ca3-7991-8b9b-92428da157f1` under
`standard`, 2.73M tokens, applied to master on 2026-10-13 after the diff was read whole.
`hydra-puppets show` prints `task_text` per task, composed by `worker_task_text` itself, now
`pub(crate)`; `src/cockpit/goal.rs` reads it in `parse_show` and clears it when the helper's
revision is stale; the detail prefers the record's task, then `task_text`, then the "not
available" line, and the rebuilt string is gone from `src/cockpit/`. The differential suite
against the recorded Python strips only the new field and documents why. A unit test in `read.rs`
checks `show` against the composer on a three-task fixture; `tests/cockpit.rs` checks the detail,
the older-helper line and the stale-revision case. fmt, clippy, 91 puppets tests, 256 library
and 147 cockpit tests green on master. One thing the builder found: historical `show` fixtures
hold duplicate JSON keys where a numeric and a string task id print the same, which the pyjson
dict preserves and the new code keeps.
