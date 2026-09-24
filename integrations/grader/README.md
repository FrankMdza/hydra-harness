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

## Dataset

`$HYDRA_GRADER_HOME/dataset.jsonl` holds one JSON object per run. Field names
and types are in [`SCHEMA.md`](SCHEMA.md).

`cursor.json` in the same directory maps each `run_id` to a SHA-256 fingerprint
of that record file. A changed fingerprint replaces that run's line on the next
`once` or `rebuild`. Writes go to a temp file and rename, so a torn dataset
never exists.

## Commands

```
python3 integrations/grader/grader.py once [--jobs N]
python3 integrations/grader/grader.py watch --interval N [--jobs N]
python3 integrations/grader/grader.py rebuild [--jobs N]
```

| Command | Behavior |
|---|---|
| `once` | Collect every run whose fingerprint is new or changed since the cursor. Prints the count and exits 0, including when the count is 0. |
| `watch` | Loop `once`, then sleep for `--interval` seconds. No daemon and no asyncio. |
| `rebuild` | Drop the cursor and rewrite the whole dataset. |

`--jobs` defaults to 4. Each new or changed record costs one `hydra show --events`
subprocess; `once` and `rebuild` run those calls on a thread pool and apply
results in run-id order.

## Modules

| File | Purpose |
|---|---|
| `grader.py` | CLI, cursor, and dataset writer |
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
python3 -m unittest discover -s integrations/grader/tests -v
```
