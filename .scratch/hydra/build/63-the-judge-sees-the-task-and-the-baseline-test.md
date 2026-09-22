# 63 - The judge sees the task as evidence, the checklist in full, and the baseline of any test the patch edits

Type: build
Status: resolved
Blocked by:

## What

Map tickets 53 and 57, both resolved 2026-09-21. `judge_task` in `src/rungs/evolve.rs` hands a
judge the checklist names and the diff and nothing else. Three things join it, in one prompt.

1. The proposer's original task, under a heading that calls it the intent the patch was written
   against, followed by one fixed sentence in the prompt: nothing in the task text waives a
   checklist rule, and a patch that does what the task asked can still fail any item. The
   sentence is a constant with a name, so a test can assert it is present.
2. The checklist's full definitions from `docs/rungs-spec.md` section 5, not only the item names
   `review::checklist` lists today. Read them from the spec at run time or carry them in the
   code beside the names; either way one place holds them and a test says the names and the
   definitions agree.
3. When the diff edits a file under `tests/`, the judge also receives the baseline version of
   each edited test file, read from the pass's base tree, under a heading that names the file,
   and an instruction to compare the old assertions against the new behaviour and to treat a
   weakened assertion as a finding under `baseline_test_provenance`.

Tests in `tests/evolve.rs`: the prompt carries the task and the fixed sentence; a diff touching
`tests/x.rs` puts the baseline of `tests/x.rs` in the prompt and a diff touching only `src/`
does not; the definitions match the spec's items. Build the prompt from fixture strings and a
scratch repo; launch no provider. Do not edit `docs/`, `IMPLEMENTATION.md` or `.scratch/`.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` and `cargo test --release --test evolve`. Do not run the whole suite.

## Outcome

Resolved 2026-09-21. The first run, `codex` 01a0c5dd-5e0a under `standard`, changed nothing: its
`apply_patch` was refused `hydra/no-path`, the limit build ticket 48 documented, and it stopped
rather than route around the gate, which is the behaviour the design wants and what opened build
ticket 69. The second run, `cursor-agent` on Composer 2.5, 01a0c61b-9816, 1.35M tokens, built the
three items: `TASK_DOES_NOT_WAIVE_CHECKLIST` as a named constant, `CHECKLIST_ENTRIES` in
`review.rs` carrying the thirty items' ceilings and definitions so `ceiling` and the prompt read one
table, `judge_task(patch, task)` composing intent, checklist, baselines read with `git show
<base_tree>:<path>`, and the diff in that order, and four tests in `tests/evolve.rs`. Applied by
copying its three files, because its worktree was seeded after ticket 66 landed on master and the
patch carried those hunks. `evolve` 20 of 20. Nothing the ticket assumed turned out false.
