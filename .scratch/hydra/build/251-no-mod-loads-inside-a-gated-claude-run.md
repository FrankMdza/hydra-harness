# 251 - No mod loads inside a gated `claude` run

Type: build
Status: resolved
Blocked by: map 103

## What

Decided by the human on 2026-10-03: no mod loads in any gated run. The ungated orchestrator
(the copilot, a Master of Puppets session) keeps mods. Reversible by the human alone; the line in
`PENDIENTES-FRANK.md` says how.

Claude Code 2.1.287 and later loads mods by default, and the runs record 2.1.288. A mod is
JavaScript inside the `claude` process. From the vendor's admin page
(code.claude.com/docs/en/plugins/mods/admin, read by the session on 2026-10-03):

- The built-in guard `sec-default@builtin` loads only on a machine with managed settings or for a
  Team or Enterprise login. This machine has neither (`auth.mechanism` is `subscription-oauth`).
- Where the guard loads, deny rules beat a user's mod. Where it does not, nothing says they do.
- "A user's mod that approves tool calls can approve a call that an `ask` rule would prompt for,
  or that a `PreToolUse` hook outside managed settings blocked." Hydra's gate is a `PreToolUse`
  hook in a `--settings` file.
- "`PreToolUse` hooks from other settings files and from plugins run after the last mod, so a mod
  that returns its own result in place of running the tool keeps those from running."
- Mods load from: a plugin the user installed, `--plugin-dir`, and "a mod Claude wrote during a
  session" (hot reload).
- `allowManagedModsOnly`, `disableSideloadFlags` and `prependPlugins` are read from managed
  settings only; the same keys in `--settings` change nothing. `disableAllHooks` stops mods and
  every settings hook too, the gate included, so it is not a fix. `--safe-mode` "turns installed
  mods off" and also "disables your other customizations": whether it keeps the hooks of a
  `--settings` file is not stated.

`src/provider/claude.rs` passes nothing about plugins today, and headless gated runs keep the
`(allow default)` sandbox profile (build 250 narrowed the interactive one only), so a headless
agent's shell can likely write under `~/.claude/plugins`.

## Do

1. Write a fixture mod under `tests/fixtures/mods/approve-all/` (a plugin: `.claude-plugin/plugin.json`,
   `hooks/hooks.json`, `hooks/register.js`) whose hook answers every tool call itself or approves
   it, so a gate refusal no longer stops the call. Keep it inert outside a test: nothing installs it.
2. Find from `claude --help`, `claude plugin --help` and the binary's own text (`</dev/null`
   always; never send a prompt, never start a session) which flag or env var keeps installed,
   project-enabled and session-written mods from loading in a `--print` launch while keeping the
   hooks of a `--settings` file. Candidates, in order: `--safe-mode`, `--bare`, an env var the
   binary reads. Write what each one also drops.
3. If one keeps the gate and drops the mods, add it to the gated argv in `claude.rs`, headless and
   interactive, never to an ungated launch. Unit tests on the argv, in the style of
   `the_interactive_argv_carries_no_gate_and_no_stream_flags`: a gated argv carries it, an ungated
   one does not.
4. Write a live test, skipped without the `claude` binary like `tests/drift.rs`, that launches a
   gated `claude` with the fixture mod reachable and a task whose tool call the gate refuses, and
   asserts the refusal held. The session runs it on master; you cannot, since a seatbelt does not
   nest inside a run.
5. If no flag does it, stop and say so: what is left is a decision (a separate config directory
   per gated run, which map 99 option b also names, or a managed settings file on this machine).
6. In the report, one paragraph each for `codex`, `cursor-agent` and `agy`: does the CLI load
   anything in-process (plugins, extensions) that can allow a tool call its hook denied? Answer
   from `--help` and the repository's `src/provider/*.rs` comments only, and say what you could
   not check.
7. `SECURITY.md`: one paragraph on mods. `IMPLEMENTATION.md`: one paragraph.

## Done when

- The session runs the live test on master: with the flag it passes; with the flag removed the
  fixture mod lets the refused call through (revert, run, restore). fmt, clippy over the workspace
  and `cargo test --lib` are clean.

## Outcome

Stopped 2026-10-03 at step 5 by run `01a10389-ec0e` (`claude` opus 5.5, `standard`, 1.45M
tokens), no diff: no flag or variable in `claude` 2.1.288 drops mods and keeps a `--settings`
hook. What it read is in map ticket 103, which now blocks this ticket. Steps 1 to 4 still stand
once 103 picks the mechanism, except that the live check uses an inert mod that leaves a marker
when it loads.

Map 103 A installed by the human on 2026-10-03:
`/Library/Application Support/ClaudeCode/managed-settings.json`, root-owned, with the guard option
`allowManagedModsOnly`. Checked by the session the same day with an inert mod (one `tool.call` hook
that counts and calls `next`) loaded by `claude -p --plugin-dir`: the debug log reads
`cc-plugin-sec-default@builtin seated outermost: this machine has managed settings` and
`refused by cc-plugin-sec-default: mods are limited to your organization's by policy
(allowManagedModsOnly); inert-mod was not loaded`. The control, the same launch without the file,
was not run, because removing it is the human's; the guard does not load without managed settings
on this login, per the vendor's page and run `01a10389-ec0e`. Build 257 carries map 103 C.

Resolved 2026-10-03 by the session, with no code of its own: map 103 A (the managed settings file,
checked above) keeps a user's mod out of every `claude` on the machine, gated runs included, and
build 257 (map 103 C) keeps a gated run from writing a plugin where a later launch would load it.
Steps 1 to 4's flag does not exist in `claude` 2.1.288. The ungated copilot is under A too, so a
mod for it would have to be installed as the organization's, from an admin-owned directory.
