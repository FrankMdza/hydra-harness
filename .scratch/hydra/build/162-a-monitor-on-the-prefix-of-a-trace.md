# 162 - A monitor on the prefix of a trace

Type: build
Status: resolved
Blocked by:

## What

Phase 3.2 of the human's plan. FailFast (arXiv 2608.03222, August 2026) trains a small monitor
that predicts an agent's failure from the prefix of its trace and stops it early, saving 14.6 to
20.4% of tokens at 5% false positives on SWE-bench Verified. Hydra has 563 records and, since
2026-09-29, a `verdict` label on 76 of them and an `outcome` on all. `baseline.py` predicts from
what is known before launch. This ticket predicts from what is known part way through.

## Do

Tests first in `integrations/grader/tests/test_monitor.py`, on synthetic records and dataset
rows, each shown failing before the change. Stdlib only, like `baseline.py`.

1. `integrations/grader/monitor.py features <record> --at <fraction>` computes, from the first
   `fraction` of a record's events (0.1, 0.25, 0.5), a fixed feature vector: events so far, tool
   calls so far, refusals so far and the first refused tool, distinct tools, output tokens so
   far if the record carries them, seconds elapsed, whether a seat was requested, and the
   pre-launch features `baseline.py` already uses. Reuse its readers; do not copy them.
2. `monitor.py train --label outcome|verdict --at <fraction>` fits the same logistic regression
   `baseline.py` uses, with the same time split and the same per-provider base rate, on the
   dataset's rows joined to their records. Writes `models/monitor-<label>-<fraction>.json` and
   prints the same report shape as `baseline.py train`, plus `rank_auc`.
3. `monitor.py score <model> <record>` prints `p` and the model version for one run at the
   fraction the model was trained at, and says how many events it read.
4. `README.md`: a `## The monitor` section, and the sentence that nothing stops a run on it yet;
   stopping is a decision for the map.

## Done when

- The tests, each failed before the change; the grader suite passes under `.venv/bin/python` if
  the worktree has it, else `python3`.
- The report gives the commands for the real home and says what it could not measure (the real
  AUC at each fraction, which needs the real dataset).

## Outcome

Resolved 2026-09-30. Built by `cursor-agent` grok-4.7-high on run 01a0f246-5ec9 under `standard`.
`integrations/grader/monitor.py` (`features`, `train`, `score`) reads the first 0.1, 0.25 or 0.5
of a record and reuses `baseline.py`'s fit, split and bars; 6 tests on synthetic records and the
committed claude fixture. The session changed one thing: the default model path is now
`models/monitor/`, because `grader.py` scores every `models/*.json` with `baseline.predict` and
would have scored a monitor file as a pre-launch model. The grader suite passes, 93 tests.

The run could not read `~/hydra`, so the session measured on the real dataset, test from
2026-09-23:

| Label | Test rows (positive) | 0.1 | 0.25 | 0.5 | Provider and model bar |
|---|---|---|---|---|---|
| outcome | 273 (29) | 0.861 | 0.873 | 0.885 | 0.714 |
| verdict | 48 (37) | 0.686 | 0.563 | 0.668 | 0.652 |

`beats_provider_model_base` is true on log loss and Brier in all six. The outcome gain is real
but partly cheap: a rate limit or a gate refusal shows in the first lines, and the tenth of the
record already carries most of it. The verdict model has 11 negatives in its test slice, so its
numbers move with a handful of runs; it waits for Frank's labels (plan step 1.3). Nothing stops a
run on the score; that stays a decision for the map. The six models sit in
`grader/models/monitor/`.
