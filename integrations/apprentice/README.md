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
3. `attempt` asks the local model each task this model has not answered under the current prompt.
4. Verifiable answers are marked right or wrong on the spot.
5. `score` prints one block per model: the verifiable score per template, and against that model's previous prompt.
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
python3 integrations/apprentice/apprentice.py score [--model NAME]
python3 integrations/apprentice/apprentice.py pairs <graded.jsonl>
```

`bank build` writes `bank.jsonl`: up to `n` verifiable and `n` open tasks. It reads each record
directly and does not call `hydra show`, so it needs no binary. For each template the runs are
ordered by a hash of the run id, and templates take turns, so the same records give the same
bank in any listing order. With fewer matching records than `n` you get fewer tasks. Prompts are
redacted with `redact.py` and each record excerpt is cut to 3,000 characters. `redact.py`,
`records.py` and `models.py` were the grader's; build ticket 220 moved them here when it
deleted the rest of the grader's Python.

`attempt` appends to `attempts.jsonl`: task id, template, prompt sha, answer, reason, model,
seconds and, for verifiable tasks, `correct`. The prompt asks for the reason first and the
answer second, as JSON, at temperature 0 with `num_predict` 300. The prompt sha is the sha-256 of
`INSTRUCTIONS`, so editing the prompt makes every task due again. Attempts are kept per model:
a task is skipped only when this `HYDRA_LOCAL_MODEL` already has a line for that task and prompt
sha, so a second model runs the whole bank in the same home. `--minutes` stops the pass
between two tasks once the budget is spent. If Ollama is down or lacks the model, the pass prints
one line, exits 1 and keeps the attempts already written. A reply that is not the JSON shape is
stored with an empty answer and an `unparsed` field, and a verifiable one counts as wrong.

`score` prints one block per model that has attempts under the newest prompt sha: the latest
attempt per task, per template and overall, and the same numbers for that model's previous prompt
sha with the delta. `--model NAME` prints one block. Open tasks are not scored.

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

## BitNet as the second apprentice

Build ticket 161. `bitnet_serve.py` puts the `bitnet.cpp` binary behind the same endpoint the
apprentice already calls. It answers `/api/generate` with `{"response": ...}` and
`/v1/chat/completions` in the OpenAI shape, runs one binary call per request, and serves one
request at a time. It reads `BITNET_BIN`, `BITNET_MODEL` and `BITNET_THREADS`. It ignores Ollama's
`format: "json"` field, because the binary has no such switch.

```
python3 integrations/apprentice/bitnet_serve.py --port 11435
HYDRA_LOCAL_LLM_URL=http://127.0.0.1:11435/api/generate HYDRA_LOCAL_MODEL=bitnet-2b \
  python3 integrations/apprentice/apprentice.py attempt
```

Attempts are kept per model. `attempt` skips a task only when this model already has a line for
that task and prompt sha, so BitNet and Gemma share one bank and one `attempts.jsonl`. `score`
prints one block per model, and `score --model bitnet-2b` prints BitNet's block alone.

**The binary under `~/models/BitNet/build/bin` does not work yet.** It is mainline llama.cpp
b9918, not the bitnet.cpp kernels. It reports the file as `Q1_0`, crashes on the Metal backend
(`Asserting on type 36`), and on the CPU (`-ngl 0`) generates `@@@@@@...` for every prompt. On
2026-09-30, on the 8-core M2 with 8 GB and other agents running (load average about 7), three
runs of a 200-token answer with 4 threads gave:

| Measure | Runs | Median |
|---|---|---|
| Generation, tokens per second | 3.6, 2.6, 3.1 | 3.1 (`llama-bench` tg200: 3.20 +/- 0.25) |
| Peak resident memory | 1.57, 1.50, 1.52 GB | 1.52 GB |
| Load time, model to first token | 9.4, 9.8, 8.6 s | 9.4 s |

The speed and memory are those of a run that emits garbage, so treat them as a floor until the
binary is rebuilt with `setup_env.py` from the BitNet checkout. A score of that model on the bank
would only measure the bug, so none is reported. The Gemma score was not taken either, because
Ollama was not running.

## What training BitNet would take

Sources: the model card of `microsoft/bitnet-b1.58-2B-4T-bf16` on Hugging Face, and the README
and `gpu/README.md` of `microsoft/BitNet`. Numbers marked "estimate" are mine.

- **Which weights.** The model card says the bf16 repository "contains the master weights in BF16
  format" and to "use this only for training or fine-tuning purposes". The ternary `i2_s` gguf
  that runs here is the packed inference form and cannot be trained. Fine-tuning starts from bf16
  and ends with a fresh conversion (`utils/convert-helper-bitnet.py`, README "Convert from
  .safetensors checkpoints") back to gguf.
- **Recipe.** The card names three stages for the released model: pre-training, supervised
  fine-tuning on instruction and conversation data, and DPO on preference pairs. It gives no
  hyperparameters and no GPU-hours. Our pairs from `pairs` fit the last stage. The ternary
  quantization happens in the forward pass during training, which the card and the README do not
  describe beyond pointing to the "Training Tips, Code, FAQ" PDF in `microsoft/unilm`. I did not
  read that PDF.
- **License.** MIT, per the card.
- **Hardware, estimate.** bf16 weights of 2.4B parameters are 4.8 GB. Full fine-tuning with Adam
  needs about 16 bytes a parameter, 38 GB, which does not fit a 24 GB 3090. LoRA on the bf16
  weights fits.
- **Cost, estimate.** Take 25 million tokens (50,000 examples of 500 tokens), 6ND for 2.4B
  parameters, about 3.6e17 FLOPs, and a 3090 sustaining 10 TFLOPs. That is about 10 hours, about
  2.20 USD at 0.22 USD an hour. Even at three times the time for a poor run it stays under 10 USD.
  Research 77 costs a from-scratch pretrain at about 67,000 USD, and its cheapest A100 is
  1.19 USD an hour and its cheapest H100 1.99. The ticket names research 85 for prices, but that
  file is about the cockpit and holds none. The 0.22 USD figure comes from the ticket text and I
  did not check it.
- **Blocker.** The pairs do not exist yet: nothing trains on them until a panel has graded open
  answers, and the bank scores need a working binary first.

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
