# 90 - The integrations in Rust

Type: task
Status: claimed
Blocked by:
Opened: 2026-09-30, by the session, at the human's request ("is there a way to make all of those
Rust based?").

## Question

`src/` is all Rust (74,861 lines tracked). `integrations/` holds 15,626 lines of Python, stdlib
only except the dragon's renderer (numpy): `puppets` 7,024, `grader` 4,838, `herdr` 1,847,
`apprentice` 1,379, `otel` 303, `local-llm` 235; plus `.scratch/hydra/frontier.py` and logo
scripts. About 8,000 of the Python lines are tests. The rule since the plan of 2026-09-29: the
learning loop lives outside `src/` and Hydra works with it stopped. The cockpit already
reimplements `puppets.py`'s `fold` in Rust (`src/cockpit/goal.rs`) and shells out to
`puppets.py` for `pending`, `approve` and `review`, and ports `sheet.py`'s window share. Where to
put the Rust, in what order, how to know a port changed nothing, and what stays Python.

## Answer

(pending: panel of three and a judge, under the human's delegation; the human can revert)
