# 93 - Where the grader reads a goal once `puppets.py` is gone

Type: task
Status: resolved
Blocked by:
Opened: 2026-10-01, by the session, from build ticket 196. The cutover run rewired every caller
of `puppets.py` to `hydra-puppets`, then stopped before deleting anything: the grader imports
three of the four Python files, which the ticket did not foresee.

## Question

Map ticket 90 ordered the port: `puppets` first, its Python deleted before the next piece, then
`grader`. But `integrations/grader/` (4,838 lines of Python, run from launchd, its suite in CI)
loads three files of `integrations/puppets/` by path:

- `labels.py` imports `puppets.py` (2,765 lines) to call its `fold` and read goal logs, for the
  human verdicts that become training labels.
- `records.py` imports `models.py` (21 lines: `load_aliases`, `canonical`).
- `baseline.py` imports `jev_probe.py` (415 lines), which was never part of the port.

And `sheet.py` (423 lines) keeps three commands the Rust did not port (`build`, `budget`,
`show`), which a human or the session runs by hand; they call `hydra show` per run.

Since build 196's first half nothing but the grader, those hand-run commands and the
differential tests uses the Python. While `puppets.py` stays there are two folds of a goal log,
and the differential tests are what keeps them equal.

Options:

- A. The grader shells out: `labels.py` runs `hydra-puppets show <goal>` and reads the JSON, as
  the cockpit runs the helper. `models.py` and `jev_probe.py` move into `integrations/grader/`,
  which is who uses them. `sheet.py`'s three commands are ported to the crate. Then `puppets.py`
  and its tests go, and the differential tests become fixtures of recorded Python output.
- B. As A, but `sheet.py` and its three commands stay in Python under `integrations/puppets/`
  until the grader's own port, since they are closer to the grader than to the helper.
- C. Leave the Python where it is until the grader is ported to Rust, and port the grader next.
  Two folds stay for that long, kept equal by the differential tests in CI.
- D. Copy the 400 or so lines of `fold` and `read_log` the grader needs into
  `integrations/grader/`, delete `puppets.py`, and accept a second fold in Python that nothing
  compares with the Rust.

## Answer

Resolved 2026-10-01 by a panel of three under the human's delegation. Unanimous: **B**. agy
gemini-3.1-pro-high 0.80, claude opus 5.5 0.48, cursor-agent grok-4.7-high 0.61; all three named
A second. `codex` was out of quota, so the Opus seat took its place. Answers in the human's
decision log, `decisiones/2026-10-01/answer-93-*.md`.

`labels.py` gets a goal's state from `hydra-puppets show <goal>`, as the cockpit gets its steps
from the helper. `jev_probe.py` moves into `integrations/grader/`. `sheet.py` and its three
hand-run commands stay in Python until the grader's own port: they hold no fold, and porting 423
lines first would only delay reaching one fold. Then `puppets.py` and its tests go, and the
differential tests become fixtures of recorded Python output.

The session checked the two premises the seats marked unknown. `sheet.py` loads `models.py` and
not `puppets.py`, so B can delete `puppets.py`; it also means `models.py` (21 lines) stays where
it is, since `sheet.py` and the grader both load it. And `labels.py` reads from the fold only
fields `show` prints (`goal.text`, each task's `id`, `class`, `grade`, `text`, `run_id` and
`human_result`, each plan's `run_id`); it also walks the raw events for `judge_run_ids`, with a
reader of its own that still skips a bad last line.

Conditions the seats attached, which build 196's second half carries:

- Before `puppets.py` is deleted, in the same step, the grader's labels are computed both ways,
  through the Python fold and through the shell-out, over every goal log there is (the real ones
  and the ones the differential tests harvest), with the router stopped. Zero differences, or
  the deletion does not happen and the question comes back as C (all three).
- The shell-out fails loudly (opus). A missing binary, a non-zero exit or JSON that does not have
  the expected shape aborts the grader run with no labels written. It never falls back to empty
  or cached labels. A damaged goal log is such a failure.
- The binary is named by an absolute path, not found on `PATH` (opus), and the launchd job can
  run it with Hydra stopped (agy, cursor).
- The grader checks the shape of what it reads (agy).
- The recorded fixtures include goal logs with human verdicts, since verdicts are what become
  labels (opus).

What reverses it: one goal on which the shell-out and the Python fold give different labels, or
`hydra-puppets show` failing from the launchd environment (then C); more than 250 ms per goal
from launchd (agy).

