# 149 - The grader reads task classes a model wrote, and says so

Type: build
Status: resolved
Blocked by:

## What

The grader's `task_class` and `grade` come from Puppets goal logs (`integrations/grader/labels.py`).
On 2026-09-29, 9 of 510 dataset rows had one. The same day three runs classified every run from
its task text: `task_class` in the seven Puppets classes, `grade`, `area`, `kind` and a summary,
one JSON object per run, 530 lines. The human keeps that file at
`$HYDRA_GRADER_HOME/task-classes-llm-2026-09-29.jsonl`; a copy of its shape is at
`.scratch/hydra/labeling/2026-09-29/task-classes.jsonl` in this checkout.

A classifier trained on the dataset needs those columns filled. But a label a model wrote is not
a label a human wrote, and the dataset must keep them apart, the way `labels_source` already
says `"puppets"` or null.

## Do

Tests first in `integrations/grader/tests/`, each shown failing before the change, on a scratch
grader home with synthetic rows. Never the real dataset.

1. `labels.py` reads every `task-classes-*.jsonl` under the grader home, newest file last so a
   later file wins on the same `run_id`. Each line gives `run_id`, `task_class`, `grade`, and may
   give `area`, `kind`, `unsure` and `source`.
2. When a row has no Puppets label, collect fills `task_class` and `grade` from that file and sets
   `labels_source` to `"llm"`. When Puppets names the run, Puppets wins and nothing changes. A
   line with `unsure: true` still fills the row, and the row gains `label_unsure: true`.
3. The row gains two new nullable fields, `area` and `kind`, from the file when present. Add
   both to `SCHEMA.md`, and `labels_source` gains the value `"llm"` there.
4. `baseline.py`'s features do not change in this ticket. Say in the report whether the current
   feature list reads `task_class` or `grade`, with the line.
5. `integrations/grader/README.md`, the Labels section: where the file lives, that a model wrote
   it, that Puppets wins, and that nothing trains on `area` or `kind` yet.

A changed label must change the row's fingerprint or the cursor, so a `once` after the file
appears rewrites the rows it touches. Say how you did that; `cursor.json` fingerprints the record
file today, not the labels.

## Done when

- The tests cover items 1 to 3 and the rewrite on a later `once`, each failed before the change,
  and the grader suite passes under `.venv/bin/python` if the worktree has it, else `python3`.
- `grader.py rebuild` on a scratch home with the fixture file fills the rows and marks them `llm`.
- The report gives the one command the human runs on the real grader home.

## Outcome

Resolved 2026-09-29. Built by `claude` sonnet on run 01a0f050-077e under `standard`, about 0.62M
tokens. The session applied the diff as written.

`labels.py` gains `collect_llm`, which reads every `task-classes-*.jsonl` under the grader home,
later file winning. `dataset_row` fills `task_class` and `grade` from it when Puppets has nothing,
sets `labels_source` to `llm`, and adds `area`, `kind` and `label_unsure`. The cursor entry of a
row a model label fills is the record fingerprint plus a digest of the label line, so a new or
changed file makes the row stale, and `work()` skips `hydra show` when the show cache already
matches the record. Five tests, each failed before the change. Grader suite 84 of 84 under
`.venv/bin/python`.

On the real grader home, `once` rewrote 523 rows in 4.9 seconds without a single `hydra show`.
The dataset now holds 540 rows: 521 labelled `llm`, 9 `puppets`, 10 with nothing. 47 carry
`label_unsure`.

`baseline.py` reads neither `task_class` nor `grade` (its features are provider, model, policy
and task-text tokens), so no model input changed. Left as the run found it: a Puppets label edit
still does not invalidate the cursor.
