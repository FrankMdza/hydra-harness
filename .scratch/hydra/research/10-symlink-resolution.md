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

---

# 2026-09-01, second pass

Everything above was written earlier the same day. This pass re-ran the open questions and closed
four more. Three findings above are now wrong, and the reason is not sloppiness on either side: the
Cursor binary was replaced between the two passes.

Versions at the time of this pass:

```
cursor-agent --version   2026.08.31-4057e58   (was 2025.10.02-bd871ac)
gemini --version         0.42.0
hermes --version         0.14.0, Python 3.11.15
claude --version         2.1.257               (map says 2.1.252)
```

macOS has no `timeout` binary, so every probe below was bounded with a two-line helper:

```sh
cat > /private/tmp/hydra-t10/to <<'EOF'
#!/bin/sh
exec perl -e 'alarm shift; exec @ARGV or die "exec: $!\n"' "$@"
EOF
chmod +x /private/tmp/hydra-t10/to
```

`to <seconds> <cmd>` kills the child with SIGALRM and exits 142 on timeout. Bounds used: 40 s for
version and status calls, 120 s for `gemini skills list` and `hermes skills list`, 180 s for the two
`cursor-agent -p` runs. Nothing timed out.

## Cursor updated itself, and the old answer expired

The map and the section above both record `cursor-agent` at 2025.10.02, with no skills concept at
all. That binary is still on disk but is no longer what runs:

```
ls -la ~/.local/bin/cursor-agent
  -> ~/.local/share/cursor-agent/versions/2026.08.31-4057e58/cursor-agent
ls ~/.local/share/cursor-agent/versions/
  2025.10.02-bd871ac   2026.08.31-4057e58
```

The symlink was repointed at 12:10 today. The new build is code-split into 74 chunks totalling
19.7 MB rather than one 17 MB file, and it has skills:

```sh
cd ~/.local/share/cursor-agent/versions/2026.08.31-4057e58
for p in ".cursor/skills" "SKILL.md" ".agents/skills" ".cursor/rules"; do
  printf '%-18s %s\n' "$p" "$(grep -lF -- "$p" *.js | wc -l)"; done
```

So the fork the ticket inherited, convert to `.mdc` or leave Cursor out, is a fork about a binary
that is no longer installed. Anything downstream of "Cursor has no skills" needs rereading.

**Where the stale number came from: an update, not a misreading.** Both builds sit side by side in
`versions/`, the 2025.10.02 directory dated Oct 5 2025 and the 2026.08.31 one dated 12:10 today, and
`~/.local/bin/cursor-agent` is a symlink whose own mtime is 12:10 today. `cursor-agent` has a
self-update path (`cursor-agent update` is in its help). So 2025.10.02 was the right answer when
ticket 03 ran, and the record was correct on the day it was written. Nothing needs correcting except
the number, and the findings that hung off it.

**Every Cursor claim below was gathered against 2026.08.31-4057e58.** Static reads are grepped from
`~/.local/share/cursor-agent/versions/2026.08.31-4057e58/` by full path, and the live probes ran the
`cursor-agent` on `PATH`, which resolves to that same build. The one exception is the `gle` section,
which is explicitly about the old 2025.10.02 bundle and says so.

One trap worth naming, because it is how the earlier answer went wrong in the first place: **there is
no `skills` subcommand.** `cursor-agent --help` lists `plugin` and `generate-rule|rule` and nothing
skill-shaped, so the CLI surface looks exactly the same as it did in October. Skills are there
anyway, as a `/skills` picker in the TUI and as prompt injection. Checking `--help` for the word
"skill" is not a test for whether this binary has skills.

## Cursor: what the installed build actually scans

Two live probes in a throwaway git repo at `/private/tmp/hydra-t10/repo`, never in `~/hydra`.
Nothing was written under `~/.cursor`; every file the probe created lives in `/private/tmp`. See the
side-effects section at the end for what the CLI itself wrote while running.

### Skill roots

The root table is a plain array in `index.js`, module `./src/utils/skill-path-utils.ts` and again
near offset 5354696:

```js
const Fr = {configDir:".cursor", subdir:"skills-cursor", thirdParty:false, builtin:true};
const jr = [
  {configDir:".cursor",  subdir:"skills", thirdParty:false, builtin:false},
  {configDir:".claude",  subdir:"skills", thirdParty:true,  builtin:false},
  {configDir:".codex",   subdir:"skills", thirdParty:true,  builtin:false},
  {configDir:".grok",    subdir:"skills", thirdParty:true,  builtin:false},
  {configDir:".agents",  subdir:"skills", thirdParty:false, builtin:false},
  Fr,
];
```

Each entry is joined onto the project directory (`Kr`, scope `project`) and onto `homedir()` (`Hr`,
scope `user`). The `thirdParty` ones are gated on a settings flag, and that flag is on by default on
this machine, proven below. So Cursor 2026.08.31 reads `~/.claude/skills/` as a first-class user
skill root. That is worth saying twice, because it means a symlink farm aimed at Claude Code alone
already lands in Cursor.

`~/.cursor/skills-cursor` is still the vendor cache, now flagged `builtin:true` in the binary
itself, which confirms ticket 03's read of the `.gitignore` from the other direction.

One filter, function `Ur`: a path is dropped if any component after `<configDir>/<subdir>` starts
with a dot, plus a hardcoded denylist for `.codex/skills` only (`imagegen`, `openai-docs`,
`opneai-docs`, `plugin-creator`, `skill-creator`, `skill-installer`).

### Rules, and how far up the walk goes

`.mdc` rules load through `loadRulesFromDirAndAncestors(ctx, startDir)`. The loop is unconditional:

```js
for(;;){
  const e = join(a, ".cursor", "rules");
  if (await this.dirIsDirectory(ctx, e)) this.loadRulesFromDirectory(ctx, a);
  const n = []; if (thirdParty) n.push("CLAUDE.md","CLAUDE.local.md"); n.push("AGENTS.md");
  for (const f of n) this.loadRulesFromMarkdownFile(ctx, a, f);
  const d = dirname(a); if (d === a) break; a = d;
}
```

It stops at the filesystem root, not at the repo root and not at `$HOME`. `loadRulesFromDirectory`
roots a ripwalk at `<dir>/.cursor/rules` with `includeGlobs: ["**/*.mdc"]`, so `.mdc` files nest to
any depth under that directory. `.cursorrules` is read once, at the git repo root, as a single
always-apply rule.

### Frontmatter fields it understands

The `.mdc` parser is hand-rolled, not a YAML library: it splits each line on the first `:`, with a
special case for a `metadata:` block and for an empty `globs:` introducing a list. The typing
function `Bi` reads, in this order:

| Field | Effect |
|---|---|
| `alwaysApply: true` | type `global`, body goes in the prompt every turn |
| `globs` (list, or comma-separated string) | type `fileGlobbed` |
| `description` (non-empty) | type `agentFetched`, the model decides; truncated to 1536 chars |
| none of the above | type `manuallyAttached`, only on an explicit @-mention |

Also read: `metadata.environments`, `metadata.disabledEnvironments`, `metadata.scopedTo` (each
comma-split), `metadata.surfaces` as a surface filter, and `disable-model-invocation` on skills.
`cursor-agent generate-rule` emits only `description`, `globs` and `alwaysApply`, which is the
narrowest set that covers all four types.

### Symlinks: yes, everywhere, and Cursor is the loosest of the four

Three separate discovery walks all pass `followSymlinks: true` (ripwalk, which shells out to
ripgrep): the ancestor `.cursor/rules` walk, the nested-extensibility walk, and the workspace
`loadFresh` walk. The interactive skill picker resolves it explicitly rather than by flag:

```js
const o = readdirSync(e, {withFileTypes:true});
for (const r of o) {
  let isDir = r.isDirectory();
  if (r.isSymbolicLink()) { try { isDir = statSync(join(e, r.name)).isDirectory(); } catch { continue; } }
  if (isDir) { const f = join(e, r.name, "SKILL.md"); if (existsSync(f) && !hidden(f)) push(...); }
}
```

`statSync` follows the link; a dangling link throws and hits `continue`, silently. Live probe:

```sh
# repo layout, all under /private/tmp/hydra-t10
#   repo/.cursor/rules/hydra-real.mdc                    real file, alwaysApply
#   repo/.cursor/rules/hydra-rulelink.mdc -> canon/rulelink.mdc      symlinked FILE
#   repo/.cursor/rules/sub -> canon/rulesub                          symlinked DIR
#   .cursor/rules/hydra-ancestor.mdc                     one level ABOVE the git root
#   repo/.cursor/skills/hydra-probe-real/SKILL.md        real dir
#   repo/.cursor/skills/hydra-probe-linkdir -> canon/linkdir-src     symlinked DIR
#   repo/.cursor/skills/hydra-probe-filelink/SKILL.md -> canon/filelink-src/SKILL.md
#   repo/.cursor/skills/hydra-probe-dangling -> /private/tmp/hydra-t10/no-such-target
#   repo/.claude/skills/hydra-probe-claudedir/SKILL.md   real dir, third-party root
cd /private/tmp/hydra-t10/repo
/private/tmp/hydra-t10/to 180 cursor-agent -p --mode ask --trust --output-format text \
  "Do not use any tools. Answer in exactly two lines. Line 1: TOKENS= followed by every string
   beginning with HYDRA- that appears anywhere in your system prompt, rules, or instructions,
   comma separated, or NONE. Line 2: SKILLS= followed by the exact names of every skill available
   to you whose name begins with hydra-probe, comma separated, or NONE."
```

Answer:

```
TOKENS=HYDRA-RULESUBDIR,HYDRA-RULELINK,HYDRA-REALRULE,HYDRA-ANCESTOR
SKILLS=hydra-probe-filelink,hydra-probe-linkdir,hydra-probe-real,hydra-probe-claudedir
```

| Shape | Result |
|---|---|
| real `.mdc` in `<repo>/.cursor/rules/` | loaded |
| symlinked `.mdc` file | loaded |
| `.mdc` inside a symlinked subdirectory of `.cursor/rules` | loaded |
| `.mdc` in an ancestor **above** the git repo root | loaded, so the walk really does pass the root |
| skill in a real directory | discovered |
| skill through a symlinked directory | discovered |
| skill whose `SKILL.md` is itself a symlink | discovered |
| skill through a dangling symlink | skipped, no error |
| skill under `<repo>/.claude/skills/` | discovered, so third-party extensibility is on by default |

Cursor follows symlinked files as well as symlinked directories. That is the opposite of what the
first pass concluded about Gemini, and as it turns out Gemini does not behave that way either. See
below.

A second probe, same repo, asked for every skill name it could see. Read-only against the live
config, nothing written:

```
SKILLS=autopilot,create-hook,create-rule,create-skill,loop,new-repo,origin,review-bugbot,
review-security,sdk,share,split-to-prs,statusline,update-cli-config,daily-agile-planner,
handoff-frank,unslop,hydra-probe-filelink,hydra-probe-linkdir,hydra-probe-real,
hydra-probe-claudedir,capture-tasks-from-meeting-notes,generate-status-report,
jira-sprint-dashboard-canvas,search-company-knowledge,spec-to-backlog,triage-issue,
content-experimentation-best-practices,content-modeling-best-practices,sanity-best-practices,
seo-aeo-best-practices,diagnosing-bugs,tdd,prototype,research,domain-modeling,codebase-design,
code-review,resolving-merge-conflicts,wizard,grilling,writing-for-agents
```

`daily-agile-planner`, `handoff-frank` and `unslop` are `~/.claude/skills` entries, and every entry
in that directory is a symlink into `/Users/USER/Flava/claude/skills/`. So the user-scope root and
symlink following are both proven live against the real config, without writing anything.

Only 3 of the 11 entries in `~/.claude/skills` came through, and the split is exact: the 8 that are
missing are the 8 carrying `disable-model-invocation` in their frontmatter. The binary honours it:

```js
if (d.content?.includes("disable-model-invocation"))
  try { l = true === parse(d.content).data?.["disable-model-invocation"] } catch {}
```

A skill hidden from Cursor's model is still discovered, just not offered.

### One thing not probed live

The user scope was proven through the real `~/.claude/skills`. Isolating it in a scratch HOME is not
possible: Cursor's auth lives under `$HOME`, and `HOME=/private/tmp/... cursor-agent status` prints
"Not logged in". Logging a scratch home in was out of bounds, so the user-scope behaviour for
`~/.cursor/skills` specifically rests on `Hr` and `Kr` being the same function with a different base
directory, plus the live `~/.claude/skills` result. Call it inferred, not probed.

### The old build, for the record

The first pass could not resolve the minified stat helper `gle` and left it open. It resolves, and
the lead was right:

```sh
grep -o 'import{readdir as [^}]*}from"fs/promises"' \
  ~/.local/share/cursor-agent/versions/2025.10.02-bd871ac/index.js
# import{readdir as Qra,readFile as Zhe,stat as gle}from"fs/promises"
```

`gle` is `stat`, not `lstat`. So even the 2025.10.02 build followed symlinks in
`loadRulesFromDirectory` and `dirIsDirectory`. The question was moot for skills at that version, but
the answer for rules was yes all along. Recorded because the old binary is still on disk and someone
may pin to it.

## The killed probe, finished: Hermes

The first pass killed a hung run and left this open. Bounded and repeated in an isolated tree:

```sh
HH=/private/tmp/hydra-t10/hermeshome
# HH/skills/hydra-probe-real/SKILL.md                    real dir
# HH/skills/hydra-probe-linkdir -> canon/hermes-linkdir  symlinked DIR
# HH/skills/hydra-probe-filelink/SKILL.md -> canon/hermes-filelink/SKILL.md
# HH/skills/cat/hydra-probe-cat/SKILL.md                 real, categorised
# HH/skills/catlink -> canon/hermes-catdir               symlinked CATEGORY dir
# HH/skills/hydra-probe-dangling -> /private/tmp/hydra-t10/no-such-target
HERMES_HOME=$HH /private/tmp/hydra-t10/to 120 hermes skills list
```

`HERMES_HOME` relocates the whole tree, and `hermes skills list` discovers and prints without a
model call, so this cost nothing. It returned well inside the 120 s bound, no hang:

| Shape | Discovered | Category shown |
|---|---|---|
| real directory | yes | none |
| symlinked directory | yes | none |
| `SKILL.md` is a symlink | yes | none |
| real, inside a category directory | yes | `cat` |
| inside a symlinked category directory | yes | `catlink` |
| dangling symlink | no, and no error | |

Hermes follows symlinks at both the skill level and the category level, and reports the category
under the **link** name, not the target's.

**Ticket 03's stated reason for this is wrong, and the wrong reason gives the wrong answer.** It
inferred symlink following from `Path.rglob` on Python 3.11. Run against the same tree on Hermes'
own interpreter:

```sh
/Users/USER/.hermes/hermes-agent/venv/bin/python -c "
import pathlib; p=pathlib.Path('$HH/skills')
print(sorted(str(x.parent.relative_to(p)) for x in p.rglob('SKILL.md')))"
# hermes python: 3.11.15
# ['cat/hydra-probe-cat', 'hydra-probe-filelink', 'hydra-probe-real']
```

`rglob` misses both symlinked directories. CPython 3.11's recursive wildcard selector skips entries
where `entry.is_dir() and not entry.is_symlink()` fails, so `**` never descends into a symlink. The
real mechanism is elsewhere, in `agent/skill_utils.py`:

```python
def iter_skill_index_files(skills_dir: Path, filename: str):
    matches = []
    for root, dirs, files in os.walk(skills_dir, followlinks=True):
        dirs[:] = [d for d in dirs if d not in EXCLUDED_SKILL_DIRS]
        if filename in files:
            matches.append(Path(root) / filename)
```

`os.walk(..., followlinks=True)`, explicit and version-independent. `tools/skills_tool.py`
`_find_all_skills` calls it; `hermes_cli/skills_hub.py` `do_list` calls `_find_all_skills`. The
conclusion in ticket 03 survives, the reasoning behind it does not, and the difference matters if
anyone reasons forward from `rglob` again.

Two more facts fell out of the same code, both relevant to the adoption question:

- `_find_all_skills` dedupes on the frontmatter `name` with a `seen_names` set, first scan wins. A
  name collision inside Hermes silently drops the loser, with no warning.
- `agent/skill_utils.py::get_external_skills_dirs` reads `skills.external_dirs` from `config.yaml`,
  expands `~` and `${VAR}`, and scans each existing directory after `~/.hermes/skills`. So Hermes
  has a sanctioned way to read a directory in place, without a symlink. Recording it as available,
  not proposing it.

## Gemini re-confirmed, and one correction to the first pass

Re-ran in a throwaway config root, same method as before:

```sh
G=/private/tmp/hydra-t10/geminihome
GEMINI_CLI_HOME=$G /private/tmp/hydra-t10/to 120 gemini skills list --all
```

`~/.gemini/skills/<name>/SKILL.md` is confirmed as the personal path at 0.42.0, with no plugin
wrapper anywhere in the probe tree. The directory does not exist on the real machine
(`ls ~/.gemini/skills` fails), so nothing there is being shadowed today. D9's "plugin-wrapped for
Gemini" stays wrong at this version.

**Correction: Gemini does discover a symlinked `SKILL.md`.** The first pass recorded that it does
not, and gave a mechanism for why. Both are wrong.

| Shape | First pass | This pass |
|---|---|---|
| real directory | discovered | discovered |
| symlinked directory | discovered | discovered |
| `SKILL.md` is a symlink, absolute target | **not discovered** | **discovered** |
| `SKILL.md` is a symlink, relative target | not tested | discovered |
| `SKILL.md` symlink with a broken target | not tested | not discovered, no error |
| dangling directory symlink | skipped silently | skipped silently |
| the `skills/` root itself a symlink | not tested | discovered through it |

I hit the likely cause of the first pass's error while making it myself. Two skill directories whose
`SKILL.md` links resolve to the same source file produce one listing, not two, because Gemini keys
on the frontmatter `name`. My first relative-link probe also silently dangled at the wrong depth and
looked exactly like "not discovered". Both failure modes are quiet.

The real mechanism, from the unminified bundle at
`/opt/homebrew/Cellar/gemini-cli/0.42.0/libexec/lib/node_modules/@google/gemini-cli/bundle/chunk-DN4XSYRG.js`:

```js
async function loadSkillsFromDir(dir) {
  const absoluteSearchPath = path54.resolve(dir);
  const stats = await fs45.stat(absoluteSearchPath).catch(() => null);
  if (!stats || !stats.isDirectory()) return [];
  const pattern = ["SKILL.md", "*/SKILL.md"];
  const skillFiles = await glob(pattern, { cwd: absoluteSearchPath, absolute: true, nodir: true,
                                           ignore: ["**/node_modules/**", "**/.git/**"] });
  ...
}
```

`stat` on the root, which is why a symlinked `skills/` works. The pattern is `SKILL.md` and
`*/SKILL.md` and nothing deeper, so Gemini supports no category nesting at all, unlike Hermes.

Precedence, from `discoverSkills`, lowest to highest: builtin, extensions, user
(`Storage.getUserSkillsDir()`), user agent skills (`getUserAgentSkillsDir()`), then project scope
but only when the folder is trusted. `clearSkills()` runs first, so each call is a full
rediscovery.

`~/.gemini/config/plugins/` is unchanged from what ticket 03 recorded:

```
~/.gemini/config/.migrated                      empty marker, Jun 21 19:27
~/.gemini/config/plugins/chrome-devtools-plugin/{installed_version.json,plugin.json,skills/}
    skills: a11y-debugging, chrome-devtools, debug-optimize-lcp, memory-leak-debugging, troubleshooting
~/.gemini/config/plugins/google-antigravity-sdk/{installed_version.json,plugin.json,skills/}
    skills: google-antigravity-sdk
```

Six plugin skills across two plugins. Nothing has migrated again since June.

## The adoption inventory

Names and shapes only, no file contents read beyond frontmatter key names. Counted with a `os.walk`
script over each root.

| Location | Entries | Symlinks | Note |
|---|---|---|---|
| `~/.claude/skills` | 11 | 11 of 11 | all point into `/Users/USER/Flava/claude/skills/<name>` |
| `~/.claude/plugins` | 31 skills under `<plugin>/skills/`, 64 unique names including the marketplace cache | 0 | `access` and `configure` each appear 3 times, in the discord, imessage and telegram plugins |
| `~/.cursor/skills` | absent | | Cursor's own personal root has never been created |
| `~/.cursor/skills-cursor` | 22 | 0 | vendor cache; ticket 03 counted 25, so it shrank by 3 |
| `~/.cursor/rules` | absent | | |
| `~/.cursor/plugins` | 10 skills | 0 | `cursor-public/sanity` and `cursor-public/atlassian` |
| `~/.hermes/skills` | 90 `SKILL.md` | 0 | ticket 03 said 24 packs; 88 are one level deep under a category, `dogfood` and `yuanbao` sit flat |
| `~/.gemini/skills` | absent | | |
| `~/.agents/skills`, `~/.codex/skills`, `~/.grok/skills` | absent | | all three are Cursor roots, so creating one has side effects |
| `~/hydra/skills` | absent | | the canonical store does not exist yet |

Re-run with:

```sh
for d in ~/.claude/skills ~/.cursor/skills ~/.cursor/skills-cursor ~/.cursor/rules \
         ~/.agents/skills ~/.codex/skills ~/.grok/skills ~/.gemini/skills ~/hydra/skills; do
  printf '%-28s %s\n' "$d" "$(ls "$d" 2>/dev/null | wc -l | tr -d ' ')"; done
find ~/.hermes/skills -name SKILL.md | wc -l
```

### Collisions

Across all six populated locations there are exactly 8 colliding names, and they are all the same
collision:

| Name | Locations |
|---|---|
| `claude-handoff` | `~/.claude/skills` + `~/.claude/plugins` |
| `implement-spec` | `~/.claude/skills` + `~/.claude/plugins` |
| `loop-me` | `~/.claude/skills` + `~/.claude/plugins` |
| `retro` | `~/.claude/skills` + `~/.claude/plugins` |
| `setup-ts-deep-modules` | `~/.claude/skills` + `~/.claude/plugins` |
| `writing-beats` | `~/.claude/skills` + `~/.claude/plugins` |
| `writing-fragments` | `~/.claude/skills` + `~/.claude/plugins` |
| `writing-shape` | `~/.claude/skills` + `~/.claude/plugins` |

The plugin side is `~/.claude/plugins/cache/mattpocock/mattpocock-skills/1.2.3/skills/in-progress/<name>/`.
So these are personal copies in `~/.claude/skills` sitting on top of the same eight skills shipped by
the mattpocock plugin. And these are exactly the eight that carry `disable-model-invocation`, which
is why Cursor lists none of them.

Nothing collides between `~/.claude/skills`, `~/.cursor/skills-cursor` and `~/.hermes/skills`. Near
misses that are not collisions: `create-skill` against `skill-creator`, `tdd` against
`test-driven-development`, `code-review` against `github-code-review` and `requesting-code-review`.

Two things worth flagging without deciding anything:

- The 8-name collision is live today, inside Claude Code alone, and predates Hydra.
- Because Cursor 2026.08.31 reads `~/.claude/skills`, `~/.codex/skills`, `~/.grok/skills` and
  `~/.agents/skills` as user roots, the collision surface is not one directory per provider. Writing
  into one provider's location changes what a second provider sees.

## Propagation mechanics

The mechanical question under the ticket's scenario is whether a CLI reads a skill file once or on
each use. Answers per provider, three from source, one partly unverified.

**Cursor: once, for skills. Live, for rules, and only inside the workspace.** Discovery reads the
whole file, not just the frontmatter:

```js
async bi(e){ const t = await readFile(e); return {skillFilePath:e, content:t, parsed:parse(t)}; }
```

and `Ei` puts that `content` straight into the in-memory skill object. There is a file watcher, but
its glob list is `[".cursor/rules/**/*.mdc","AGENTS.md","CLAUDE.md","CLAUDE.local.md",".cursorrules",
".cursorignore"]`, rooted at the workspace directory. No `SKILL.md`, and nothing under `$HOME`. So a
mid-session rewrite of a skill is invisible to a running Cursor session; a mid-session rewrite of a
project `.mdc` rule is picked up. Separately, the directory-listing cache has a 20 s TTL
(`CURSOR_RIPWALK_CACHE_TTL_MS`, default `2e4`).

**Gemini: once.** `loadSkillFromFile` returns `{name, description, location, body}`, body included,
at discovery time. `activate_skill` then serves `skill.body` from that in-memory object:

```js
return { llmContent: `<activated_skill name="${skillName}">\n  <instructions>\n    ${skill.body}\n ...` };
```

No disk read at activation. A rewrite lands only on the next `discoverSkills`, which runs at session
start.

**Hermes: on each use.** `tools/skills_tool.py::skill_view` re-reads the file every call:

```python
# Read the file once — reused for platform check and main content below
content = skill_md.read_text(encoding="utf-8")
```

`_find_all_skills`, which builds the name-and-description listing, reads the first 4000 bytes of
each `SKILL.md` on each call too. Whatever Hermes bakes into its system prompt at startup is fixed
for that session, but a skill fetched mid-session comes off disk fresh.

**Claude Code: metadata once, body unverified here.** The available-skills listing is fixed at
session start, which I can see directly in this session's own system prompt. That the body is read
at invocation rather than cached is documented behaviour I did not probe, because probing it means
launching a nested `claude`, and the brief's one hang warning made that a bad trade. Mark it
unverified.

## What I did not reach

- **Whether `gemini skills link <path>` makes a symlink or a copy.** Still open from the first pass.
  Not retried, because the symlink question it was meant to settle is now answered directly for
  every shape that matters.
- **Whether `installed_version.json` is load-bearing for a Gemini plugin.** Still open from ticket
  03, and now less interesting, since the personal path needs no plugin.
- **Cursor's user scope in isolation.** Proven through the real `~/.claude/skills` instead, for the
  auth reason given above. `~/.cursor/skills` specifically is inferred from shared code, not probed.
- **Claude Code's body-read timing.** Documented, not probed.
- **What happens when the same skill is reachable from two Cursor roots at once**, for example
  `~/.claude/skills/foo` and `~/.cursor/skills/foo` both linking to one canonical directory. Cursor
  has a `scope` field and an ordering, and Claude Code documents loading such a skill once, but I
  did not construct the case.

## Side effects on live config, stated plainly

The brief said read-only against real config. Two writes happened anyway, both by the vendor CLIs
rather than by me, and both worth knowing about:

- **`~/.hermes/skills/.hub/` was created.** Running `hermes skills list` once under the real
  `HERMES_HOME`, to learn the command's output shape before isolating it, calls `ensure_hub_dirs()`.
  That created `.hub/` with `audit.log` (0 bytes), `lock.json`, `taps.json` and empty `index-cache/`
  and `quarantine/`. It did not exist before, at 15:36 today. I left it in place rather than delete
  Hermes' own bookkeeping.
- **`~/.cursor/` gained session state.** The two `cursor-agent -p` runs created
  `~/.cursor/projects/private-tmp-hydra-t10-repo/` (a `.workspace-trusted` marker, `mcp-auth.json`,
  `repo.json`) and chat transcripts under `~/.cursor/chats/`. `cli-config.json` was rewritten by the
  CLI with the same content. No skills or rules directory under `~/.cursor` was created or touched;
  both remain absent.

Nothing was written to `~/.claude/skills`, `~/.gemini`, `~/hydra/skills` or `~/Flava`. Every probe
tree lived under `/private/tmp/hydra-t10/` and is deleted.
