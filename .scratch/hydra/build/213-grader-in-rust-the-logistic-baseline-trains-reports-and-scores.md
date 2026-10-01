# 213 - grader in Rust: the logistic baseline trains, reports and scores

Type: build
Status: resolved
Blocked by: 211, 212

## What

Map ticket 94, the grader's port. This is "plan 211" of `.scratch/hydra/research/94-the-grader-port-plan.md`: read that section whole,
and "What puppets taught" above it. It names the Python files and functions to port, the inputs
the differential test compares on, what must be identical, what is normalized, and the one thing
most likely to make Python and Rust differ.

Map 94: byte for byte with CPython 3.14.7. Build 211 measured `exp`, `log`, `log1p` and the
compensated `sum`; read its outcome first and use `pyjson::math`.

## Do

1. Port what the plan's section names into `crates/grader` (package `hydra-grader`, binary
   `hydra-grader`), with the Python's arguments, output and exit codes. The crate depends on
   `hydra-pyjson` and not on `hydra-puppets` or `hydra`.
2. Record the Python once: `crates/grader/tests/record.py` runs the Python on every input of this
   ticket from an empty environment, with frozen data files, and writes
   `crates/grader/tests/fixtures/`. The Rust tests compare with those files and start no
   `python3`. Follow `crates/puppets/tests/record.py` and `differential.rs`. Record with the
   `python3` on `PATH`, which is 3.14.7, and write its version into the fixtures.
3. Break the Rust on purpose once (one constant, one word) and say how many comparisons fail.
4. The Python stays. `IMPLEMENTATION.md`: a passage on what this ticket ported and what it
   normalizes.

## Done when

- The report lists each command and input compared, what was normalized and why, and what the
  plan assumed that turned out false. fmt and clippy clean over the workspace; `cargo test --lib`
  and `cargo test -p hydra-grader` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f6d9-5449` (claude opus 5.5, `standard`, 7.0M tokens). The session
took the port on its tests, its deliberate breaks and two live comparisons of its own, and
applied it to master as written.

Built: `crates/grader/src/baseline.rs` (`target`, the features, `fit`, the three measures,
reliability, the provider and model rates, `train`, `predict`, `report`, `row_from_record`) and
`baseline_cli.rs` (`hydra-grader baseline train|score`, with `argparse`'s argument rules).
`hydra-pyjson` gained `math::pow` and `math::sqrt`, and its probe now also hashes `x ** 2`, `sqrt`
and `log1p` of every count below 200,000 plus 100,000 more below 100,000,000.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 12 unit
and 9 differential tests, 1,116 comparisons over 279 recorded cases (exit code, stdout, stderr
and every file left behind: 18 models trained, 40 refusals under the floor, 31 datasets that end
in a traceback, 105 records scored, 43 scores refused or raised, 41 wrong command lines, one
help); `hydra-pyjson` 22 and 3 probes; `hydra-puppets` unchanged. The run broke the Rust nine
ways: a plain fold for the compensated sum failed 45 comparisons, `RATE` or `STEPS` 37 each.

The session ran both implementations side by side, the Python being the real `baseline.py` under
3.14.7: on `fixtures/baseline/main.jsonl` the three models and reports are byte-identical; on a
read-only copy of the real `~/hydra/grader/dataset.jsonl` (685 rows) the `outcome` and `verdict`
models and reports are byte-identical and `task` is refused by both with the same sentence (6 of
its rarer class in the test slice).

Normalized, and only these: where Python ends in a traceback the Rust's last stderr line starts
with the same class; on exit 2 only the last stderr line is compared, with `baseline.py` read as
`hydra-grader baseline`; help exits 0 with the Rust's own usage.

What the plan assumed that is false: `x ** 2` was not on its list of risky operations. It is
libm `pow`, differs from `x * x` on 2,683 of 2,000,000 floats here, and the compiler folds a
visible `powf(x, 2.0)` into a multiplication, so `math::pow` hides the exponent behind
`black_box`. A launch time is `fromtimestamp(millis / 1000)`, a float, not the id's integer
milliseconds. A dataset that is missing or is a directory is no error: `train` refuses with "0
labelled rows".

Left open: the command is spelled `hydra-grader baseline train|score`, the run's choice, since
`monitor.py` has a `train` and a `score` too. `label-queue` and `label-import` answer "not
ported yet" until build 217 lands. The model on disk today (`baseline-outcome-315589bf9365`) is
not what a fresh train on today's dataset gives under 3.14.7 (`...c640f023d3e0`); the cutover's
gate has to retrain first, as map 94 says. `py_max` keeping a NaN has a unit test and no
differential case.

