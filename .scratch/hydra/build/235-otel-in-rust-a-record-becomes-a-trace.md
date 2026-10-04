# 235 - otel in Rust: a record becomes a trace

Type: build
Status: resolved
Blocked by: 224

## What

Map ticket 95. This is "O1" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`: read that section whole, and the plan's
answers under it. It names the files and functions to port, what the differential test compares
on, what must be identical, what is normalized, and the one thing most likely to differ.

## Do

1. Port what the plan's section names into `crates/otel` (package and binary `hydra-otel`), with the
   Python's arguments, output and exit codes. The crate depends on `hydra-pyjson` and
   `hydra-records` as it needs, never on `hydra`, `hydra-puppets` or `hydra-grader`.
2. Record the Python once, as `crates/grader/tests/record_collect.py` does: an empty environment,
   frozen data, shell stubs for every program the Python calls, the checkout written as `<repo>`.
   The Rust tests compare with the fixtures and start no `python3`. Fixtures under 1 MB.
3. Break the Rust on purpose once and say how many comparisons fail.
4. The Python stays until the piece's cutover. `IMPLEMENTATION.md`: a passage on what was ported.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.

## Outcome

Built 2026-10-04 by a Grok 4.6 run (`01a1066b-7e06`), reviewed and applied by the session.

`crates/otel` (package and binary `hydra-otel`, on `anyhow`, `hydra-pyjson`, `hydra-records` and
`sha2` for the trace and span ids) ports `integrations/otel/record_to_otlp.py`: `cli.rs` its
argparse, `convert.rs` the rows, `hydra show --events` on a copy under a temporary
`HYDRA_ROOT`, the spans and the batch. 65 recorded cases, 260 comparisons; span ids of 15 hex
digits fail 28.

**Measured by the session on 40 real records:** `--all` from Python and Rust wrote 80
byte-identical files; 8.4 s and 9.4 s, nearly all of it the `hydra show` calls.

**Left as the Python has it:** `normalized` joins the run id into a path under the temporary
`runs/`, so an id holding `../` would write outside it. Run ids come from Hydra's own records,
so the port keeps parity rather than diverge before the cutover; the cutover ticket (236)
should refuse such an id.
