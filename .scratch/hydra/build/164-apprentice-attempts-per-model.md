# 164 - Apprentice attempts are kept per model

Type: build
Status: resolved
Blocked by:

## What

Found by build 161. `integrations/apprentice/apprentice.py attempt` skips a bank task when
`attempts.jsonl` already holds an attempt with the same `(task_id, prompt_sha)`, whatever the
model (`apprentice.py:404`). So a second model, BitNet or any other, writes nothing on a bank
Gemma already answered. `score` (near line 452) reads rows by `prompt_sha` alone, so two models
would also be mixed in one number. The README's workaround today is a second
`HYDRA_GRADER_HOME` with a copied bank.

## Do

Tests first in `integrations/apprentice/tests/test_apprentice.py`, with the stub Ollama the file
already has, each shown failing before the change. Stdlib only.

1. `attempt` keys what is done on `(task_id, prompt_sha, model)`, where `model` is the value the
   attempt row already stores (the `HYDRA_LOCAL_MODEL` it ran with). A second model runs the
   whole bank; the same model twice still skips.
2. `score` prints one block per model under the current prompt sha, and `--model NAME` narrows
   it to one. The existing comparison against the previous prompt sha stays per model.
3. `README.md` (the apprentice one): drop the second-home workaround from the BitNet section and
   say the attempts are per model.

## Done when

- Each new test failed before the change; the apprentice suite passes under `.venv/bin/python`
  if the worktree has it, else `python3` (if `plistlib` fails to import on Python 3.14, say so
  and run the other files).
- The report says what `score` printed on the test fixture for two models.

## Outcome

Resolved 2026-09-30. Built by `cursor-agent` grok-4.7-high on run 01a0f2c0-acd2 under
`standard`. The session applied it as written. `attempt` keys done work on `(task_id,
prompt_sha, model)`; `score` prints one block per model at the newest prompt sha, each against
its own previous sha, and `--model NAME` narrows it. Two tests; the apprentice suite passes, 38
tests. All 40 real attempts carry `model`, and `score` on the real home prints the one block,
`gemma3:1b-it-qat` 13 of 20. The README's second-home workaround is gone.

