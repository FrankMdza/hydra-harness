# Grader dataset schema

One JSON object per line in `dataset.jsonl` under `HYDRA_GRADER_HOME` (default
`$HYDRA_ROOT/grader`). Each line describes one Hydra run or resume record.

| Field | Type | Meaning |
|---|---|---|
| `run_id` | string | Run identifier from the record envelope |
| `provider` | string | Provider from line 0 |
| `model` | string or null | Canonical model after `integrations/puppets/aliases.json` |
| `recorded_model` | string or null | Model spelling stored on the record |
| `policy` | string or null | Policy from `run_start` |
| `task_text` | string or null | Task text, redacted with the same secret classes as `hydra export`, and with the instruction preamble stripped |
| `preamble` | boolean | True if the original task text began with an instruction preamble |
| `task_class` | string or null | Puppets task class when a goal log names the run, else the class a model wrote |
| `grade` | string or null | Puppets grade when a goal log names the run, else the grade a model wrote |
| `area` | string or null | Area a model wrote in `task-classes-*.jsonl`, null otherwise. Nothing trains on it yet |
| `kind` | string or null | Kind a model wrote in `task-classes-*.jsonl`, null otherwise. Nothing trains on it yet |
| `label_unsure` | boolean, present only when true | The model-written line for this run said `unsure: true` |
| `outcome` | string or null | Outcome from `hydra show` projection |
| `human_verdict` | string or null | Puppets human verdict when present |
| `tokens` | object | Usage counts from the projection (`raw_input`, `output_tokens`, and so on) |
| `duration_s` | number or null | Seconds from line 0 `ts` to `run_end.ended_at` |
| `labels_source` | string or null | `"puppets"` when a goal log names the run, `"llm"` when only a model-written class file does, otherwise null |
| `collected_at` | string | UTC timestamp when this line was last written |

## Cursor

`cursor.json` in the same directory maps `run_id` to a SHA-256 fingerprint of the
committed bytes of that run's `.jsonl` file. When the fingerprint changes, `once`
or `rebuild` replaces that run's line in `dataset.jsonl`.

## Exclusions

Records whose `run_start.command` is not `run` or `resume`, or whose line 0 has
no `provider`, are not collected.
