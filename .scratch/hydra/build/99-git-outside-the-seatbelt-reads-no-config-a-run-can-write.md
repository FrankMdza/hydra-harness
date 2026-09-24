# 99 - Git outside the seatbelt reads no configuration a run can write, and a failed worktree setup always leaves a record

Type: build
Status: open
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 4, 6, 13 and the code half of 15, findings H5, H47, H39, H40, H6 and H17.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/worktree.rs, src/run.rs and src/skills/mod.rs.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

`CheckoutGit` and the plain git runner ignore global and system git configuration and pass the identity explicitly; a launch from inside `worktrees/` is refused; every fallible step after `git worktree add` in `worktree::create` either reaches the failure path or is rolled back, with a test; partial skill injection is recorded; `phase_spawn` is under 150 lines; the unused `_layout` parameter is gone. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome
