# 158 - The review queue reads the scores

Type: build
Status: claimed
Blocked by:

## What

Build 143's decision D6 left `scores.jsonl` unread: "nothing reads the file yet, so the model does
not pick which runs get reviewed". Since 2026-09-29 there are two models, `outcome` and
`verdict`, and 1,106 score lines. The `verdict` model is the one that says whether a diff will
need a correction (AUC 0.71 on 47 test rows). The human reviews diffs one task at a time with
`puppets.py review <goal> <task>` and gives verdicts with `puppets.py verdict`.

Phase 1.7 of the human's plan: the scores order the reviews. This ticket reads the file; it
changes no assignment and launches nothing.

## Do

Tests first in `integrations/puppets/tests/`, on a scratch puppets home and a scratch grader
home, each shown failing before the change.

1. `puppets.py queue <goal-id>`: the tasks of the goal whose current run has ended and has no
   human verdict yet, ordered by the `verdict` model's `p` for that run, highest first, from
   `$HYDRA_GRADER_HOME/scores.jsonl` (the newest `model_version` per run wins). A task with no
   score sorts last, marked `no score`. Columns: task id, provider, model, run id, `p`, and
   `attempt`. One line at the top says which model version the scores came from and how many
   tasks have none.
2. `board` gains, in its task grid, the same `p` beside a task that waits for a review, so the
   cockpit and `hydra-board` show it without another command. Keep the column short.
3. `README.md` of puppets: the command, and the sentence that the order is a suggestion and
   the human still reviews every task.

Nothing writes `scores.jsonl`. Nothing in `sheet.py rank` reads it: that is the part of D6 that
stands, and the report says so.

## Done when

- The tests, each failed before the change; the puppets suite passes.
- On the real puppets home, `puppets.py queue <the goal of 2026-09-24>` runs and the report
  quotes its first lines, or says why it could not (a run cannot read the real grader home, so
  say that and quote the scratch run instead).
