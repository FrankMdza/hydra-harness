# 23 - MCP config across providers, against the ticket 10 shape

> **Corrected by ticket 29, 2026-09-03.** Two claims below are wrong. Cursor's approval state is
> *not* stuck outside a path Hydra owns: `CURSOR_DATA_DIR` moves `projects/<slug>/`, holding
> `mcp-approvals.json` and `mcp-disabled.json`, while `CURSOR_CONFIG_DIR` keeps `cli-config.json`
> and `chats/`, so the redirect costs neither the login nor ticket 13's harvest source. That also
> makes `mcp disable` per-run, so Cursor *can* scope a server off per run and Q6's "only one of the
> three" is now two of three. Probed live on the same build. Everything else here stands.

Written 2026-09-03. Every claim below is a command whose output is quoted or summarised in place.
Where something comes from reading a binary rather than running one, the text says so. The partial
progress recorded on the ticket was narration only, so nothing here rests on it; its one substantive
claim, that Claude reads the worktree's `.mcp.json`, turns out to be true and incomplete in a way
that matters.

## Builds under test

Checked at 17:02 local, before any probe ran.

```
claude --version         # 2.1.257 (Claude Code)
cursor-agent --version   # 2026.09.02-c22c1a3
agy --version            # 1.1.25
```

| CLI | Today | What the ticket says | Moved? |
|---|---|---|---|
| `claude` | 2.1.257 | 2.1.257 | no |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026.08.31-4057e58 | yes |
| `agy` | 1.1.25 | 1.1.23 | yes, twice, ticket 24 ran 1.1.24 |

The dead agent's `t23-claude-strings.txt` is still good, because `claude` has not moved. Its
`t23-agy-strings.txt` came off 1.1.23 and was re-pulled:

```
strings "$(which agy)" > agy125-strings.txt   # 833,330 lines
```

## The rig

Built fresh at
`/private/tmp/claude-501/-Users-USER/af206428-ef30-4368-bce7-83b9f81aa27e/scratchpad/t23b/`. Session
scratch, so it will not survive long.

```
mkdir -p "$T"/{repo,outside,home,external,store/snap-abc123}
cd "$T/repo" && git init -q -b main && echo x > README.md && git add -A && git commit -qm init
git worktree add -q "$T/wt" -b t23-run
```

`$T/home` is an isolated `HOME`. `$T/wt` is the linked worktree standing in for a Hydra run.

### The probe server, and the trick that made this ticket cheap

`$T/mcpserver.py` is a 30-line stdio MCP server that answers `initialize`, `tools/list`,
`resources/list` and `prompts/list`, and exposes one tool named `probe_<MARKER>`. Every probe config
points at it with a different marker, so a tool name identifies which config file produced it.

Then it got one more line, and that line is what made the rest tractable:

```python
with open(os.environ.get("T23_SPAWNLOG", .../"spawnlog.txt"), "a") as fh:
    fh.write("%s SPAWNLOG marker=%s ppid=%d cwd=%s\n" % (now, marker, os.getppid(), os.getcwd()))
```

The server appends a line when it starts. So "did this provider read that config file" becomes a
question about a text file rather than a question about a model's output, it works identically on all
three CLIs, and it distinguishes **connected** from **listed but never launched**, which turned out to
be the difference that matters twice.

## Free oracles, per provider

Worth writing down first, because it changes what a future ticket has to pay for.

`claude`, free and complete. Two routes.

```
claude mcp list                 # needs no login, works under an isolated HOME, health-checks
claude mcp get <name>           # prints the winning scope by name
claude -p "x" --model bogus-model-xyz
```

The third one is the good one. The model is rejected before the API call, so the run costs nothing,
but the session transcript at `~/.claude/projects/<slug>/<uuid>.jsonl` already carries an attachment
of type `deferred_tools_delta`:

```json
{"type":"deferred_tools_delta",
 "addedNames":[..., "mcp__t23-clash__probe_PROJECT", "mcp__t23-projonly__probe_PROJONLY"],
 "pendingMcpServers":[], "needsAuthMcpServers":[], "failedMcpServers":[]}
```

Server names, tool names and three failure buckets, for nothing. Ticket 24 found the same file
carries `skill_listing`; this is its MCP sibling and nobody had looked.

`cursor-agent`, free and complete, and it needs no login either:

```
cursor-agent mcp list                    # id: ready | not loaded (needs approval) | disabled
cursor-agent mcp list-tools <id>         # real tool names, and it really connects
```

`agy`, no free route to the resolved set. `agy mcp list` reads the user-level file only.
`/mcp` is refused in print mode, which at least says so for free:

```
$ agy --add-dir "$T/wt" -p "/mcp" --output-format json
{"status":"ERROR","error":"/mcp is not available in print mode (it opens the interactive MCP
 server view); pass --disable-slash-commands to send /mcp to the model as literal text",
 "num_turns":0,"usage":{"total_tokens":0}}
```

So `agy` cost four paid turns and the other two cost none. The spawn log is what made four enough.

## Q1: where the config lives

### `claude`, seven scopes

Named in the binary's own strings (`t23-claude-strings.txt`, line 384463 onward), in the order it
prints them:

```
Local config (private to you in this project)
Project config (shared via .mcp.json)
User config (available in all your projects)
Dynamic config (from command line)
Enterprise config (managed by your organization)
claude.ai config
Agent config (from agent frontmatter)
```

Four of them fired live. Mapping each to a path took the probes below.

| Scope | Path | Fired |
|---|---|---|
| local | `~/.claude.json` → `projects["<main repo root>"].mcpServers` | yes |
| project | `.mcp.json`, walking up from cwd | yes |
| user | `~/.claude.json` → root `mcpServers` | yes |
| dynamic | `--mcp-config <file or JSON string>` | yes |
| enterprise | `managed-mcp.json` | absent on this machine |
| claude.ai | cloud connectors | not probed |
| agent | agent frontmatter | not probed |

Plugins are a source too, though not a scope: the strings carry
`Suppressing plugin MCP server "<x>": duplicates manually-configured "<y>"` and
`duplicates earlier plugin server`, so a plugin server loses to a hand-written one of the same name.

**Project scope walks up from cwd and merges every hit.** Not the nearest one, all of them:

```
$ mkdir -p "$T/repo/.worktrees/wt2" && git -C "$T/repo" worktree add .worktrees/wt2 -b t23-run2
# .mcp.json at $T/repo declares t23-parentrepo, .mcp.json at wt2 declares t23-nestedwt
$ cd "$T/repo/.worktrees/wt2" && HOME="$T/home" claude mcp list
t23-parentrepo: ... ✔ Connected
t23-nestedwt:   ... ✔ Connected
```

The walk is a plain filesystem walk, not a git-root walk. It found `.mcp.json` two levels up, past a
git worktree boundary, and it found one in a directory that is not a repo at all:

```
$ mkdir -p "$T/outside/deep"        # .mcp.json at $T/outside declares t23-outside
$ cd "$T/outside/deep" && HOME="$T/home" claude mcp list
t23-outside: ... ✔ Connected
```

Consequence for Hydra: **a worktree placed inside the user's repo inherits the repo's `.mcp.json`,
and a worktree placed in `/tmp` inherits anything sitting above it there.** A sibling directory is
safe; an ancestor is not.

**Local scope is keyed by the main repo root, not by the worktree.** This one surprised me. Asked to
write a local-scope server while sitting in a linked worktree:

```
$ cd "$T/wt" && HOME="$T/home" claude mcp add -s local t23-addedlocal python3 "$T/mcpserver.py" ADDEDLOCAL
Added stdio MCP server t23-addedlocal ... to local config
File modified: .../home/.claude.json [project: .../t23b/wt]
```

The message names the worktree. The file disagrees. The entry landed under
`projects["/private/tmp/.../t23b/repo"]`, the main working tree, and a hand-written entry under
`projects[".../t23b/wt"]` was never read. So `claude`'s two project-shaped scopes resolve the project
two different ways in the same process: `.mcp.json` from cwd, local scope from the repo. **A Hydra
run that touches local scope writes into the user's main repo entry**, which is exactly the kind of
write outside a Hydra-owned path that ticket 10 rules out.

`--add-dir` does not add a project MCP root:

```
$ cd "$T/outside" && claude -p "x" --model bogus-model-xyz --add-dir "$T/wt"
mcp tools: []
```

### `cursor-agent`, two files and a plugin source

From the bundle at `~/.local/share/cursor-agent/versions/2026.09.02-c22c1a3/`, the loader is explicit
about it, and it appears in that literal form in seven different modules:

```js
m = join(homedir(), ".cursor", "mcp.json"),
u = join(projectRoot, ".cursor", "mcp.json"),
h = Object.assign(Object.assign({}, b.mcpServers), f.mcpServers)
```

That is the whole path table. Two files, plus servers shipped by a plugin
(`pluginDefinitionSource`), plus a `managedDefinitionSource` that this machine has nothing for. The
user-facing string agrees: `Configure servers in .cursor/mcp.json or ~/.cursor/mcp.json, or install a
plugin that ships an MCP server.`

The project root comes from `--workspace`, or cwd when that is absent. `--add-dir` does not move it:

```
$ cd "$T/outside" && HOME="$T/home" cursor-agent mcp list
t23-cursor-project: ready          <- the USER definition of that name
t23-cursor-useronly: ready
$ HOME="$T/home" cursor-agent --workspace "$T/wt" mcp list
t23-cursor-project: not loaded (needs approval)   <- now the PROJECT definition
t23-cursor-useronly: ready
$ HOME="$T/home" cursor-agent --add-dir "$T/wt" mcp list
t23-cursor-project: ready          <- back to the user definition, --add-dir did nothing
```

Cursor keeps per-project MCP state, and it keeps it in the user's home rather than the workspace.
Five files, named in the bundle, under `~/.cursor/projects/<path-slug>/`:

```
mcp-approvals.json   mcp-auth.json   mcp-cache.json   mcp-disabled.json   mcp-group-selections.json
```

`cursor-agent` does not read `.mcp.json`. It was sitting in the worktree throughout, with two servers
in it, and never appeared in a Cursor listing. Given that ticket 26 found Cursor reads Claude Code's
`settings.json` for hooks, I expected the same courtesy here. It is not there.

### `agy`, one documented file, one undocumented one

`agy` ships its own MCP reference inside the binary. From `agy125-strings.txt` line 18937:

```
## Configuration File (`mcp_config.json`)
### Location
*   **Global Configuration**: `~/.gemini/config/mcp_config.json` (applies to all sessions).
*   **Plugin Configuration**: `plugins/<plugin_name>/mcp_config.json` (active when the plugin
    is enabled).
```

Two locations. The binary is wrong about its own product, or at least short by one. Four candidate
worktree paths went in at once, each with its own marker, and one paid turn sorted them:

```
$T/wt/.agents/mcp_config.json                          -> AGYAGENTSROOT
$T/wt/.agents/plugins/t23probe/mcp_config.json         -> AGYPLUGIN
$T/wt/mcp_config.json                                  -> AGYWTROOT
$T/wt/.gemini/config/mcp_config.json                   -> AGYWTGEMINI

$ cd "$T/outside" && agy --add-dir "$T/wt" --model gemini-3.8-flash-low -p "Reply OK." --output-format json
status SUCCESS  turns 1  total_tokens 15361
$ cat "$T/spawnlog.txt"
2026-09-03T17:15:59 SPAWNLOG marker=AGYPLUGIN ...
2026-09-03T17:15:59 SPAWNLOG marker=AGYAGENTSROOT ...
```

Two fired. `<worktree>/mcp_config.json` and `<worktree>/.gemini/config/mcp_config.json` did not, and
neither did `.mcp.json`.

So `<worktree>/.agents/mcp_config.json` is real and undocumented. The plugin route works too, and it
needs the full plugin shape, a `plugin.json` beside the `mcp_config.json`. That route is worth
knowing because it costs nothing to verify: a skill dropped in the same plugin shows up in the free
`/skills` listing, namespaced by plugin, which proves the plugin loaded without spending a turn.

```
$ agy --add-dir "$T/wt" -p "/skills" --output-format json     # num_turns 0, total_tokens 0
{'name': 't23probe:t23-agy-marker', 'plugin': 't23probe',
 'path': '.../wt/.agents/plugins/t23probe/skills/t23-agy-marker/SKILL.md'}
```

One detail with teeth: the server process starts with the **launch cwd**, not the worktree.

```
SPAWNLOG marker=AGYAGENTSROOT ppid=79430 cwd=/private/tmp/.../t23b/outside
```

A stdio server configured with a relative path in `args` breaks under `--add-dir`, and breaks
silently.

`agy`'s user scope is one file, `~/.gemini/config/mcp_config.json`, which on this machine is the
zero-byte file it has been since June. Nothing about invalid JSON appeared in the logs, so `agy`
tolerates it.

## Q2: is a worktree-scoped file read at all

This was the load-bearing question. All three read one, and all three want to be told about the
worktree in a different way.

| Provider | Worktree file | What the launch needs |
|---|---|---|
| `claude` | `<worktree>/.mcp.json` | cwd must be the worktree or below it. `--add-dir` does nothing |
| `cursor-agent` | `<worktree>/.cursor/mcp.json` | cwd, or `--workspace <worktree>`. `--add-dir` does nothing |
| `agy` | `<worktree>/.agents/mcp_config.json` | `--add-dir <worktree>` is required. cwd is not enough |

**Ticket 10's `agy` rule reproduces exactly on MCP.** Same directory, same file, two runs:

```
$ cd "$T/wt"      && agy --model gemini-3.8-flash-low -p "Reply OK." --output-format json
status SUCCESS  total_tokens 14295      spawnlog: (none)

$ cd "$T/outside" && agy --add-dir "$T/wt" --model gemini-3.8-flash-low -p "Reply OK." ...
status SUCCESS  total_tokens 15362      spawnlog: AGYAGENTSROOT, AGYPLUGIN
```

Sitting in the worktree loads nothing. Being told about it from an unrelated directory loads
everything. Ticket 13's launch requirement for `agy` covers MCP for free, and it is again the
difference between the config existing and the config existing.

### The git problem, which nobody has raised

Ticket 10's shape depends on a directory that ignores itself, so `git status` stays clean. Two of
the three MCP files can do that. Claude's cannot, because it sits at the worktree root.

```
$ printf '*\n' > "$T/wt/.cursor/.gitignore"
$ printf '*\n' > "$T/wt/.agents/.gitignore"
$ cd "$T/wt" && git status --short
?? .mcp.json
$ git add -A && git status --short
A  .mcp.json
$ git check-ignore -v .cursor/mcp.json .agents/mcp_config.json     # both IGNORED
```

(The listing above is trimmed to the injected files; my other probe paths are noise.)

So injecting `.mcp.json` dirties the worktree the user's repo is looking at, `git add -A` stages it,
and there is nowhere to hide it: ticket 10 already established that a linked worktree has no private
`info/exclude`. **`--mcp-config <path outside the worktree>` avoids the whole problem**, is already
proven below, and is the obvious answer. But it is a design choice, not a detail, and it means
Claude's MCP injection does not use the same mechanism as Claude's skill injection.

## Q3: is a symlinked config file followed

Yes, on all three, absolute or relative, including into a ticket-10-style snapshot directory.

```
$ ln -s "$T/store/snap-abc123/mcp.json" "$T/wt/.mcp.json"
$ cd "$T/wt" && HOME="$T/home" claude mcp list
t23-snapshot: ... ✔ Connected

$ ln -s ".hydra/mcp.json" "$T/wt/.mcp.json"          # relative, inside the worktree
t23-relsymlink: ... ✔ Connected

$ ln -s "$T/store/snap-abc123/cursor-mcp.json" "$T/wt/.cursor/mcp.json"
$ HOME="$T/home" cursor-agent mcp list
t23-cursor-project: not loaded (needs approval)      # and it spawns once approved, below

$ ln -s "$T/store/snap-abc123/agy-mcp.json" "$T/wt/.agents/mcp_config.json"
$ cd "$T/outside" && agy --add-dir "$T/wt" --model gemini-3.8-flash-low -p "Reply OK." ...
spawnlog: AGYSYMLINK, AGYDUP
```

Two things worth pulling out.

**Cursor follows a symlinked MCP config and refuses a symlinked hook config.** Ticket 26 found
`refusing to load config path that contains a symlink` on the four project-scoped hook entries, and
concluded Hydra must write a real hook file. That guard does not extend here: the MCP loader has no
`pathContainsSymlink` call, and the symlinked `.cursor/mcp.json` loaded in a real paid run as well as
in `mcp list`. Two loaders in one binary, two rules. Do not generalise either one.

**A dangling symlink is silence, not an error.** `claude` reported `No MCP servers configured` and
exited 0. The binary carries a string for the other case,
`Can't read .mcp.json: it isn't a regular file or is larger than 5[MB]`, so a symlink to a real file
passes the regular-file check because the stat follows the link. A broken link produces nothing at
all, which is the failure mode ticket 10 already got bitten by on Gemini and `SKILL.md`.

## Q4: the format

The stdio shape is identical on all three, and I mean identical: the same JSON body, byte for byte,
loaded in `claude`, `cursor-agent` and `agy` in this session.

```json
{ "mcpServers": { "<name>": { "command": "python3", "args": ["...", "MARKER"], "env": {} } } }
```

Everything else diverges.

| | `claude` | `cursor-agent` | `agy` |
|---|---|---|---|
| file name | `.mcp.json` | `mcp.json` | `mcp_config.json` |
| project path | worktree root | `.cursor/` | `.agents/` |
| remote key | `"type"` + `"url"` + `"headers"` | `"url"` | `"serverUrl"`, and `"url"` since a later release |
| remote types | `stdio`, `sse`, `http` (or `streamable-http`), `ws`, `sdk` | http | stdio or SSE |
| extra keys | env and header `${VAR}` expansion, policy predicates | | `enabledTools`, `timeoutSeconds`, `tools.eager` |
| reserved names | `__proto__` and others refused by name | | |
| mixing modes | | | `MCP server %q cannot have both command and serverUrl` |

The `claude` transport list and the reserved-name rule come from the binary's strings
(`Valid types are: stdio, sse, http (or streamable-http), ws, sdk`,
`"__proto__" is a reserved MCP server name and was not loaded`). The `agy` remote keys come from its
embedded doc and its own error strings. I did not probe remote transports on any provider.

So: **one canonical document can generate all three, and cannot be all three.** A store entry holding
a name, a command, args and env renders to three files with three names in three directories. A store
entry holding a URL needs a per-provider renderer, because the key is spelled differently in each and
`agy` refuses the combination the other two allow. That is the same conclusion ticket 10 reached about
`SKILL.md`, arrived at from the opposite direction: the portable subset is real and it is small.

## Q5: what happens to a server the user already configured

Three providers, three answers, and for once one of them speaks up.

| | `claude` | `cursor-agent` | `agy` |
|---|---|---|---|
| user and project, same name | project wins | project wins | not probed |
| does it say so | **yes, a printed diagnostic** | no | no |
| two project sources, same name | nearest `.mcp.json` wins, silently | not applicable, one file | root `.agents/` beats plugin, silently |
| command-line source | `--mcp-config` beats project | none | none |

`claude` is the only surface anywhere on this map that reports a collision:

```
$ cd "$T/wt" && HOME="$T/home" claude mcp list
t23-clash: python3 .../mcpserver.py PROJECT - ✔ Connected
...
MCP config diagnostics ⚠

[Conflicting scopes]
├ Server "t23-clash" is defined in multiple scopes with different endpoints: user (... USER),
  project (... PROJECT). OAuth tokens are stored per endpoint, so authenticating in one context
  will not carry over.
└ Keep the correct endpoint and remove the others: `claude mcp remove t23-clash -s user` or
  `claude mcp remove t23-clash -s project`

$ HOME="$T/home" claude mcp get t23-clash
  Scope: Project config (shared via .mcp.json)
```

Ticket 24 found every skill collision resolved silently on every provider. This one names both
endpoints and tells you the fix. Worth saying out loud because it means Hydra can read a Claude
collision instead of modelling it, the same way ticket 24 said to ask rather than compare strings.
The diagnostic fires on `mcp list`, which is free.

Dynamic beats project:

```
$ claude -p "x" --model bogus-model-xyz --mcp-config "$T/dynamic-clash.json"
mcp tools: ['mcp__t23-clash__probe_DYNCLASH', 'mcp__t23-projonly__probe_PROJONLY']
```

`agy` collapses a duplicate with no comment. `t23-agy-plugin` declared in both
`.agents/mcp_config.json` and `.agents/plugins/t23probe/mcp_config.json`, one process starts, the
root file wins:

```
spawnlog: AGYSYMLINK, AGYDUP        # AGYDUP is the root file's version, AGYPLUGIN never ran
```

### The Cursor hazard, which is worse than a silent drop

Cursor merges with `Object.assign({}, user, project)`, so the project entry replaces the user one by
name. Project entries need approval and user entries do not. Put those two rules together and you get
this:

```
$ rm -rf "$T/home/.cursor/projects"          # clear approvals
$ cd "$T/wt" && HOME="$T/home" cursor-agent mcp list
t23-cursor-project: not loaded (needs approval)
t23-cursor-useronly: ready
$ cat "$T/spawnlog.txt"
SPAWNLOG marker=CURSORUSER        <- the human's server, started
SPAWNLOG marker=CURSORUSERONLY
```

The listing says the name is not loaded. The process that started under that name is the **user's**
definition, not the project's. Control run, same everything, with the user-scope duplicate deleted:

```
$ cat "$T/spawnlog.txt"
SPAWNLOG marker=CURSORUSERONLY    <- and nothing for t23-cursor-project
```

So an unapproved project entry does not shadow the human's server, it just fails to replace it, and
the display reports the opposite. Once approved, the project command is what runs:

```
$ HOME="$T/home" cursor-agent mcp enable t23-cursor-project
✓ Enabled and approved MCP server: t23-cursor-project
$ HOME="$T/home" cursor-agent mcp list-tools t23-cursor-project
Tools for t23-cursor-project (1):
- probe_CURSORPROJECT ()
```

For Hydra this is the collision case that produces the wrong answer rather than no answer. Inject a
server named `github` into a worktree, forget to approve it, and the run gets the human's `github`
with the human's token while `mcp list` claims nothing is loaded.

## Q5b: the approval gate, which nobody asked about and which decides everything

`claude mcp list` reports a project `.mcp.json` server as pending approval. A headless run in the
same directory connects to it anyway.

```
$ cd "$T/wt" && claude mcp list
t23-clash: ... - ⏸ Pending approval (run `claude` to approve)
t23-projonly: ... - ⏸ Pending approval (run `claude` to approve)

$ claude -p "x" --model bogus-model-xyz
mcp tools: ['mcp__t23-clash__probe_PROJECT', 'mcp__t23-projonly__probe_PROJONLY']
pendingMcpServers: []
$ cat "$T/spawnlog.txt"
SPAWNLOG marker=PROJECT ...
SPAWNLOG marker=PROJONLY ...
```

Both processes ran. No entry was written to `~/.claude.json`, `hasTrustDialogAccepted` stayed false,
and no approval was recorded anywhere. **In print mode the approval gate does not apply.** Good news
for injection, since it means Hydra needs no approval step on Claude. Less good as a fact about the
world: any repo's `.mcp.json` is auto-trusted by any headless `claude -p` run in it.

Which surfaces can approve, for the interactive case, took six probes and the answer is narrow:

| Where | Key | Approves? |
|---|---|---|
| `~/.claude/settings.json` | `enableAllProjectMcpServers: true` | yes |
| `~/.claude/settings.json` | `enabledMcpjsonServers: ["name"]` | yes |
| `<worktree>/.claude/settings.json` | either key | **no** |
| `<worktree>/.claude/settings.local.json` | either key | **no** |
| `~/.claude.json` `projects[<worktree>]` | `enabledMcpjsonServers` | no, wrong key, see Q1 |

The project-scope settings file is the one Hydra would want and it is the one that does not work. It
does not matter for headless runs, and it matters a lot the day someone runs Hydra interactively.

Cursor's approval is per workspace and lives outside the worktree:

```
$ cursor-agent --workspace "$T/wt" --approve-mcps -f --model composer-2.5 -p "Reply with the single word OK."
{"subtype":"success","result":"OK","usage":{"inputTokens":7362,"outputTokens":37,"cacheReadTokens":8915}}
$ cat "$T/spawnlog.txt"
SPAWNLOG marker=CURSORPROJECT ...
$ cat ~/.cursor/projects/<slug>/mcp-approvals.json
[ "t23-cursor-project-b4481aba0bd1c986" ]
```

`--approve-mcps` works headlessly, the symlinked project config loaded in the real run, and the
approval persists to the user's home under a hash of the server definition, so changing the command
invalidates it. **That write is outside any path Hydra owns**, and it is per workspace, so every run
in a fresh worktree leaves another directory in `~/.cursor/projects/`.

`agy` has no approval step that appeared in any of the four paid runs. Servers from a worktree
config just start.

## Q6: can a server be scoped off

Only one of the three can do it per run, and it can do it three ways.

`claude`, all free, all verified by the absence of a spawn log line rather than by the absence of a
tool name, so these stop the process rather than hiding it:

```
$ claude -p "x" --model bogus-model-xyz --strict-mcp-config
mcp tools: []           spawnlog: (none)

$ claude -p "x" --model bogus-model-xyz --strict-mcp-config --mcp-config "$T/dynamic.json"
mcp tools: ['mcp__t23-dynamic__probe_DYNAMIC']

$ claude -p "x" --model bogus-model-xyz --restricted
mcp tools: []           spawnlog: (none)

$ echo '{"disabledMcpjsonServers":["t23-clash"]}' > s.json
$ claude -p "x" --model bogus-model-xyz --settings s.json
mcp tools: ['mcp__t23-projonly__probe_PROJONLY']        # t23-clash gone, per name
```

Two of those deserve a note.

`--restricted` alone drops every MCP server, and `claude --help` says it does not. The help reads
"ignores user, project and local settings files (managed settings and `--settings` still apply; add
`--strict-mcp-config` to skip MCP servers too)". Three runs, alternating, same worktree, same config
directory: baseline three servers, `--restricted` zero, baseline three servers again, and no server
process started under `--restricted`. Either the help is stale or the behavior is, but on 2.1.257 the
flag is stronger than its documentation.

`allowedMcpServers` in a `--settings` file is a kill switch and nothing more. `[]` blocks everything,
which is expected; `["t23-projonly"]` and `["*"]` also block everything, which is not. So the entry
grammar is not a bare server name and I did not work out what it is. `deniedMcpServers` from
`--settings` had no effect at all, which fits the binary's note that the allow direction is
managed-settings territory. **`--strict-mcp-config` is the flag to use.** It is documented, it is
exact, and it composes with `--mcp-config` to give a run precisely the servers Hydra chose.

`cursor-agent`: no flag. `cursor-agent mcp disable <id>` works, and writes into the user's home:

```
$ cd "$T/wt" && HOME="$T/home" cursor-agent mcp disable t23-cursor-useronly
✓ Disabled MCP server: t23-cursor-useronly
$ cat "$T/home/.cursor/projects/<wt-slug>/mcp-disabled.json"
[ "t23-cursor-useronly" ]
$ cd "$T/outside" && HOME="$T/home" cursor-agent mcp list
t23-cursor-useronly: ready              <- the disable did not follow
```

Per workspace, stored outside the workspace. Usable, and it costs a write in `~/.cursor/`.

`agy`: no flag either. `agy --help` has nothing matching `mcp`, and neither do its strings.
`agy mcp disable <name>` exists and, from the subcommand help and the changelog embedded in the
binary, edits the user-level `mcp_config.json`, which makes it global rather than per run. I did not
run it, because running it would modify the user's config. Same reason the `agy` global-versus-worktree
collision is untested: it needs a write to `~/.gemini/config/mcp_config.json`.

## The delegation hole is real, and Hydra owns two thirds of it

**A delegated child inherits every MCP server the human configured, on all three providers, and only
`claude` has a per-run way to stop it.**

Probed directly rather than argued. A user-scope server was added to a **copy** of `~/.claude.json`
and the run pointed at the copy, so nothing of the user's was touched:

```
$ cp ~/.claude.json "$T/cfgdir/.claude.json"
$ python3 -c "...inject t23-humanuser into the COPY's root mcpServers..."
$ cd "$T/wt" && CLAUDE_CONFIG_DIR="$T/cfgdir" claude -p "x" --model bogus-model-xyz
mcp tools: ['mcp__t23-clash__probe_PROJECT',
            'mcp__t23-humanuser__probe_HUMANUSER',      <- the human's server
            'mcp__t23-projonly__probe_PROJONLY']
$ cat "$T/spawnlog.txt"
SPAWNLOG marker=HUMANUSER ...       <- and its process ran
```

An isolated worktree, a child that Hydra spawned, and the human's MCP server is in its tool list and
running. For the other two the same conclusion falls out of Q1 with no probe left to run: Cursor
merges `~/.cursor/mcp.json` into every workspace, and `agy`'s own documentation says the global file
"applies to all sessions". Neither has a flag.

| | inherits the human's servers | per-run off switch |
|---|---|---|
| `claude` | yes, probed | `--strict-mcp-config`, or `--restricted` |
| `cursor-agent` | yes, from the loader and from `mcp list` at any cwd | none, only a per-workspace write into `~/.cursor/` |
| `agy` | yes, per its own docs | none, only a global edit of the user's file |

Why this is ticket 11's problem and not a footnote here. Ticket 11 says a child's permissions come
from a policy the parent passes at spawn, deny by default for anything unlisted. An MCP server is a
tool set with credentials attached, and it arrives without passing through that policy. Ticket 12
says a delegated child receives no credential of any kind, and this is the exception: an inherited
MCP server carries the human's token inside the server process, so the child gets the access without
ever being handed the secret. Depth is capped at 2 and fan-out is not, so one delegation with a wide
fan-out puts every one of the human's connected services in front of every child.

The one piece of good news, and it is real. Ticket 26 proved all three providers can hard-block an
MCP call from a config file Hydra writes per run: `beforeMCPExecution` on Cursor, `PreToolUse`
matcher `mcp__*` on Claude, `PreToolUse` matcher `call_mcp_tool` on `agy`. So the hole is about
**exposure, not execution**. The tools are in the child's prompt and the servers are running as
processes; whether a call goes through is still Hydra's to decide. That is a much smaller hole than
it looked like, and it is still a hole, because a policy engine that cannot say "this child sees no
MCP" on two of three providers is not doing what ticket 11 says it does.

## What ticket 10's shape survives, and what it does not

Survives unchanged. Per-run injection into the worktree works on all three. Symlinks into an
immutable snapshot work on all three. A first-party root per provider is the right model, and the
roots are the same three families as the skill roots. `agy` needs `--add-dir` here for the same
reason and with the same consequence.

Does not survive.

**The self-ignoring directory covers two providers, not three.** `.cursor/mcp.json` and
`.agents/mcp_config.json` sit inside a directory that can carry a `.gitignore` of `*`.
`.mcp.json` sits at the worktree root and cannot. `--mcp-config` sidesteps it.

**"Nothing written outside a path Hydra owns" does not hold for Cursor.** Approving an injected
server writes `~/.cursor/projects/<slug>/mcp-approvals.json`, and disabling one writes
`mcp-disabled.json` beside it. One directory per worktree, and worktrees are disposable.

**Failing the run on a name collision does not transfer.** Ticket 10 fails a run when the store
collides with the world, and ticket 24 endorsed it because no rule about who wins could be written.
MCP is not in that position: `claude` prints the conflict and picks project, Cursor picks project and
lies about it until approved, `agy` picks the outer file. Failing the run is still defensible, but
the reason ticket 24 gave for it is gone here, and Cursor's failure mode is one Hydra should detect
rather than prevent, because the detection is free.

## Costs

| Provider | Runs | Cost |
|---|---|---|
| `claude` | ~20 free oracle runs, plus a dozen `mcp list` / `mcp get` | zero, every run rejected before the API call |
| `cursor-agent` | 1 paid, plus ~15 free `mcp list` / `list-tools` / `enable` / `disable` | 7,362 input, 8,915 cache read, 37 output, on `composer-2.5` |
| `agy` | 4 paid, plus several free `/skills` and `mcp list` | 60,383 total tokens, all `gemini-3.8-flash-low` |

`agy` is the expensive one because it has no free route to its resolved MCP set. If a future ticket
needs `agy` MCP facts, budget a turn each and batch the questions into one run the way the four-path
probe above did.

## What the CLIs wrote, and what was cleaned up

Nothing of the user's MCP configuration changed. Verified after every phase:

```
~/.gemini/config/mcp_config.json   0 bytes, unchanged since June
~/.cursor/mcp.json                 does not exist, never created
~/.mcp.json                        does not exist
~/.claude.json root mcpServers     []
~/.claude/settings.json            same 8 keys, no MCP key added
```

Every user-scope MCP probe ran under `HOME="$T/home"` or against a **copy** of `~/.claude.json`
handed to `CLAUDE_CONFIG_DIR`. The copy stays in scratch.

Removed after the paid Cursor run, because it recorded an approval for a probe server:

```
rm -rf ~/.cursor/projects/private-tmp-...-t23b-wt/    # mcp-approvals.json, mcp-auth.json, worker.log
```

Left in place, as provider bookkeeping for scratch paths, matching what ticket 24 left:

- `~/.claude/projects/-private-tmp-...-t23b-{wt,repo,outside}/`, the free-oracle transcripts
- one entry in `~/.claude.json` under `projects[".../t23b/repo"]`, all-empty defaults, written by
  Claude Code's git-root resolution when runs happened in the worktree. It carries
  `mcpServers: {}`, `enabledMcpjsonServers: []` and `disabledMcpjsonServers: []`, so it records no
  MCP state
- `~/.cursor/chats/<hash>/` for the one paid Cursor run
- `agy` bookkeeping under `~/.gemini/antigravity-cli/`. It wrote no MCP file

No probe MCP server process is still running (`pgrep -fl mcpserver.py` is empty).

## Recommendation: its own decision ticket, and a narrow one

Fold the mechanics into the harness spec. Per-run injection, symlinks into the snapshot, the
first-party root per provider, `--add-dir` for `agy`, `--workspace` for Cursor. All of that is ticket
10's shape working, and it needs a table in the spec rather than a decision.

Open a decision ticket for the three things that are choices rather than facts. It should be small,
it should not reopen ticket 10, and it has exactly three questions.

**Does Claude's MCP config go in the worktree or on the command line?** `.mcp.json` at the worktree
root cannot self-ignore, so it dirties a tree ticket 10 promised stays clean, and it is the only
provider where Hydra has a second option. `--mcp-config` plus `--strict-mcp-config` gives an exact
server set, writes nothing into the worktree, and makes the run record reproducible in a way the
other two cannot match. Choosing it means Claude's MCP injection deliberately stops mirroring
Claude's skill injection, and that asymmetry should be written down on purpose rather than
discovered later.

**What does Hydra do about Cursor's approval state?** Every injected Cursor server needs approval, and
approving writes into `~/.cursor/projects/<slug>/`. `--approve-mcps` at spawn is the cheap answer and
it approves everything the workspace can see, including whatever the user's repo carries in its own
`.cursor/mcp.json`. Naming servers individually needs a write in the user's home per worktree. Either
way ticket 10's rule about writes bends, and someone should decide which way.

**Does a delegated child get MCP at all?** This is the one that cannot fold into a spec, because it
contradicts something ticket 11 already decided. On `claude` the answer can be enforced today. On
Cursor and `agy` it cannot, and the fallback is ticket 26's hook gate blocking `mcp__*`,
`beforeMCPExecution` and `call_mcp_tool`, which stops the calls and leaves the tools in the prompt
and the server processes running with the human's tokens. The honest options are to accept exposure
and gate execution, or to route MCP-bearing delegations to `claude` only. Both are decisions, and
ticket 11 should make them rather than this ticket.

Everything else in this file is a fact, and facts belong in the spec.
