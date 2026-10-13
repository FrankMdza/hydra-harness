# 293 - The plan board in the cockpit: who has what, before and after launch

Type: build
Status: resolved
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

Built by `codex` `gpt-6.1-sol` on Hydra run `01a133fa-e0cf-7653-b5a0-e27f81da38cd` under
`standard`, 2.24M tokens, applied to master on 2026-10-13 after the diff was read whole. The
goal page is a board of cards, one per task, with assignment, grade, launch approval and state
before any run exists; Enter opens the task's text and, once it has a run, the marked answer,
outcome, tokens and `c` for the graph; one column under 100 cells. Three tests in
`tests/cockpit.rs`; 256 library, 146 cockpit and 13 stop tests green on master, fmt and clippy
clean. Two things the review changed or noted. A task whose verdict was `fixed` was mapped to
`failed` on a card without a run record; the session changed the match to `done`. The planned
task text in the detail is rebuilt from the launch prompt's wording ("Done means", "Not
confirmed"), which lives in `crates/puppets/src/launch.rs`; the hydra crate cannot import it, so
the two strings can drift, and a later ticket should hand the cockpit the composed text instead.
The builder reported that the library tests needed `GIT_OBJECT_DIRECTORY` and
`GIT_ALTERNATE_OBJECT_DIRECTORIES` removed from the run's environment, which the env allowlist
hands every run; no test on master depends on that. The board was not driven in a real terminal
by the builder or the session, only through buffers.
