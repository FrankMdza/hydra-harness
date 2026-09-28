# 143 - The local classifier scores every run the grader collects

Type: build
Status: open
Blocked by:

## What

Map ticket 77, job 1. The human's interactive session of 2026-09-28 (run `01a0e68b`) wrote
`integrations/grader/baseline.py`, a stdlib logistic regression over pre-launch features, with
`integrations/grader/tests/test_baseline.py`; the session committed both as they were before this
ticket. A three-seat panel (claude opus, codex, composer) reviewed six decisions and the human
chose, the same night:

- **D1.** Two models, not one label swapped later: `outcome` (the run will not complete) and
  `verdict` (the diff will need a correction). Different targets, trained and scored apart.
- **D2.** Two time slices, fit and test. No calibration slice: it spent scarce rows and the fitted
  temperature (1.10) moved test Brier the wrong way (0.0920 to 0.0925).
- **D3.** Labels for `verdict` come from the human reading the `## Outcome` of resolved build
  tickets (90 of them name 158 dataset runs), on the agent's original diff; ambiguous is unknown.
- **D4.** Jev stays out of the model. Not in this ticket.
- **D5.** The bar is a per-provider-and-model base rate, not the constant: 14 of 18 test positives
  are `rate_limited`, 8 of them codex, so the outcome model may only have learned quota.
- **D6.** Scores go to a file on every collect, and nothing in `integrations/puppets/sheet.py`
  reads them yet, so the model does not choose which runs get reviewed (and labelled).

Nothing leaves the machine: no network call, and the dataset, records and scores stay under
`$HYDRA_GRADER_HOME` and `$HYDRA_ROOT`.

## Do

Tests first in `integrations/grader/tests/`, each shown failing before the change, on synthetic
rows and records only (never the real dataset, which a run cannot read):

1. **Two slices (D2).** `train` takes `--test-from` only; no `--cal-from`, no temperature (store
   `temperature: 1.0` or drop it, and `predict` must not need it). The minority floor applies to
   both slices. Test: a dataset whose rows fall in two slices trains, and the model has no
   calibration rows.
2. **The per-provider bar (D5).** `report` adds `provider_model_base_rate`: for each test row, the
   fit slice's positive rate for its `(provider, model)`, falling back to the provider's rate and
   then the overall rate when a pair was unseen. It reports log loss and Brier for it beside the
   constant and the model, and `beats_provider_model_base: true|false` on both measures. Test: on
   rows where the label is decided by provider alone, the model does not beat that base and the
   report says so.
3. **Two models side by side (D1).** A model file carries its label; `score` prints the label; a
   helper names the file per label (`models/outcome.json`, `models/verdict.json` under the grader
   home). `--label verdict` keeps refusing below the floor, with the message saying how many
   labelled rows exist.
4. **Scores on every collect (D6).** `grader.py`'s collect step, after it writes new dataset rows,
   scores each new run with every model present under `$HYDRA_GRADER_HOME/models/` and appends
   `{run_id, label, p, model_version, scored_at}` to `$HYDRA_GRADER_HOME/scores.jsonl`, once per
   run and model version (a second collect writes nothing new). No model present: nothing written,
   no error. Test on a scratch grader home with a fixture record.
5. **A labelling queue for the human (D3).** `baseline.py label-queue <out.csv>` walks
   `.scratch/hydra/build/*.md`, and for each resolved ticket whose `## Outcome` names a run id,
   writes one row: ticket number, run id, provider, model, the first 400 characters of the
   Outcome, and an empty `verdict` column for `applied`, `fixed` or `unknown`. Reuse how
   `integrations/puppets/jev_probe.py` (`build`) finds tickets and run ids; do not label anything
   yourself (build 134 showed a regex gets it wrong). Then say, in the report, the existing path by
   which a `human_verdict` reaches the dataset (`integrations/grader/labels.py`), and add
   `baseline.py label-import <csv>` only if that path cannot take the CSV as it is. Test with two
   fixture tickets.

6. **A schedule the human can install.** Nothing collects on its own today: there is no launchd
   job for the grader. Add `integrations/grader/launchd/co.hephaestuslab.hydra-grader.plist`, a
   template that runs `grader.py once` every 15 minutes (`StartInterval` 900) with
   `HYDRA_GRADER_HOME` and `HYDRA_ROOT` set, logs to a file under the grader home, and a README
   section with the two commands to install and remove it. Do not install it: the human does.
   Test: the plist parses (`plistlib`) and names `grader.py once`.

`integrations/grader/README.md`: the two models, the bar, where scores go, the queue. Run the
grader's own tests (`python3 -m unittest discover integrations/grader/tests` or `pytest` on PATH;
say which). This ticket touches no Rust; do not run cargo.

## Outcome
