# 199 - The read side matches Python where the review found it does not

Type: build
Status: resolved
Blocked by: 197

## What

A read-only review by `codex` (gpt-6-astra) on 2026-09-30 of build 193 (`dceb144`) found no gap
in `locked_update`'s lock, and ten places where `crates/puppets` and `puppets.py` give different
output on the same log, each probed against `python3`. It also found three ways the differential
test can pass on unequal outcomes. Map 90 asks for identical output, and 194 builds the writers on
this code, so these close first. Line numbers are as of `dceb144`; build 197 moved some.

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
9. **The integer digit limit** (`json.rs:182`, `puppets.py` `_bad_line`). An integer of 4,301
   digits raises `ValueError` in Python (the 4,300-digit default of
   `sys.set_int_max_str_digits`), which `_bad_line` does not catch, so `read_log` and `repair`
   both die with a traceback; the Rust keeps the number. Such a line is a bad line in both:
   `_bad_line` catches `ValueError`, and the Rust parser refuses an integer past 4,300 digits.
10. **Invalid UTF-8 and the locked callback** (`log.rs:226`). A good line then an unterminated
    `0xff`: Python raises `UnicodeDecodeError` before `decide` runs, Rust runs `decide` and
    returns `UnterminatedLog`. Decode before deciding, and fail as Python does. Python decodes in
    buffered chunks, so say in the report what happens on a file larger than one chunk.

A second review by `codex` on 2026-10-01, of build 197 (`69132ad`), found these. They change the
Python and the Rust together, as 197 did, with tests in `integrations/puppets/tests/` and inputs
in the differential test.

11. **`repair --yes` is not crash-safe** (`puppets.py` `cmd_repair`, `log.rs` `repair`). It
    overwrites the log in place, then truncates, then appends the `repair` event. Killed after
    the overwrite, `xxxxxxxxxxxxxxxx\n{"event":"note"}\n` becomes two `note` lines and reads as a
    clean log. Killed after the rewrite and before the event, the log is clean and the old
    approvals are live. Make it one step: build the whole new content, good lines and the
    `repair` event together, write it to a temporary file beside the log, fsync, and rename it
    over the log. A writer that was waiting on the old file's lock must not append to the file
    the rename replaced: after taking the lock, every locker (`_locked_update`, `locked_update`,
    both `repair`s) compares the inode of its descriptor with the path's and reopens when they
    differ. Test it with a second process that holds the lock path open across a repair.
12. **`removed` past `i64`** (`fold.rs`). Two repairs with `removed: 9223372036854775807` leave the
    Rust revision saturated at the same value, so an approval between them stays live; Python
    counts on. A `removed` wider than `i128` is a `BigInt` and counts as zero in Rust. One rule
    in both: `removed` counts when it is an int from 1 to 1,000,000,000, else zero, and the
    revision is wide enough not to saturate on any log that fits in memory.
13. **A lone `\r` inside the file** (`puppets.py` `_partition_log`, `damaged_line_numbers`,
    `read_log`; `log.rs` `split_lines`). `splitlines` breaks on `\r`, so `BAD\r` followed by good
    lines reads as an unterminated tail: readers miss everything after it and `repair --yes`
    writes only `BAD\r` to the torn file and drops the good lines that follow from both files.
    Split on `\n` only, everywhere a goal log is read. A CRLF line still parses, since `\r` is
    JSON whitespace; a tail that ends in `\r` with no `\n` is still unterminated.
14. **The "repair voids table approval" input tests nothing** (`differential.rs`). Its last line
    is bad, so every read command stops before the fold. After `repair --yes`, run `show` and
    `pending --json` on each repaired copy and compare those too.
15. **`mask_repair_ts`** (`differential.rs`). It masks `ts` on any line containing
    `"event":"repair"`, a nested one included, and on `repair` events that were already in the
    input. Mask only the `ts` of the one line `repair --yes` appended.

Items 1 to 10 change nothing in `integrations/puppets/` except `_bad_line` in item 9. If one of
them cannot match Python without changing more of the Python, stop on that item, say so, and
finish the rest.

## Done when

- Each input failed before the change. The puppets Python suite passes. fmt and clippy clean over
  the workspace, `cargo test --lib` and `cargo test -p hydra-puppets` pass, the differential test
  green.
- `IMPLEMENTATION.md`: the "Known divergence" sentence of the `crates/puppets` passage says what
  is left, or goes.

## Outcome

Resolved 2026-10-01. Run `01a0f5ea` (claude opus 5.5, `standard`, 19.9M tokens, 45 minutes), the
lock and `repair` read line by line by the session, the rest by its tests, applied to master as
written.

Built, by item: (1) strings are `PyStr`, a list of code points, so a lone surrogate is read and
written back, and `ParseError::Unsupported` is gone; (2) ints and floats compare exactly at any
width; (3) `queue` sorts with `pysort.rs`, a port of CPython's `list.sort`, checked against
`python3` 3.14.7 on 624 lists with NaN; (4) the comparator replaces only `argv[0]`, masks only a
stamp of `utc_now()`'s shape, and compares the exception class at the start of the Rust message;
(5) bytes decode by `json.detect_encoding`; (6) `run_end` reads `run_id` only where Python does;
(7) `repr` follows `str.isprintable` from a generated table, `printable.rs`, 737 ranges of
Unicode 16; (8) an empty model version prints `none`; (9) an integer past 4,300 digits is a bad
line in both; (11) `repair --yes` writes the whole new log, the `repair` event included, to
`<goal>.jsonl.repair-tmp`, fsyncs and renames it over the log, and every locker compares its
descriptor's inode with the path's after taking the lock and reopens when they differ; (12)
`removed` counts from 1 to 1,000,000,000 and the Rust revision is `i128`; (13) lines split on
`\n` only, everywhere; (14) `show` and `pending` run on each repaired copy; (15) only the line
`repair --yes` appended has its `ts` masked.

Measured on master: fmt and clippy clean over the workspace; the puppets Python suite 204
passed; `cargo test -p hydra-puppets` 31 unit tests and 7 differential tests, 1,304 command runs
over 163 inputs; the ignored real-log test passed, on the one goal log there is; `cargo test
--lib` 224 passed.

What the ticket assumed that is false: item 10. Since build 197 `_locked_update` reads bytes, so
on a good line and an unterminated `0xff` both run `decide` and raise `UnterminatedLog`. No code
changed for it.

Found and fixed outside the ticket: a NaN task id (Python finds a NaN dict key by identity, the
Rust never matched it); a line nested 40,000 deep, which aborted the Rust with a signal while
Python read it, closed by a 1 GiB stack for the command thread and a cap of 100,000 levels.

Left open: the nesting cap is the run's number, and a line between 100,000 levels and Python's
own limit reads in Python and stops the Rust. `pysort.rs` was written from memory of
`listobject.c` and is checked against one Python version. The two Rust waiter tests depend on a
300 ms sleep. A repair killed before the rename leaves a torn file that copies lines still in
the log. No test kills a real `repair --yes`. `src/cockpit/goal.rs` still uses the old `removed`
rule: build 202.

