# 238 - local-llm in Rust: the cutover

Type: build
Status: open
Blocked by: 237

## What

Map ticket 95, "L2" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`.

## Do

Close what the review found; the README's `mcp.md` block names the binary; delete the script and
its test. `config/mcp.md` is protected: say what the human has to change there and do not edit
it.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.
