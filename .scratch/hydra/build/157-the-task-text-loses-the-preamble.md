# 157 - The dataset's task text loses the preamble

Type: build
Status: claimed
Blocked by:

## What

Every run's task on the record starts with the preamble Hydra composes from
`config/preamble.md` (`src/preamble.rs`, `compose`), about 630 characters before the task the
human wrote. The grader copies that into `task_text` (`integrations/grader/grader.py`,
`dataset_row`), so 514 of 536 rows on 2026-09-29 began with "Hydra launched this run and wrote
its instructions at spawn." Anything that reads `task_text` reads the preamble first: the
classifier's token features in `baseline.py`, and the three runs that classified the tasks on
2026-09-29 saw 870 characters of task in a 1,500 character cut.

## Do

Tests first in `integrations/grader/tests/test_grader.py`, on synthetic rows, each shown failing
before the change.

1. `dataset_row` strips the preamble from `task_text` when the text begins with it. The preamble
   is the current one from `config/preamble.md` composed the way `preamble.rs` does, read once
   per collect through a small function in `records.py`; and, since the preamble has changed
   over time, also strip when the text begins with the first line of the shipped preamble and
   the task follows the first blank line after the sentence "say so and stop." Keep the exact
   rule simple and say it in the README. A row whose text does not begin with a preamble is
   untouched.
2. The row gains `preamble: true|false`, so nothing is lost: a reader that wants the preamble
   knows the run had one.
3. `SCHEMA.md` and the README's dataset section say so. `.scratch/hydra/labeling/2026-09-29/`
   is not touched.
4. A changed `task_text` must reach the dataset: say in the report whether `once` rewrites the
   rows (the cursor fingerprints the record, not the rule) or whether the human must run
   `rebuild`, and give the one command.

Do not change `baseline.py` or the classes file. `redact.py` runs after the strip, as today.

## Done when

- The tests, each failed before the change; the grader suite passes under `.venv/bin/python`
  if the worktree has it, else `python3`.
- The report shows one synthetic row before and after, and the command for the real home.
