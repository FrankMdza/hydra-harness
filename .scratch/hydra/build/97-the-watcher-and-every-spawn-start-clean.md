# 97 - The watcher and every spawn Hydra starts get a cleared environment

Type: build
Status: claimed
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 1 and 9, findings H3 and H46.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/watch.rs, src/provider/codex.rs (the login probe only) and any other `Command::new` in `src/` that starts a provider binary or proposer code.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

A test that reads the running watcher's environment and fails without the clear. The Outcome lists every `Command::new` in `src/` with its environment policy. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome
