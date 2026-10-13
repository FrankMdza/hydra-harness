# 238 - local-llm in Rust: the cutover

Type: build
Status: resolved
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

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra runs `01a1342b-7041` (385k tokens, stopped) and
`01a1342f-10ac-7552-abf6-841768b3a40e` (692k tokens) under `standard`, applied to master on
2026-10-13 after the diff was read whole. The first attempt found that `cargo test -p
hydra-local-llm` already passes all 43 replay cases, so the review of 237 was closed by the code
as it stood; it then stopped when the floor refused a Python heredoc that mentioned git, which is
the classifier working, and the second task text said so up front. The script, its test and the
empty `tests/` directory are deleted; both READMEs, `IMPLEMENTATION.md`, the grader README and
the crate's doc comments name the binary; the one deliberate difference stays documented (a URL
that is not `http://` is a tool error in Rust where the Python died). The session changed one
thing: the builder's `config/mcp.md` block gave `target/release/hydra-local-llm` as a relative
command, which a provider would resolve from its worktree, where no `target/` exists; both
READMEs now give it as an absolute path, the form the previous block used. `config/mcp.md` is
untouched and the human's line is on his list.
