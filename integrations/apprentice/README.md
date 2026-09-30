# hydra-apprentice

A bank of tasks about Hydra run records, the local model's attempts at them, and the preference
pairs a panel's grading leaves behind. Map ticket 87 is the design, build ticket 160 is this
code. Nothing under `src/` imports it. It uses only the Python standard library and makes one
kind of network call, to the Ollama at `HYDRA_LOCAL_LLM_URL`.

**Nothing trains on the pairs yet.** Phase 3 of the plan decides when a LoRA is worth a rented
GPU. The number to watch until then is the verifiable score.

## The loop

1. `bank build` turns the records in `$HYDRA_ROOT/runs/` into tasks.
2. Verifiable tasks carry the answer the script computed from the record. Open tasks carry none.
3. `attempt` asks the local model each task it has not answered under the current prompt.
4. Verifiable answers are marked right or wrong on the spot.
5. `score` prints the verifiable score per template, and against the previous prompt.
6. Change the prompt (`INSTRUCTIONS` in `apprentice.py`), run `attempt` again, read the delta.
7. The session collects the week's open answers and a panel grades them.
8. The session writes the grades as a `graded.jsonl` file.
9. `pairs` turns that file into `pairs.jsonl`.
10. The pairs wait. Hydra works the same with the whole loop stopped.

## Environment

| Variable | Default | Purpose |
|---|---|---|
| `HYDRA_ROOT` | `~/hydra` | Root whose `runs/` directory is read |
| `HYDRA_GRADER_HOME` | `$HYDRA_ROOT/grader` | The apprentice's files live in its `apprentice/` directory |
| `HYDRA_LOCAL_LLM_URL` | `http://127.0.0.1:11434/api/generate` | The Ollama generate endpoint |
| `HYDRA_LOCAL_MODEL` | `gemma3:1b-it-qat` | The model that attempts |

## Commands

```
python3 integrations/apprentice/apprentice.py bank build [--n 20]
python3 integrations/apprentice/apprentice.py attempt [--limit N] [--minutes M]
python3 integrations/apprentice/apprentice.py score
python3 integrations/apprentice/apprentice.py pairs <graded.jsonl>
```

`bank build` writes `bank.jsonl`: up to `n` verifiable and `n` open tasks. It reads each record
directly and does not call `hydra show`, so it needs no binary. For each template the runs are
ordered by a hash of the run id, and templates take turns, so the same records give the same
bank in any listing order. With fewer matching records than `n` you get fewer tasks. Prompts are
redacted with `integrations/grader/redact.py` and each record excerpt is cut to 3,000 characters.

`attempt` appends to `attempts.jsonl`: task id, template, prompt sha, answer, reason, model,
seconds and, for verifiable tasks, `correct`. The prompt asks for the reason first and the
answer second, as JSON, at temperature 0 with `num_predict` 300. The prompt sha is the sha-256 of
`INSTRUCTIONS`, so editing the prompt makes every task due again. `--minutes` stops the pass
between two tasks once the budget is spent. If Ollama is down or lacks the model, the pass prints
one line, exits 1 and keeps the attempts already written. A reply that is not the JSON shape is
stored with an empty answer and an `unparsed` field, and a verifiable one counts as wrong.

`score` prints the latest attempt per task for the newest prompt sha, per template and overall,
and the same for the sha before it with the delta. Open tasks are not scored.

`pairs` reads one line per graded task: `task_id`, `local_answer`, `better_answer`, `verdict`
and `graded_by`. The verdict is about the local answer. `better` keeps the local answer as
`chosen` and `better_answer` becomes `rejected`. `worse` swaps them. `same` writes nothing.
Unknown task ids and other verdicts are named and skipped. Each pair is a TRL-style line with
message lists for `prompt`, `chosen` and `rejected`, plus `harness` (the provider the task is
about), `graded_by`, `graded_at` and `source: "apprentice"`. Answers are redacted. Running it
twice on the same file does not duplicate lines.

### How the pairs relate to `hydra export`

They do not map. `src/export_preference.rs` builds its pairs from run records, corrections and
evolve passes, behind the sign-off manifest, and reads no external pairs file. Getting these
pairs through `hydra export` needs a change under `src/`, which this ticket does not make. So
`pairs.jsonl` is in the apprentice's own shape, close to the TRL form export writes, and stays
on the machine.

## Templates

Verifiable, graded by `check`:

| Template | Check | Answer |
|---|---|---|
| `outcome` | exact | `success`, `failed`, `refused` or `unfinished`, from `run_end` |
| `provider_model` | one_of | `provider/model`, or the provider alone when no model is recorded |
| `refused_count` | number | error tool results whose text starts `hydra/` (`claude` records only) |
| `first_refused_tool` | exact | the tool behind the first such result (`claude`, at least one refusal) |
| `interactive` | exact | `yes` when `launch_mode` is `interactive` |
| `seats_launched` | number | runs launched by `seat_request` lines with decision `launched` |
| `policy` | exact | the policy name, or `none` |

Open, graded by a panel:

| Template | Asks |
|---|---|
| `run_line` | write the `hydra run` line for this task text |
| `provider_policy` | pick provider and policy and say why in two sentences |
| `summarize` | say in one sentence what this run did (`claude` records with a final text) |
| `refusal_meaning` | say what this refusal line means (`claude` records with a refusal) |

Refusals and final text are read from `claude`'s frame shape only, so the other providers get
no task on them. `outcome` here is a plain reading of `run_end` and can differ from the
projection `hydra show` derives.

## The nightly job

`launchd/co.hephaestuslab.hydra-apprentice.plist` runs `attempt --minutes 30` at 04:00 local and
logs to `$HYDRA_GRADER_HOME/apprentice/launchd.log`. It is a template and nothing installs it.
Replace `/Users/USER` in a copy, then load it:

```
cp integrations/apprentice/launchd/co.hephaestuslab.hydra-apprentice.plist ~/Library/LaunchAgents/
launchctl bootstrap gui/501 ~/Library/LaunchAgents/co.hephaestuslab.hydra-apprentice.plist
```

`501` is the first macOS user's id; `id -u` prints yours. To remove it:

```
launchctl bootout gui/501 ~/Library/LaunchAgents/co.hephaestuslab.hydra-apprentice.plist
rm ~/Library/LaunchAgents/co.hephaestuslab.hydra-apprentice.plist
```

## Tests

```
python3 -m unittest discover -s integrations/apprentice/tests
```

They run on the fixture records, generated records and a stub Ollama. They never touch
`~/hydra`.
