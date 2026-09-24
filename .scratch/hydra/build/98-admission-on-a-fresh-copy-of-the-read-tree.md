# 98 - Rung two admits a fresh copy of the tree it read, with a toolchain the proposer cannot change

Type: build
Status: claimed
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 2 and 12, findings H2, H38 and H16.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/rungs/evolve.rs and src/env.rs.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

Admission runs on a copy of the read tree made after the proposer's processes are stopped, with a scratch cargo home and absolute tool paths; a test shows that a change the proposer makes to its checkout or to the cargo configuration it can reach after the read does not change what admission runs. The canary test no longer mutates the process environment under parallel tests. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome
