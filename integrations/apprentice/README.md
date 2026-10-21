# hydra-apprentice

A bank of tasks about Hydra run records, the local model's attempts at them, and the preference
pairs a panel's grading leaves behind. Map ticket 87 is the design, build ticket 160 the first
code. The apprentice is `hydra-apprentice`, the Rust binary of `crates/apprentice`, built beside
`hydra` by `cargo build --release --workspace`. Nothing under `src/` imports it, and it makes one
kind of network call, to the model server at `HYDRA_LOCAL_LLM_URL` on this machine's loopback.

Build ticket 234 deleted the Python it was ported from on 2026-10-04: `apprentice.py`,
`records.py`, `models.py`, `redact.py`, `bitnet_serve.py` and their four test files. Before it
went, `crates/apprentice/tests/record_*.py` ran it on every input of the crate's comparison tests
and wrote its output under `crates/apprentice/tests/fixtures/`. The recorders' first lines say
how to run them against a checkout from before that ticket. This directory keeps what a human
installs and reads: this file and the launchd template.

**Nothing trains on the pairs yet.** Phase 3 of the plan decides when a LoRA is worth a rented
GPU. The number to watch until then is the verifiable score.

## The loop

1. `bank build` turns the records in `$HYDRA_ROOT/runs/` into tasks.
2. Verifiable tasks carry the answer the code computed from the record. Open tasks carry none.
3. `attempt` asks the local model each task this model has not answered under the current prompt.
4. Verifiable answers are marked right or wrong on the spot.
5. `score` prints one block per model: the verifiable score per template, and against that model's previous prompt.
6. Change the prompt (`INSTRUCTIONS` in `crates/apprentice/src/attempt.rs`), rebuild, run `attempt` again, read the delta.
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
hydra-apprentice bank build [--n 20]
hydra-apprentice attempt [--limit N] [--minutes M]
hydra-apprentice score [--model NAME]
hydra-apprentice pairs <graded.jsonl>
```

The binary is `target/release/hydra-apprentice` after the release build; nothing puts it on
`PATH`.

`bank build` writes `bank.jsonl`: up to `n` verifiable and `n` open tasks. It reads each record
directly and does not call `hydra show`, so it needs no other binary. For each template the runs
are ordered by a hash of the run id, and templates take turns, so the same records give the same
bank in any listing order. With fewer matching records than `n` you get fewer tasks. Prompts are
redacted with the redactor of `crates/records` and each record excerpt is cut to 3,000
characters.

`attempt` appends to `attempts.jsonl`: task id, template, prompt sha, answer, reason, model,
seconds and, for verifiable tasks, `correct`. The prompt asks for the reason first and the
answer second, as JSON, at temperature 0 with `num_predict` 300. The prompt sha is the sha-256 of
`INSTRUCTIONS`, so editing the prompt makes every task due again. Attempts are kept per model:
a task is skipped only when this `HYDRA_LOCAL_MODEL` already has a line for that task and prompt
sha, so a second model runs the whole bank in the same home. `--minutes` stops the pass
between two tasks once the budget is spent. If the model server is down or lacks the model, the
pass prints one line, exits 1 and keeps the attempts already written. A URL naming a host other
than this machine's loopback is refused the same way before any request; the Python connected
to it. A reply that is not the JSON shape is stored with an empty answer and an `unparsed`
field, and a verifiable one counts as wrong.

`score` prints one block per model that has attempts under the newest prompt sha: the latest
attempt per task, per template and overall, and the same numbers for that model's previous prompt
sha with the delta. `--model NAME` prints one block. Open tasks are not scored.

`pairs` reads one line per graded task: `task_id`, `local_answer`, `better_answer`, `verdict`
and `graded_by`. The verdict is about the local answer. `better` keeps the local answer as
`chosen` and `better_answer` becomes `rejected`. `worse` swaps them. `same` writes nothing.
Unknown task ids and other verdicts are named and skipped. Both answers must be recorded
attempts. `pairs` reads `attempts.jsonl` beside `bank.jsonl`, and a row whose `local_answer` or
`better_answer` that file does not hold for its task is named and skipped. Without
`attempts.jsonl` every row is skipped. So `chosen` and `rejected` hold only text that `attempt`
wrote, never the panel's own words. Each pair is a TRL-style line with
message lists for `prompt`, `chosen` and `rejected`, plus `harness` (the provider the task is
about), `graded_by`, `graded_at` and `source: "apprentice"`. Answers are redacted. Running it
twice on the same file does not duplicate lines.

### How the pairs relate to `hydra export`

They do not map. `src/export_preference.rs` builds its pairs from run records, corrections and
evolve passes, behind the sign-off manifest, and reads no external pairs file. Getting these
pairs through `hydra export` needs a change under `src/`, which no ticket has made. So
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

Build ticket 161 wrote `bitnet_serve.py`, a shim that put the `bitnet.cpp` binary behind the
endpoint the apprentice calls. Map ticket 95 dropped it rather than port it, because the binary
it ran does not work, and build ticket 234 deleted it. Any server on loopback that answers
`/api/generate` with `{"response": ...}` can still be the second apprentice:

```
HYDRA_LOCAL_LLM_URL=http://127.0.0.1:11435/api/generate HYDRA_LOCAL_MODEL=bitnet-2b \
  target/release/hydra-apprentice attempt
```

Attempts are kept per model, so BitNet and Gemma share one bank and one `attempts.jsonl`.
`score --model bitnet-2b` prints BitNet's block alone.

**The binary under `~/models/BitNet/build/bin` did not work on 2026-09-30.** It is mainline
llama.cpp b9918, not the bitnet.cpp kernels. It reports the file as `Q1_0`, crashes on the Metal
backend (`Asserting on type 36`), and on the CPU (`-ngl 0`) generates `@@@@@@...` for every
prompt. On the 8-core M2 with 8 GB and other agents running (load average about 7), three runs
of a 200-token answer with 4 threads gave:

| Measure | Runs | Median |
|---|---|---|
| Generation, tokens per second | 3.6, 2.6, 3.1 | 3.1 (`llama-bench` tg200: 3.20 +/- 0.25) |
| Peak resident memory | 1.57, 1.50, 1.52 GB | 1.52 GB |
| Load time, model to first token | 9.4, 9.8, 8.6 s | 9.4 s |

The speed and memory are those of a run that emits garbage, so treat them as a floor until the
binary is rebuilt with `setup_env.py` from the BitNet checkout. A score of that model on the bank
would only measure the bug, so none is reported.

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

`launchd/co.hephaestuslab.hydra-apprentice.plist` runs
`target/release/hydra-apprentice attempt --minutes 30` at 04:00 local and logs to
`$HYDRA_GRADER_HOME/apprentice/launchd.log`. It has no `RunAtLoad`, so loading it starts no
pass. It is a template and nothing installs it. To install it, or to replace a copy installed
before build ticket 234, build the workspace and write the template with your home filled in:

```
cargo build --release --workspace
launchctl bootout gui/$(id -u)/co.hephaestuslab.hydra-apprentice; sed "s|/Users/USER|$HOME|g" integrations/apprentice/launchd/co.hephaestuslab.hydra-apprentice.plist > ~/Library/LaunchAgents/co.hephaestuslab.hydra-apprentice.plist && launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/co.hephaestuslab.hydra-apprentice.plist
```

`bootout` prints an error when no job is loaded, and the rest still runs. A copy installed
before build ticket 234 runs `/usr/bin/python3 integrations/apprentice/apprentice.py`, a file
that no longer exists, and fails every night until it is replaced.

To remove it:

```
launchctl bootout gui/$(id -u)/co.hephaestuslab.hydra-apprentice && rm ~/Library/LaunchAgents/co.hephaestuslab.hydra-apprentice.plist
```

## Tests

```
cargo test -p hydra-apprentice
```

`bank.rs`, `attempt.rs` and `pairs.rs` run the built binary on every recorded case and compare
its exit code, output and files with what the Python gave; `attempt.rs` answers with a stub
model server of its own. `launchd.rs` reads the template. No test starts `python3`, contacts a
real model server or touches `~/hydra`.
