# 293 - The plan board in the cockpit: who has what, before and after launch

Type: build
Status: claimed
Blocked by:

## What

Frank, 2026-10-04: the goal table (`t`) should let him understand who is assigned to what before
he launches a plan, and follow each agent after. Today it is one table of rows.

1. The goal page becomes a board of cards, one per task: the task's title, the provider and
   model assigned, its grade and whether a launch approval is needed and given (build 264's
   fields), and its state (planned, waiting approval, live, done, failed). Before launch, every
   card shows its assignment, so the plan can be read whole; a task with no assignment says so.
2. Enter on a card opens that task's detail as a page: the full task text the agent was or will
   be given, then, once it has a run, the run's answer (`result_text`, marked as agent text per
   map 105), its outcome, tokens, and `c` for the run graph of build 271.
3. Cards flow in columns to the terminal's width (one column under 100 cells), scroll like the
   other pages, and keep the existing keys; `t` still toggles the page.
4. No new reader: everything comes from the snapshot the cockpit already builds (the goal fold,
   `hydra-puppets show`, the run records).

## Done when

- Tests in `tests/cockpit.rs` on fixture goals: a planned goal shows every assignment before any
  run exists; a card with a finished run opens its task and its answer; a narrow terminal draws
  one column. fmt, clippy, `cargo test --lib`, `tests/cockpit.rs` and `tests/stop.rs` clean.
  `COCKPIT.md` and one paragraph of `IMPLEMENTATION.md`.

## Outcome
