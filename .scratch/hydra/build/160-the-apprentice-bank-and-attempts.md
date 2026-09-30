# 160 - The apprentice: a bank of tasks about records, the local model's attempts, and the pairs

Type: build
Status: resolved
Blocked by:

## What

Map ticket 87. Read its `## Answer` first. Then `integrations/grader/README.md` and `records.py`
(how to walk records and call `hydra show`), `integrations/local-llm/hydra-local-llm` (how Ollama
is called), `integrations/grader/judge.py` (build 146: the prompt shape that made the 1B model
stop contradicting itself, reason first and then the answer), `docs/run-record-contract.md`,
and `src/export_preference.rs` for the shape of a preference pair.

## Do

A new directory `integrations/apprentice/`, Python, stdlib only, nothing under `src/`. Tests in
`integrations/apprentice/tests/` on the fixture records under `tests/fixtures/records/` and a
stub Ollama, each shown failing before the change.

1. **`apprentice.py bank build [--n 20]`** writes `bank.jsonl` under `$HYDRA_GRADER_HOME/apprentice/`
   from the records in `$HYDRA_ROOT/runs/`: `n` verifiable tasks and `n` open tasks, seeded by
   run id so the same records give the same bank. A verifiable task carries `prompt`, `answer`,
   `check` (`exact`, `number`, `one_of`) and the run id it came from; the prompt shows the
   record's relevant lines redacted with `integrations/grader/redact.py` and cut to 3,000
   characters. Templates, at least six verifiable and four open, documented in the README:
   the run's outcome; the provider and model; the number of tool calls the gate refused and the
   first refused tool; whether the run was interactive; how many seats a `seat_request`
   launched; the policy; and for open tasks: write the `hydra run` line for this task text;
   pick provider and policy for this task and say why in two sentences; say in one sentence
   what this run did; say what this refusal line means.
2. **`apprentice.py attempt [--limit N] [--minutes M]`** asks the local model each task not yet
   attempted with the current prompt sha, through Ollama (`HYDRA_LOCAL_LLM_URL`,
   `HYDRA_LOCAL_MODEL`, the defaults of `integrations/local-llm/`), with the reason-first JSON
   shape, `temperature 0`, a `num_predict` cap, and stops at the minute budget. It appends to
   `attempts.jsonl`: task id, prompt sha, answer, reason, model, seconds, and for verifiable
   tasks `correct: true|false`. Ollama down stops the pass with one line and exit 1, keeping
   what was written.
3. **`apprentice.py score`** prints, per template and overall, the verifiable score of the latest
   attempt per task, and the same for the previous prompt sha if there is one, so a prompt change
   shows as a delta.
4. **`apprentice.py pairs <graded.jsonl>`** reads a file the session writes after a panel grades
   the open answers (one line per task: task id, `local_answer`, `better_answer`, `verdict`
   `better|same|worse`, `graded_by`) and writes `pairs.jsonl`: prompt, chosen, rejected,
   harness (the provider the task is about, or `hydra`), graded_by, graded_at. Only `better`
   and `worse` become pairs. Say in the README how this maps onto what `export_preference.rs`
   reads, and if it cannot map without a change under `src/`, say so and write the file in your
   own shape.
5. **A launchd template** `integrations/apprentice/launchd/co.hephaestuslab.hydra-apprentice.plist`,
   04:00 local, `attempt --minutes 30`, log under the apprentice home. Do not install it.
6. **`integrations/apprentice/README.md`**: the loop in ten lines, the commands, the templates,
   the nightly job and how to remove it, and the sentence that nothing trains on the pairs yet.

## Done when

- Tests for the bank's determinism and each template on the fixtures, for `attempt` against a
  stub Ollama including the minute budget and the down case, for `score`, and for `pairs`; each
  failed before the change; the suite passes under `.venv/bin/python` if the worktree has it,
  else `python3`.
- The report gives the three commands the human runs on the real home, in order, and what the
  run could not measure (the real model's answers; how long 40 tasks take on the M2).

## Outcome

Resolved 2026-09-30. Built by `claude` sonnet on run 01a0f0c6-2463 under `standard`, about 1.54M
tokens. The session applied it as written. `integrations/apprentice/`: `bank build`, `attempt`,
`score`, `pairs`, a launchd template, the README, 31 tests on the fixture records and a stub
Ollama. Seven verifiable templates and four open ones. The suite passes under `.venv/bin/python`.

The session built the real bank: 20 verifiable and 20 open tasks from 563 records, in
`grader/apprentice/bank.jsonl`. `attempt` against the real model waits for a moment with no
agent alive, since Ollama beside the agents put the swap at 17 GB on 2026-09-29.

Two things the run said: `bank build` reads `run_end` itself and does not call `hydra show`, so
its outcome names (`success`, `failed`, `refused`, `unfinished`) are not the derived ones; and it
read `better|same|worse` as a verdict on the local answer, `worse` swapping in the panel's, which
is what the ticket meant. Nothing trains on the pairs.
