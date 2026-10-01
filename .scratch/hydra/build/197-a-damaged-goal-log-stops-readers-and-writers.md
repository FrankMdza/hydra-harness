# 197 - A damaged goal log stops readers and writers

Type: build
Status: claimed
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
