# hydra-grader

Collect a local grading dataset from Hydra run records and Puppets goal logs.

The grader is `hydra-grader`, the Rust binary of `crates/grader`, built beside `hydra` by
`cargo build --release --workspace`. Build ticket 220 deleted the Python it was ported from
on 2026-10-01: `grader.py`, `baseline.py`, `monitor.py`, `triage.py`, `judge.py`, `labels.py`,
`jev_probe.py` and their tests. Before it went, `crates/grader/tests/record*.py` ran it on
every input of the differential tests and wrote its output under
`crates/grader/tests/fixtures/`, which `crates/grader/tests/differential.rs` compares the
binary with. This directory keeps what a human installs and reads: this file, `SCHEMA.md`
and the launchd template.

Nothing under `src/` imports this directory or depends on the binary. The only network call
is the local judge's, to Ollama on this machine.

## Environment

| Variable | Default | Purpose |
|---|---|---|
| `HYDRA_ROOT` | `~/hydra` | Root whose `runs/` directory is walked |
| `HYDRA_GRADER_HOME` | `$HYDRA_ROOT/grader` | Cursor, dataset, and `hydra show` cache |
| `HYDRA_BIN` | `hydra` | Binary used for `hydra show <id> --events` |
| `HYDRA_PUPPETS_HOME` | `$HYDRA_ROOT/puppets` | Goal logs for task labels |
| `HYDRA_PUPPETS_BIN` | `hydra-puppets` beside `HYDRA_BIN` when that is an absolute path, else `target/release/hydra-puppets` of the checkout the binary is under | The helper that folds a goal log. An absolute path; `PATH` is never searched |
| `HYDRA_PUPPETS_DATA` | `integrations/puppets` under `HYDRA_HOME`, else under the first directory above the binary that holds `integrations/puppets/prompts` | The directory `aliases.json` is read from |

### Where the alias table and the helper are found

`aliases.json` maps a recorded model spelling to the `model` written to the dataset. The
Python read the file beside its own checkout and ignored the environment. A binary has no
checkout of its own, so it follows the one rule `hydra-puppets` follows (build tickets 221
and 220): `HYDRA_PUPPETS_DATA` when set, else `integrations/puppets` under `HYDRA_HOME`, else
the first directory above the binary that holds `integrations/puppets/prompts`. A directory
that is not there, or one with no `aliases.json` in it, ends the command with exit 1 and one
line: `FileNotFoundError: no aliases.json in <dir>; set HYDRA_PUPPETS_DATA to the directory
that holds aliases.json, or HYDRA_HOME to the checkout`. It is never read as an empty table.
Point `HYDRA_PUPPETS_DATA` at another table and the dataset's `model` column changes with no
other sign, so the launchd template names the directory.

A `hydra-grader` copied out of its checkout finds no `hydra-puppets` unless
`HYDRA_PUPPETS_BIN` names it or `HYDRA_BIN` is an absolute path with the helper beside it. A
bare `HYDRA_BIN=hydra` is enough to run `hydra show` and not enough to find the helper. The
pass then ends with a `LabelsError` that names both variables. The launchd template sets
`HYDRA_BIN` to an absolute path for this reason.

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
`*.jsonl` file is one goal. The grader runs `hydra-puppets show <goal>` on it,
reads the state that prints, and maps each `run_id` to task class, grade, human
verdict, and task text. When a run id appears in more than one goal log, the
last sorted goal id wins. Runs with no matching goal log keep the task text
from the record and leave the label fields null.

There is one fold of a goal log, the helper's, and the grader does not carry a
second (map ticket 93). The cost is that a pass fails where it used to guess. A
missing helper, a `show` that exits non-zero, which a damaged goal log makes it
do, or output without the fields the grader reads is a `LabelsError` before
the dataset, the cursor, the show cache or the scores are written. `once` and
`rebuild` exit 1 with that one line, and `watch` prints the failure and tries
again after its interval. Repair the log with `hydra-puppets repair <goal>` and
the next pass collects what the failed one left.

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
`kind` yet, and the classifier reads neither `task_class` nor `grade`.

## Commands

```
hydra-grader once [--jobs N]
hydra-grader watch --interval N [--jobs N]
hydra-grader rebuild [--jobs N]
hydra-grader backfill
```

| Command | Behavior |
|---|---|
| `once` | Collect every run whose fingerprint is new or changed since the cursor. Prints the count and exits 0, including when the count is 0. |
| `watch` | Loop `once`, then sleep for `--interval` seconds. No daemon and no async runtime. |
| `rebuild` | Drop the cursor and rewrite the whole dataset. |
| `backfill` | Score every row in `dataset.jsonl` with every model. Collects nothing, runs no `hydra show`, leaves the cursor and dataset alone. Prints the lines appended and exits 0, including when that is 0. |

`--jobs` defaults to 4. Each new or changed record costs one `hydra show --events`
subprocess; `once` and `rebuild` run those calls on worker threads and apply
results in run-id order.

One pass runs in a grader home at a time. A pass takes an `flock` on the home directory and
holds it until it returns. A second `once`, `rebuild`, `backfill` or turn of `watch` that
finds the lock taken prints `grader: another pass is running in <home>; nothing done` on
stderr, writes nothing, and exits 0. launchd starts a pass every fifteen minutes and a slow
one can run into the next. Before the lock, both wrote through the same `.tmp` names and one
of them died with `FileNotFoundError`. `baseline train`, `baseline label-import`, `triage run`
and `judge label` write under the same home and do not take the lock.

## The classifier

`hydra-grader baseline` is a logistic regression over what a router knows before a launch:
provider, model, policy and the task's length. There are three models, trained and scored apart:

| Label | Positive means | File |
|---|---|---|
| `outcome` | the run will not complete | `$HYDRA_GRADER_HOME/models/outcome.json` |
| `verdict` | the diff will need a correction | `$HYDRA_GRADER_HOME/models/verdict.json` |
| `task` | the run will fail for a reason in the task | `$HYDRA_GRADER_HOME/models/task.json` |

`outcome` counts every run that did not complete, so a model can beat the bar by learning which
provider is out of quota or which policy refuses. `task` removes that. It is 1 for `failed`,
`crashed` and `timed_out`, 0 for `completed`, and the row is left out for `rate_limited`, `gated`,
`skill_collision`, `auth_failed` and any other outcome. The list is `TASK_FAILURES` in
`crates/grader/src/baseline.rs`. A floor of 10 rows of the rarer class applies, and `train`
refuses when a slice has too few task failures.

```
hydra-grader baseline train DATASET [MODEL] [--label outcome|verdict|task] [--test-from YYYY-MM-DD]
hydra-grader baseline score MODEL RUN
```

`train` splits by launch time into a fit slice and a test slice and refuses when either has fewer
than 10 rows of its rarer class. The refusal says how many labelled rows exist. With no `MODEL`
it writes the label's file above. Its report measures the model against two bars on the test
slice: the fit slice's overall rate, and the fit slice's rate for each run's provider and model
(falling back to the provider, then to the overall rate). `beats_provider_model_base` says
whether the model beats the second on log loss and on Brier. Most failures in the dataset are
rate limits, so a model that only learned which provider is out of quota will not beat it.

The numbers are CPython 3.14.7's to the byte: the sum is the compensated one Python has had
since 3.12, and `x ** 2` is the C library's `pow`. A model trained by the Python under 3.14.7
and one trained by the binary on the same dataset are the same file. The launchd job used to
run `/usr/bin/python3`, which is 3.9.6 here, and its `sum()` is not compensated, so about one
score in ten written before the cutover differs from the binary's in the last digit of `p`.
`scores.jsonl` is append-only and no old line is rewritten.

## The monitor

`hydra-grader monitor` predicts the same three labels from a prefix of the run's record, plus
the pre-launch features the classifier already computes. It uses the classifier's readers
for those fields, for the launch-time split and for the fit. It does not copy them.
The prefix is the first `max(1, floor(fraction * lines))` lines of the record. The
fractions are 0.1, 0.25 and 0.5. A record of ten lines is read as 1, 2 and 5 lines.

The prefix features are fixed names: events so far, tool calls so far, refusals so
far, the first refused tool, distinct tools, output tokens so far when a usage frame
in the prefix carries them, seconds from the first line of the prefix to the last,
and whether a `seat_request` line has appeared. Provider, model, policy and the first
refused tool are one-hot categories, the way the classifier's vocabulary works. Output
tokens are the latest usage report in the prefix, not a sum, and the printed feature
is null when the prefix has none. A refusal is a hook `permissionDecision` of `deny`,
or a tool result whose text carries Hydra's refusal voice (`hydra/`, other than
`hydra/allow`). The echoed tool result after a hook is not a second refusal. Tool
names are read from each provider's own frame: `claude` `tool_use`, `cursor-agent`
`tool_call` on `started`, `agy` `step_type: tool`, `codex` `item.completed`.
Two more features read repeated calls. A call's key is its tool name plus a hash of its
input (`claude` `input`, `cursor-agent` `args`, `codex` `command`, `agy` `tool_info.parameters`);
no input keys on the name alone. `repeated_calls_so_far` counts calls whose key was already
seen in the prefix, and `longest_repeat_run` is the longest streak of consecutive calls with one key. `features` prints both; the fit leaves them out, because on 2026-09-30 they lowered the
outcome AUC at every fraction (0.838, 0.860, 0.880 with them against 0.856, 0.867, 0.881 without, 576 rows).

```
hydra-grader monitor features RECORD --at 0.1|0.25|0.5
hydra-grader monitor train DATASET [MODEL] --at FRACTION [--label outcome|verdict|task] [--test-from YYYY-MM-DD] [--records DIR]
hydra-grader monitor score MODEL RECORD
```

`train` joins each dataset row to `<run_id>.jsonl` under `--records` (default
`$HYDRA_ROOT/runs`). A row with no record is left out. It uses the same logistic
regression, the same launch-time split and the same per-provider base rate as
the classifier, and it refuses on the same floor of 10 rows of the rarer class. With
no `MODEL` it writes `$HYDRA_GRADER_HOME/models/monitor/monitor-<label>-<fraction>.json`.
The report has the same shape as `baseline train`, including `rank_auc` on the
model and on both bars. A usage count so large that its spread has no square as a float ends
`train` with `OverflowError` before it writes anything, as it ended the Python.

`score` prints `p` and the model version for one record, at the fraction stored on
the model, and `events_read`, how many lines it used.

A collect scores every JSON file directly in `models/` with the classifier's `predict`, which
does not read a prefix. That is why the monitor's default is the `models/monitor/`
subfolder: a monitor file moved up into `models/` would be scored as a pre-launch
model. Nothing stops a run on this score yet. Stopping is a decision for the map.

## Triage

`hydra-grader triage run` reads every dataset row whose outcome is not `completed`, reads its
record, and writes one line per run to `$HYDRA_GRADER_HOME/triage.jsonl`: `run_id`, `outcome`,
`cause`, `evidence` (the deciding line, cut to 300 characters) and `triaged_at`. `triage table`
prints counts per outcome and cause and per provider and cause. `triage sample --n 20 --seed S`
prints runs for a human to check. The first rule that matches wins:

1. `collision`: `run_end.refusal` is `skill_collision`, or the outcome is.
2. `quota`: a `result` frame with `api_error_status` 429, or the providers' "You've hit your ...
   limit" sentence anywhere in the record, or the outcome `rate_limited`. A `rate_limit_event`
   frame alone is not quota: `claude` sends one on healthy runs.
3. `gate`: `run_end.refusal` is set, or a refusal by the monitor's reading is the last tool
   event, or the outcome is `gated` and the record holds a refusal.
4. `never_started`: no provider frame (`raw`) at all, the shape build 147 writes.
5. `provider_crash`: no `run_end`, a `run_end.error` starting "the child died on signal ", the
   outcome `crashed`, or an exit code of 128 and up.
6. `stall`: more than ten minutes between the last line before `run_end` and `run_end`.
7. `unknown`: nothing above.

On 2026-09-30, over 74 runs: 29 of 31 `gated` read `gate`, 27 of 27 `rate_limited` read `quota`,
and 6 read `unknown`. No run read `stall`.

## Scores

Every collect that writes new or changed rows scores each of those runs with every model under
`$HYDRA_GRADER_HOME/models/` and appends one line per run and model version to
`$HYDRA_GRADER_HOME/scores.jsonl`: `run_id`, `label`, `p`, `model_version`, `scored_at`. A run
already scored by that model version is skipped. With no model there, nothing is written.
`hydra-puppets queue` and `board` read the verdict scores to order reviews, from the same file
as before the cutover. The ranking of `hydra-puppets assign` does not read the file, so the
model does not choose which agent is assigned.

A collect scores only its own rows, so a model trained later never sees the older ones. Run
`hydra-grader backfill` after `baseline train` writes a model. It scores every row in
`dataset.jsonl` that the model's version has not scored, so running it twice appends nothing the
second time. With no model or no dataset it prints 0.

## The verdict queue

Verdict labels come from the human reading the `## Outcome` of resolved build tickets.

```
hydra-grader baseline label-queue OUT.csv [--tickets DIR] [--dataset PATH]
hydra-grader baseline label-import OUT.csv [--home DIR] [--dataset PATH]
```

`label-queue` writes one row per dataset run an Outcome names, with the first 400 characters of
that Outcome and an empty `verdict` column. A run id that matches no dataset run, or several,
is printed as a `skip:` line. Fill `verdict` with `applied`, `fixed` or `unknown`; leave it
`unknown` when the Outcome is ambiguous. `label-import` writes the `applied` and `fixed` rows to
`$HYDRA_GRADER_HOME/ticket-verdicts.json` and to the dataset's `human_verdict`. Collect reads that
file too, so a rewritten line keeps its verdict, and it wins over a Puppets `human_verdict`.

## The local judge

```
hydra-grader judge label [--limit N] [--only-unlabelled]
hydra-grader judge agree
```

`label` walks `dataset.jsonl` and asks a local Ollama model whether each run needed a human
correction. It sends the redacted task (cut at 3,000 characters) and the run's result text (cut at
4,000) and diff stat, read from `hydra show` at the time and never cached. The URL and model come
from `HYDRA_LOCAL_LLM_URL` and `HYDRA_LOCAL_MODEL`, the same defaults `hydra-local-llm`
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

## The Jev probe and the sheets

`hydra-grader jev build|patches|score` is the local half of the Jev probe. `ask`, the one
command that called a remote API, was not ported and went with `jev_probe.py`.
`hydra-grader sheet build|budget|show` writes and prints the corpus sheets `hydra-puppets
assign` ranks from (build ticket 219).

## Schedule

Nothing collects on its own. `launchd/co.hephaestuslab.hydra-grader.plist` is a template that
runs `hydra-grader once` every 15 minutes and logs to `$HYDRA_GRADER_HOME/launchd.log`. Its
paths assume the checkout is `~/hydra`. It names `target/release/hydra-grader` as the program,
sets `HYDRA_BIN` to `target/release/hydra`, whose neighbour `hydra-puppets` the grader runs, and
sets `HYDRA_PUPPETS_DATA` to `integrations/puppets`. None of the three needs a Hydra run alive.
Run `once` by hand first so the grader home exists.

Install it, or replace a copy already installed, with these two commands from the checkout.
The first builds the three binaries. The second stops the old job if there is one, writes
the template with your home in place of `/Users/USER`, and starts it:

```
cargo build --release --workspace
launchctl bootout gui/$(id -u)/co.hephaestuslab.hydra-grader; sed "s|/Users/USER|$HOME|g" integrations/grader/launchd/co.hephaestuslab.hydra-grader.plist > ~/Library/LaunchAgents/co.hephaestuslab.hydra-grader.plist && launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/co.hephaestuslab.hydra-grader.plist
```

`bootout` prints an error when no job is loaded, and the rest still runs. **A job installed
before build ticket 220 runs `/usr/bin/python3 integrations/grader/grader.py once`, a file
that no longer exists.** Run the two commands before, or in the same sitting as, taking the
change that deleted it. Until then every fifteen minutes the old job fails and writes a
"can't open file" line to `launchd.log`, and nothing is collected.

Remove it:

```
launchctl bootout gui/$(id -u)/co.hephaestuslab.hydra-grader && rm ~/Library/LaunchAgents/co.hephaestuslab.hydra-grader.plist
```

## Where the code is

| File under `crates/grader/src/` | Purpose |
|---|---|
| `collect.rs`, `collect_cli.rs` | The pass: cursor, dataset writer, scores on every collect, and the lock on the grader home |
| `baseline.rs`, `baseline_cli.rs`, `queue.rs` | The three classifiers, their report, and the verdict queue |
| `monitor.rs`, `monitor_cli.rs` | Prefix features, the same regression on a fraction of a record, and score |
| `records.rs` | Walk `runs/`, read and write the grader's JSON files, and project usage |
| `labels.rs` | Per-run labels from what `hydra-puppets show` prints for each goal log |
| `triage.rs`, `judge.rs`, `jev.rs`, `sheet.rs` | Triage, the local judge, the Jev probe's local half, and the sheets |
| `redact.rs` | Secret redaction ported from `src/export.rs` |
| `data.rs`, `argparse.rs` | Where the data directory is found, and the one reading of `argparse` every command shares |

Two Python files outlived the cutover because Python still loaded them: `records.py` and
`redact.py`, with `models.py`, moved to `integrations/apprentice/`. Build tickets 234 and 236
deleted them on 2026-10-04 with the apprentice's and otel's Python; the Rust for all three is
`crates/records`.

## Benchmark

Measured by the session on 2026-09-24 over the 434 record files on this machine (424 collected,
the ten harness runs excluded), `rebuild` after a clean `HYDRA_GRADER_HOME`, an M2 with 8 GB and
two agent runs alive, with the Python the binary replaced:

| `--jobs` | wall time |
|---|---|
| 1 | 44.4 s |
| 4 | 15.7 s |

The cost is one `hydra show --events` subprocess per record, about 0.1 s each, and the four
workers overlap them. A steady-state `once` after one new run costs one subprocess. The binary
has not been timed on the same corpus.

## Tests

```
cargo test -p hydra-grader
cargo test -p hydra-puppets --test grader_labels
```

The first runs the unit tests, the differential tests against the recorded Python, the tests
of what build ticket 220 changed (`tests/cutover.rs`) and the launchd template's
(`tests/launchd.rs`). The second runs the real `hydra-puppets` over every recorded goal log and
compares the grader's labels with the ones the Python fold gave. No test starts `python3`, and
the fixtures cannot be recorded again from this checkout: a new differential case needs a
checkout from before build ticket 220.
