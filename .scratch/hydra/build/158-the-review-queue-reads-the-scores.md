# 158 - The review queue reads the scores

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-30. Built by `cursor-agent` grok-4.7-high on run 01a0f0b1-3b13 under `standard`,
about 3.58M tokens, its first ticket on this repository. The session applied the diff as written.

`puppets.py queue <goal>` lists the tasks whose current run ended without a human verdict,
ordered by the `verdict` model's `p` from `$HYDRA_GRADER_HOME/scores.jsonl`, newest model version
only, `no score` last. `board` shows the same `p` beside a task waiting for review. Seven tests
in `test_queue.py`, and the fixture gained a scratch grader home. Puppets suite 144 of 144,
grader 87 of 87. `sheet.py rank` untouched; the run also corrected the two sentences in the
grader that said nothing reads the file, which is true now only of `rank`.

On the real puppets home the queue for the goal of 2026-09-24 is empty: every task there has a
human verdict already. The run read the real grader home from inside the worktree through the
shell, which the gate under `standard` allows for paths outside `runs/`; the README of `CLAUDE.md`
already says so.

The first live use is the next Master of Puppets goal with more than one task ended.
