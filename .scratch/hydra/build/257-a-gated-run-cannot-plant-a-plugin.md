# 257 - A gated run cannot plant a plugin

Type: build
Status: claimed
Blocked by:

## What

Map 103 C, decided by the human on 2026-10-03. A mod is JavaScript a provider loads into its own
process; on `claude` one can approve a call Hydra's gate refused. Part A of the same answer is a
managed settings file the human installs, which keeps users' mods out of every `claude` on the
machine. This ticket closes the other path: a gated run writing a plugin where a later launch
would load it. Build 251 found these, by reading `claude` 2.1.288 and the `agy` provider file:

- `claude` loads a plugin from `.claude/skills/<name>/` as `<name>@skills-dir`, in the user's
  home and in a project; a plugin directory carries `.claude-plugin/plugin.json`.
- `claude`'s installed plugins live under `~/.claude/plugins/`, and `enabledPlugins` in a
  settings file turns one on, `.claude/settings.json` and `.claude/settings.local.json` in a
  project included.
- `agy` reads `.agents/plugins/` beside the gate's own `.agents/` hook file, and can import
  `claude` plugins.

## Do

1. In every gated run, headless and interactive, refuse writes to: `~/.claude/plugins/`,
   `~/.claude/skills/`, and inside the worktree `.claude/skills/`, `.claude/settings.json`,
   `.claude/settings.local.json`, `.agents/plugins/` and any path with a `.claude-plugin`
   component. Two layers: the macOS profile in `src/sandbox.rs` (kernel, catches shell), and the
   gate for the file tools. The gate's code is in `src/policy/`; `policies/` is protected config
   and stays untouched, so if the gate can only learn these paths from a policy file, do the
   sandbox layer, stop, and say so.
2. Hydra itself writes skill links and the gate's hook config into the worktree before the
   provider starts. Check that those writes happen outside the sandboxed process and still work;
   if one happens inside it, say which and stop.
3. Tests: the composed profile carries each deny after the worktree allow (style of
   `tests/sandbox_*.rs`); a live `sandbox-exec` probe, skipped without the binary, where
   `mkdir -p <worktree>/.claude/skills/x && touch <worktree>/.claude/skills/x/f` is refused and a
   write to `<worktree>/src/f` is allowed; gate tests in `tests/gate.rs` style for `Write` to
   `.claude/skills/x/SKILL.md` and `.agents/plugins/p.json`. Each fails with its rule removed;
   say so. The live probe runs on master; you cannot run it inside a seatbelt.
4. `SECURITY.md`: one paragraph. `IMPLEMENTATION.md`: one paragraph.

## Done when

- fmt, clippy over the workspace, `cargo test --lib`, `cargo test --test gate`, the sandbox tests
  and `cargo test --test spawn` are clean.

## Outcome
