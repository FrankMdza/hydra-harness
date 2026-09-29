# 146 - A local judge labels the runs, and says how often it agrees with the human

Type: build
Status: resolved
Blocked by:

## What

The grader's `verdict` model cannot train: 5 of 510 dataset rows carry a human verdict, and
`train` wants 10 rows of the rarer class in each of two time slices. The human is labelling a
queue of 170 runs by hand. That leaves 340 rows with no label.

The human delegated the question of who labels them on 2026-09-29. A panel of three vendors voted
(runs `01a0ebb0-5967`, `01a0ebb0-783c`, `01a0ebb2-3574`), and none chose a judge behind a vendor's
API: the project's rule is that neither the dataset nor the records go to any API, and the export
allowlist does not exist. The decision is a local judge, measured against the human's labels, with
the human's labels alone as the fallback.

The local model is `gemma3:1b-it-qat` on Ollama, the one `integrations/local-llm/` already calls.
Nobody knows whether a 1B model can judge a run. This ticket builds the judge and the measure.
It decides nothing about using the labels.

Nothing leaves the machine. The only network call is to Ollama on `127.0.0.1`.

## Do

Tests first in `integrations/grader/tests/test_judge.py`, each shown failing before the change, on
synthetic rows, a synthetic `hydra show` answer and a stub HTTP server standing in for Ollama.
Never the real dataset, which a run cannot read.

1. `integrations/grader/judge.py`, stdlib only. `label [--limit N] [--only-unlabelled]` walks
   `dataset.jsonl` under the grader home. For each row it builds one prompt from the row's
   `task_text` and the run's `result_text` and `diff_stat`, which it reads from the grader's own
   `hydra show` cache through `records.py`, the way `grader.py` does. Both texts pass through
   `redact.py` and are cut to a character budget, the task to 3,000 and the result to 4,000, the
   cut marked in the text.
2. It asks Ollama's `/api/generate` with `format: "json"`, `temperature: 0` and a fixed
   `num_predict`, and reads back `{"needs_correction": true|false, "failure_class":
   "quota|gate|agent|task|none", "reason": "<one sentence>"}`. The URL and the model come from
   `HYDRA_LOCAL_LLM_URL` and `HYDRA_LOCAL_MODEL`, with the defaults `integrations/local-llm/` uses.
3. It appends one line per run to `$HYDRA_GRADER_HOME/judge-labels.jsonl`: `run_id`,
   `needs_correction`, `failure_class`, `reason`, `judge_model`, `prompt_sha256`, `judged_at`. A
   run that judge model and prompt have already labelled is skipped, so a second pass appends
   nothing. An answer that is not the JSON above is written with `needs_correction: null` and the
   raw text under `unparsed`, cut to 500 characters, and counted.
4. A row whose `outcome` is `rate_limited` or `gated` is not sent to the model. It is written with
   `failure_class` `quota` or `gate`, `needs_correction: null` and `judge_model: "rule"`. Those
   are not failures of the agent, and a judge should not spend a call to say so.
5. Ollama down, or the model not pulled, stops the pass with one sentence on stderr and exit 1,
   keeping the lines already written.
6. `agree` reads the dataset's `human_verdict` and the judge's labels and prints, over the runs
   that have both: how many, how many agree (`fixed` against `needs_correction: true`, `applied`
   against `false`), the agreement as a fraction, and the two by two table. With fewer than 50
   such runs it says the number is loose. It prints how many answers were unparsed.
7. `integrations/grader/README.md`: a `## The local judge` section with the two commands, what the
   file holds, and the sentence that nothing reads the labels yet.

Do not change `baseline.py`, `grader.py` or the dataset's schema. The judge's labels stay in their
own file, apart from the human's.

## Done when

- The tests cover items 1 to 6, each failed before the change, and the whole
  `integrations/grader/tests/` directory passes under `.venv/bin/python` if the worktree has it, or
  `python3` if not. Say which you used.
- `judge.py label --help` and `judge.py agree --help` print their usage.
- The report gives the two commands the human runs on the real grader home, and says what the run
  could not check: the real model's answers, and how long a pass over 510 rows takes.

## Outcome

Resolved 2026-09-29. Built by `claude` sonnet on run 01a0ebb5-0ce5 under `standard`, about 1.19M
tokens. The session applied the diff and then changed the prompt. The judge works as a tool and
fails as a judge.

What the run built: `integrations/grader/judge.py` with `label` and `agree`, stdlib only, and 15
tests on synthetic rows, a stub `hydra` and a stub Ollama. The grader suite is 79 of 79 under
`.venv/bin/python`.

**The ticket's premise was false, and the run said so.** The ticket told it to read `result_text`
and `diff_stat` from the grader's `hydra show` cache. That cache keeps the outcome and the token
counts and nothing else, on purpose since the security review, and
`test_the_cache_holds_only_outcome_and_counts` holds it there. The run left the cache alone:
`judge.py` calls `hydra show` for each run and keeps none of the text.

What the session changed. With the prompt the ticket asked for, `needs_correction` and then a
reason, the 1B model contradicted itself in 9 of 18 parsed answers on a pilot of 25 runs: it
answered true and wrote "no correction needed". Its `failure_class` was noise, `gate` for a task
that wrote a file. The session reworded the prompt to ask for the reason first and then for
`result_ok`, the positive of what the file stores. On the same runs: 0 contradictions of 19, 0
unparsed. `parse_answer` takes both shapes, and a label from the model now carries
`failure_class: null`.

Measured on the real grader home, the whole dataset in about seven minutes: 512 lines written, 56
by rule, 16 unparsed. Of the 446 parsed answers from the model, 422 say the result is fine and 24
say it needs a correction.

**It says yes to almost everything.** Three measures, all loose and all pointing the same way:

- Of 12 runs whose outcome is `failed`, it marks 3.
- Of the 5 runs with a human verdict, it agrees on 3, and marks neither of the 2 `fixed`.
- Against the 25 hand labels of build 134's probe, for the 15 tickets the verdict queue maps to one
  run: it agrees on 3 of 15. All 12 `changed` diffs it calls fine. Answering "needs a correction"
  every time would score 12 of 15.

Part of that is the input and not the model. The judge reads the agent's own closing text, which
says the work is done, and a diff stat. The label `changed` came from a session reading the diff.
Build 134 measured Jev ranking those same diffs with an AUC of 0.964.

So the decision this ticket served is reversed by its own measure: map decision D1 of 2026-09-29
said to fall back on the human's labels when the local judge agrees on fewer than 43 of 50. It
does not reach 4 of 15. Nothing reads `judge-labels.jsonl`, and nothing should.
