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
and types are in [`SCHEMA.md`](SCHEMA.md). The `task_text` field has the instruction
preamble stripped: either the exact current preamble text from `config/preamble.md`,
or an older version identified by starting with "Hydra launched this run and wrote its
instructions at spawn." and cutting at the first blank line after "say so and stop.".
A row whose text does not begin with a preamble is untouched. The `preamble` boolean
field is `true` if the text began with one.

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

A model can fill those fields too. Every `task-classes-*.jsonl` under
`HYDRA_GRADER_HOME` holds one JSON object per run: `run_id`, `task_class`,
`grade`, and optionally `area`, `kind`, `unsure` and `source`. A model wrote
these files, not a human. Files are read in name order, so a later file wins on
the same `run_id`. When Puppets names the run, Puppets wins and the file is
ignored for that run. Otherwise the row takes `task_class` and `grade` from the
file, gets `area` and `kind`, and `labels_source` is `"llm"`. `unsure: true`
still fills the row and adds `label_unsure: true`. The cursor entry for such a
run is the record fingerprint plus a digest of its line, so the next `once`
rewrites the row when the file appears or changes. Nothing trains on `area` or
`kind` yet, and `baseline.py` reads neither `task_class` nor `grade`.

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

## The monitor

`monitor.py` predicts the same two labels from a prefix of the run's record, plus the
pre-launch features `baseline.py` already computes. It calls `baseline.py`'s readers
for those fields, for the launch-time split and for the fit. It does not copy them.
The prefix is the first `max(1, floor(fraction * lines))` lines of the record. The
fractions are 0.1, 0.25 and 0.5. A record of ten lines is read as 1, 2 and 5 lines.

The prefix features are fixed names: events so far, tool calls so far, refusals so
far, the first refused tool, distinct tools, output tokens so far when a usage frame
in the prefix carries them, seconds from the first line of the prefix to the last,
and whether a `seat_request` line has appeared. Provider, model, policy and the first
refused tool are one-hot categories, the way the baseline's vocabulary works. Output
tokens are the latest usage report in the prefix, not a sum, and the printed feature
is null when the prefix has none. A refusal is a hook `permissionDecision` of `deny`,
or a tool result whose text carries Hydra's refusal voice (`hydra/`, other than
`hydra/allow`). The echoed tool result after a hook is not a second refusal. Tool
names are read from each provider's own frame: `claude` `tool_use`, `cursor-agent`
`tool_call` on `started`, `agy` `step_type: tool`, `codex` `item.completed`.

```
python3 integrations/grader/monitor.py features RECORD --at 0.1|0.25|0.5
python3 integrations/grader/monitor.py train DATASET [MODEL] --at FRACTION [--label outcome|verdict] [--test-from YYYY-MM-DD] [--records DIR]
python3 integrations/grader/monitor.py score MODEL RECORD
```

`train` joins each dataset row to `<run_id>.jsonl` under `--records` (default
`$HYDRA_ROOT/runs`). A row with no record is left out. It uses the same logistic
regression, the same launch-time split and the same per-provider base rate as
`baseline.py`, and it refuses on the same floor of 10 rows of the rarer class. With
no `MODEL` it writes `$HYDRA_GRADER_HOME/models/monitor/monitor-<label>-<fraction>.json`.
The report has the same shape as `baseline.py train`, including `rank_auc` on the
model and on both bars.

`score` prints `p` and the model version for one record, at the fraction stored on
the model, and `events_read`, how many lines it used.

`grader.py` scores every JSON file directly in `models/` with `baseline.predict`, which
does not read a prefix. That is why the monitor's default is the `models/monitor/`
subfolder: a monitor file moved up into `models/` would be scored as a pre-launch
model. Nothing stops a run on this score yet. Stopping is a decision for the map.

## Scores

Every collect that writes new or changed rows scores each of those runs with every model under
`$HYDRA_GRADER_HOME/models/` and appends one line per run and model version to
`$HYDRA_GRADER_HOME/scores.jsonl`: `run_id`, `label`, `p`, `model_version`, `scored_at`. A run
already scored by that model version is skipped. With no model there, nothing is written.
`puppets.py queue` and `board` read the verdict scores to order reviews. `sheet.py rank` does not
read the file, so the model does not choose which agent is assigned.

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

## The local judge

```
python3 integrations/grader/judge.py label [--limit N] [--only-unlabelled]
python3 integrations/grader/judge.py agree
```

`label` walks `dataset.jsonl` and asks a local Ollama model whether each run needed a human
correction. It sends the redacted task (cut at 3,000 characters) and the run's result text (cut at
4,000) and diff stat, read from `hydra show` at the time and never cached. The URL and model come
from `HYDRA_LOCAL_LLM_URL` and `HYDRA_LOCAL_MODEL`, the same defaults `integrations/local-llm/`
uses, and that URL is the only network call. `--only-unlabelled` skips runs that already carry a
human verdict. If Ollama is down or the model is not pulled, the pass stops with exit 1 and keeps
the lines it wrote.

It appends one line per run to `judge-labels.jsonl` in the grader home: `run_id`,
`needs_correction`, `failure_class`, `reason`, `judge_model`, `prompt_sha256`, `judged_at`. A run
the same model and prompt already labelled is skipped. An answer that is not the expected JSON
gets `needs_correction: null` and the raw text under `unparsed`, cut at 500 characters. Runs
with outcome `rate_limited` or `gated` are labelled `quota` or `gate` by rule, with
`judge_model: "rule"`, and never sent to the model.

The prompt asks the model for its reason first and then for `result_ok`, and the file stores the
opposite as `needs_correction`. A label from the model carries `failure_class: null`: only the
rule names a class.

`agree` counts the runs that have both a human verdict and a parsed judge label, and prints how
many agree (`fixed` against `needs_correction: true`, `applied` against `false`), the fraction, the
two by two table and the number of unparsed answers. Under 50 runs it says the number is loose.

Nothing reads the labels yet.

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
| `monitor.py` | Prefix features, the same regression on a fraction of a record, and score |
| `records.py` | Walk `runs/`, cache `hydra show`, and project usage |
| `labels.py` | Fold Puppets goal logs into per-run labels |
| `judge.py` | The local judge: `label` and `agree` |
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
