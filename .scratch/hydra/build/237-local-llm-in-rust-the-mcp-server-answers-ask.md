# 237 - local-llm in Rust: the MCP server answers `ask`

Type: build
Status: resolved
Blocked by: 224

## What

Map ticket 95. This is "L1" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`: read that section whole, and the plan's
answers under it. It names the files and functions to port, what the differential test compares
on, what must be identical, what is normalized, and the one thing most likely to differ.

Map 95, question C: loopback only, as in build 232.

## Do

1. Port what the plan's section names into `crates/local-llm` (package and binary `hydra-local-llm`), with the
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

Built 2026-10-04 by a Grok 4.6 run (`01a10644-6546`), reviewed and applied by the session.

`crates/local-llm` (package and binary `hydra-local-llm`, on `hydra-pyjson` and `hydra-records`):
the stdio MCP server, `initialize`, `tools/list`, `tools/call` for `ask`, posting to Ollama
through the loopback client. `tests/record_ask.py` recorded 43 cases against an HTTP stub,
`tests/ask.rs` replays them; with compact JSON separators 41 of 172 comparisons fail. Stated
difference: a URL that is not `http://` kills the Python with a traceback, and the Rust answers
a tool error and keeps serving.

**Measured by the session against a live Ollama:** the same `initialize`, `tools/list` and an
`ask` ("the capital of France", 8 tokens) to the Python and the Rust gave byte-identical output,
both "Paris". The Python stays until build 238.
