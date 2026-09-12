# 44 - A run can rewrite the hook config Hydra wrote inside its worktree

Type: build
Status: open
Blocked by:

## What

On `cursor-agent` the hook config lives at `<worktree>/.cursor/hooks.json`, written by Hydra at
spawn (`src/provider/cursor.rs:75`), and `standard` grants `file_write` over the whole worktree.
Nothing in `policies/` or `gate.py` names that file, so a run may rewrite it and drop the hook that
gates it. Whether Cursor reloads hooks mid-session is unmeasured; the file is still Hydra's own
gate config sitting inside the one directory the run may write, which harness spec 15.2 row 9
exists to forbid for the script and forgot for the config that points at it. The same class:
`<worktree>/.cursor/mcp.json`, `<worktree>/.claude/settings.json` and `settings.local.json`,
`.mcp.json`, and whatever `agy` reads from `.agents/`. On `claude` the gate config is outside the
worktree and passed by `--settings`, and Claude Code snapshots hooks at startup, so the exposure
there is the project settings file adding hooks rather than removing Hydra's.

Deny `file_write` to those paths under every policy, in the same place the pattern floor lives,
with a test per provider in `tests/gate.rs` that feeds the hook a write to each and reads the
refusal back. Amend harness spec 7.5 and 15.2 to name them. Found by map ticket 59's market
research on 2026-09-12; it is the trust-handoff class the July 2026 sandbox escapes used.
