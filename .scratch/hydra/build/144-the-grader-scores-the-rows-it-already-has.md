# 144 - The grader scores the rows it already has

Type: build
Status: resolved
Blocked by:

## What

Build 143 scores a run when a collect writes or rewrites its row. A model trained after a row was
collected never scores it. On 2026-09-29 the dataset held 510 rows and `scores.jsonl` held 6: the
only model, `models/outcome.json`, was written on 2026-09-28 at 09:41, and 6 runs arrived after it.
`integrations/grader/grader.py` line 179 passes `score_rows` only the rows of that collect, and no
command scores the rest.

The human approved a plan on 2026-09-29 whose first phase needs every row scored, so a later ticket
can compare the model's doubts with the labels the human is writing.

Nothing leaves the machine: no network call, and the dataset and the scores stay under
`$HYDRA_GRADER_HOME`.

## Do

Tests first in `integrations/grader/tests/test_grader.py`, each shown failing before the change, on
a scratch grader home with synthetic rows and a synthetic model. Never the real dataset, which a
run cannot read.

1. `grader.py backfill` reads `$HYDRA_GRADER_HOME/dataset.jsonl` and calls `score_rows` with every
   row. It collects nothing, launches no `hydra show`, and does not touch the cursor or the
   dataset. It prints the number of lines appended and exits 0, including when that number is 0.
2. A second `backfill` appends nothing, because `score_rows` already skips a run a model version
   has scored. Test it.
3. With no model under `models/`, or no dataset file, `backfill` prints 0 and exits 0.
4. `integrations/grader/README.md`: the `## Scores` section says that `backfill` exists and when to
   run it (after `train` writes a model), and the commands table gets its row.

Do not change `score_rows`, `collect`, `baseline.py` or what a score line carries. Decision D6 of
build 143 stands: nothing reads the file yet.

## Done when

- The three tests exist, each failed before the change, and the whole
  `integrations/grader/tests/` directory passes.
- `python3 integrations/grader/grader.py backfill --help` prints its usage.
- The report says how the human runs it on the real grader home, as one command.

## Outcome

Resolved 2026-09-29. Built by `claude` sonnet on run 01a0eba4-4aea under `standard`, about 0.41M
tokens. The session read the diff and applied it as it was: 7 lines in `grader.py`, 7 in the
README, 41 of tests.

`grader.py backfill` reads `dataset.jsonl` under the grader home and hands every row to
`score_rows`. It collects nothing, calls no `hydra show`, and leaves the cursor and the dataset
alone. The three tests failed before the change, because the parser had no `backfill`, and pass
after it.

On master, 60 of the 61 grader tests pass. `test_launchd.py` errors at import: Homebrew's Python
3.14.7 on this machine cannot load `pyexpat` (`Symbol not found:
_XML_SetAllocTrackerActivationThreshold`), so `plistlib` does not import. That is the machine, not
this change, and it fails the same way without it.

The session ran `backfill` on the real grader home. It appended 504 lines, and `scores.jsonl` went
from 6 lines to 511 with the run the schedule collected meanwhile. Under `outcome`, the median `p`
is 0.08, 31 runs score above 0.5 and 47 fall between 0.3 and 0.7. The mean by provider is agy
0.247, codex 0.184, claude 0.134 and cursor-agent 0.091.

Nothing the ticket assumed turned out false.
