# 165 - An outcome label without quota or gate

Type: build
Status: resolved
Blocked by:

## What

Step 1.5 of the human's plan. `baseline.py target(row, "outcome")` counts every outcome other
than `completed` as the thing to avoid (`baseline.py:51-56`). On 2026-09-30 the test slice held
29 positives: 15 `rate_limited`, 10 `gated`, 3 `failed`, 1 `skill_collision`. A model that learns
which provider is out of quota or which policy refuses beats the bar without learning anything
about the task. Build 163 now names a cause for each of those runs in `triage.jsonl`.

## Do

Tests first in `integrations/grader/tests/`, on synthetic rows, each shown failing before the
change. Stdlib only.

1. A third label in `baseline.LABELS`, `task`: 1 for `failed`, `crashed`, `timed_out`; 0 for
   `completed`; `None` (left out) for `rate_limited`, `gated`, `skill_collision`, `auth_failed`
   and any other outcome. Write the list once, as a constant, and name it in the docstring.
2. `baseline.py train --label task` and `monitor.py train --label task` work with no other
   change beyond what the label needs; `grader.py` scores a `task` model like the other two.
3. The refusal floor (`MIN_MINORITY`) applies as for the others. If the real data cannot meet
   it, `train` refuses and says so; that is an expected result, not a bug.
4. `integrations/grader/README.md`: the label, why it exists, and the outcome list.

## Done when

- Each new test failed before the change; the grader suite passes.
- The report gives the train command for the real home and says it could not run it there.

## Outcome

Resolved 2026-09-30. Built by `claude` sonnet on run 01a0f2c1-0e88 under `standard`. The session
applied it as written. `baseline.py` gains the label `task` and the constant `TASK_FAILURES`
(`failed`, `crashed`, `timed_out`); `completed` is 0 and every other outcome is left out.
`POSITIVE` replaces the inline "not completed or fixed" in both `baseline.py` and `monitor.py`,
so `monitor.py train --label task` works too. Three tests; the grader suite passes, 107 tests.

On the real dataset `train --label task` refuses, as the ticket allowed: 507 rows carry the
label, and the test slice from 2026-09-23 has 248 of them with 3 positives, under the floor of 10.
The whole dataset holds 13 `failed` and 1 `crashed` against 493 `completed`. That is the number
behind phase 1's exit condition: the runs rarely fail for a reason in the task, so this label
waits for more failures, or for Frank's verdicts, before a model can exist.

