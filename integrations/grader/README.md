# hydra-grader

Collect a local grading dataset from Hydra run records and Puppets goal logs.
Nothing under `src/` imports this directory. The package uses only the Python
standard library and makes no network calls.

## Environment

| Variable | Default | Purpose |
|---|---|---|
| `HYDRA_ROOT` | `~/hydra` | Root whose `runs/` directory is walked |
| `HYDRA_GRADER_HOME` | `$HYDRA_ROOT/grader` | Cursor, dataset, and `hydra show` cache |
| `HYDRA_BIN` | `hydra` | Binary used for `hydra show <id> --events` |
| `HYDRA_PUPPETS_HOME` | `$HYDRA_ROOT/puppets` | Goal logs for task labels |

## Dataset and cursor

`$HYDRA_GRADER_HOME/dataset.jsonl` holds one JSON object per run. Field names
and types are in [`SCHEMA.md`](SCHEMA.md).

`$HYDRA_GRADER_HOME/cursor.json` maps each `run_id` to a SHA-256 fingerprint
of that record file. A changed fingerprint replaces that run's line on the next
`once` or `rebuild`. Writes go to a temp file and rename, so a torn dataset
never exists.

The same directory holds `show-cache.json`, the memoized answers from
`hydra show --events`.

## Labels

Task labels come from Puppets goal logs under `HYDRA_PUPPETS_HOME`. Each
`*.jsonl` file is one goal; `labels.py` folds it the same way `puppets fold`
does and maps each `run_id` to task class, grade, human verdict, and task text.
When a run id appears in more than one goal log, the last sorted goal id wins.
Runs with no matching goal log keep the task text from the record and leave the
label fields null.

## Commands

```
python3 integrations/grader/grader.py once [--jobs N]
python3 integrations/grader/grader.py watch --interval N [--jobs N]
python3 integrations/grader/grader.py rebuild [--jobs N]
python3 integrations/grader/grader.py backfill
```

| Command | Behavior |
|---|---|
| `once` | Collect every run whose fingerprint is new or changed since the cursor. Prints the count and exits 0, including when the count is 0. |
| `watch` | Loop `once`, then sleep for `--interval` seconds. No daemon and no asyncio. |
| `rebuild` | Drop the cursor and rewrite the whole dataset. |
| `backfill` | Score every row in `dataset.jsonl` with every model. Collects nothing, runs no `hydra show`, leaves the cursor and dataset alone. Prints the lines appended and exits 0, including when that is 0. |

`--jobs` defaults to 4. Each new or changed record costs one `hydra show --events`
subprocess; `once` and `rebuild` run those calls on a thread pool and apply
results in run-id order.

## The classifier

`baseline.py` is a logistic regression over what a router knows before a launch: provider,
model, policy and the task's length. There are two models, trained and scored apart:

| Label | Positive means | File |
|---|---|---|
| `outcome` | the run will not complete | `$HYDRA_GRADER_HOME/models/outcome.json` |
| `verdict` | the diff will need a correction | `$HYDRA_GRADER_HOME/models/verdict.json` |

```
python3 integrations/grader/baseline.py train DATASET [MODEL] [--label outcome|verdict] [--test-from YYYY-MM-DD]
python3 integrations/grader/baseline.py score MODEL RUN
```

`train` splits by launch time into a fit slice and a test slice and refuses when either has fewer
than 10 rows of its rarer class. The refusal says how many labelled rows exist. With no `MODEL`
it writes the label's file above. Its report measures the model against two bars on the test
slice: the fit slice's overall rate, and the fit slice's rate for each run's provider and model
(falling back to the provider, then to the overall rate). `beats_provider_model_base` says
whether the model beats the second on log loss and on Brier. Most failures in the dataset are
rate limits, so a model that only learned which provider is out of quota will not beat it.

## Scores

Every collect that writes new or changed rows scores each of those runs with every model under
`$HYDRA_GRADER_HOME/models/` and appends one line per run and model version to
`$HYDRA_GRADER_HOME/scores.jsonl`: `run_id`, `label`, `p`, `model_version`, `scored_at`. A run
already scored by that model version is skipped. With no model there, nothing is written.
Nothing reads the file yet, so the model does not pick which runs get reviewed.

A collect scores only its own rows, so a model trained later never sees the older ones. Run
`grader.py backfill` after `baseline.py train` writes a model. It scores every row in
`dataset.jsonl` that the model's version has not scored, so running it twice appends nothing the
second time. With no model or no dataset it prints 0.

## The verdict queue

Verdict labels come from the human reading the `## Outcome` of resolved build tickets.

```
python3 integrations/grader/baseline.py label-queue OUT.csv [--tickets DIR] [--dataset PATH]
python3 integrations/grader/baseline.py label-import OUT.csv [--home DIR] [--dataset PATH]
```

`label-queue` writes one row per dataset run an Outcome names, with the first 400 characters of
that Outcome and an empty `verdict` column. A run id that matches no dataset run, or several,
is printed as a `skip:` line. Fill `verdict` with `applied`, `fixed` or `unknown`; leave it
`unknown` when the Outcome is ambiguous. `label-import` writes the `applied` and `fixed` rows to
`$HYDRA_GRADER_HOME/ticket-verdicts.json` and to the dataset's `human_verdict`. Collect reads that
file too, so a rewritten line keeps its verdict, and it wins over a Puppets `human_verdict`.

## Schedule

Nothing collects on its own. `launchd/co.hephaestuslab.hydra-grader.plist` is a template that
runs `grader.py once` every 15 minutes and logs to `$HYDRA_GRADER_HOME/launchd.log`. Replace
`/Users/USER` with your home, add `HYDRA_BIN` if `hydra` is not on the plist's `PATH`, and run
`once` by hand first so the grader home exists. Then install it:

```
cp integrations/grader/launchd/co.hephaestuslab.hydra-grader.plist ~/Library/LaunchAgents/ && launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/co.hephaestuslab.hydra-grader.plist
```

and remove it:

```
launchctl bootout gui/$(id -u)/co.hephaestuslab.hydra-grader && rm ~/Library/LaunchAgents/co.hephaestuslab.hydra-grader.plist
```

## Modules

| File | Purpose |
|---|---|
| `grader.py` | CLI, cursor, dataset writer, and scores on every collect |
| `baseline.py` | The two classifiers, their report, and the verdict queue |
| `records.py` | Walk `runs/`, cache `hydra show`, and project usage |
| `labels.py` | Fold Puppets goal logs into per-run labels |
| `redact.py` | Secret redaction ported from `src/export.rs` |

## Benchmark

Measured by the session on 2026-09-24 over the 434 record files on this machine (424 collected,
the ten harness runs excluded), `rebuild` after a clean `HYDRA_GRADER_HOME`, an M2 with 8 GB and
two agent runs alive:

| `--jobs` | wall time |
|---|---|
| 1 | 44.4 s |
| 4 | 15.7 s |

The cost is one `hydra show --events` subprocess per record, about 0.1 s each, and the four
workers overlap them. A steady-state `once` after one new run costs one subprocess. The table the
agent first wrote here named minutes it had never measured, because a gated run cannot read the
corpus; the numbers above replaced it.

## Tests

```
python3 -m unittest discover -s integrations/grader/tests
```
