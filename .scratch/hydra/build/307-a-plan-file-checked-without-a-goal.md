# 307 - A plan file checked without a goal: `hydra-puppets check <plan.md>`

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session, from map 121's measurement: to score thirty plan drafts
the session had to copy the import rules of `crates/puppets/src/write.rs::tasks` into a
script, because the only way to run them is through a goal with a chosen and approved plan.

## What

`write.rs::tasks` is the one place a plan is validated: the fenced JSON array is extracted
(`extract_tasks_json`), each task must carry `id`, `text`, `class` and `grade`
(`missing_keys`), class and grade must be in `TASK_CLASSES` and `TASK_GRADES` (`named`), and
`task_graph_offences` (build 299) refuses duplicate ids, unknown deps and cycles. All of that
runs only after `choose` and `approve --plan` on a real goal. A proposer being measured, a
human reading a draft before choosing, or a session grading an old plan has no way to ask
"would this import" short of staging a goal. 121 found that 4 of 15 drafts from one model
would have been refused at import; the number was only countable with a reimplementation.

## Do

1. `hydra-puppets check <path>`: read the file, run exactly the checks `tasks` runs in the
   order it runs them, print each offence on its own line to stderr in the words `tasks`
   uses, and exit 1 on any offence, 0 on none. On 0, print one line to stdout: the task
   count and how many tasks have an empty `done`. No goal, no home, no log line written.
2. Do it by calling the same functions, not by copying them. If `extract_tasks_json`,
   `missing_keys`, `named` and `task_graph_offences` need to become `pub(crate)` for a new
   module, do that; if `tasks` can be split so its validation half is one function both call,
   better, and say which in the Outcome.
3. `tests/`: one integration test under `crates/puppets/tests/` that feeds the command a clean
   plan, a plan with a raw newline inside a string, one with a comment after a value, one
   with a dep that does not resolve, and one with a cycle, and reads the exit code and the
   stderr line back for each. The thirty drafts under `.scratch/hydra/research/121-drafts/`
   are real inputs: run the command over all of them and put the count of refused files per
   proposer in the Outcome; it should say 4 for `ministral/` and 0 for `opus/`.
4. The puppets README (`integrations/puppets/README.md`) gains the command in one sentence
   beside `tasks`. Amend `IMPLEMENTATION.md` where `write.rs` is described.

## Done means

`hydra-puppets check` on a file answers what `tasks` would answer at import, from the same
code; the test above; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --
-D warnings`, `cargo nextest run --release -p hydra-puppets` green.

## Outcome
