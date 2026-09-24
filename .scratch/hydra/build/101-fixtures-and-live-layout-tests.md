# 101 - A fixture corpus for CI, a codex dead-hook probe, and tests with a live layout

Type: build
Status: open
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 17 and 18, findings H14, H15, H36 and H18.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: tests/, a new fixtures directory, src/reap.rs for the typed line-0 view.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

CI's record walks read a sanitized, version-labelled fixture corpus kept apart from live measurements; an automated probe shows what codex does when its hook dies; `review_blocked`'s refusal wiring and G42's tree half are driven with a scratch live layout; reap reads line 0 through a typed view. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome
