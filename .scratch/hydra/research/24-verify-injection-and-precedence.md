# 24 - Injection shape and skill precedence, verified against current builds

Written incrementally, 2026-09-03. Every claim below is one command whose output is quoted or
summarised in place. The salvaged evidence from the agent that died on 2026-09-02 is re-checked
rather than re-derived, and where a build moved under it that is said out loud.

## Builds under test

Checked before any probe ran.

| CLI | Version today | Version the salvaged evidence ran against |
|---|---|---|
| `claude` | 2.1.257 | 2.1.257, unchanged |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026.08.31-4057e58 or later, moved |
| `agy` | 1.1.24 | 1.1.23 at ticket 10's third pass, moved |

## The dead agent's rig survived, and its raw outputs are on disk

The rig is still at
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t24/`, intact:
a store git repo, a snapshot worktree at sha `57dd5052`, a user repo, and a linked run worktree
`wt-run1` carrying `.claude/skills/`, `.cursor/skills/`, `.agents/skills/`, `.codex/` and `.grok/`.
So the salvaged claims can be checked against the JSON the CLIs actually returned rather than
against a summary of it.

`claude-run1.json`, the one paid Claude run, returned seven names:

```
t24-clash        | Marker CLASH-USER-CLAUDEROOT
t24-user-only    | Marker USERONLY-USER
t24-alpha        | Marker ALPHA-STORE
t24-dirname-only | Marker IDENT-PROJECT   (frontmatter name is t24-frontmatter-name)
t24-fmclash-dir  | Marker FMCLASH-PROJECT
t24-repo-own     | Marker REPOOWN-PROJECT
t24-rootclash    | Marker ROOTCLASH-CLAUDEROOT
```

`cursor-run1.json` returned nine, with `t24-clash` listed twice:

```
t24-clash          | Marker CLASH-USER-CURSORROOT
t24-cursoruser-only| Marker CURSORUSER-USER
t24-user-only      | Marker USERONLY-USER
t24-alpha          | Marker ALPHA-STORE
t24-clash          | Marker CLASH-PROJECT
t24-dirname-only   | Marker IDENT-PROJECT
t24-rootclash      | Marker ROOTCLASH-CURSORROOT
t24-fmclash-dir    | Marker FMCLASH-PROJECT
t24-repo-own       | Marker REPOOWN-PROJECT
```

Two things fall out that the salvaged summary did not say.

Claude listed `t24-clash` once, with the user-scope marker, and it listed `t24-dirname-only` under
the directory name while carrying the marker `IDENT-PROJECT` from the linked snapshot. So the
project-scope link was read, and only the colliding entry was dropped. The drop is per name, not
per root.

Both providers rendered `t24-dirname-only` by its directory name even though its frontmatter says
`name: t24-frontmatter-name`. That is the identity split, visible in raw JSON rather than inferred.

