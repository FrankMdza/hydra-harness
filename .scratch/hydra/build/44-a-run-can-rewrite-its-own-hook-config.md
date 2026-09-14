# 44 - A run can rewrite the hook config Hydra wrote inside its worktree

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-12.

**Built.** `protected_worktree_paths(worktree)` in `src/policy/mod.rs`, beside
`denied_write_paths` and `own_write_paths`, is the list: `cursor::hooks_file` and
`cursor::mcp_file`, `agy::hooks_file` and `agy::mcp_file`, the new
`claude::project_config_files` (`.claude/settings.json`, `.claude/settings.local.json`,
`.mcp.json`), and `Provider::skill_root()` for all three. Ten entries, every provider's on every
provider's gate. `compile` puts them in the config as `protected_worktree_paths`, resolved and
casefolded like the rest, and `protected_root` in `gate.py` checks that key first, before the own
carve-out, with the same empty default the other keys use so an older compiled gate cannot crash
open. Because both the `Write` check and the shell check go through `protected_root`, a shell
command naming `.cursor/hooks.json` is refused whole with no second list. The refusal code is the
existing `hydra/protected-path`.

**The `.gitignore` files stay writable.** `claim` already reads a repo's own ignore file rather
than replacing it, and the ignore file decides what `hydra reap` stages, not what the gate allows.
A test pins that `.cursor/.gitignore` and `.agents/.gitignore` still pass under `standard`.

**What `agy` reads from the worktree.** `.agents/hooks.json` and `.agents/mcp_config.json`,
both Hydra's own, loaded only when the worktree arrives on `--add-dir` (`src/provider/agy.rs`).
The module comment also names `rules/`, `plugins/` and `workflows/` under `.agents/` as things
the binary reads; those are the human's, not gate config, and are left alone.

**Measured.** `cargo test --test gate`: 64 passed, 0 failed, four of them new
(`a_run_cannot_rewrite_its_own_hook_or_provider_config_under_any_policy`, 150 write probes;
`the_config_denial_takes_nothing_else_out_of_the_worktree`;
`the_compiled_gate_carries_the_config_denials_resolved_inside_the_worktree`;
`a_shell_command_naming_the_hook_config_is_refused_whole`). `cargo fmt` clean. `cargo clippy --
-D warnings` fails on one lint in `src/ouroboros/evolve.rs:717` (`double_ended_iterator_last`),
a file this ticket did not touch and another session has modified in the working tree; nothing in
the files this ticket changed is flagged. The full suite was not run, per the session's memory
limit.

**Assumed and found otherwise.** Nothing in the ticket was false. Two things it left open: whether
Cursor or agy reloads hooks mid-session is still unmeasured and the denial does not depend on it,
and `hydra show`'s replay recompiles from `run_start`, so an old record where such a write was
allowed reads as a gate leak now. `policy_sha` is unchanged, the list is code.

Amended: `docs/harness-spec.md` 7.5 (a third kind of entry) and 15.2 (a paragraph under row 9),
`IMPLEMENTATION.md` after the ticket 40 paragraphs.

**Narrowed the same day.** The first version put the three skill roots on the whole-command
list, and the corpus walk read `ls -la .claude/skills/` on run `01a09405-f451`, three hours
older than the change, as a leak. A shell command naming a skill root is how `cursor-agent` and
`agy` read a skill, so the roots moved to a second list, `write_denied_worktree_paths`, that only
the `file_write` check reads. The config files keep the whole-command refusal. Build ticket 49
carries the general problem the walk exposed: a record has no mark of which gate code answered.
