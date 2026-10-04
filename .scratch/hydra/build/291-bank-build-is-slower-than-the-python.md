# 291 - bank build is slower than the Python it replaced

Type: build
Status: resolved
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

Built 2026-10-04 by a Grok 4.6 run (`01a106b4-a233-70f0`), reviewed and measured by the session.

**Where the time went, on a 250-record synthetic corpus:** `list_records` (the JSON scan) 92 of
110 ms. Every line was decoded into a `PyStr` of code points before it was walked. The ticket's
suspects were small: `current_preamble` per record 4 ms, `pick`'s copies 5 ms.

**What changed:** `hydra-pyjson`'s `loads_bytes*` and `loads_str` parse valid UTF-8 with a new
byte parser (`Utf8Parser`), copying an unescaped string in one slice; UTF-16, UTF-32 and a
surrogate encoded as UTF-8 still take the code-point parser. `PyStr::from` and `push_str` map
ASCII bytes directly. `bank.rs` reads the preamble once per build and sorts `pick` on the digest
bytes. Two new pyjson tests; the 63 recorded bank cases pass unchanged.

**Measured by the session**, since the parser sits under every crate:
- The new parser against the old over every line of this machine's 826 records and four
  truncations of each: 2,515,850 inputs, 0 disagreements in value or error text.
- Old and new release binaries on the real corpus: `bank build --n 200` byte-identical, 9.3 s
  to 7.5 s, which is where the deleted Python was (7.3 s); `hydra-otel --all` over 20 records
  byte-identical; `hydra-grader rebuild` into scratch homes, 816 rows identical except
  `collected_at`.

**The 110 ms synthetic profile did not predict the real 9.3 s**: the real corpus is 811 MB with
records up to 16 MB. And a session trap, recorded for the next one: `cargo build --release` at
the root builds only the `hydra` package, so the first old-versus-new comparison ran the old
apprentice twice. Build the crates with `--workspace` or `-p`.
