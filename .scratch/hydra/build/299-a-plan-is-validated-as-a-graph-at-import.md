# 299 - A plan is validated as a graph at import: unique ids, deps that resolve, no cycles

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from the map 118 panel (`118-synthesis.md`, question 4),
which found that `crates/puppets/src/write.rs::tasks` checks each task's class and grade and
nothing about the plan as a whole.

## What

`hydra-puppets tasks` imports the chosen plan's JSON array. Today a plan with two tasks of the
same id silently keeps the last (`fold.rs` replaces by id), a `deps` entry naming a task that
does not exist is accepted and never satisfied, and a cycle of deps leaves every task in it
waiting forever. None of these needs a model to catch. The 118 panel named this validator as the
first piece of any regression gate for a planner, and it is useful before any planner exists:
the frontier proposers are large models and still produce such plans.

## Do

1. In `write.rs::tasks`, before any event is written: refuse the import when an id repeats, when
   a `deps` entry names no task in the plan, or when the deps graph has a cycle. The refusal names
   every offence on stderr and writes nothing, so the human fixes the plan or picks another.
2. `paths` are not checked against disk: a plan may name files it will create.
3. One paragraph in `integrations/puppets/README.md` under the event that documents `tasks`, and
   one in `IMPLEMENTATION.md`.

## Done when

- Three tests in `crates/puppets/`: a duplicate id, an unknown dep and a two-task cycle each
  refuse with the offence named and leave the goal log unchanged; a valid plan still imports.
  fmt, clippy, `cargo test -p hydra-puppets`.

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra run `01a1342d-0a38-7f12-9347-e448463988ae` under
`standard`, 607k tokens, applied to master on 2026-10-13 after the diff was read whole.
`task_graph_offences` in `write.rs` walks the deps iteratively, reports every duplicate id,
unknown dep and cyclic task, and `tasks` exits 1 without writing when any is found; ids compare
with the fold's key equality, so `1` and `1.0` are one id, which the builder found and the test
pins. Four tests in `crates/puppets/tests/task_graph.rs` drive the built binary on a seeded goal
log and check the log is byte-identical after a refusal. fmt, clippy and `cargo test -p
hydra-puppets` green on master. Nothing the ticket assumed turned out false.
