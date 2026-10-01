# 199 - The read side matches Python where the review found it does not

Type: build
Status: open
Blocked by: 197

## What

A read-only review by `codex` (gpt-6-astra) on 2026-09-30 of build 193 (`f465e4d`) found no gap
in `locked_update`'s lock, and ten places where `crates/puppets` and `puppets.py` give different
output on the same log, each probed against `python3`. It also found three ways the differential
test can pass on unequal outcomes. Map 90 asks for identical output, and 194 builds the writers on
this code, so these close first. Line numbers are as of `f465e4d`; build 197 moved some.

## Do

Tests first: each numbered item is an input in `crates/puppets/tests/differential.rs` (or a unit
test where the item is about the comparator itself), shown failing before the change.

1. **A lone surrogate** (`json.rs:245`). `import` can write one: a plan task whose text is
   `"\ud800"` passes validation and `append_event` writes the ASCII escape. Python reads it and
   `show` prints it again; the Rust exits 1. Carry it through: the string type holds what Python's
   `str` holds, and `dumps` writes the escape back. `ParseError::Unsupported` goes.
2. **Wide numbers** (`py.rs:23`, `:43`, `:54`, `:244`). Two task ids `2**140` as an int and as the
   float `1.393796574908164e42` are one task in Python and two in Rust. Two distinct `BigInt` ids
   with tied scores sort in Python and raise in Rust. `-2**127` does not equal its exact float
   because of the `1.7e38` cutoff. Equality and ordering between int and float follow Python at
   any width.
3. **NaN scores** (`read.rs:262`). Tasks `z` then `a` with `p` NaN and 0.5: Python prints `z, a`,
   Rust `a, z`. Python's tuple comparison does not fall through to the id when the scores are
   unordered. Reproduce what `sorted` does, not a total order.
4. **The comparator** (`differential.rs:130`, `:213`, `:227`). Replace only `argv[0]` of each
   pending item, not every occurrence of the executable's path. Mask a torn stamp only when the
   24 characters are a timestamp of `utc_now()`'s shape. For a Python traceback, compare the
   exception class to the class the Rust names at the start of its message (`hydra-puppets:
   <Class>: ...`), not a substring anywhere in stderr.
5. **Encodings** (`log.rs:116`, `:135`). `json.loads(bytes)` detects UTF-16 and UTF-32 with and
   without a BOM (`json.detect_encoding`). A newline-terminated UTF-16BE goal line is read by
   Python and dropped by Rust. Follow `detect_encoding` per line as the Python does.
6. **`run_end` for an unknown task** (`fold.rs:319`). `{"event":"run_end","task":"missing"}` with
   no tasks and no pending runs: Python never reads `run_id` (the task test short-circuits and the
   comprehension is empty), Rust raises `KeyError`. Read each field only where Python does; check
   the other events for the same pattern.
7. **`repr` of a string** (`py.rs:147`). `board` on a goal whose text is `["\u2028"]`: Python
   prints `['\u2028']`, Rust prints the raw character. U+200B too. Use Python's printable rule
   (`str.isprintable`: Unicode categories Cc, Cf, Cs, Co, Cn, Zl, Zp, and Zs other than space),
   with a table small enough to read, and say where it came from.
8. **An empty model version** (`read.rs:446`). `"model_version":""` on the chosen score row:
   Python prints `verdict none`, Rust `verdict ` and two spaces.
9. **The integer digit limit** (`json.rs:182`). An integer of 4,301 digits raises `ValueError` in
   Python (the 4,300-digit default of `sys.set_int_max_str_digits`), and the Rust keeps it.
10. **Invalid UTF-8 and the locked callback** (`log.rs:226`). A good line then an unterminated
    `0xff`: Python raises `UnicodeDecodeError` before `decide` runs, Rust runs `decide` and
    returns `UnterminatedLog`. Decode before deciding, and fail as Python does. Python decodes in
    buffered chunks, so say in the report what happens on a file larger than one chunk.

Nothing changes in `integrations/puppets/`. If an item cannot match Python without changing the
Python, stop on that item, say so, and finish the rest.

## Done when

- Each input failed before the change. fmt and clippy clean over the workspace, `cargo test
  --lib` and `cargo test -p hydra-puppets` pass, the differential test green.
- `IMPLEMENTATION.md`: the "Known divergence" sentence of the `crates/puppets` passage says what
  is left, or goes.
