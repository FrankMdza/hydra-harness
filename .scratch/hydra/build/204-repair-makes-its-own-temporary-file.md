# 204 - `repair` makes its own temporary file

Type: build
Status: resolved
Blocked by: 194

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of build 199 (`02f7abd`). It found no
schedule of cooperating writers and repairs that loses a line, matched its own port of
`pysort.rs` against `sorted` on 2,600 lists, and matched all 3,306 boundary points of
`printable.rs` against `str.isprintable`. Three findings are left. It waits on 194 only because
194 is editing the same files.

## Do

Tests first, in `integrations/puppets/tests/`, `crates/puppets/src/log.rs` and the differential
test, each shown failing before the change.

1. **A symlink at the temporary name** (`crates/puppets/src/log.rs` `replace_log`,
   `puppets.py` `_replace_log`). Both open `<goal>.jsonl.repair-tmp` with truncation and follow a
   symlink. With that name linked to the log, a repair killed during the write leaves the log
   empty or partial; linked elsewhere, repair overwrites that file. Create the temporary file
   exclusively and without following links (`O_CREAT | O_EXCL | O_NOFOLLOW`). When the name
   exists: a regular file left by a killed repair is removed and the create tried once more; a
   symlink or anything else is refused with a named error and exit 1, nothing written. Same
   message in both.
2. **stdout and a lone surrogate** (`crates/puppets/src/read.rs` near 50, `PyStr::to_utf8`).
   Python's stdout uses `surrogateescape` in the C or POSIX locale and `strict` otherwise, and
   `PYTHONIOENCODING` overrides both. With `surrogateescape`, a goal text of `"\udc80"` prints
   byte `0x80` from `board`; the Rust always raises `UnicodeEncodeError`. Follow Python's rule
   from the same environment variables (`PYTHONIOENCODING`, `PYTHONUTF8`, `LC_ALL`, `LC_CTYPE`,
   `LANG`), probe `python3` for each combination you claim, and add differential inputs run under
   `LC_ALL=C` and under `LC_ALL=en_US.UTF-8`.
3. **Quadratic output from a nested line.** A 200 KB line of 99,999 nested arrays makes `show`
   build about 20 GB in both implementations. Do not change this here: the nesting cap is a
   question the human has open. Measure it at 1,000, 3,000 and 10,000 levels in both and put the
   numbers in the report.

`integrations/puppets/README.md` and `IMPLEMENTATION.md`: the repair passage.

## Done when

- Each test failed before the change. The puppets Python suite passes; fmt and clippy clean over
  the workspace; `cargo test --lib` and `cargo test -p hydra-puppets` pass with the differential
  test green.

## Outcome

Resolved 2026-10-01. Run `01a0f62d` (codex gpt-6.1-sol, `standard`, 3.7M tokens), diff read whole
by the session and applied to master with one change.

Built: `replace_log` and `_replace_log` create `<goal>.jsonl.repair-tmp` with `O_CREAT | O_EXCL |
O_NOFOLLOW`; a regular file left there is removed and the create tried once more; a symlink, a
directory or anything else is refused as `UnsafeRepairTemporary` with exit 1, and `repair --yes`
checks it before it writes the torn file, so nothing is written. `print` in `read.rs` follows
Python's stdout rule from `PYTHONIOENCODING`, `PYTHONUTF8`, `LC_ALL`, `LC_CTYPE` and `LANG`, with
`surrogateescape` where Python uses it, for UTF-8, ASCII and Latin-1.

The session's change: the Rust created the temporary file with the default mode before setting
the log's; it now creates it `0600`, as the Python does.

Measured on master: the puppets Python suite 205 passed; `cargo test -p hydra-puppets` 44 unit
tests and 12 differential tests, the ignored real-log test too; fmt and clippy clean.

Nested `show`, measured and left as it is (the cap is the human's question): 1,000 levels print
2.0 MB, 3,000 print 18 MB, 10,000 print 200 MB, the same bytes in both; Python 0.08, 0.09 and
0.48 seconds, Rust 0.02, 0.03 and 0.26.

What the ticket assumed that is false: "strict otherwise". `PYTHONUTF8=1` selects
`surrogateescape` under `en_US.UTF-8` too.

Left open: a `PYTHONIOENCODING` codec other than those three stops the Rust with `LookupError`.
No test kills a real repair.

