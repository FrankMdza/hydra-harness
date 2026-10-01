# 93 - Where the grader reads a goal once `puppets.py` is gone

Type: task
Status: open
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
