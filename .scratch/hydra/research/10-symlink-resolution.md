# 10 - Do Cursor and Gemini follow symlinked skills

Closes the gap ticket 03 left open, where the question could not be answered without writing into
live config. It can be answered without doing that, and the answers change what injection means for
both providers.

## What I did, and what I did not check

Everything below was run on 2026-09-01 against the installed builds, `gemini` 0.42.0 and
`cursor-agent` 2025.10.02. Nothing was written to `~/.claude`, `~/.cursor`, `~/.gemini` or
`~/.hermes`. The Gemini tests ran in a throwaway config root through `GEMINI_CLI_HOME`, which
relocates the whole `.gemini` tree, and used `gemini skills list --all`, which discovers and prints
without making a model call. So this cost nothing and touched nothing.

Two things I could not settle:

- Whether `gemini skills link <path>` produces a symlink or a copy. The command hung with no output
  and no prompt, and I killed the run rather than let it sit. Unverified.
- Whether `cursor-agent`'s rule loader follows symlinks. It calls a stat helper minified to `gle`
  inside a 17 MB bundle, and I could not resolve it to `stat` against `lstat` statically. The
  question turns out to be moot at this version anyway, for the reason below.

## Gemini: yes, for directories, no for files

`gemini` 0.42.0 discovers a skill through a symlinked **directory**, and does not discover one
through a symlinked `SKILL.md` inside a real directory.

Four probes in one isolated `~/.gemini/skills/`, listed together:

| Probe | Shape | Discovered |
|---|---|---|
| `hydra-probe-copied` | real directory, real `SKILL.md` | yes, Enabled |
| `hydra-probe-linked` | symlink to a directory elsewhere | yes, Enabled |
| `hydra-probe-filelink` | real directory, `SKILL.md` is a symlink | no |
| `hydra-probe-dangling` | symlink to a path that does not exist | no, and no error |

`gemini skills list --all` printed the first two with a `Location` resolving through the link to the
canonical file, and said nothing at all about the other two. A dangling link is skipped silently,
which is the right failure mode for a symlink farm and a bad one for debugging it.

The mechanism, read off the bundle and consistent with the results: discovery walks the skills
directory with `readdir(dir, { withFileTypes: true })` and joins `SKILL.md` onto each entry. A
`Dirent` for a symlink reports `isSymbolicLink()`, not `isFile()`, which is why a linked `SKILL.md`
is invisible while a linked directory is fine, since the join happens before anything stats the
file.

**Rule for the injection design: link the directory, never the file.**

### Two corrections to the record

**The path is `~/.gemini/skills/<name>/SKILL.md`.** Ticket 03 recorded
`~/.gemini/config/plugins/<plugin>/skills/<name>/SKILL.md`, reached by wrapping a skill in a plugin.
Gemini 0.42.0 has first-class skills that need no plugin at all. Both probes were found at the plain
path with no `plugin.json` anywhere.

**D9's "plugin-wrapped for Gemini" is therefore wrong at this version.** Gemini takes the same
unwrapped `SKILL.md` as everyone else, in a directory Hydra can symlink. That removes the one place
in D9 where a provider needed different treatment. Whether to amend D9 is a decision for ticket 10,
not a finding.

`gemini skills` also has `link`, `install`, `enable`, `disable` and `uninstall` subcommands, with
`link` documented as "Links an agent skill from a local path. Updates to the source will be
reflected immediately." If it makes a symlink, it is the sanctioned way to do what D9 wants, and
Hydra should call it rather than write links by hand. Testing that is one command, once it stops
hanging.

## Cursor: the question does not apply at this version

`cursor-agent` 2025.10.02 has no concept of a skill. Searching the 17 MB bundle:

| Pattern | Hits |
|---|---|
| `.cursor/skills` | 0 |
| `"skills"` | 0 |
| `SKILL.md` | 0 |
| `.cursor/rules` | 1 |
| `.cursorrules` | 2 |

What it loads instead is `.mdc` rule files. `loadRulesFromSubdirectories` globs
`*/**/.cursor/rules/**/*.mdc`, and a second loader walks **up** from the project root joining
`.cursor/rules` at each level until it runs out of parents. There is no separate personal scope:
`~/.cursor/rules` is picked up only because it happens to sit on the walk up from a project inside
your home directory. `.cursorrules` at the root is read as a single always-apply rule.

So ticket 03's `~/.cursor/skills/` is not a path this build reads. That claim came from
documentation rather than from this binary, which matches ticket 03's own note that Cursor's
documented path already diverged from disk once.

**This is a version fact, not a product fact.** The installed build is from 2025.10.02, eleven
months old. Cursor may well have added skills since. Updating `cursor-agent` is a change to the
user's tooling and was not mine to make, so the current state stands as recorded.

## What this leaves for ticket 10

- Gemini injection is a directory symlink, or `gemini skills link` if that turns out to make one.
  Either way, propagation is immediate and the "copy step" branch of the ticket closes for Gemini.
- Cursor injection is not a skills question at all right now. Either Hydra converts a `SKILL.md`
  into an `.mdc` rule and writes it where the upward walk finds it, or Cursor is a provider that
  gets no shared skills until it is updated. That is a real fork in the ticket, and it is new.
- Three of four providers are now settled for symlinks: Claude Code yes, documented and proven,
  Hermes yes from its glob semantics, Gemini yes for directories, proven here.
