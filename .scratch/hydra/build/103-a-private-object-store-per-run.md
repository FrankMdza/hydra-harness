# 103 - Each run writes git objects into its own store

Type: build
Status: claimed
Blocked by: 99

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item map ticket 74's answer, and item 3, findings H1.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/policy/mod.rs, src/sandbox.rs, src/worktree.rs, src/reap.rs, src/run.rs and the gate's write list.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

A run's `git add` works with the parent's `objects/` fully denied; reap moves the run's objects into the parent from outside after git checks them; a test plants a forged pack in the run's store and shows it never reaches the parent or a later worktree; `tests/spawn.rs` measures `git add` under the real profile outside a seatbelt. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome
