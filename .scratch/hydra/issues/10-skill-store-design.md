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
