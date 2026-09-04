# 29 - Where MCP config goes, and what to do about Cursor's approval state

Type: grilling
Status: resolved
Blocked by: 23

## Question

Graduated from ticket 23, which found MCP config rides ticket 10's injection shape on all three
providers and then found two places it does not fit. The mechanics fold into the harness spec as
written; these two need a decision first.

- **Where Claude's MCP config goes.** `claude` reads `.mcp.json` by walking up from cwd and merging
  every ancestor's file, nearest name wins silently. It also takes `--mcp-config` on the command
  line, and `--strict-mcp-config` makes that the only source. A file in the worktree matches ticket
  10's shape and inherits the ancestor merge; the command line does not, and it is the only form
  that composes with the strictness flag. Decide which, and say what happens to the ancestor merge
  either way, since a worktree inside `~/hydra` would pick up anything Hydra leaves at the repo root.

- **Cursor's approval state, which lands outside any path Hydra created.** Approving a Cursor server
  writes into `~/.cursor/projects/<slug>/`, so a run cannot approve its own server without writing
  into the user's home, and D9 forbids that. Meanwhile an unapproved project entry does not shadow
  the human's same-named server, it fails to replace it, and the human's server process starts.
  Decide whether Hydra ships Cursor MCP config at all in v1, and if it does, how it stops a
  same-named user server from running in place of the one the run asked for.

One thing not to reopen here. `.mcp.json` sits at the worktree root and cannot self-ignore, so
`git status` shows it and `git add -A` stages it, while `.cursor/mcp.json` and
`.agents/mcp_config.json` both can. That is a fact for whichever answer wins rather than a decision.

Whether a delegated child gets MCP at all is not this ticket. Ticket 23 found a child inherits
user-scope servers on all three and that only `claude` can scope them off per run. Ticket 27 owns
that, since it is a delegation policy question.

Read the `## Answer` on ticket 23 and
[research/23-mcp-config-injection.md](../research/23-mcp-config-injection.md) first.

## The third question is answered, 2026-09-03

Ticket 27 settled whether a delegated child gets MCP at all, so it is not this ticket's. **Deny in a
policy means no MCP call succeeds, not that the child sees nothing.** The gate blocks calls on all
three; the inherited server still starts and its tool descriptions still reach the model's context on
Cursor and `agy`. The record carries which servers were visible, and `claude --restricted` drops MCP
entirely, so the clean version exists on one provider.

That narrows this ticket to its first two bullets, and it sharpens the second: since a child cannot
call an inherited server anyway, the question about Cursor's approval state is about whether Hydra
ships Cursor MCP config at all, not about protecting a child from the human's servers.

## Answer, 2026-09-03

Both bullets decided. The second one changed shape before it could be answered, because the fact it
rested on turned out to be wrong.

Versions re-read at the top of the session, all three unchanged since ticket 23 probed them:
`claude` 2.1.257, `cursor-agent` 2026.09.02-c22c1a3, `agy` 1.1.25.

### The finding that reopened the second bullet

Ticket 23 concluded that approving a Cursor server writes outside any path Hydra created, and called
it D9 broken by the vendor. There is a lever it did not find. `cursor-config/dist/paths.js` resolves
two roots with two different functions:

```js
function o(){ const e=process.env.CURSOR_CONFIG_DIR; if(e?.trim()) return e;
              const t=process.env.XDG_CONFIG_HOME;
              return t?.trim() ? join(t,"cursor") : join(homedir(),".cursor") }
function c(){ const e=process.env.CURSOR_DATA_DIR;   if(e?.trim()) return e;
              return join(homedir(),".cursor") }
function l(){ return join(c(),"projects") }
```

`projects/<slug>/`, which holds `mcp-approvals.json` and `mcp-disabled.json`, hangs off the data
dir. `cli-config.json`, which holds `authInfo`, and `chats/`, which is ticket 13's harvest source,
hang off the config dir. Setting one moves the approval state and leaves the login and the transcript
where they were.

Probed live rather than read, on 2026.09.02-c22c1a3, with `CURSOR_DATA_DIR` pointed at a scratch path
under `~/hydra`:

```
$ CURSOR_DATA_DIR="$D" cursor-agent --workspace "$S/wt" mcp list
t29-probe: not loaded (needs approval)
$ cd "$S/wt" && CURSOR_DATA_DIR="$D" cursor-agent mcp enable t29-probe
✓ Enabled and approved MCP server: t29-probe
$ find "$D" -type f
$D/projects/<flattened-worktree-path>/mcp-approvals.json
$ CURSOR_DATA_DIR="$D" cursor-agent mcp list
t29-probe: Error: Connection failed          # past the gate, spawned, and /bin/echo is no server
$ cursor-agent mcp list                      # same cwd, no redirect
t29-probe: not loaded (needs approval)
$ ls ~/.cursor/projects | wc -l
41                                           # unchanged at every step, before and after
```

`mcp disable` redirects too, writing `mcp-disabled.json` beside the approvals, and a later listing
in the same workspace reports `t29-probe: disabled`. The probe data directory was removed.

Two mechanics that decide how the launch is written: **`mcp enable` resolves the project from cwd and
ignores `--workspace`**, erroring with `MCP server 't29-probe' not found in configuration`, while
`mcp list` honours `--workspace`. And the slug is the flattened absolute workspace path, so the same
tree always produces the same directory name.

So Cursor can be given a config root Hydra owns, ticket 22's rule is satisfiable literally, and the
second bullet stops being "does Hydra ship no Cursor MCP config in v1".

### Claude's config goes on the command line, and the ancestor merge is deleted rather than defended

**Recorded runs get `--mcp-config <file in the run's spawn directory>` plus `--strict-mcp-config`.**
The run's server set is exactly what Hydra passed. Ticket 23 verified the strict flag by absence of a
spawn log line, so the other scopes do not start rather than being hidden from the tool list.

The ancestor merge is not defended against, it stops existing. Under the strict flag the walk up from
cwd never happens, so it no longer matters that the walk is a plain filesystem walk that crosses git
boundaries, that it merges every hit rather than the nearest, or where Hydra puts its worktrees. The
alternative was choosing worktree locations so that nothing sits above them, which is a rule someone
breaks later by putting a worktree somewhere reasonable.

The deciding argument was the record. Under the strict flag the run's MCP set is an argument Hydra
wrote, so the record can claim it and be right. Under a worktree `.mcp.json` the set is the state of
five directories at spawn time and the record can only report what Hydra intended.

Two costs, both deliberate. **Claude's MCP injection stops mirroring Claude's skill injection**, and
that asymmetry is written into the harness spec on purpose rather than found later by someone
wondering why the two do not match. And a recorded `claude` run cannot reach the human's own
user-scope servers at all.

One thing it fixes for free: ticket 27 recorded that a delegated child sees inherited servers whose
calls the gate then blocks. On `claude` that residue is gone, because under the strict flag the
inherited servers never start.

**Interactive runs get `--mcp-config` and not `--strict-mcp-config`.** The human's servers survive
and Hydra's are added. The case for using the strict flag in both modes is real and worth recording:
if interactive has more servers than recorded, you build against a set the recorded path will not
have and find out later. It loses to D3. Stripping the human's own MCP servers out of a session they
are sitting in front of is the harness overriding them in the one mode D3 hands to the vendor. The
interactive record already says its set is not exact, because a thin record cannot say otherwise.

Nothing is written into the worktree on either mode, so ticket 23's git problem never arises.
`.mcp.json` does not appear at the worktree root, `git status` stays clean, and `git add -A` has
nothing to stage.

### Cursor ships, with a data directory Hydra owns

**Each Cursor run sets `CURSOR_DATA_DIR` to its spawn directory** and approves Hydra's servers
individually, with `cursor-agent mcp enable <name>` run with cwd inside the worktree, before the
spawn.

Per-server `mcp enable` rather than `--approve-mcps` at spawn, because `--approve-mcps` approves
everything the workspace can see, including whatever the user's own repo carries in its
`.cursor/mcp.json`. The cost of naming servers individually is one process per server and a `cd`,
which is nothing next to approving files Hydra did not write.

**The redirect follows the tree, not the launch mode.** A tree Hydra created gets it. A
`--shared-tree` run does not, because D9's rule is about paths Hydra created and the tree is the
thing Hydra did or did not create. Seeding a per-run data directory by copying the human's existing
approvals for that slug was considered and rejected: it makes Hydra decide that servers are approved
for a run the human never approved them for, and the approval is keyed to a hash of the server
definition, so a copied approval can approve a command that has since changed.

**A Cursor `--shared-tree` run gets no injected MCP config at all.** This follows ticket 22's rule
one level down. A provider Hydra cannot give a config root it owns is not given the config, rather
than launched with the gap recorded. The run still happens, with the human's servers and none of
Hydra's.

The consequence for the map: Hydra never writes an approval into `~/.cursor/` under any launch it
makes. Anything Cursor writes there of its own accord during a shared-tree run is the human's own
Cursor doing what it does.

The redirect buys a second thing that was not asked for. `mcp disable` redirects with it, and the
slug is per workspace, so **Hydra gets a per-run kill switch for inherited user-scope servers on
Cursor**. Ticket 23 concluded only `claude` could scope a server off per run. That is now two of
three, and ticket 27's exposure residue narrows to `agy` alone.

### `agy` is unchanged, and one thing it must never be asked to do

`agy` rides ticket 10's shape as ticket 23 found it: `<worktree>/.agents/mcp_config.json` plus
`--add-dir <worktree>`, inside a root that carries its own `.gitignore` of `*`, no approval step,
servers from a worktree config just start. No decision was needed and none was taken.

No config-root lever exists. The binary's strings carry `AGY_CLI_*` display flags, `AGY_ADC_AUTH`,
`ANTIGRAVITY_*` editor and sidecar variables, and `GEMINI_API_KEY`, with nothing resolving a config
or data directory. Every path is hardcoded under `~/.gemini/config/` and `~/.gemini/antigravity-cli/`,
so `HOME` is the only lever and ticket 21 already ruled it out, since a wrong `HOME` logs the
provider out.

**Hydra never calls `agy mcp disable`.** Ticket 23 read it as editing the user-level
`mcp_config.json` and declined to run it. That reading is the reason: it is global rather than per
run, so one delegated child could turn a server off for every future run and for the human's own
interactive use.

### The launch check that governs the degradation

A run that quietly comes up short is what ticket 22's rule forbids, so dropping Cursor's MCP config
on a shared tree needed governing rather than recording.

**At launch Hydra compares the servers the run's policy admits against the servers the launch can
deliver, and fails the launch if it cannot deliver one.** A run whose policy admits no servers
proceeds untouched. Cursor plus `--shared-tree` is then one instance of a general rule rather than a
special case with a provider's name in it, and the rule catches combinations nobody has met yet.

The check is possible because of the admission field below. Without it Hydra could not tell the
difference between a run that lost something and a run that wanted nothing.

No per-run human flag to proceed degraded, for now. Three outs already exist: drop `--shared-tree`
and take a worktree with the redirect, switch to a policy that admits no servers, or route to another
provider. The map names the per-run flag shape once already, in D16's `--allow-billed` and ticket
31's floor escape, so adding a third instance stays cheap the day something needs it. Ticket 17's
discipline applies, which is that the lever gets built when the records show it is wanted.

### The pre-flight is fatal, on Cursor, in both launch modes

After the enables and before the spawn, Hydra runs a free `cursor-agent mcp list`. **Any injected
server the listing does not report as ready fails the run.**

Cursor earns this and the other two do not. It is the only provider whose failure mode produces a
wrong answer rather than no answer: an unapproved project entry does not shadow the human's
same-named server, it fails to replace it, so the listing says the name is not loaded while the
human's server process starts with the human's token. On `claude` the strict flag already guarantees
the set, so a pre-flight would verify nothing. On `agy` there is no free route to a resolved MCP set,
and paying a turn per run to check config is not worth it.

Fatal interactively too, and the reason is worth keeping. The case for warning instead is that a
human is present and can read the listing. The listing is the part that lies. It fails before the TUI
opens, so the cost is an error message rather than a lost session.

### Where the artifacts live, and the test that reaping puts on the record

**Per-run artifacts live in `~/hydra/spawn/<run-id>/`, a sibling of `runs/`.** Three things go there:
the `--mcp-config` file for `claude`, the `CURSOR_DATA_DIR` root for Cursor, and ticket 27's compiled
per-run gate script, which needed a home outside the worktree and never got a path.

A directory rather than the run file's own directory, because ticket 13 derives liveness from the run
file's opening envelope and says nothing appends to a run file except the process that owns it. A run
that is a directory invites a second writer into it. This keeps D10's "one file per run, named by a
UUIDv7" exactly as ticket 09 wrote it.

Putting them inside the worktree under a self-ignoring `.hydra/` was disqualified by ticket 27 rather
than by preference: the gate script has to sit where the child cannot write it, and that induction is
what makes the gate hold from the first tool call.

**The spawn directory is reaped with the worktree by ticket 11's cleanup**, and that puts a test on
the record rather than being a cleanup preference. D10 says the record is the source of truth. If
reading a spawn artifact is needed to interpret a record, the record is not the source of truth and
is missing a field. So the record answers "which servers did this run get" on its own, carrying:

- the **injected** set, what Hydra wrote
- the **resolved** set, what the provider reported, in the shape ticket 09 uses for `skills.linked`
- the canonical document's **sha**

Both sets, because they can differ on interactive `claude` without the strict flag and on `agy`,
which has no per-run off switch. The sha because a server name without a version says what was asked
for and not what command it named, and an MCP server definition is a command Hydra spawns.

### One document, protected, with per-server admission

Ticket 23 found one canonical document can generate all three provider files and cannot be any of
them, since the file name, the directory and the remote key all differ. That document is now placed.

**It is Hydra's fifth protected config**, beside the delegation policy set, the environment
allowlist, the export allowlist and ticket 28's instruction preamble, and a ninth protected path on
ticket 15's list. A rung-two patch touching it is refused before review rather than blocked by it. An
MCP server definition is a command Hydra spawns, so a loop that can edit that document can run
arbitrary code inside every future run, which is the argument D11 used to strike config from rung
one's write surface.

**Each server carries an admission field keyed on the policy name**, the way ticket 10's store
entries carry a `providers` field that admits a run. A `judge` run receiving a filesystem server
would be the policy set saying one thing and the server set saying another, since ticket 27 made
`judge` allow nothing at all. This does not reopen ticket 27: the policy stays a capability
allowlist, and the admission sits in this document pointing at a policy by name.

The two refusals, the launch check and the pre-flight, **share one outcome value**, with a field
naming which check fired. They differ in when Hydra noticed rather than in what went wrong, and the
signal rung one needs is that the run never got its servers. Ticket 30 set the precedent, carrying a
Hydra-set field rather than splitting a value when a provider's own reason text could not identify
which gate fired. The value is distinct from the gated outcome, so rung one never reads an MCP
misconfiguration as a policy refusal, which is R2's reason and stands.

### An observation the harness spec inherits, with no ticket opened

While reading the `agy` binary for a config-root lever, its embedded changelog says
`~/.gemini/config/projects/` holds project-specific permission configs that take precedence over the
global settings in `~/.gemini/antigravity-cli/settings.json`. That is hook and policy territory,
tickets 26, 27, 30 and 31, all closed, and it is not MCP. It is recorded here rather than ticketed,
because nothing on the route to the destination waits on it.

### What this amends

- **Ticket 23** was wrong about D9, not merely incomplete. Its answer and findings both say approving
  a Cursor server writes outside any path Hydra created. `CURSOR_DATA_DIR` moves it.
- **Ticket 21** governs what a child *inherits*. `CURSOR_DATA_DIR` is Hydra *setting* a variable on a
  child, which that ticket's allowlist does not name as a category.
- **Ticket 27** gains a second provider that can scope a server off per run, and its gate script
  gains a path.
- **Ticket 10** keeps its shape for `agy` and Cursor and loses `claude` to the command line.
- **Tickets 15, 19 and 28** gain a protected config and a protected path, and **ticket 16**'s
  reconciliation moves from four and eight to five and nine.
- **Ticket 09** gains three record fields and one outcome value.
- **Tickets 11 and 13** gain the spawn directory, in cleanup and in launch respectively.
