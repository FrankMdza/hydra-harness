# 291 - bank build is slower than the Python it replaced

Type: build
Status: claimed
Blocked by:

## What

Measured by the session on 2026-10-04 (build 231's outcome): `hydra-apprentice bank build --n 200`
over this machine's ~800 records took 9.2 s, where the deleted Python took 7.3 s, both writing a
byte-identical `bank.jsonl`. `hydra-otel --all` over 40 records was 9.4 s against 8.4 s, but
there nearly all the time is `hydra show`, so this ticket is the bank alone.

Find where the time goes and make `bank build` faster without changing a byte of its output.

1. Build a synthetic corpus of a few hundred records in the test's scratch directory (sizes like
   real ones: some over 1 MB, many tool results), and time `bank build` on it in release. Profile
   it however works on macOS without new dependencies (`cargo build --release` plus `sample`, or
   timers around `list_records`, `scan_record`, `facts`, `pick`, the dump and the write). Say what
   takes the time, with numbers.
2. Fix what the numbers point at. Likely candidates, unverified: `PyStr` code-point work on every
   line of every record, `current_preamble(root)` read once per record instead of once per build,
   `to_utf8` copies inside `pick`'s sort keys.
3. The 63 recorded cases in `crates/apprentice/tests/fixtures/bank.json` keep passing unchanged:
   that is the proof the output did not move. Add a timing number to the report, before and after,
   on the same synthetic corpus.

## Done when

- `cargo test -p hydra-apprentice` and `-p hydra-records` pass with no fixture changed; fmt and
  clippy over the workspace clean; the report has the before and after times and what moved them.

## Outcome
