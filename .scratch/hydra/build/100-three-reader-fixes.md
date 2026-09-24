# 100 - A codex quota failure reads as rate limited, export refuses an output that aliases its sibling, and the codex MCP fragment is real TOML

Type: build
Status: claimed
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 5, 10 and 11, findings H4, H41 and H43.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/normalize/codex.rs, src/export.rs and src/provider/codex.rs (the MCP fragment only).

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

Each with a test that fails before: record 01a0cfdb-e5c3's shape derives `rate_limited`; `--out` naming the preference sibling by another spelling is refused; quotes, backslashes and newlines round-trip through the MCP fragment. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome
