# 103 - How a gated `claude` run keeps mods out

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-03, by the session, from build 251, which stopped at its step 5.

## What build 251 found

The human decided that no mod loads in a gated run. Run `01a10389-ec0e` (`claude` opus 5.5,
1.45M tokens) read `claude` 2.1.288's own code and found no flag or variable that drops mods and
keeps the hooks of a `--settings` file, which is where Hydra's gate lives:

- `--safe-mode` keeps only managed-settings hooks, so the gate goes, and so do skills and MCP.
- `--bare` drops settings hooks and refuses OAuth, which this machine uses.
- `disableAllHooks` drops every non-managed hook.
- Mods sit behind a remote feature flag, and the local override for it is a stub in this build.
- From the vendor's admin page: the guard that makes deny rules beat a mod loads only with managed
  settings or a Team or Enterprise login. This machine has neither.

It also found, not measured: `claude` loads plugins from `.claude/skills/<name>/` as
`<name>@skills-dir`, project ones included, so a run could leave one in its worktree for the next
launch in that checkout; and `agy` reads `.agents/plugins/` beside the gate's own hook file and
can import Claude plugins. `codex` and `cursor-agent` have plugin commands whose reach was not
checked.

## Options

- A. A managed settings file on this machine (`/Library/Application Support/ClaudeCode/`, needs
  the human's admin password) with the guard option `allowManagedModsOnly`. Every `claude` on the
  machine then refuses a user's mod, the copilot's included, unless the human installs it as a
  managed one from an admin-owned directory. It also makes deny rules beat any mod.
- B. A separate `claude` config directory per gated run, seeded from the human's without
  `plugins/`. Map 99 option b names the same thing. Not yet known whether OAuth survives it.
- C. Hydra refuses writes to `.claude/skills/`, `.claude-plugin/`, `.agents/plugins/` and the
  user's plugin directories in every gated run, gate and sandbox both. This stops a run from
  planting a mod; it does not stop one the human installed.
- D. A and C.

The session's lean is D: A is the only switch the vendor documents for this, and C closes the
path a run could use on its own. Verifying either needs only an inert mod that leaves a marker
when it loads; no bypass has to be built.

## Answer

Resolved 2026-10-03 by the human, D (A and C), as the session leaned. A is his to install, since
it needs his admin password: `PENDIENTES-FRANK.md` has the file and the command. C is build 257.
Build 251 then verifies both with an inert mod that only leaves a marker when it loads.
