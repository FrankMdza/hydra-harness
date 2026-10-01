# 197 - A damaged goal log stops readers and writers

Type: build
Status: resolved
Blocked by: 193

## What

Map ticket 91 (2026-09-30, read its `## Answer`): a newline-terminated line of a goal log that is
blank or is not JSON makes the log damaged, and nobody folds around it. Today `read_log`
(`integrations/puppets/puppets.py:295`) raises a raw `JSONDecodeError` on a bad interior line and
skips a bad last one, and `_locked_update` (`puppets.py:332`) skips every bad line and appends.
`crates/puppets/src/log.rs` (build 193) copies both. This ticket changes the Python and the Rust
together, so `crates/puppets/tests/differential.rs` keeps comparing them byte for byte.

A line that is only an unterminated tail (no newline at the end of the file) is not damage. It
keeps today's rules: readers ignore it, writers raise `UnterminatedLog`.

## Do

Tests first, each shown failing before the change: Python tests under
`integrations/puppets/tests/` (a new `test_damaged_log.py`), Rust unit tests in `log.rs`, and the
edge inputs of `differential.rs`.

1. **A named error.** `DamagedLog(goal_id, line_numbers)` in the Python and in the crate, with one
   message in both: `goal log <id> is damaged at line <n>[, <n>...]; wrote nothing; puppets.py
   repair <id>`. Line numbers are 1-based over the file's lines.
2. **Readers.** `read_log` raises `DamagedLog` for any newline-terminated line that is blank after
   `strip()` or fails `json.loads`, the last line included. `show`, `board`, `pending` and `queue`
   catch it, print the message on stderr and exit 1, with nothing on stdout and no traceback.
   Every other command that reads through `read_log` or `fold` does the same.
3. **Writers.** `_locked_update` and `locked_update` raise `DamagedLog` after reading and before
   calling `decide`, and append nothing. Every command that writes prints the message and exits 1.
   `UnterminatedLog` keeps its own message; when a log is both damaged and unterminated, report
   the damage.
4. **`repair`.** The dry run also lists each bad line as `bad line <n>: <first 300 characters>`
   before the torn tail, and says what `--yes` would move. `repair --yes`, under the lock: write
   the bad lines in file order and then the torn tail into `<goal>.torn-<UTC timestamp>.txt`,
   fsync it, then rewrite the log with only its good lines, fsync. If it removed one or more
   newline-terminated lines, it then appends `{"event":"repair","removed":<count>}` through the
   same lock (with `seq` and `ts` as any event). A log with only a torn tail gets no `repair`
   event, as today.
5. **The fold.** On a `repair` event, `revision += removed + 1`. That puts the revision above any
   an earlier approval named, so table, launch, diff and degraded approvals before it are voided
   by the existing rule. Plan approvals stay. Same in `fold.rs`.
6. **The differential test.** The edge inputs "bad interior line", "blank interior line" and "bad
   last line" now expect the named error from both, compared exactly, not by exception class. Add
   a log with two bad lines and a torn tail, and a log that holds a `repair` event with approvals
   on both sides of it. `repair --yes` on a damaged log must leave the same files in both.
7. `integrations/puppets/README.md` and `IMPLEMENTATION.md`: the passage on the goal log's tail
   and `repair`, and nothing else. Do not touch `src/cockpit/`; build 198 does.

## Done when

- The new tests failed before the change. `python3 -m unittest discover -s
  integrations/puppets/tests` passes. fmt and clippy clean over the workspace, `cargo test -p
  hydra-puppets` passes with the differential test green, `cargo test --lib` passes.
- The report lists every Python command that reads or writes a goal log and says, for each, what
  it prints on a damaged log.

## Outcome

Resolved 2026-10-01. Run `01a0f5d0` (cursor-agent composer-2.5, `standard`, 2.7M tokens, six
minutes), diff read whole by the session, applied to master and corrected there.

Built as the ticket says: `DamagedLog` in the Python and the crate with one message, `read_log`
and the locked update refusing before `decide`, `repair` listing and moving bad lines and
appending a `repair` event, the fold's revision bump, nine Python tests in `test_damaged_log.py`,
and the edge inputs of the differential test. Every Python command that reads or writes a goal
log prints the one line and exits 1; `new` has no log to damage; `repair` is the way out.

Five corrections by the session, each a difference between the two implementations or a way to
lose a good line:

- The run's Python chose bad lines with `decode(errors="replace")` and then parsed strictly, so
  invalid UTF-8 in a terminated line was a traceback in Python and `DamagedLog` in Rust. A line is
  now bad when `json.loads(line)` on its bytes raises, in both.
- That same rule keeps a BOM on the first line readable, as it was before. The run's version
  called it damage, and `repair --yes` would have moved the `goal` event out of the log.
- The Rust called a line with a lone surrogate damaged, which Python reads. `repair` would have
  removed a good line. It is no longer bad; reading it still fails by name until build 199.
- The Rust `repair` truncated the log to zero before rewriting it. It now writes the good lines
  over the old bytes and then cuts, as the Python does, so a crash leaves the log long and never
  empty. The second lock type the run added went; `Flock` covers it.
- `removed` that is not a positive int: Python raised or converted, Rust skipped the bump. Both
  now add `removed` when it is a positive int, and always one.

Measured on master: the puppets Python suite 194 passed; fmt and clippy clean over the workspace;
`cargo test -p hydra-puppets` 11 unit tests and the differential test over 123 inputs. The
session added two inputs: a healthy log with `repair` events between table approvals, and invalid
UTF-8 inside a terminated line.

Left open: `repair --yes` rewrites in place and is not atomic; a rename would break the `flock`
other writers wait on. A `removed` wider than `i64` saturates in Rust and not in Python.

