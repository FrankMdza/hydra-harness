# 136 - A worker run is told what done means and reports what it could not confirm first

Type: build
Status: resolved
Blocked by:

## What

Master of Puppets launches a worker with `task.text` as the whole task (`cmd_launch` in
`integrations/puppets/puppets.py`, near line 1709, through `build_run_argv`). Nothing in that
text says when the work is finished, and nothing asks the worker to separate what it measured
from what it assumed. Agents in this repo have reported numbers they never measured (handoff
section 5: composer on build 132), and the panel then has to find that out.

Anthropic's guidance for Opus 5.5 (Addy Osmani, "Getting the most out of Opus 5.5",
2026-09-22) asks for two things this change adds: say what "done" looks like, and "mark
anything you couldn't confirm, and say where you looked".

Test first: the tests below go in and fail before the code changes.

## Do

1. Plan tasks gain an optional `done` field, a string: the check that says the task is finished.
   `prompts/plan.md` shows it in the example task object and asks the planner to fill it with
   something a reviewer can check (a command and its expected result, a file that exists, a
   test that passes). `TaskState` carries it, and the fold reads it from the plan event.
   Older goal logs with no `done` still fold.
2. A function, for example `worker_task_text(task) -> str`, builds what `cmd_launch` passes
   instead of `task.text`: the task text, then a line `Done means: <done>`, or
   `Done means: not stated by the plan; say in your report what you took it to be.` when the
   field is missing, then a closing block that asks the report to start with a section headed
   `Not confirmed` (every claim the run could not check, and where it looked), before what
   changed.
3. `argv_sha256` is taken over the argv that is actually launched, so it covers the new text.
   Check that the `launch_intent` event still matches the spawned argv.
4. Tests in `integrations/puppets/tests/test_puppets.py`, written first and shown failing:
   - a task with `done` produces text containing `Done means: <that string>` and `Not confirmed`;
   - a task without `done` produces the fallback line;
   - a goal log written before this change (no `done` anywhere) still folds;
   - the argv built for launch contains the composed text, not the bare `task.text`.
5. Run the puppets tests: `python3 -m pytest integrations/puppets/tests -q` (or `unittest` if
   pytest is missing; say which). This ticket touches no Rust; do not run cargo.

The judge prompts (`build_diff_seat_prompt`, `prompts/diff.md`, `prompts/choose.md`) are out of
scope.

## Outcome

Run `01a0e70c-f8e8` (claude sonnet, `standard`). The four tests failed before the change and pass
after; the run proved it by reverting its own edits by hand, because the gate refuses `git stash`
inside a worktree (the lock under `.git/worktrees/<id>/`). `argv_sha256` needed no change: it is
taken over the same argv that is spawned. The session changed two things. It cut a claim the run
added to `prompts/plan.md`, that a worker told what done means "reports fewer unmeasured
numbers", which nobody measured. And it fixed the pre-existing failure the run found and left
alone: `test_review_records_judge_verdict_without_opening_hunk` calls `cmd_review`, which chdirs
into a scratch repo that `tearDown` then deletes, so a later test's subprocess inherited a dead
cwd; the test now restores the cwd in its `finally`. Puppets suite 137 of 137 on master.
`python3 -m pytest` fails on this Mac (no pytest for that interpreter); `pytest` on PATH works.
