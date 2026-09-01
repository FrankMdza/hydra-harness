# 03 - Skill discovery rules per provider

Type: research
Status: resolved

## Question

D9 says one canonical `~/hydra/skills/`, symlinked into the Claude and Cursor locations and plugin-wrapped for Gemini. Confirm each half of that before anyone builds it.

- **Claude Code.** Which directories are scanned, whether symlinked entries are followed, how plugin skills differ from `~/.claude/skills` entries, and what frontmatter is required.
- **Cursor.** The same questions for `~/.cursor/skills-cursor`, which currently holds 25 skills. Is that path stable or a version detail?
- **Gemini.** It finds skills through plugins at `~/.gemini/config/plugins/<plugin>/skills/`. What is the minimum viable `plugin.json`, and does anything else in a plugin have to exist?
- **Hermes.** `~/.hermes/skills` holds 24 packs in nested category directories, a different layout from the other three. Does that nesting matter?

Report the frontmatter fields each provider requires or ignores. If a single `SKILL.md` cannot satisfy all four, say which field forces a per-provider generated wrapper.

## Answer

One unchanged `SKILL.md`, carrying only `name` and `description`, satisfies all four providers. No frontmatter field forces a generated per-provider wrapper, so D9's build step stays a symlink farm rather than a compiler.

What broke instead was D9's Cursor target. `~/.cursor/skills-cursor` is Cursor's own managed cache of built-in skills, proven by the comment in `~/.cursor/.gitignore` and by a sync manifest. The personal path Cursor actually scans is `~/.cursor/skills/`, which is documented and absent on this machine. D9 has been corrected in the map.

Gemini's plugin wrapper is a directory-level `plugin.json`, not an edit to any skill file, so D9 already priced it in. But Gemini's live plugin path `~/.gemini/config/plugins/` already diverges from every public doc found, and carries a `.migrated` marker, so treat that path as a version detail rather than a contract.

Two gaps stay open, left deliberately unconfirmed because this was a read-only investigation of live config: whether Cursor and whether Gemini follow symlinked skill directories. Claude Code does, documented and proven live here. Hermes does, inferred from Python 3.11 glob semantics with the source read directly. If either of the remaining two does not follow symlinks, D9's whole approach needs a copy step for that provider. Ticket 10 has to close those two gaps before it can specify injection.

Detail: [research/03-skill-discovery-rules.md](../research/03-skill-discovery-rules.md)
