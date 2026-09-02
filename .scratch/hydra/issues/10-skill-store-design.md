# 10 - Skill store layout and injection

Type: grilling
Status: open
Blocked by: 03, 05

## Question

D5 makes the skill store the second thing both systems share, and D9 fixes the broad approach. This ticket makes it precise.

Settle:

- The layout of `~/hydra/skills/`: flat or nested, and what metadata sits alongside each `SKILL.md`
- What "inject" means per provider, given the discovery rules ticket 03 found: symlink, generated wrapper, or copy
- What happens to the skills already living in `~/.claude/skills`, `~/.cursor/skills-cursor` and `~/.hermes/skills`. Adopted into the canonical store, left in place, or shadowed
- How a skill declares which providers it applies to, if some are provider-specific
- Which skills Ouroboros may edit and which are off limits, which is the pin concept from ticket 05
- How a change propagates: immediately through symlinks, or on the next `hydra` launch

Scenario to test against: Ouroboros rewrites a skill mid-session while three provider CLIs are running. Who sees the old version, who sees the new one, and is that acceptable?

## What the research changed

- **D9's Cursor path was wrong.** `~/.cursor/skills-cursor` is Cursor's managed cache of its own built-ins. The personal path Cursor scans is `~/.cursor/skills/`, which does not exist on this machine yet.
- **One `SKILL.md` serves all four providers unchanged**, carrying only `name` and `description`. No frontmatter field forces a generated wrapper, so injection can stay a symlink farm.
- **Two gaps have to close before this ticket can specify injection.** Claude Code follows symlinked skill directories, documented and proven live. Hermes does, from its glob semantics. Whether Cursor and Gemini follow symlinks is unconfirmed, because ticket 03 was read-only against live config. If either does not, that provider needs a copy step and the propagation question changes with it.
- **Gemini's plugin path is a version detail, not a contract.** `~/.gemini/config/plugins/` already diverges from every public doc and carries a `.migrated` marker.

## What the symlink test found

Both gaps above are closed, and the answers are not symmetrical.

- **Gemini follows a symlinked skill directory** and ignores a symlinked `SKILL.md` inside a real
  one, silently. So injection links directories, never files. Its personal path is
  `~/.gemini/skills/<name>/SKILL.md` with no plugin wrapper, which makes D9's "plugin-wrapped for
  Gemini" wrong at 0.42.0.
- **Cursor has no skills concept** at the installed 2025.10.02 build. It reads `.mdc` rules found by
  walking up from the project root. So the ticket's injection question forks for Cursor: convert to
  `.mdc`, or leave Cursor out of the shared store until it is updated.

Detail, method and the two things still unverified in
[research/10-symlink-resolution.md](../research/10-symlink-resolution.md).

## What the second evidence pass found, 2026-09-01

Appended under `# 2026-09-01, second pass` in
[research/10-symlink-resolution.md](../research/10-symlink-resolution.md).

- **The Cursor fork above is dead.** `cursor-agent` self-updated at 12:10 on 2026-09-01, from
  2025.10.02 to 2026.08.31-4057e58, and the new build has skills. The section above that says
  "Cursor has no skills concept" was accurate the day it was written and is false now. Do not grill
  against it. One trap: there is still no `skills` subcommand, so grepping `--help` is not a test
  for whether the binary has skills.
- **The provider locations are not independent.** Cursor's skill roots are `.cursor/skills`,
  `.agents/skills` and `.cursor/skills-cursor`, plus `.claude/skills`, `.codex/skills` and
  `.grok/skills` behind a third-party flag that is on by default here, each joined to both the
  project directory and `homedir()`. So `~/.claude/skills` is already a Cursor root, and injecting
  for Claude Code injects for Cursor at the same time. This ticket's injection question is not four
  independent targets, it is an overlapping set.
- **Gemini does follow a symlinked `SKILL.md`.** The first pass said it ignores one silently and
  gave a `Dirent.isFile()` mechanism for why. Both wrong. The original error came from a relative
  link at the wrong depth dangling quietly, plus Gemini deduping on frontmatter `name`.
- **Ticket 03's Hermes reasoning was wrong, with the right conclusion.** Hermes follows symlinks
  through `os.walk(..., followlinks=True)`, not the `rglob` semantics ticket 03 reasoned from.
  `rglob` on Hermes' own Python 3.11.15 misses symlinked directories, tested.
- **The propagation scenario has a mechanical answer now.** Hermes re-reads a skill from disk on
  every call. Cursor and Gemini both cache the body at discovery and stay frozen until session
  start. Claude Code's metadata is fixed at session start. A mid-session rewrite splits two against
  one. Whether that is acceptable is still this ticket's question.
- **Eight name collisions**, all between `~/.claude/skills` and the mattpocock plugin cache, all
  carrying `disable-model-invocation`, all predating Hydra.
- D9's "plugin-wrapped for Gemini" is re-confirmed wrong at gemini 0.42.0.
