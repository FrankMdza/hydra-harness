# 164 - Apprentice attempts are kept per model

Type: build
Status: claimed
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
