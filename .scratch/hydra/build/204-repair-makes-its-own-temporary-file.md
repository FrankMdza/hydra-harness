# 204 - `repair` makes its own temporary file

Type: build
Status: claimed
Blocked by: 194

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of build 199 (`3a8603a`). It found no
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
