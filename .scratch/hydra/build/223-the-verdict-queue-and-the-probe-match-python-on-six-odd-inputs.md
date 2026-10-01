# 223 - The verdict queue and the probe match Python on six odd inputs

Type: build
Status: claimed
Blocked by:

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of builds 212, 213 and 217 ran the
Python on hard inputs and read the Rust for the same. It found no difference in the record scan,
the redactor or the logistic baseline, and no recorded fixture that is a failure of the recorder.
It found six in build 217's `queue.rs` and `jev.rs`, each with the Python's behaviour reproduced.
Two of them write a different persistent label file than the Python does.

## Do

For each, add the input to `crates/grader/tests/record_queue.py` or `record_jev.py`, record it
from the Python, see the Rust test fail, then fix the Rust.

1. **A blank first line in the CSV** (`crates/grader/src/queue.rs` near 248). `\nrun_id,verdict\n
   x,fixed\n` with a dataset row for `x`: Python's `csv.DictReader` takes the blank record as an
   empty header, `label-import` returns `(0, 2)`, writes `{}` and leaves `x` unlabelled. The Rust
   skips the blank line and labels `x` as fixed.
2. **A missing run id cell** (`queue.rs` near 291). `verdict,run_id\nfixed\n`: Python keeps the
   missing cell as `None` and writes `{"null":"fixed"}`; the Rust writes `{"None":"fixed"}`.
3. **The field-size limit** (`queue.rs` near 220). A field of 131,073 characters makes Python
   raise `_csv.Error: field larger than field limit (131072)` before either output file changes;
   the Rust imports the verdict.
4. **A patch label the probe does not know** (`crates/grader/src/jev.rs` near 273).
   `[{"ticket":1,"label":"unknown","patch":"a.patch"}]` with the patch file present: Python writes
   the row and then raises `KeyError: 'unknown'`; the Rust counts it as `as_written` and exits 0.
5. **Numbers as strings in `score`** (`jev.rs` near 419). `needs_correction` of `"0_0.8"` and
   `"０.２"`: Python's `float()` reads 0.8 and 0.2; the Rust's `parse::<f64>()` refuses both. Use
   or add a `float()` to `hydra-pyjson` beside `int_literal`, checked against `python3` on the
   forms `float()` takes (underscores, non-ASCII digits, `inf`, `nan`, `infinity`, signs,
   surrounding whitespace, `1e5`, `.5`, `5.`).
6. **Overflow in the Brier term** (`jev.rs` near 292). `needs_correction: 1e200` on a `changed`
   answer: Python's `(p - y) ** 2` raises `OverflowError` after printing the lines through `n`;
   the Rust prints `brier: inf` and exits 0. Check `baseline.rs` and `monitor.rs` for the same
   `** 2` on a value that can overflow and say what you found.

No dependency is added. `IMPLEMENTATION.md`: the passage on build 217.

## Done when

- Each input failed before its fix; fmt and clippy clean over the workspace; `cargo test -p
  hydra-grader` and `-p hydra-pyjson` pass.
