# 03 - Skill discovery rules per provider

Type: research
Status: answered

## Verdict

One `SKILL.md` file, unchanged, can satisfy all four providers. The minimum frontmatter every provider will load is just `name` and `description`; none of the four rejects extra keys on the live scan path. No field forces a generated per-provider wrapper.

What D9 does need, and what breaks if you skip it, is not a frontmatter change but two structural fixes:

1. **Cursor's target path is wrong in the ticket.** `~/.cursor/skills-cursor` is Cursor's own built-in/managed skill cache, not the user skills directory. The real, documented personal path is `~/.cursor/skills/`, which does not exist yet on this machine. Symlinking into `skills-cursor` would be symlinking into vendor-owned state.
2. **Gemini's plugin wrapper is a directory-level manifest, not a SKILL.md edit.** D9 already calls this "plugin-wrapped," so it isn't a new cost, just confirmation the plan already accounted for the one place a wrapper is unavoidable.

Everything else is a naming convention (keep the skill's `name:` field equal to its own directory basename) that costs nothing to satisfy for all four at once.

Two real gaps remain unconfirmed and should be closed with a throwaway symlink test before anyone commits to the symlink-farm design: whether Cursor and Gemini follow a symlinked skill directory the way Claude Code and (almost certainly) Hermes do. See "What's still open" at the end.

## Claude Code

**Directories scanned:** personal `~/.claude/skills/<name>/SKILL.md`, project `.claude/skills/<name>/SKILL.md`, an enterprise-managed location, and plugin skills at `<plugin>/skills/<name>/SKILL.md` wherever the plugin is enabled. Source: [Extend Claude with skills](https://code.claude.com/docs/en/skills), directory table.

**Symlinks:** followed, and documented as such: "A `<skill-name>` entry in the enterprise, personal, or project locations can be a symlink to a directory elsewhere on disk. Claude Code follows the symlink and reads `SKILL.md` from the target directory, and if the same target is reachable from more than one location, Claude Code loads the skill once." Plugin skills use a separate symlink mechanism (marketplace-level, not needed here since D9 doesn't plugin-wrap Claude). Source: same doc, symlink section.

This machine already proves it. Every entry in `~/.claude/skills/` (`claude-handoff`, `daily-agile-planner`, `handoff-frank`, `unslop`, and seven more) is a symlink into `/Users/USER/Flava/claude/skills/<name>`, confirmed with `readlink`. `unslop` loaded correctly into this very session's system prompt through a `@~/.claude/skills/unslop/SKILL.md` include in `CLAUDE.md`, so the symlink-follow claim isn't just documented, it's live and working right now.

**Frontmatter:** only `name` and `description` are required. Claude Code additionally reads its own extensions: `disable-model-invocation`, `allowed-tools`, `disallowed-tools`, `context`, `background`, `shell`, `arguments`, `paths`, `metadata`, `license`, `compatibility`. The open [Agent Skills](https://agentskills.io) spec defines six portable fields (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`); Claude Code accepts all six unmodified and tolerates its own extra fields on top. A stricter validator exists, but only for claude.ai skill uploads and the Skills API ("Unexpected key(s) in SKILL.md frontmatter... Allowed properties are: allowed-tools, compatibility, description, license, metadata, name") — that path isn't in Hydra's design, since Hydra reads skills off disk, not through an upload API.

**Path stability:** `~/.claude/skills/` is the documented personal path, not an internal or versioned detail.

## Cursor

**The ticket's premise needs correcting.** `~/.cursor/skills-cursor` (25 entries on this machine) is not the path Cursor's own docs name. It showed up because Cursor keeps its built-in and team-managed skills there, confirmed straight from the machine's own `~/.cursor/.gitignore`:

```
# Plugin cache (rules, skills, agents)
# Built-in Cursor skills
!skills-cursor/
!skills-cursor/**
# User's personal skills
!skills/
!skills/**
```

and from `~/.cursor/skills-cursor/.cursor-managed-skills-manifest.json`, which lists `builtinSkillIds` (`create-rule`, `create-skill`, `create-subagent`, `migrate-to-skills`, `shell`, `update-cursor-settings`) and `managedSkillIds` (`canvas`, `cursor-blame`), plus a `.sync-manifest.json` tracking `lastSyncedAt` timestamps for all 25 entries. That's a vendor sync cache, not a place to symlink third-party skills into.

**The real personal path**, per [Cursor's Agent Skills docs](https://cursor.com/docs/skills) and [cursor.com/help/customization/skills](https://cursor.com/help/customization/skills), is `~/.cursor/skills/` (and `~/.agents/skills/` as an alias), walked recursively — "Cursor walks the skills root recursively and picks up any `SKILL.md` it finds." Neither directory exists yet on this machine (`ls ~/.cursor/skills` and `ls ~/.agents/skills` both fail). For compatibility, Cursor also reads `~/.claude/skills/` and `~/.codex/skills/` directly, so in principle Cursor could pick up Hydra's skills through the Claude-facing symlinks alone, without a Cursor-specific tree at all — worth weighing against D9's plan to symlink into a Cursor-specific location too.

**Frontmatter:** required fields are `name` (must match the parent folder name) and `description`. Optional: `paths`, `disable-model-invocation`, `icon`, `color`, `metadata`. No mention of `allowed-tools`, `license`, or `compatibility`, but nothing suggests Cursor rejects unrecognized keys either; it's a YAML frontmatter parse, and the extras Claude Code needs (`license`, `compatibility`, `metadata`) are all in the portable spec Cursor claims compatibility with.

**Symlinks:** not mentioned anywhere in Cursor's documentation, positive or negative. Unconfirmed.

**Path stability:** the documented path (`~/.cursor/skills/`) reads as stable and intentional. The path this machine actually populated (`skills-cursor`) is Cursor's own internal cache name, which is exactly the kind of thing that can be renamed across versions without notice, since it was never a public contract to begin with.

## Gemini

Two live plugins on this machine, both under `~/.gemini/config/plugins/<name>/`, both with the identical minimal shape:

```
<plugin>/
├── installed_version.json   # {"version": "..."}
├── plugin.json
└── skills/
    └── <skill-name>/SKILL.md
```

`chrome-devtools-plugin/plugin.json` has `name`, `version`, `description`, `author`, `repository`, `license`, `keywords`. `google-antigravity-sdk/plugin.json` has the same set minus `repository`, confirming `repository` is optional. Every real `SKILL.md` under both plugins (`chrome-devtools`, `a11y-debugging`, `debug-optimize-lcp`, `memory-leak-debugging`, `troubleshooting`, `google-antigravity-sdk`) carries exactly `name` and `description` in frontmatter and nothing else, so on the frontmatter question Gemini is the most restrictive-by-convention but not restrictive-by-enforcement: nothing suggests extra keys would break it, they're just absent in every sample seen.

**Minimum viable `plugin.json`:** treat this with medium confidence, because the two public schema descriptions I could find don't match this machine's actual install path. The closest documented schema, [antigravity.google/docs/cli/plugins/](https://antigravity.google/docs/cli/plugins/), says only `name` is strictly required (alphanumeric, hyphens, underscores), with `description` and `$schema` optional — but that doc places plugins at `~/.gemini/antigravity-cli/plugins/<name>/`, and this machine's live plugins sit at `~/.gemini/config/plugins/<name>/` instead. `~/.gemini/config/.migrated` (an empty marker file) confirms this config tree already went through one migration, and `~/.gemini/antigravity-cli/` still exists in parallel, holding a different set of state (cache, conversations, keybindings) with no `plugins/` subdirectory of its own. So the path Gemini 0.42.0 actually reads today is the post-migration one, and the public docs haven't caught up to it. Given that gap, the safer minimum to target is what both real, working plugins on this exact machine actually contain: `name`, `version`, `description`. Nothing in either example suggests `author`, `license`, `keywords`, or `repository` are load-bearing; they read as convention, matching what Claude Code and Hermes both do with their own optional metadata fields.

`installed_version.json` is present in both real plugins and absent from every documented schema I found. Whether the loader gates on its presence, or it's purely an updater's bookkeeping file, is unresolved — flag it as a real unknown rather than assume either way.

**Symlinks:** undocumented. Could not test empirically without writing into `~/.gemini`, which was off-limits for this investigation.

**Path stability:** low. This is the second provider (after Cursor) where the path actually in use on disk doesn't match the public docs, and the `.migrated` marker proves the path already moved once in this product's history. Hardcode `~/.gemini/config/plugins/` for now, but build in a check rather than assuming it holds.

## Hermes

Primary source read directly: `/Users/USER/.hermes/hermes-agent/tools/skill_manager_tool.py`, function `_validate_frontmatter` (lines 217-253). Hard requirements, quoted from the code:

- Content starts with `---`.
- A closing `\n---\n` exists.
- The frontmatter parses as a YAML mapping.
- `name` is present.
- `description` is present and ≤ 1024 characters (`MAX_DESCRIPTION_LENGTH`).
- Non-empty body after the frontmatter.

`version`, `author`, `license`, `platforms`, and `metadata` are not enforced by the validator; they're present on essentially every shipped skill by convention only, per this same tool's own in-repo authoring guide (`~/.hermes/skills/software-development/hermes-agent-skill-authoring/SKILL.md`, "Required Frontmatter" section, which explicitly names `tools/skill_manager_tool.py::_validate_frontmatter` as its source of truth).

**Category nesting is not load-bearing.** `_resolve_skill_dir(name, category=None)` (line 271) returns `SKILLS_DIR / category / name` when a category is given and `SKILLS_DIR / name` when it isn't — both are valid inputs to the same function. Discovery (`_find_skill`, lines 278+, and the skill-listing path around line 353) walks every configured skills root with `skills_dir.rglob("SKILL.md")`, a recursive glob that finds `SKILL.md` at any depth, category or not. This machine proves both shapes work side by side: `~/.hermes/skills/yuanbao/SKILL.md` sits with no category directory at all, right next to `~/.hermes/skills/apple/apple-notes/SKILL.md`, which has one. `~/.hermes/skills/.bundled_manifest` and `.usage.json` both key every skill by its flat name only (`apple-notes:<hash>`, `large-codebase-auditing: {...}`), with no category prefix anywhere in the key, confirming category is directory-placement sugar, not part of a skill's identity. The one constraint that does exist: a category value must be "a single directory name" (the validator's category-format error message), so nesting is capped at one level, matching every example seen (no `apple/foo/apple-notes` two-deep case exists).

**Symlinks:** not addressed by name anywhere in the tool or its docs, but the discovery mechanism is `pathlib.Path.rglob`, and `hermes --version` reports Python 3.11.15. In Python versions before 3.13, `Path.glob`/`rglob` traverse symlinked directories unconditionally during a recursive (`**`) match; the `recurse_symlinks` keyword that lets you opt out (default: don't follow) was only added in 3.13. So on this Hermes install, a symlinked skill directory under `~/.hermes/skills/` should be picked up by `rglob("SKILL.md")` the same as a real one. This is inferred from CPython's own documented glob semantics for 3.11, not stated by Hermes itself, so call it high confidence rather than confirmed.

**Path stability:** `SKILLS_DIR = HERMES_HOME / "skills"` is a fixed constant in the source, and the tool's own docstring calls it out directly: "All skills live in `~/.hermes/skills/` (single source of truth)." Stable.

## Comparison table

| Provider | Real scan path | Required frontmatter | Symlinks followed | Path confidence |
|---|---|---|---|---|
| Claude Code | `~/.claude/skills/<name>/SKILL.md` | `name`, `description` | Yes, documented and proven live on this machine | High, documented |
| Cursor | `~/.cursor/skills/<name>/SKILL.md` (not `skills-cursor`, which is vendor-owned) | `name` (must match folder), `description` | Undocumented | Path is documented but doesn't exist yet on disk |
| Gemini | `~/.gemini/config/plugins/<plugin>/skills/<name>/SKILL.md` | `name`, `description` | Undocumented | Low; live path already diverges from every public doc found, and has migrated once |
| Hermes | `~/.hermes/skills/[<category>/]<name>/SKILL.md` | `name`, `description` (≤1024 chars) | Very likely (Python 3.11 `rglob` default), not stated by Hermes itself | High, constant in source |

## What's still open

Before building the symlink farm, resolve two things a read-only audit can't settle:

1. **Does Cursor follow a symlinked skill directory under `~/.cursor/skills/`?** Not in the docs either way. Cheapest test: create `~/.cursor/skills/` (it doesn't exist), drop one real symlink into it pointing at a throwaway skill, and run `cursor-agent` to see if it lists it. This is destructive to nothing (the directory doesn't exist yet), so it's safe once someone other than this read-only investigation does it.
2. **Same question for Gemini's plugin `skills/` subdirectory**, and separately, **whether `installed_version.json` is required for a plugin to load at all**, or just informational. Test by pointing a throwaway plugin's `skills/` at a symlink and seeing whether `gemini` picks it up, and by omitting `installed_version.json` from a second throwaway plugin to see whether it still loads.

Neither test needs touching the live directories this investigation was told to leave alone (`~/.claude`, `~/.cursor`, `~/.gemini`, `~/.hermes`); both can run against a scratch plugin/skill name that gets deleted after, but that's a build-time decision, not a research one, so it's left here rather than done.

## Sources

- [Extend Claude with skills](https://code.claude.com/docs/en/skills) — official Claude Code docs, symlink behavior, directory table, frontmatter reference.
- [Agent Skills open standard](https://agentskills.io) — the six-field portable frontmatter spec Claude Code, Cursor, and Hermes all claim or imply compatibility with.
- [Cursor Agent Skills](https://cursor.com/docs/skills) and [cursor.com/help/customization/skills](https://cursor.com/help/customization/skills) — official Cursor docs, directory locations, frontmatter fields.
- `~/.cursor/.gitignore` and `~/.cursor/skills-cursor/.cursor-managed-skills-manifest.json` — local, primary evidence that `skills-cursor` is Cursor's own vendor cache, not the user directory.
- [Antigravity CLI plugin system](https://antigravity.google/docs/cli/plugins/) — closest public schema for `plugin.json`, though its stated install path doesn't match this machine's live one.
- `~/.gemini/config/plugins/*/plugin.json`, `~/.gemini/config/.migrated` — local, primary evidence of the live plugin shape and the prior path migration.
- `/Users/USER/.hermes/hermes-agent/tools/skill_manager_tool.py` — Hermes's own frontmatter validator and skill-directory resolver, read directly.
- `~/.hermes/skills/software-development/hermes-agent-skill-authoring/SKILL.md` — Hermes's own skill-authoring guide, naming the validator as its source of truth.
- Python `pathlib` glob/symlink semantics for `Path.rglob` (3.11 vs. the `recurse_symlinks` parameter added in 3.13) — used to infer Hermes's symlink behavior, not directly stated by Hermes.
