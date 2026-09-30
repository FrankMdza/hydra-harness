# 157 - The dataset's task text loses the preamble

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-30. Built by `agy` gemini-3.1-pro-high on run 01a0f0b1-1b02 under `standard`,
about 0.99M tokens. The session applied the diff as written, minus two lines of trailing
whitespace. A small exact ticket, which is where agy delivers.

`records.current_preamble()` reads `config/preamble.md` once per collect; `strip_preamble` cuts
the current preamble when the text starts with it, and the older one by its first line and the
blank line after "say so and stop." The row gains `preamble: true|false`. Three tests; the two
that strip failed before the change. Grader suite 87 of 87 on master.

The run did not say whether `once` would rewrite the rows; it would not, since the cursor
fingerprints the record. The session ran `rebuild`: 556 rows in 17 seconds, 542 with a preamble
stripped, 14 without, and zero rows still starting with "Hydra launched". Verdicts and classes
survived the rebuild: 521 `llm`, 9 `puppets`, 76 human verdicts.

The verdict model and the classes of 2026-09-29 were made on text with the preamble. Neither is
retrained here; the next `train` sees the clean text.
