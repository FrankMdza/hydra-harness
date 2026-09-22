# Hydra harness spec

How Hydra launches, feeds and records the three agent CLIs.

This is one of three documents. `docs/run-record-contract.md` fixes the format Hydra writes and
Rungs reads. `docs/rungs-spec.md` covers both rungs of self-improvement. The three are
meant to be read together and nothing else crosses between the harness and Rungs, per D5.

Provider versions this document was written against. Read 2026-09-04, re-read 2026-09-05 when
the first slice was built:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.261 | 2026-09-04 morning read 2.1.257 and it moved the same afternoon, so a `claude` claim below is on .257 unless it says .261. Section 6.7's two 2026-09-05 measurements are on .261 |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-05, unchanged since 2026-09-02 |
| `agy` | 1.1.26 | 2026-09-05, unchanged since 2026-09-04 |

**Two of the three self-update without asking, and it has invalidated a finding four times.** `agy`
alone moved 1.0.10 to 1.1.23 to 1.1.24 to 1.1.25 to 1.1.26 in four days. Every provider-specific
claim below carries the version it was measured on. Re-read all three versions before trusting any
of them, and re-measure rather than inherit when one has moved.

## 1. What Hydra is

A router. One command picks a provider, builds that provider's environment, creates a worktree,
injects the shared skills, compiles a permission gate, launches the vendor's own CLI as a child
process on a pty Hydra owns, and tees the structured output into a run record.

Hydra is not an agent. It runs no model of its own, holds no provider credential, and interprets no
shell command. It decides where a run may act and records what it did.

The core is Rust (D8). Hydra is the parent process rather than a client of a terminal, because it
has to be the parent to tee anyway and Ghostty exposes no pid and no exit status (ticket 02). One
dependency carries this: `portable-pty`.

There is no daemon. Run records on disk are the source of truth and nothing needs to be running for
the harness to work (D10).

## 2. The routable providers

Three: `claude`, `cursor-agent`, `agy`.

`agy` is Antigravity, and it is the Google provider (ticket 18). `gemini` 0.42.0 is specced as a
cold fallback on an API key and is never run without a human typing `--allow-billed`, because
Google retired the individual Code Assist tier for that client and its OAuth path returns
`IneligibleTierError` on this machine. `hermes` 0.14.0 is not routable (ticket 22): it holds no
credential that works, and it is the one provider Hydra cannot gate through the environment, since
`~/.hermes/.env` loads with `override=True` and beats anything Hydra passes.

That last case sets a rule with a longer life than the decision:

> **A provider Hydra cannot gate is either given a config root Hydra owns, or is not routed.** There
> is no third branch where Hydra launches it and records that the gate did not apply.

`HERMES_HOME` is what the first branch looks like in practice, and it is specced without being
built. `CURSOR_DATA_DIR` is what the first branch looks like when it is built (section 6.5).

Model selection is a flag inside a provider, never a provider of its own. `agy` serves
`claude-opus-4-6-thinking` and `claude-sonnet-4-6` on the Google subscription, reachable as
`hydra run --provider agy --model claude-opus-4-6-thinking`. Hydra never reaches for it on your
behalf (ticket 20). Every provider-specific fact in this document is a property of the binary and
not of the model: the hook format, the MCP path, `--add-dir`, the environment allowlist, the
permission mode. A distinct provider row would duplicate eleven settled facts to carry one field.

## 3. Two launch modes

A vendor CLI keeping its own TUI cannot also speak a structured protocol, so interactive and
recorded are two launch modes rather than one (ticket 13).

**Recorded.** Hydra drives the provider through a structured transport, owns the pty, and writes
every frame into the run record as it arrives. Skills are injected, MCP is injected, the gate is
compiled and passed, and the instruction preamble rides the task string. This is the mode
delegation uses and the mode Rungs learns from.

**Interactive.** The vendor owns the terminal and draws its own interface. Hydra launches it and
gets out of the way. Without `--policy`: no skill injection (ticket 10), no gate, no preamble. With
`--policy <name>`, on `claude` only, since build ticket 28: a worktree, the skill snapshot, and a
compiled gate passed as `--settings`, with denials harvested from the transcript (the ticket 50
paragraph in section 3). Either way the run records thin and is harvested from the vendor's own
transcript when the process exits (section 10).

The consequence is worth stating plainly rather than leaving in the mechanics: **Rungs learns
from run records, and a thin record teaches it little.** A TUI is what would let the structured
transport become the interactive path too, and that is deferred with the TUI (D3).

**An interactive `claude` run may carry a policy, since map ticket 50.** `hydra run --interactive
--policy <name>` compiles the gate, seeds a worktree, links the snapshot and hands `claude` the
same settings a recorded run gets; `claude` takes `--settings` interactively and the hooks
apply, measured on 2026-09-07 with two refusals in Hydra's own words on a session a human was
steering. What the transcript then holds is every denial, as an error tool result carrying the
reason, and nothing for an allow, so the record carries `permission_decision` for denials and
says out loud that allows were not recorded. No policy is the default, and the other two
providers cannot take one this way because they read their hooks out of a worktree.

## 4. Three populations, and only one is gated

Ticket 28 found three, not two. The gate exists for the first one only.

| Population | Preamble | Gate | Injection | Record |
|---|---|---|---|---|
| A recorded run Hydra launches | yes | yes | yes | full |
| An interactive run Hydra launches | no | no | no | thin, harvested at exit |
| An agent working on `~/hydra` itself | no | no | no | none |

The third population reads `~/hydra/CLAUDE.md`, which is what a `CLAUDE.md` always was: advice to
an agent editing this repo, with nothing enforcing it. It is not one of these three documents and
nothing else places it, so section 15.3 gives its wording.

**A reader must not assume the third population is gated on the strength of shared phrasing.** The
three deliveries say similar things and only the first is backed by a mechanism.

## 5. What Hydra writes under `~/hydra`, and where

Seven directories, four of them per run. The answer used to be one, and a reader who assumes it is
still one will look for the gate script in the wrong directory. This table is the whole list, so a
reader who cannot find a path here has found a bug rather than an omission.

| Path | Holds | Created | Reaped |
|---|---|---|---|
| `<worktree>/` | the child's checkout, the injected skill root, the hook config for `cursor-agent` and `agy`, the MCP config for `cursor-agent` and `agy` | per run, from the parent's repo | `hydra reap`, after committing uncommitted changes to the child's branch |
| `~/hydra/skills-snapshots/<sha>/` | an immutable git worktree of the skill store at the sha the run pinned | per sha, shared and refcounted | when no live run references it |
| `~/hydra/spawn/<run-id>/` | the compiled gate script, the seatbelt profile `sandbox.sb` on macOS, the hook file `hook.py` on `codex`, the `--mcp-config` file for `claude`, Cursor's data root at `cursor-data/` | per run | with the worktree |
| `~/hydra/runs/<run-id>.jsonl` | the run record | per run | `hydra prune`, which moves rather than deletes |
| `~/hydra/skills/` | the skill store, with its own git history | once, and it outlives every run | never |
| `~/hydra/skill-worktrees/<run-id>/` | a `curate` pass's own checkout of the store, where it writes its proposal | per curate pass | `hydra reap`, after committing to `rungs/curate/<run-id>` |
| `~/hydra/pruned/<run-id>.jsonl` | where `hydra prune` moves a record, because it moves rather than deletes | per pruned record | never |

**The root is a property of the machine, and a launch from a checkout that is not the root refuses
first.** Map ticket 61, resolved 2026-09-21 after a cold-clone review ran the quick start from
`/tmp` and was shown a corpus it had not made. `~/hydra` is one corpus per machine, which is what
rung one learns from and what the install makes coincide with the checkout. `hydra run` from a
checkout elsewhere, with `HYDRA_ROOT` unset, refuses before writing anything and names the root
it would have written into, the checkout it ran from, and the variable; set to that root or
another, the launch proceeds. `hydra doctor` says what a launch would find: the root and the
policy path, the four binaries and their versions against the ones the code was measured on,
the policy set sha, and each protected config as present or missing, exit 1 when one is missing.
Neither the versions nor the configs were checked at spawn before it; `doctor` is a new check.

The last three are not per run and are here anyway. A reader asking where a pruned record went, or
where a curator's proposal is before a human sees it, had nowhere in this spec to look. Ticket 40
settled the checkout's location: it is a sibling of `worktrees/` rather than a subdirectory of the
spawn directory, because `hydra reap` removes a spawn directory whole and an uncommitted proposal is
work.

`~/hydra/spawn/` is a sibling of `runs/` and not a subdirectory of it. Ticket 13 derives liveness
from the run file's opening envelope and forbids anything but the owning process from appending to
it; a run that is a directory invites a second writer. This keeps D10's one file per run exactly as
ticket 09 wrote it.

**Hydra never writes outside a path it created** (D9). The spawn directory is what keeps that true
for the gate script, which ticket 27 required to sit outside the worktree it guards. It is also
what keeps it true on Cursor, where approving an injected MCP server would otherwise write into
`~/.cursor/projects/`.

**The spawn directory dies with the worktree, and that puts a test on the record.** If reading a
spawn artifact is needed to interpret a run record, the record is not the source of truth and is
missing a field. The record therefore answers "which servers did this run get" and "what did the
gate refuse" on its own. The compiled gate script is a spawn artifact and this test applies to it,
which ticket 42 settled on 2026-09-06 after it failed: the script held the only copy of the paths
the policy's scopes resolved to, and the contract's `run_start.scopes` now carries them.

**A `hydra evolve` pass writes only the last row.** The record contract's section 2 splits a run into
two kinds, and the second one is work Hydra did itself rather than a provider process it launched.
The pass spawns a proposer and four judges, each of which is an ordinary provider run with the three
other rows above; the pass itself launches nothing, so it gets no worktree, no skills snapshot and no
spawn directory. Its file is `run_start`, `run_end` and nothing between, and `provider: null` is what
a reader checks to tell the two kinds apart. The contract's section 5.5 owns the field-by-field
shape. `hydra curate` gets no pass record, because the curator is an ordinary recorded run and its
supervisor decides nothing after the run ends.

## 6. The launch sequence

Every step below runs before the child process starts. Several of them can refuse the launch, and
refusing before spawning is deliberate: a run that comes up short quietly is what section 2's rule
forbids.

### 6.1 Pre-flight

1. **Auth.** Refuse to start a provider that is not logged in. Auth never happens inside a run
   (section 11).
2. **Depth.** Computed from `parent_run_id`. Capped at 2, configurable.
3. **Policy containment.** A child's policy may never be wider than its parent's (section 7.3). A
   refusal here still writes a run record.
4. **Concurrency.** The cap refuses rather than queueing or blocking, returning a typed error naming
   what is in flight and its run ids. Queueing needs a live scheduler, which D10 rules out, and
   blocking parks the orchestrator that delegation exists to protect.
5. **Skill collision.** Compare the store against the user-scope index and against the repo's own
   project-scope skills. A hit fails the run (section 6.4).
6. **MCP launch check.** Compare the servers the run's policy admits against the servers this
   launch can deliver. A shortfall fails the run (section 6.5).
7. **Cursor MCP pre-flight.** After the approvals and before the spawn, a free `cursor-agent mcp
   list`. Any injected server the listing does not report as ready fails the run, in both launch
   modes (section 6.5).

### 6.2 Environment

Two categories, and the allowlist is only the first. A reader who takes the allowlist for the whole
environment story will miss the second.

**Inherited, from an allowlist.** Hydra builds each child's environment from a list rather than
passing its own through. `GEMINI_API_KEY` is exported on line 1 of `~/.zshrc`, so without this every
child inherits it and D6 is false while nobody notices.

```
HOME    the real home, or omit entirely. Never a different path. agy hard-fails without it
USER    claude reads its keychain account name from process.env.USER. Without it, claude is logged out
PATH    must contain /usr/bin for the keychain read, and every shell tool the providers call
TERM    rendering. Read by claude and cursor-agent
SHELL   claude only, for shell classification
TMPDIR  claude only, and it falls back on its own if absent
```

Passed through when the network needs it: `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` and their
lowercase twins, and `NODE_EXTRA_CA_CERTS` for the two Node providers.

`LANG` is not on the list. None of the providers reads it. Keep it for libc and Python encoding if
you like, but it is not a provider dependency.

**`HOME` is the rule to get right.** `security list-keychains` returns only the system keychain when
`HOME` points anywhere but the real home, so the login keychain drops out of the search list and
every keychain-backed provider comes up unauthenticated with exit code 0. Three of four fail this
way, silently. An absent `HOME` is safer than a wrong one, because the search list falls back to
`getpwuid`. Any future sandbox that redirects `HOME` logs the user out of three providers and
reports success.

**Three names Hydra must never set.** `CLAUDE_CONFIG_DIR` loses the Anthropic login even when set to
its own default value, because the keychain service name gains a hash of the config dir the moment
the variable is defined. `AGY_ADC_AUTH` is the real auth switch for Google rather than
`GEMINI_API_KEY`. `EDITOR` stays off for `agy`, which spawns it from the artifact view, so a
detached child would hang on an editor nobody is watching.

**The scrub is an injection boundary as much as a secret one.** `NODE_OPTIONS=--require <file>` runs
attacker-named JavaScript inside `cursor-agent`. `CURSOR_AUTH_TOKEN` and `CURSOR_API_KEY` are
write-through: `cursor-agent` passes the environment value to `setAuthentication()`, so an inherited
value overwrites the stored keychain login rather than overriding it for one run. This shell also
exports `CLAUDE_CODE_MESSAGING_SOCKET`, `CLAUDE_CODE_MESSAGING_TOKEN` and `CLAUDE_CODE_SESSION_ID`,
and an unscrubbed child `claude` writes `messagingSocketPath` from that variable into its own pid
record and joins the parent's messaging plane.

**Set by Hydra, for Hydra's own purposes.** A different category with a different rule: an inherited
variable is a leak to stop, a Hydra-set one is a lever to use.

| Variable | Value | Why |
|---|---|---|
| `CURSOR_DATA_DIR` | `~/hydra/spawn/<run-id>/cursor-data/` | moves Cursor's MCP approval state into a path Hydra owns |

`CURSOR_CONFIG_DIR` is a second lever, deliberately not pulled. It moves `cli-config.json`, which
holds `authInfo`, and `chats/`, which section 10 harvests. Setting it would log the provider out and
move the transcript, which is the `HOME` finding in a narrower form. `agy` has no such lever at all:
its paths are hardcoded under `~/.gemini/`, so `HOME` is the only one and it is ruled out.

### 6.3 The worktree

One git worktree per recorded run, branch named from the run id, when the cwd is a repo. The spawn
seeds it with the parent's uncommitted changes applied as a patch, because a clean checkout would
hide exactly the work in progress most delegations are about.

Two children editing one file with neither aware of the other is the most likely way delegation
produces garbage, and the worktree is what prevents it.

`--shared-tree` opts out, for the case where you deliberately want two agents in the same files. It
costs MCP on Cursor (section 6.5) and it is not on the MCP delegate face (section 9).

### 6.4 Skills

One canonical store at `~/hydra/skills/<name>/SKILL.md`, flat, its own git repo living inside the
`~/hydra` working tree and ignored by it. Beside each `SKILL.md` sits an optional `hydra.toml`
carrying `origin`, `pinned` and `providers`. `SKILL.md` keeps only `name` and `description`, the
two fields ticket 03 proved portable; Hydra's own metadata stays in the sidecar, which no provider
opens.

**Injection is per run into the worktree, never into a location in the user's home.** D9's symlink
farm survives as a mechanism and dies as a location. Recorded runs only.

| Provider | Root | Also needs |
|---|---|---|
| `claude` | `<worktree>/.claude/skills/` | cwd is the worktree |
| `cursor-agent` | `<worktree>/.cursor/skills/` | cwd or `--workspace` |
| `agy` | `<worktree>/.agents/skills/` | `--add-dir <worktree>` |

Into that directory Hydra drops a `.gitignore` holding `*`, then links in the skills whose
`providers` field admits this run. `git status` reports nothing and `git add -A` stages nothing,
verified on all three current builds (ticket 24). A linked worktree has no private `info/exclude`,
so hiding the links through git's exclude machinery would also hide them in the user's main working
tree; the self-ignoring directory needs no write outside a directory Hydra created.

**`agy`'s row carries a launch flag because a directory it is not told about is not its workspace.**
A probe placed in the launch directory was invisible twice, as a real directory and as a symlink,
and appeared as soon as the directory arrived as `--add-dir`, from an unrelated cwd too. This is not
an optimisation. It is the difference between the skills loading and not, and it recurs for hooks
and for MCP.

**The links point into a snapshot, not into the store.** `~/hydra/skills-snapshots/<sha>/` is a git
worktree of the store checked out at the sha the run pinned. Without it, versioning the store is
decorative: a symlink resolves to the live working tree, so an edit mid-session would reach a
session already running. Snapshots are shared per sha and refcounted, because delegation caps depth
and not fan-out. Two runs on the same sha see identical bytes because they are looking at the same
bytes.

**A run may invoke what it was linked and may read it.** `Skill` answers to `skill_invoke`, granted
by every policy that reads at all, and the run's own snapshot is a read scope of its own,
`skill_snapshot`, resolved from the sha on the run's record, which is what lets contract 7.7
re-derive the same answer at read time. A run may write nowhere near it. The provider names the link
path as the skill's base directory and the gate resolves that link into the snapshot before
comparing, so a sibling file opened through the link is inside the scope and the same path opened
for writing is not. Measured on 2026-09-06 under ticket 39: the body arrives on the `Skill` call
with no read the gate sees, and a sibling arrives through `Read` on the link path.

**If the store is dirty at launch, Hydra commits it before pinning**, with authorship in the commit
trailer. Launching from HEAD and ignoring the working tree would mean editing a skill, running, and
watching nothing happen, which would happen on day one. Committing is the only option where the sha
in the run record is always true. `hydra curate` pins before it checks the store out, so its launch
finds the store clean and pins the sha the checkout already sits on (ticket 41).

**The skills already on disk do not move.** The 11 skills in `~/.claude/skills` stay where they are.
This is the only honest option rather than generosity: every provider reads a user-scope root as
well as a project one, and `HOME` cannot be moved, so those skills are in every run whatever this
spec decides. Hydra scans the user-scope roots into a read-only index, rebuilt at every launch. A
skill with no sidecar is pinned and unproposable by definition, because Hydra does not own the file.

The index has to scan `~/.codex/skills` and `~/.grok/skills` as well, which `cursor-agent` reads at
both scopes. Neither exists on this machine and either appears the day the matching CLI is
installed. A Cursor run also inherits any `.codex/skills` or `.grok/skills` the user's own repo
carries, which Hydra never wrote and does not check.

**Collisions are checked twice.** At write, the curator refuses to create or rename a store skill to
a name the index already holds, so no collision Hydra causes reaches a run. At launch, a hit fails
the run as its own outcome. Failing looks harsh until you notice what the write check did to the
odds: a launch-time collision means the world moved under the store, it should be loud, and the fix
is one rename.

**The launch check asks the provider rather than comparing strings.** There is no shared notion of a
skill's identity: `claude` keys on the directory name, `agy` keys on the frontmatter `name`, and
Cursor's listing has no name field at all, only a full path. One probe directory named
`t24b-dirname-only` declaring `name: t24b-frontmatter-name` is a different skill to each of them. So
the check compares both the directory name and the frontmatter name against every scope and root
that provider reads, then diffs the provider's own resolved listing against what Hydra linked. A
name Hydra linked that the provider does not list is a collision, whichever side won. Two of the
three listings are free.

**The store needs a write-time YAML validator, not a regex.** A description containing a colon
followed by a space breaks the skill on two of three providers, silently: an unquoted YAML scalar
may not contain `: `, so the frontmatter fails to parse, `agy` drops the skill from its listing and
Cursor keeps it with an empty description. `~/.claude/skills/handoff-frank` fails this today and has
been reaching Cursor with no description since 31 August.

**Injection has a measured side effect on `claude`, and the spec states it rather than hiding it.**
Claude Code fits its whole skill listing into a character budget that scales with the model and
renders the overflow as bare names. Same worktree and model, one run with nine injected links and
one without: 7,987 characters against 7,991, bare entries 14 against 8. Three of the six newly
stripped were the user's own plugin skills, and nothing warned. So the run record's skill sha plus
its linked-name list does not reproduce what a run's model saw, which is why the record also carries
the resolved listing the provider reported.

**`judge` is the exception, since map ticket 47.** A `judge` run is linked nothing: the policy
allows no read and no invoke, the diff it judges rides its prompt, and `--restricted` hides every
skill on the filesystem regardless, so the links were never reachable. The `skills` group is
absent on its record the way it is on a run with no checkout, and the probe under `judge` is not
asked about skills. Build ticket 25.

### 6.5 MCP

One canonical document generates the per-provider files and is none of them. The stdio server body
is byte-identical across the three, but the file name, the directory and the remote key all differ
(`type`+`url`, `url`, `serverUrl`), and `agy` refuses `command` and `serverUrl` together.

**Each server carries an admission field keyed on a policy name**, the way a store skill carries
`providers`. A `judge` run receiving a filesystem server would be the policy set saying one thing
and the server set saying another. This does not reopen the policy shape: the policy stays a
capability allowlist and the admission sits in the document pointing at a policy by name.

| Provider | Where the config goes | Launch flags |
|---|---|---|
| `claude` | `~/hydra/spawn/<run-id>/mcp.json` | `--mcp-config <file>`, plus `--strict-mcp-config` on recorded runs only; `judge` carries the second flag and no file |
| `cursor-agent` | `<worktree>/.cursor/mcp.json` | cwd or `--workspace`, plus `CURSOR_DATA_DIR` and a per-server `mcp enable` |
| `agy` | `<worktree>/.agents/mcp_config.json` | `--add-dir <worktree>` |

**Claude's MCP config goes on the command line, and its skills go in the worktree. The asymmetry is
deliberate.** A reader who expects the two to match will go looking for a bug. The reason is that
`--strict-mcp-config` makes a run's server set an argument Hydra wrote rather than the state of five
directories at spawn time, so the record can claim the set and be right. Under a worktree
`.mcp.json` the record could only report what Hydra intended. It also deletes the ancestor merge
rather than defending against it: the walk up from cwd never happens, so it stops mattering that the
walk crosses git boundaries or where Hydra puts its worktrees. And `.mcp.json` sits at the worktree
root where the self-ignoring `.gitignore` trick cannot reach it, so with the config on the command
line `git status` stays clean.

Two costs, both accepted. A recorded `claude` run cannot reach the human's own user-scope servers at
all. And the two injection shapes no longer mirror each other.

**Interactive `claude` runs get `--mcp-config` without `--strict-mcp-config`.** The human's servers
survive and Hydra's are added. Using the strict flag in both modes has a real case, that a set which
differs between modes means you build against servers the recorded path will not have, and it loses
to D3. Stripping a human's own MCP servers out of a session they are sitting in front of is the
harness overriding them in the one mode D3 hands to the vendor.

**Cursor gets a data directory Hydra owns.** `CURSOR_DATA_DIR` moves `projects/<slug>/`, which holds
`mcp-approvals.json` and `mcp-disabled.json`, into the run's spawn directory, while `cli-config.json`
with its `authInfo` and `chats/` stay under `CURSOR_CONFIG_DIR`, which Hydra does not set. So the
approval state moves and the login and the harvest source do not.

Hydra approves its own servers individually with `cursor-agent mcp enable <name>`, run with cwd
inside the worktree, before the spawn. Not `--approve-mcps`, which approves everything the workspace
can see including whatever the user's own repo carries in its `.cursor/mcp.json`. Two mechanics
decide how the launch is written: `mcp enable` resolves the project from cwd and ignores
`--workspace`, while `mcp list` honours it.

Copying the human's existing approvals for that slug into the per-run directory was considered and
rejected. It makes Hydra decide that servers are approved for a run the human never approved them
for, and the approval is keyed to a hash of the server definition, so a copied approval can approve
a command that has since changed.

**The redirect follows the tree, not the launch mode.** A tree Hydra created gets it. A
`--shared-tree` run does not, and therefore gets no injected MCP config at all rather than an
approval written into `~/.cursor/`. That is section 2's rule one level down. The run still happens,
with the human's servers and none of Hydra's, and if the run's policy admits a server the launch
fails instead.

**Cursor gets a fatal pre-flight and the other two do not.** After the enables and before the spawn,
a free `cursor-agent mcp list`; any injected server the listing does not report as ready fails the
run. Cursor earns it because it is the only provider whose failure mode produces a wrong answer
rather than no answer: an unapproved project entry does not shadow the human's same-named server, it
fails to replace it, so the listing says the name is not loaded while the human's server process
starts with the human's token. On `claude` the strict flag already guarantees the set. On `agy`
there is no free route to a resolved MCP set, and paying a turn per run to check config is not worth
it.

Fatal interactively too. The case for warning instead is that a human is present and can read the
listing, and the listing is the part that lies. It fails before the TUI opens, so the cost is an
error message rather than a lost session.

**`agy` keeps the inherited-server residue.** A delegated child inherits the human's user-scope
servers on all three providers. The gate blocks every call, so nothing executes, but on `agy` the
server process still starts and its tool descriptions still reach the model's context. `claude`
closes this with `--strict-mcp-config`, which stops the inherited servers starting at all, and
Cursor closes it with a per-run `mcp disable` under the redirect. On `claude` the claim was
re-measured under ticket 43 on 2.1.263, 2026-09-06, with the free probe and a stub server in each
scope: a user-scope server in `~/.claude.json` and a project-scope one in `.mcp.json` both load with
no flags, and neither loads under `--restricted`, with or without `--strict-mcp-config`. A server
passed on `--mcp-config` loads under `--restricted`. So `--restricted` drops what settings define and
keeps what the argv passes, and a `judge` run, which passes nothing, gets an empty set on this
version. The flag is on the `judge` argv anyway, because the vendor's help text ties skipping MCP to
`--strict-mcp-config` rather than to `--restricted`, and the argv should say what it means. `agy` has neither, and **Hydra
never calls `agy mcp disable`**, because it is global rather than per run: one delegated child would
turn a server off for every future run and for the human's own interactive use.

**Symlinks are followed for MCP on all three providers**, absolute and relative, confirmed in a paid
Cursor run. That is a loader property and not a provider property: the same Cursor binary follows a
symlinked MCP config and silently refuses a symlinked hook config. Section 6.6's "real files, never
links" is a hook rule and does not generalise.

**A collision between an injected server and the human's own splits three ways.** A project server
wins on `claude` and `claude mcp list` prints a diagnostic naming both endpoints, free. `agy`
collapses duplicates silently. Cursor is the hazard described above. Claude's own approval gate does
not apply in print mode: `claude mcp list` reports pending approval and `claude -p` connects anyway.

### 6.6 The gate

The permission gate is a `PreToolUse` hook, written per spawn, from a config Hydra compiles out of
the run's policy. It is not the permission layer under another name: a denial holds on `claude`
under `--permission-mode bypassPermissions`, on `agy` with the model otherwise unconstrained, and on
`cursor-agent` in print mode with `--force` passed.

**On Cursor the gate registers `beforeReadFile` as well, and the reason is that reads are a
capability.** `read-only` and `standard` both scope reads to the worktree, and rung two's proposer is
kept out of `~/.claude`, `~/.cursor`, `~/.agents` and `~/hydra/runs/` by the same mechanism. Cursor
defines twenty-one hook events and six of them can return a permission decision, so `preToolUse`
alone leaves the file-read path ungated on that provider (ticket 26). `claude` and `agy` route reads
through `PreToolUse` like every other tool, so one event covers them. A hook blocks a file write
before it reaches disk on all three, probed on Cursor with the path and the content in hand and no
file afterwards, which is what makes section 7.5's path denials deliverable rather than aspirational.

| Provider | Config path | Delivered by | On a broken hook |
|---|---|---|---|
| `claude` | `~/hydra/spawn/<run-id>/settings.json` | `--settings <path>` | fails **open** on a response in a dialect it does not recognise |
| `cursor-agent` | `<worktree>/.cursor/hooks.json`, `failClosed: true` | cwd or `--workspace` | honours the flag |
| `agy` | `<worktree>/.agents/hooks.json` | `--add-dir <worktree>`, without which it reports `hooks: []` rather than an error | fails **closed**; a non-zero exit denies every tool |

**Real files, never links, rewritten at every spawn.** `cursor-agent` refuses a symlinked project
config silently and runs the command anyway. All three snapshot their hook configuration at process
start, tested with a hook that overwrote its own config file every time it fired, so mid-run tamper
fails on all three. The residual hole is the next run in the same directory, which the rewrite
closes and the worktree keeps small.

**Hydra authors two config shapes and one script.** Cursor reads Claude Code's hook format and
translates it, so one Claude-shaped config could gate two providers. That convenience is rejected
for the gate: Cursor's conversion path writes `failClosed: false` into every converted hook, and
only Cursor's own format can say otherwise. Depending on a translation layer for a security gate, on
the provider that self-updates most, trades away the wrong thing.

One script, because policy evaluation is the part that has to be correct and the response dialect is
a thin layer at the end. Three scripts would triple what must stay right in order to vary the one
part that differs. Cursor accepts Claude's response dialect on `preToolUse`, so the script answers
in two shapes rather than three.

**The dialect is passed as an argument, never inferred.** `claude` ignores a response in a shape it
does not recognise and allows the call, so a script that guesses wrong is an open door on exactly
one provider. Hydra writes the config, so Hydra writes the provider onto the command line, and the
script denies when the argument is absent.

**A hook is a veto and never a key.** Measured on all three: a deny beats an allow, and a hook
cannot grant at all. On `claude` a hook deny returns immediately while a hook allow is still checked
against the permission rules, with the shipped comment reading `"but deny rule overrides"`. On
`cursor-agent` no code path reads `"allow"`; the merge computes it and nothing consults the result.
On `agy` a hook allow did not lift the headless auto-deny on a file write. Declaration order does
not change it.

That result is what makes composition safe, and composition is what Hydra does.

**Hydra's hooks and the user's both run, and the record says so.** The user's
`~/.claude/settings.json` carries three `permissions.deny` rules and three `PreToolUse` guards
totalling 418 lines, and `cursor-agent` reads that same file. Hydra does not isolate, for a reason
sharper than convenience: **Hydra's policies bound where a run acts, and the user's guards bound
what a shell command means.** Only the second stops a `git push --force`, which `standard` otherwise
permits from inside a worktree by its own definition. Dropping the user's file would remove
protection Hydra has never specced and would then have to build to stand still.

`agy` shows the asymmetry. The guards reach `claude` and `cursor-agent` because both read
`~/.claude/settings.json`, and nothing reaches `agy`, because no user-scope hook config has been
written for it and `~/.gemini/config/hooks.json` does not exist. **That is a fact about this machine
rather than a property of Hydra**, and it changes the day someone writes one. The spec states it and
builds nothing on it, which is section 2's rule applied to protection the user owns.

**The price of loading the user's file is that the same file can add context.** The user's
`UserPromptSubmit` hook fires inside every recorded `claude` run and its `additionalContext` reaches
the model before the first turn, measured under ticket 46. Hydra keeps loading the file anyway.
`--setting-sources project,local` beside `--settings` was measured to drop the user's hooks while
Hydra's own gate hook still fired, so the isolation is one flag away, and it is not taken because
the four `PreToolUse` guards live in the same file and go with it. Section 8 says what the added
text is. `judge` never sees it, because `--restricted` ignores user settings. On `cursor-agent` the
same thing can happen through `beforeSubmitPrompt`, and `hookAdditionalContexts` rides every tool
call in its stream, but no `~/.cursor/hooks.json` exists on this machine and a Cursor run is paid,
so it is unmeasured. Build ticket 09's drift note carries it to the next Cursor run.

**`--restricted` is used for `judge` and nowhere else.** It ignores user, project and local settings
while honouring `--settings`, refuses `bypassPermissions`, gates writes to settings and
tool-configuration files, and drops every MCP server the settings define while keeping one passed on
`--mcp-config`. Ticket 23 measured "drops MCP entirely" on 2.1.257 and ticket 43 measured the
narrower reading on 2.1.263, which is why `judge` carries `--strict-mcp-config` as well (6.5).
Stripping Bash and dropping MCP is the point for a policy that allows nothing and a cost everywhere
else.

**`--setting-sources project` is measured and deliberately unused.** It drops the user's hooks while
keeping all 25 tools, `bypassPermissions` and Hydra's own `--settings` hooks, so isolation on
`claude` is cheap. It is recorded here so a later reader who finds it does not assume it was an
oversight. Composition wins anyway, for the reason above.

**Two findings the recorder inherits from the gate.** On `agy` the first deny short-circuits, so
Hydra's hook may never fire and the reason text belongs to the user's. On Cursor every hook's
message is concatenated, and one probe reached the model as `Rejected: t30 CUR-PROJ says allow`,
quoting the hook that allowed. So the decider on a permission decision is a field Hydra sets from
the config it wrote, never one parsed out of a reason string.

**`claude` gives a second source, and it is corroboration rather than the answer.**
`--include-hook-events` puts `hook_started` and `hook_response` frames on the stream, naming the
hook and carrying its `exit_code`, `outcome` and stdout. Recorded runs pass it. The Hydra-set
decider field stays the thing the record keys on, because two of three providers cannot produce
these frames at all and a record whose shape depends on the provider is what that field exists to
prevent.

**It pulls the user's hook stdout into every recorded `claude` record.** Those guards are 418 lines
of Python and can print whatever they like, so a recorded run now carries content authored by a
script the user edits. Section 14 says what that does to export.

**`agy`'s `overwrite` is forbidden.** Its `PreToolUse` can shallow-merge into a tool call's arguments
so a rewritten call is what runs. Hydra never uses it. The deciding reason is the record: a
rewritten call means the record no longer shows what the model actually asked for, and rung one
learns from what agents tried, so a silently corrected trajectory would teach the curator that the
agent got it right.

### 6.7 The command lines

Every recorded run is print mode with `--output-format stream-json`. That is forced rather than
chosen: no routable provider speaks ACP, so there is no second transport to weigh (ticket 32, and
section 18 for what that struck).

Recorded run, `claude`, any policy but `judge`:

```
cwd = <worktree>
claude --print \
  --output-format stream-json \
  --verbose \
  --include-hook-events \
  --forward-subagent-text \
  --settings ~/hydra/spawn/<run-id>/settings.json \
  --mcp-config ~/hydra/spawn/<run-id>/mcp.json \
  --strict-mcp-config \
  --permission-mode bypassPermissions \
  --model <model> \
  <task string with preamble>
```

`bypassPermissions` is safe here because a hook deny beats it, measured, and because a detached
child has nobody to answer a prompt. The gate does the refusing.

Recorded run, `claude`, policy `judge`:

```
cwd = <worktree>
claude --print \
  --output-format stream-json \
  --verbose \
  --include-hook-events \
  --restricted \
  --strict-mcp-config \
  --settings ~/hydra/spawn/<run-id>/settings.json \
  --model <model> \
  <task string with preamble>
```

No `--permission-mode`, which `--restricted` refuses. No `--mcp-config`, and `--strict-mcp-config`
beside its absence, so the server set is empty by argument rather than by what `--restricted`
happens to drop this version. No `--forward-subagent-text`, because `judge` allows nothing and
spawns nothing. The pre-flight probe carries both flags on a `judge` run, so `mcp.resolved` is a
measured `[]` rather than null (ticket 43).

**`--verbose` is not optional.** `--print --output-format=stream-json` refuses to start without it:
`When using --print, --output-format=stream-json requires --verbose`. Measured on 2.1.257 and again
on 2.1.263, 2026-09-06.

**`< /dev/null` came off, because it was a finding about a pipe.** Through a pipe on 2.1.257,
`claude` waits three seconds for stdin and writes `no stdin data received in 3s, proceeding without
it` before starting, and the redirect is what stopped a detached child paying for it. Re-measured on
2.1.261 on 2026-09-05 over a pty Hydra owned: no wait, no warning, 0.83 seconds from spawn to the
first frame, off the run record's own timestamps. **Hydra always launches through a pty**, because
ticket 02 made it the parent and it has to be the parent to tee anyway, so the redirect is not
reachable from where Hydra stands and buys nothing there. The original claim is true of the shape it
was measured on and does not carry to the shape Hydra launches, which is the difference worth
keeping rather than the flag.

**The provider writes one thing after its last frame, and it is not a frame.** `claude` 2.1.261
ends a print-mode run on a pty with a bare `ESC[?25h`, show-cursor, with no trailing newline,
because stdout is a tty. A tee that assumes every line is a frame either drops it or fails to parse
it. The record contract's section 3 says what happens to it: it is kept, as a JSON string rather
than an object. Measured 2026-09-05; expect the same class of thing from the other two and do not
assume this is the only one.

**`--include-partial-messages` stays off**, and so does `cursor-agent`'s twin
`--stream-partial-output`. Recorded runs are detached and unwatched, `hydra logs --follow` reads
whole lines, and no export format wants intra-message granularity. They are off with a reason rather
than by neglect: the TUI, a live `--follow`, and a stall detector all want them, and all three are
deferred. `agy` has no such flag and emits `step_update` with `state: ACTIVE` regardless, so it alone
carries a heartbeat in v1 and nothing else does.

Recorded run, `cursor-agent`:

```
cwd = <worktree>
CURSOR_DATA_DIR=~/hydra/spawn/<run-id>/cursor-data/ cursor-agent mcp enable <name>   # once per admitted server
CURSOR_DATA_DIR=~/hydra/spawn/<run-id>/cursor-data/ cursor-agent mcp list            # fatal pre-flight
CURSOR_DATA_DIR=~/hydra/spawn/<run-id>/cursor-data/ cursor-agent \
  --output-format stream-json \
  --force \
  --model <model> \
  -p <task string with preamble>
```

`--force` is safe here for the same reason `bypassPermissions` is: Cursor rejects a gated call in
print mode with `--force` passed. Cursor has no permission channel in print mode and no ACP, so it
cannot be asked and can only be gated.

Recorded run, `agy`:

```
cwd = <worktree>
agy -p=<task string with preamble> \
  --output-format stream-json \
  --add-dir <worktree> \
  --dangerously-skip-permissions \
  --model <model>
```

**`-p=<prompt>` with the equals sign, not `-p <prompt>`.** A bare `-p` swallows the next argument as
the prompt and silently drops the real one.

`--add-dir` carries three separate things on this provider: the skills, the hook config and the MCP
config. Without it, none of them load and `agy` reports `hooks: []` rather than an error.

`--dangerously-skip-permissions` is what makes the gate the only thing refusing. Without it `agy`
reports `permission_mode: request-review` and auto-denies acting tools mid-run, surfacing the
refusal as a tool `ERROR` rather than an answerable request. It cannot be asked anything once the
process is running: its stream-json input accepts exactly one message shape,
`{"event":"user","message":{"content":"..."}}`, and a `permission_response` comes back as
`warning: ignoring unsupported stream input message event`. So the permission mode is a launch
decision.

**Do not assume `agy` writes where the parent stands.** With `--dangerously-skip-permissions` a probe
that wrote `probe.txt` put it in `~/.gemini/antigravity-cli/scratch/`, not the launch directory.

Interactive run, `claude` only today (`src/interactive.rs` refuses the other two): the vendor's own
invocation, plus `--mcp-config` without the strict flag. Without `--policy`: no `--settings`, no
injection, no preamble. With it: `--settings` carrying the compiled gate, a worktree and the
snapshot. No Cursor pre-flight runs here, because no interactive Cursor launch exists.

## 7. The delegation policy

### 7.1 Capabilities, not tool names

A policy is a named allowlist, deny by default, expressed in capabilities that Hydra maps to
provider tool names through a table Hydra owns.

There is no `denied` list. With deny by default it could only subtract from the allow list, and a
policy where two rules can disagree is a bug with a schema. Argument conditions live inside an allow
entry, in the `write_file(<target>)` shape `agy` already prints in its own rule strings.

A deny list was rejected for a second reason worth keeping: the providers self-update, so a deny
list authored today knows nothing about a tool shipped next week and its failure mode is silent
permission. An allowlist fails toward a run that cannot do its job, which someone notices.

Capabilities rather than provider tool names, because the three providers share no vocabulary. They
share no notion of a skill's identity and none for an MCP server either, so a policy written in one
provider's words does not survive being handed to another.

**Check the table against the provider at run time rather than trusting it.** `agy` returns its tool
names in the `init` frame of every run at no cost, so Hydra compares what the provider says it has
against what the table knows. **A tool name the table does not recognise is denied and recorded.** A
self-update that renames a tool then breaks the next run loudly instead of quietly widening what a
child may do. The cost is accepted in that direction on purpose.

Two known table entries a reader would otherwise get wrong: `agy` names one action `write_to_file`
as a step and `write_file` as a permission rule, so the table was expected to need both. Measured:
one row, `write_to_file`, is what `src/policy/table.rs` carries, and no `write_file` row has been
needed.

**`Skill` is its own capability, `skill_invoke`, rather than one of the nineteen tools that answer to
`provider_native`.** A run that is linked a skill and cannot invoke it was given a listing entry
rather than a skill. Ticket 39 measured the delivery on `claude`: an allowed `Skill` call brings the
body into context with no file read the gate sees, so the capability carries that tier on its own.
`ToolSearch` answers to `skill_invoke` too, for the same reason. Ticket 43 measured on 2.1.263 that
an MCP tool passed on `--mcp-config` lands in print mode's `deferred_tools_delta` listing, so a
policy that admits a server and denies `ToolSearch` has admitted a listing entry the model cannot
call. `ToolSearch` fetches schemas and executes nothing; the gate judges the call that follows.

### 7.2 The five policies

| Name | What it allows |
|---|---|
| `judge` | nothing |
| `curate` | reads the corpus, the skill store, its skill snapshot and its own checkout of the store, invokes linked skills, writes only inside its own checkout of the store |
| `read-only` | reads inside the worktree and its skill snapshot, invokes linked skills, no shell, no writes |
| `standard` | reads, writes, shell and network reads inside the worktree, reads its skill snapshot, invokes linked skills |
| `permissive` | everything, and the only policy that may spawn in-process subagents |

Five because each is a use case this design already has. A sixth should arrive with a reason.

`skill_invoke` and the `skill_snapshot` scope arrived with ticket 39 on 2026-09-06. Every policy that
reads at all may invoke a linked skill and read its snapshot. `judge` reads nothing and gets neither,
and `permissive` has both through `anywhere` and its own grant.

`standard` allows network reads. Denying `WebFetch`, `search_web` and `read_url_content` would make
a child's inputs fully reconstructable from the record, and it would break ordinary coding work, at
which point people route around `standard` and the strictness buys nothing.

`curate` writes into its own git checkout of the skill store, at
`~/hydra/skill-worktrees/<run-id>/`, outside both the store's working tree and the run's own
worktree. It and `permissive` are the two policies that break the worktree-write assumption, which
section 7.5 handles.

**No policy writes into `~/hydra/skills/` itself.** The store's working tree is one of the paths
section 7.5 denies under every policy, and the only things that write it are `Store::pin`, which
commits a dirty store before a launch pins it, and a human. Ticket 40 settled this: a curator writing
into the live working tree would hand its half-finished proposal to whichever run launches next, and
`docs/rungs-spec.md` 3.6 is the paragraph that says so. The two scope names are separate,
`skill_store` for the store and `skill_checkout` for the pass's checkout, so neither name resolves
two ways.

**The caller passes a name, never a list.** A name is something a human wrote once. A list is
security composed at call time by the least accountable party in the chain, and on the MCP face that
party is a model.

### 7.3 Monotonic narrowing

**A child's policy may never be wider than its parent's.** Wider means capability set containment
over the five, which needs nothing new since they are already capability sets.

Without this a poisoned parent names `permissive`, and the depth cap does nothing about it because
the cap bounds recursion rather than escalation. With it, text a run read can only ever act inside
the envelope of the run that read it, and a human at depth 0 stays the only thing that can widen
anything.

**The five are a partial order rather than a chain.** `curate` writes to the store and cannot touch
a worktree; `standard` is the reverse; neither contains the other. **An incomparable pair is refused
rather than resolved.** The cost, accepted: a `standard` parent cannot delegate a `curate` child, so
a curator pass starts at depth 0 from a human. That reads like the right answer on its own, since
rung one's write surface is what a laundered instruction would most want.

**A spawn refused this way still writes a run record**, with outcome `gated`, the requested policy
and the parent's beside it, and no child body. No child process starts, so without the record the
attempt is invisible, and under D10 anything not in a run record did not happen. Attempted
escalation is exactly the signal the corpus should carry.

### 7.4 The pattern floor

A capability allowlist is not enough on its own, because `shell` swallows everything.

Hydra ships argument conditions on the `shell` capability covering the destructive git forms, `aws
configure` and `sso`, and Slack webhooks. It lives as one file in the delegation policy directory,
beside the five policies, rather than as a field repeated in each of them where a policy edited to
drop it would be a silent hole. It inherits the protection the policy set already has.

**It applies on all three providers and under every policy, `permissive` included.** The user's own
guards cover `claude` and `cursor-agent` whatever Hydra does, so exempting `permissive` would empty
the floor on `agy` alone, which is the one provider where it is the only thing present. `permissive`
bounds what a delegated agent may do, not what the human may do, and a human who wants a force push
runs it themselves.

The initial patterns, which are spec work rather than a decision inherited from a ticket:

| Target | Patterns |
|---|---|
| Destructive git | `git push` carrying `--force`, `-f`, `--force-with-lease`, `--force-if-includes`, `--all`, `--mirror`, or a refspec beginning `+`; `git push` to a protected branch; `git branch -D`; `git update-ref`; `git symbolic-ref` in write form; `git reflog expire`; `git reset --hard`; `git clean` carrying `-f` with `-d` or `-x`; `git gc`; `git prune`; `git worktree` in every form but `list` |
| AWS | `aws configure`, `aws sso`, and `aws s3` with `cp`, `mv`, `rm`, `sync` or `rb` |
| Slack | any `hooks.slack.com` URL, and the API methods `chat.*`, `files.upload`, `conversations.create` |

Each pattern consumes `-C` and `--git-dir` before reading the subcommand, and refuses a subcommand
that arrives from a shell substitution rather than trying to evaluate it.

The last three git forms arrived with ticket 40, which measured a `standard` run's shell allowed to
run all of them. `gc` and `prune` delete unreachable objects in the parent repo, and the tree a run's
`diff_stat` is measured against is exactly that. `worktree remove --force` discards a sibling run's
uncommitted work. A child has no business running any of the three, and none names a path the gate
could see, so the floor is the only place they can be refused.

**What the floor leaves alone on purpose: the run's own scratch.** `git checkout -- .`,
`git restore .`, `git stash drop` and `git clean -f` discard only the run's own uncommitted edits, and
a run reverting its own changes is ordinary work. The record carries every tool call whether or not
the working tree survives. Ticket 40 looked at drawing that line tighter and did not. That is the shape the
user's own `guard_git.py` uses across 231 lines, and this floor is a thinner version of it rather
than a replacement.

**It is a floor and never a gate, and the spec says so out loud.** It is a deny list in the one
place section 7.1 permits one, and it inherits that shape's failure mode: silent permission on the
spelling nobody wrote down. The evidence is in the user's own config, which carries two separate
rules for `git push --force` and `git push -f`, because one spelling does not catch the other, and
then 231 lines of parser beside them anyway. On `claude` and `cursor-agent` the user's guards catch
what the floor misses. **On `agy` nothing does.** Do not read this table as coverage.

A human lifts the floor per run with a flag, in the shape section 16 describes.

### 7.5 The gate script

Generated per spawn with the policy compiled into it, written to
`~/hydra/spawn/<run-id>/gate`, outside the worktree it guards.

**What the script is compiled from.** The policy files at `policy_sha`, the policy name, whether the
human lifted the floor on this run, and the resolved absolute path of every scope the policy
grants: the worktree, the store, the curate checkout when there is one, the corpus, the script's
own path and the parent repo's `.git`. All of it but the policy files is on `run_start` as
`policy`, `floor_lifted` and `scopes`, so a reader can reproduce the compile without the script
(contract 5.1).

Not a script that reads a policy file when it fires. Anything read at fire time is a file that may
have changed since the previous tool call. All three providers snapshot their hook *config* at
process start, but the config is a pointer and the script is its target, and the target is read
fresh on every invocation.

**Putting the script outside the worktree closes that by construction.** The child's write access is
scoped to the worktree by the very policy the script enforces, so the first call is checked by the
original script, which is what denies the write that would have replaced it. The induction holds for
`judge`, `read-only` and `standard`, and breaks for `permissive` and `curate`, which write outside
their worktree. Those two need the base case below.

**The gate denies writes to a fixed set of paths under every policy, `permissive` and `curate`
included, before the policy is consulted at all.** That is where `permissive` stops being
permissive. The list has two kinds of entry since ticket 40, a third since build ticket 44, and all
of it is compiled into the script per spawn.

**Denied outright:** the five protected configs of section 15.1; the generated gate script;
everything under `~/hydra/runs/` and `~/hydra/pruned/`; the skill store at `~/hydra/skills/` and its
snapshots at `~/hydra/skills-snapshots/`; the parent repo's `.git`, which `hydra run` knows because
it ran `git worktree add` from it; and the three per-run parents, `~/hydra/worktrees/`,
`~/hydra/spawn/` and `~/hydra/skill-worktrees/`.

**Carved out of those denials, per spawn:** the run's own worktree, and on a `curate` run its own
checkout of the store. The check is "inside a denied path and not inside an own path". The run's own
spawn directory is not carved out, because nothing a gated tool call does has business there: Hydra
writes it at spawn and the vendor process writes its own data root through no tool the gate sees.

**Denied inside that carve-out, since build ticket 44 on 2026-09-12:** the hook and provider config
the run's own binary reads out of the worktree. `.cursor/hooks.json` and `.cursor/mcp.json`,
`.agents/hooks.json` and `.agents/mcp_config.json`, `.claude/settings.json`,
`.claude/settings.local.json` and `.mcp.json`, all relative to the worktree. Every provider's
files are denied on every provider's gate, because a file a `claude` run writes into `.cursor/`
lands on the branch `hydra reap` commits all the same. `standard` grants `file_write` over the
whole worktree and two providers read their hook config from exactly there, so before this a run
could rewrite the file that points at its gate. The gate checks this list before the own carve-out,
and the shell check of `protected_path_named_by` reads the same list, so a command naming one of
these is refused whole. The three skill roots Hydra links into, `.claude/skills/`,
`.cursor/skills/` and `.agents/skills/`, are a separate list, `write_denied_worktree_paths`,
that only the `file_write` check reads: a write there replaces a link, and a shell command naming
the root is how `cursor-agent` and `agy` read a skill. The first version of ticket 44 put the
roots on the whole-command list and refused `ls -la .claude/skills/` on a run from the same
morning, which is why the two lists are two. The `.gitignore` files Hydra drops beside them stay
writable: a repo may track its own, and the ignore file decides what reap stages rather than what
the gate allows.
`protected_worktree_paths` in `src/policy/mod.rs` is the list, built from the provider modules.

The rule behind the list is one sentence. **A run writes only inside what Hydra created for it.**
Everything else under `~/hydra` belongs to another run, to the corpus, or to the store, and the
parent directories are denied rather than the siblings enumerated because siblings appear and
disappear while the run is live. A directory Hydra adds later under `~/hydra` is covered the day it
exists only if it is added here, which is why section 5's table and this list are meant to be read
against each other.

Ticket 40 measured why the flat list of seven was not enough. Under `standard`, whose `shell` carries
no scope, `rm -rf` on a sibling's worktree, on `~/hydra/pruned/`, on the store's `.git` and on the
parent repo's `.git` all came back allowed, and a `Write` into a sibling's gate script came back
allowed under `permissive`. None of those loses the record, and every one of them loses something a
record cannot rebuild, or kills a sibling run: its hook command checks the script's sha before
running it, so a replaced gate fails closed as `hydra/gate-tampered` on every call that follows. Each group earns its place
differently.

The **delegation policy directory** and the **generated script** earn it twice over: without them a
child under `permissive` can rewrite the policy constraining its siblings, and the induction above
has no base case.

The **other four configs** earn it because admission refusal alone protects them against a rung-two
patch and against nothing else. The environment allowlist, the export allowlist, the instruction
preamble and the canonical MCP document are read by every run and writable by a `permissive` one, and
each has a failure mode section 15.1 already names. An MCP server definition is a command Hydra
spawns, so a run that edits that document runs code of its choosing inside every future run.

**`~/hydra/runs/` and `~/hydra/pruned/` earn it because the record is the evidence.** A loop that
learns from the corpus must not be able to edit the corpus, and D10 makes the record the source of
truth for everything else besides. `hydra prune` moves a record rather than deleting it, and that
promise is empty if a child can delete the moved copy. This denial is a path block like the ones
above it, so the gate delivers it outright: a hook sees the path and refuses the write before it
reaches disk, measured on all three providers (ticket 26).

**The store and its snapshots earn it because they are the other half of D5's seam.** The record
and the skill store are the two things that cross between the harness and Rungs, and rung one
proposes rather than lands. A `permissive` run writing a skill into `~/hydra/skills/` lands one with
no proposal, which is not a leak in the curator but a second door beside it. A snapshot is where a
run's injected links resolve, so a run that writes into one edits its own instructions, which ticket
28's authorship test says cannot happen. Both were reachable before ticket 40, through `shell` under
`standard` and through `anywhere` under `permissive`.

**The per-run parents and the parent repo's `.git` earn it because a run's blast radius is its own
run.** A sibling's worktree holds uncommitted work `hydra reap` has not yet committed; a sibling's
checkout holds a proposal no human has seen; a sibling's spawn directory holds the gate its every call
runs through, and a replaced one ends that run. The parent repo's `.git` is shared by every worktree, so it holds every run's branch
and, on a machine where the parent is Hydra's own checkout, Hydra's own history.

**What the gate does not do is stop a run invoking `hydra` itself.** `hydra export`, `hydra prune`
and `hydra reap` are a binary reached through `shell`, so any rule against them is an argv match with
the pattern floor's failure mode in section 7.4: silent permission on the spelling nobody wrote down.
`./target/debug/hydra export` walks past it. So the spec does not claim the gate keeps Rungs away
from those three. What keeps them away is that no Rungs code path calls them, which is a property
of Hydra's own source rather than of a run, and `docs/rungs-spec.md`'s `evidence_is_read_only`
checklist item is what holds it true under a self-edit. Naming the right mechanism matters more here
than having two: a floor stated as a guarantee is the Hermes failure in miniature.

**Calling back into `hydra` per tool call is ruled out.** The small reason is a subprocess on every
call inside Cursor's 60 second hook budget. The large one is that a gate depending on the parent
still running turns a dead parent into a fail-open, on the provider that already fails open.

**On macOS the kernel stands behind the script, since build ticket 68.** Map ticket 62 asked whether
containment should be argv or the operating system, and the answer is the operating system's own
sandbox under the same user: every recorded launch is `/usr/bin/sandbox-exec -f <spawn>/sandbox.sb
<binary> <argv>`, and the profile denies `file-write*` under every path 7.5's deny list names,
parents included, allows it again under the run's own worktree and the vendor's own data root,
and denies `file-read*` under the corpus. SBPL takes the last matching rule, so a sibling worktree
stays refused while the own one is writable. Measured 2026-09-21 on all four providers: a corpus
write spelled as two shell variables, which the string floor cannot see, fails with "operation not
permitted"; a symlink is resolved before the check; a hard link into a denied path cannot be made
from inside; and `HOME` and the keychain are untouched, so the logins survive. The gate still
decides capabilities and writes the record; the kernel is what refuses the write. Two costs are
stated. The profile is inherited by every descendant, so a delegated child, which is Hydra itself,
cannot read its parent's record or write its own, and delegation from inside a macOS run is map
ticket 64 until it is redesigned. And a provider that brings its own seatbelt cannot apply it
inside Hydra's, so `codex` runs with its own off where Hydra wraps it. A Mac without
`sandbox-exec` refuses the launch rather than running bare; Linux launches unwrapped with
`sandbox` null on the record, and a port through landlock or bwrap is the roadmap's.

### 7.6 In-process subagents

Every policy but `permissive` denies them.

All three providers can spawn a subagent inside a run without going through `hydra run`: Claude's
`Task`, `agy`'s `invoke_subagent` and `define_subagent`, Cursor's `subagentStart`. None gets a run
id, none writes a run file, and none counts against the depth cap. D10 makes the record the source
of truth, and an in-process subagent is work with no record at all, on the exact trajectory
Rungs learns from.

All three surfaces are gateable, so this is enforceable rather than aspirational. The cost belongs
in the spec rather than in a footnote: **a delegated `claude` child cannot use `Task`**, which is a
real capability loss on coding runs.

## 8. Instructions and data

**A run's instructions are what Hydra wrote at spawn. Everything the run reads afterwards is data.**
Three things are instructions:

- the task string Hydra passed
- the policy and hook config Hydra compiled for that run
- the skills linked at the pinned sha

**Authorship is the test and timing is the proxy.** The two give the same answer in every case but
two. Inherited MCP tool descriptions load at process start, so timing would call them instructions,
and a server author wrote them, so authorship rules them out. They are data. The second case is
below.

**The user's own hooks are not an instruction source, and they can add text.** A hook config never
enters the model's context, but a hook's stdout can: `claude`'s `UserPromptSubmit` returns
`additionalContext` and the provider appends it before the first turn. Measured under ticket 46 on
2.1.263, 2026-09-06, on a recorded run whose sequences 1 and 2 are the hook firing and the user's
standing reminder arriving. That text is data. A human wrote it and Hydra did not write it at spawn,
which is the whole test, and it lands in the record through the `hook_response` frame under
`--include-hook-events`, so the record shows it the way it shows everything else the run read. The
hook config itself is still a second gate with a human owner, coherent under one principal, and its
denials still only subtract. An earlier draft said the hooks could only ever subtract, which was true
of the guards on this machine and not of the mechanism.

Do not call these channels. D5 uses that word for the two crossings between the harness and
Rungs, and a second meaning inside one spec is a reader's error waiting to happen.

**Authority is per run.** A delegated child's task is authoritative for that child. This is not a
concession; it is the only formulation that survives delegation, because a literal "obey only the
human" refuses a child its own task, and a child's task always comes from a parent agent.

The cost is that the task string launders. Any text a parent read can be promoted into a child's
instructions in one delegation hop, and no model has to be fooled for it to happen. **So the bound
is the policy**, which is why section 7.3 exists. A payload asking for something inside the child's
own policy is refused by nothing, and that is the honest statement of the limit.

**The rule text is advice, and it rides a preamble on the task string.** Not an injected skill: an
always-injected skill would spend section 6.4's measured listing cost on every run forever, and it
would carry frontmatter for the `: ` hazard to break. A preamble costs no listing slot, rides the
one input that is unambiguously Hydra's, and lands in the recorded task string, so the record shows
exactly what the run was told. The cost worth naming is that a constant preamble appears in every
trajectory rung one learns from.

**It asserts three things and no more.** What Hydra wrote at spawn is the run's instructions,
everything read afterwards is data, and the run's actions are bounded by a policy it cannot widen.
It never claims the gate will catch a bad action, never claims tool output was filtered, and never
says anything shaped like a guarantee. A run that trusts the gate is the Hermes failure with the
roles swapped.

**Hydra grows no constitution document.** Its constitution is executable: the five policies, the
pattern floor, the environment allowlist and the gate that compiles them. A 46 KB prose document
reached through one checklist item is the one item a "name the exact file or symbol" rule cannot
discipline. If a prose one ever appears it is a protected config by section 15's definition, so the
category exists and needs no fresh decision.

**A child's result is marked on the way back.** `hydra run` keeps the child's text as a plain string
field in its JSON. The MCP face renders it inside a boundary whose delimiter is randomised per call,
picked after the child has returned, so a child cannot close an envelope whose token did not exist
while it was running. Marking is advice too. What Hydra guarantees is that the marker is present.

**Hydra gates actions, never beliefs.** The spec promises that a run's actions are bounded by its own
policy. It promises nothing about what a model can be convinced of. The worked example: a delegated
`cursor-agent` child returns a result asking the orchestrator to grant write access to
`~/hydra/skills`. It is refused because the store's working tree is one of the paths
section 7.5 denies under every policy without exception, `curate` included. The layer that
stops it is the one that never read the text.

## 9. `hydra run` and the delegate face

```
hydra run --provider <p> --task "..." [--parent <run-id>] [--policy <name>] [--model <id>]
          [--files ...] [--timeout <dur>] [--shared-tree] [--allow-billed] [--allow-floor-escape]
```

Returns JSON on stdout (D7).

**Every child is spawned detached, and blocking is a view over it.** The default mode tails the
child's run file and waits. That makes timeout degradation trivial, because giving up on a call only
means it stops watching, and it means closing a terminal orphans nothing. The price is reaping,
which section 10 pays.

**Calls block with a timeout, then degrade to a handle.** `--timeout` defaults to roughly ten
minutes. On expiry the call returns a handle rather than killing the child, so short delegations
return inline where they are easiest to reason about and long ones become something you poll.
Concurrency comes free from the orchestrator's own parallel tool calls, so Hydra needs no scheduler,
only a cap on live children.

**The result stays small.** `run_id`, `outcome`, `outcome_source`, `exit_code`, `error`, the final
assistant text, the worktree branch, a diff stat, `usage` with its `usage_source`, and the run file
path. Nothing else. The reasoning matters more than the field list: delegation exists to keep a
child's trajectory out of the orchestrator's context, so a result that returns the whole trajectory
defeats the feature it implements. Ask for more explicitly with `hydra show <run-id> --events`.

**The result is a projection of the record, and the contract owns the names.** Ticket 11 called
these two fields `status` and `status_source`; the record contract's sections 8 and 8.2 name them
`outcome` and `outcome_source`, and one design does not get two names for one field. The contract
also drops `status_source`'s `provider` value, because no routable provider produces a run-level
status worth reading, and replaces it with `refusal` for the four outcomes Hydra decides itself. Exit
codes are useless as a common signal, which is why the outcome is derived rather than read: `claude`
has two meaningful ones, `cursor-agent` three, `gemini` nine.

The diff stat is the one returned field with no record equivalent, computed from git at return time.

**The MCP face is narrow: `delegate(provider, task, files?, policy?)`.** No `--shared-tree`, no
timeout override, no depth, no `model`. Those four are how a delegation escapes its worktree,
outlives its budget, spawns a tree, or drops to an older model generation to dodge a limit it
noticed. They are human decisions rather than per-call choices a model makes unattended. `model` is
on this list for a concrete reason: `agy` serves Claude Opus 4.6, three releases behind what `claude
--model opus` resolves to, so a `model` argument is exactly that lever. The CLI keeps the full
surface.

`policy?` takes a name from the five, and section 7.3 bounds which names a given caller may pass.

**Depth is capped at 2, configurable**, computed at spawn from `parent_run_id`. A child that
delegates is how three subscriptions get drained overnight and you find out from a billing page.
Section 7.6 closes the cap's blind spot.

## 10. Run lifecycle

**Process exit is the authoritative end of a run.** `claude` always writes exactly one `result`
frame, `cursor-agent` only on success. The `result` frame is optional enrichment and the schema must
not require a terminator event.

**Liveness is derived, never announced.** The pid and the process start time go in the run file's
opening envelope, written at launch. A run is alive when a process matching both still exists. The
start time is not optional, because macOS recycles pids and a bare pid can point at something
unrelated. No sidecar, no lockfile, no second source of truth to contradict D10.

**Nothing appends to a run file except the process that owns it.** Death is a conclusion a reader
draws, never an event a bystander writes.

**Interactive runs are harvested at exit.** Every provider writes its own transcript: `claude` keeps
append-only `.jsonl` under `~/.claude/projects/<slug>/`, `agy` keeps `conversation_summaries.db` and
a `brain` directory under `~/.gemini/antigravity-cli/`, `cursor-agent` keeps per-chat directories
under `~/.cursor/chats`. When an interactive run exits, Hydra converts the vendor's transcript into
the run record. That keeps the run record the single contract D5 requires, and an interactive
session is recorded late rather than not at all.

`CURSOR_DATA_DIR` does not move the harvest source. `chats/` hangs off `CURSOR_CONFIG_DIR`, which
Hydra does not set.

**Take `agy`'s transcript path from the run rather than constructing it.** A hook reports it as
`~/.gemini/antigravity-cli/brain/<conversationId>/.system_generated/logs/transcript_full.jsonl`,
under the user's home rather than in the workspace.

**Harvest at exit is the contract; live tailing is a per-provider optimisation.** Only `claude` is
confirmed to write an append-only log during the run, so only `claude` interactive runs can stream.
A contract three providers cannot meet is not a contract, so the spec states the streaming answer
per provider rather than promising one it breaks twice in three.

**Harvest is idempotent and any process can do it.** The run envelope names the provider and its
session id, so harvest is a pure function of two files on disk. `hydra status` finds a dead run with
an unharvested body and completes it.

**Session identity.** Hydra mints the run id. The provider's own session id sits beside it as a
field: `agy` gives a `conversation_id` in its `init` event and `--conversation <id>` resumes it,
`claude` has its own. `hydra resume <run-id>` reads that field and hands the provider its own resume
flag. Hydra never reconstructs conversation state itself. A resumed run gets a fresh run id linked to
the old one, so one file per run survives intact. Where a provider has no resume concept, resume
fails loudly rather than starting a fresh conversation dressed up as a continuation.

**Observation is read-only.** `hydra logs <run-id> --follow` tailing the run file is the only view.
No attach, no input, no answering a permission prompt from outside. Children run detached and
unwatched, D5 forbids a second channel, and two of three providers cannot be asked anything mid-run
anyway, so this is the only coherent shape rather than a compromise.

**Nothing is cleaned up automatically.** `hydra prune` and `hydra reap` are commands you type. Prune
moves run files rather than deleting them unless a flag says otherwise, because D12 keeps lossless
export a standing constraint and anything deleted is training data destroyed. Reap never removes a
worktree with uncommitted changes: it commits them to the child's own branch first, then removes the
worktree and the run's spawn directory together. Rungs cannot call either at either rung.

This is the Hermes lesson turned on our own tooling. Hermes told its model that archiving was
recoverable while the tool underneath called `shutil.rmtree`, so a guarantee written in a prompt is
not a guarantee.

**The scenario, worked.** You close the Ghostty window on a session with two children mid-flight.
Hydra dies. The children are detached, so they keep running and keep writing their own run files.
The interactive session's run file holds an envelope with a pid nothing matches any more, and the
vendor transcript sits on disk unharvested. The next `hydra status` reconciles all three: it derives
the parent as dead, harvests its body from the vendor transcript, and reports the two children as
alive by matching pid and start time. If they later die badly, their worktrees show up as abandoned
and wait for a `hydra reap` you type. Nothing was lost and nothing needed to be running.

## 11. Auth

**Hydra holds no provider credential.** Every vendor CLI owns its own login in the macOS Keychain.
Hydra launches the CLI and stays out of the way, which satisfies D6 by writing no code. The only
secret Hydra ever handles is `GEMINI_API_KEY`, and only on a run where a human typed
`--allow-billed`.

v1 ships the agent-CLI plane only. The raw-model plane is one gated path, `GEMINI_API_KEY` to
`gemini`, and nothing else. Nothing on that plane has ever made a successful call. The spec names
LiteLLM with virtual keys as the design for the day the plane gets built, and builds none of it.

**Auth never happens inside a run.** Three layers: pre-flight before launch, refusing to start a
provider that is not logged in; a run that hits the wall anyway fails fast with a remediation string
naming the provider and the command to run; and if a provider ever asks for a pasted code, Hydra
suspends teeing, hands the bare pty to the human, and resumes teeing after. This is structural
rather than a policy choice, because a detached child cannot complete a browser login.

Neither routable Google surface asks for a pasted code. `gemini` uses a loopback redirect. `agy`
carries a first-class `AuthResult_HeadlessAuthRequired` state, so a login it cannot complete becomes
a typed result rather than an improvised terminal prompt, and Hydra reads that signal rather than
sniffing for a paste prompt. The teeing suspension stays as the generic net for the others.

**Auth failure is its own run outcome**, distinct from task failure, carrying a remediation string.
An expired token is not a task failure, and Rungs must never evolve a skill because a login
lapsed.

**A delegated child receives no credential of any kind.** No virtual key, because the raw plane does
not ship. No inherited secret, because of the environment allowlist. No `--allow-billed`, because it
is not inheritable. No browser login, because it is detached.

**Hydra never writes into a vendor's own config.** Redirecting a provider's config root, as
`CURSOR_DATA_DIR` does, is not the same as editing the user's, and the rule holds unchanged. The one
specced exception is dormant: if the `gemini` fallback is ever promoted, Hydra sets the auth type per
run through `GEMINI_CLI_HOME` rather than editing `~/.gemini/settings.json`, which pins
`selectedAuthType` and is why `gemini` never read the key that was sitting in the environment all
along.

**Hydra filters its own injected values at write time**, and only those: the exact string it injected
for that run, which in v1 means `GEMINI_API_KEY` on an `--allow-billed` run and nothing otherwise.
Hydra knows the value at injection time, so it can strip it from frames as it writes them. That is a
guarantee it can keep. The leak this closes has nothing to do with where the key is stored: an agent
runs `env`, the provider streams that output back as a frame, and frames are written verbatim.
Anything broader is best-effort and belongs to export.

Three plaintext credential exposures elsewhere on this machine are a non-goal for Hydra. It reads
none of those files.

## 12. Run outcomes

A run ends on exactly one outcome. Several of them exist because a provider reports success on a run
that did no work, and each one exists so rung one does not learn a lesson from a run that taught
none.

| Outcome | Means | Rung one |
|---|---|---|
| `completed` | the task ran to an end | learns |
| `failed` | the task ran and failed | learns |
| `timed_out` | the watcher gave up; the child may still be running | learns |
| `crashed` | the process died | learns |
| `gated` | the gate refused, or a spawn was refused for naming a wider policy | never |
| `auth_failed` | not logged in, or the login lapsed mid-run | never |
| `skill_collision` | a linked skill name the provider did not resolve as Hydra's | never |
| `mcp_unavailable` | the launch check or the Cursor pre-flight refused | never |
| `rate_limited` | the provider refused its own limit | never |

**All three providers exit 0 and report success on a run where the gate refused everything.** So
`gated` derives from the event stream, never from the exit status. So does every other outcome in
the lower half of that table.

**The rung-one column says what rung one learns from, never what Hydra keeps.** Every run in this
table is recorded in full, verbatim, whatever its outcome. D10 and the never-discard rule make that
non-negotiable, and a reader who collapses "rung one never learns from this" into "throw it away"
has the record backwards. A denial, a gate refusal, an expired token and a rate limit all land on
disk complete.

The outcomes rung one ignores are not signal-free, they carry a different signal. A run that failed
on too narrow a policy is a policy bug rather than a skill bug. Nothing in v1 reads that, and D5
requires every field Rungs reads to be one the harness writes rather than the converse, so
writing it with no reader is allowed and deliberate.

**A rate-limited `claude` run reports `subtype: "success"` with `is_error: true` and
`api_error_status: 429`.** A recorder keying off `subtype` files an exhausted run as a clean one.

`mcp_unavailable` stays distinct from `gated` so rung one never reads an MCP misconfiguration as a
policy refusal. Two checks produce it and they differ only in when Hydra noticed, so which one fired
is a field Hydra sets rather than a second outcome.

**That table describes a provider run.** A `hydra evolve` pass takes its outcome from the same enum
and reads two of the rows differently, because a pass and the patch it judged are two different
things to succeed at. A pass that refused a patch on a protected path, or whose panel blocked, or
whose admission gate caught a build failure, is `completed`: the pass did its job. `failed` means the
pass itself broke, git or the worktree. `gated` never applies, because nothing gates a harness run.
Filing a gate that worked as a failure would put it in the corpus as a lesson, which is the mistake
this table exists to prevent, pointed the other way. The contract's section 8.4 owns this.

## 13. The tool-call adapter

Mapping the providers' incompatible tool-call representations onto one shape is the harness's job,
per run record event type, and it is the piece with the most load on it for the least settled
detail.

**The adapter may return `unclassified`, and that is a feature.** `agy` gives a permission denial and
a file-not-found the same `state: "ERROR"` and the same `error.type: "TOOL_ERROR"`, and both messages
contain the word "permission". A substring rule is wrong rather than weak. An adapter that says it
cannot tell keeps the gap visible per provider rather than averaged away, and guessing produces a
curator that rewrites a skill because a file was missing.

**Nothing upgrades an `unclassified` outcome to success.** A label read off the terminator would be a
lie on three providers.

**Export depends on this adapter more than anything else does.** Extracting "every path this run
touched" runs through it, so a tool shape it fails to recognise is a path nobody sees. A run holding
any tool call the adapter could not classify is not exportable (section 14). The adapter must be
allowed to say it cannot answer, and export must treat that as a refusal rather than an empty
answer.

Getting the adapter wrong is recoverable because the raw frame survives. An adapter bug fixed next
month fixes last month's records too. Until it is right, the corpus is smaller rather than leakier.

## 14. `hydra export`

Export is the one path by which anything leaves this machine, and it is a command a human types. No
run, no curator and no delegation policy can reach it.

**Default deny, allowlisted by path.** A run exports only if every path it read or wrote sits inside
a repo declared exportable, computed from the record at export time. The allowlist is a file in the
repo and one of section 15's protected configs. It is the only one whose failure mode is data
leaving the machine rather than a bad action taken on it.

**Eligibility is computed over a run's whole subtree.** A parent exports only if every run beneath it
is exportable, because a parent's tool result carries a child's summary of work done in another
worktree. Without the subtree check, delegation walks content out of an excluded workspace and into
the corpus. A child exports as its own line, since it is a complete task attempt on its own terms.

**Scoping beats scrubbing**, and that is why the allowlist carries the weight. File contents are the
largest class in a trajectory by volume, no pattern touches them, and the aggregate leaks even when
no single string does: a run that read twelve files describes the shape of a private codebase.
Redaction then shrinks to four classes with a fixed shape, no model pass, and a residue named rather
than implied: private repository and project names, customer data inside a fixture, secrets split
across messages, unprefixed proprietary tokens, and the stdout of the user's own hooks, which
section 6.6 puts on the stream of every recorded `claude` run. Nothing catches those. The allowlist
is what carries them.

**Hook stdout gets no redaction class of its own**, and the reason is worth stating rather than
leaving as an omission. The export gate scopes by path and hook stdout is not a path, so the
allowlist still decides whether the run leaves at all, and the write-time prefix sweep already reads
every frame including these. What it does not get is the "fixed shape" the four classes rest on,
because a user can edit those scripts to print anything.

**A hit on the verification scan aborts the entire export.** It emits nothing and exits non-zero.
Output that the redactor's own rules still flag means the redactor is broken, and a partial corpus
from a broken redactor is worse than no corpus.

**A prefix sweep runs on the write path and again at harvest**, for `AIza`, `sk-ant-`, `ghp_`,
`ya29.` and PEM headers, setting a flag on the envelope and leaving the frame verbatim. This belongs
to the harness rather than to export because it is what makes a live credential sitting in `runs/`
visible the day it lands rather than at the first export, and it covers harvested bodies, which
never pass the write path at all.

Enforcing eligibility during the run rather than auditing it afterwards is an extension. All three
providers hard-block a tool call from a config Hydra writes, but whether a block can be scoped to a
path argument on all three is unmeasured. Written as an audit, the spec is implementable today.

## 15. Protected configs and protected paths

The closed tickets grew two lists at different times and neither running count was ever
recomputed. They are reconciled here once.

### 15.1 The five protected configs

Human-authored, read by a run, and writable by nothing a run does. Rung one cannot reach any of them
because its write surface is the skill store alone. A rung-two patch touching one is refused before
review rather than blocked by it.

**All five are also denied at run time under every policy**, per section 7.5. The two mechanisms
answer different questions and neither substitutes for the other: admission refusal stops a proposed
patch, and the gate stops a running child. A config protected only at admission is protected against
rung two and against nothing else, which is what the seam check found and section 15.2's table now
records.

| # | Config | Failure mode if written |
|---|---|---|
| 1 | The delegation policy set, including the pattern floor file | a loop widens its own permissions and walks around every gate |
| 2 | The environment allowlist | a secret reaches a child by inheritance |
| 3 | The export allowlist | data leaves the machine |
| 4 | The instruction preamble | a run is told a different rule than the one Hydra enforces |
| 5 | The canonical MCP document | an MCP server definition is a command Hydra spawns, so a loop that edits this runs arbitrary code inside every future run |

**The pattern floor is protected by inheritance, not by its own entry.** It is a sixth file and it
lives in the delegation policy directory precisely so the count stays at five.

### 15.2 The protected paths: twelve

Every ticket that states a number says nine. **The items those tickets actually name come to
eleven.** The running count was carried from seven through the preamble and the MCP document without
folding in the export allowlist or either of the two additions the delegation policy work made. That
correction was arithmetic rather than a new decision.

**The twelfth is a decision, and the seam check made it.** The run records directory was a run-time
denial the Rungs spec relied on and no document listed, so it is a new row rather than a
recount.

Matched case-insensitively, because `write_file(path="bible.md")` on macOS writes `BIBLE.md` and an
exact-case membership test misses it.

| # | Path | Kind | Refused at admission | Denied at run time |
|---|---|---|---|---|
| 1 | the review module: quorum arithmetic, the severity table, the checklist loader, `ApprovedPatch` | code | yes | no |
| 2 | the protected-path list itself | code | yes | no |
| 3 | the environment allowlist | config | yes | **yes, under every policy** |
| 4 | the hook-config writer | code | yes | no |
| 5 | the delegation policy directory | config | yes | **yes, under every policy** |
| 6 | the run-record writer | code | yes | no |
| 7 | the admission gate | code | yes | no |
| 8 | the export allowlist | config | yes | **yes, under every policy** |
| 9 | the generated per-run gate script | generated | not applicable | **yes, under every policy** |
| 10 | the instruction preamble | config | yes | **yes, under every policy** |
| 11 | the canonical MCP document | config | yes | **yes, under every policy** |
| 12 | the run records directory, `~/hydra/runs/` | data | not applicable | **yes, under every policy** |

**What admission refuses, as built.** The five config rows, 3, 5, 8, 10 and 11, since build ticket
36. The code rows 1, 2, 6 and 7, since build ticket 40 on 2026-09-12; between the two tickets a
patch to any of them reached the panel, and rungs spec 4.4's sentence about the opening scenario
was false for every pass run in that window. Map ticket 59 found it. Row 4 since build ticket 66 on
2026-09-21: until then the writer lived inside `src/run.rs`, which is the whole launch sequence and
stays editable by rung two, so only the compiler half under `src/policy/` was refused and a patch to
`write_spawn_config` reached the panel. Map ticket 60 found it, and the writer is `src/spawn_config.rs`
now, its own protected row, with `src/policy/` mapped to row 9 alone.

**Entry 12 arrived from the seam check and is the one row that is not source.** The other eleven are
things a rung-two patch could propose changing. A patch cannot propose changing the corpus, so its
admission column is not applicable, and it is listed here because the run-time denial is what
`docs/rungs-spec.md` relies on when it says a loop that learns from the evidence cannot edit the
evidence. Before the seam check that reliance had no entry on either side.

**"Protected" was carrying two mechanisms under one word**, which is why the table has two columns.
Admission refusal is a rung-two check on a proposed patch. Run-time denial is the gate refusing a
tool call. The five config rows get both, entries 9 and 12 get run-time only, and the five code rows
get admission only. Section 7.5 states the run-time set as one list and says why each group is on it.

**Entry 9 is on the list under protest, and the spec says so.** The gate script is generated per run
into `~/hydra/spawn/<run-id>/` and reaped with the worktree, so no rung-two patch can touch it in
any ordinary sense. What a patch can touch is the code that generates it, which is entry 4 under
another name. It stays listed because the run-time denial is real and load-bearing, and dropping it
from the list would lose that. A later cleanup that splits the list by mechanism should keep the
denial and drop the admission column for this row.

**Entries 4 and 9 are different objects and are easy to collapse by mistake.** One is the writer,
one is its output.

**Two code rows arrived on 2026-09-22 without a number of their own**: `src/sandbox.rs`, the
seatbelt profile that the run-time denials of rows 5, 9 and 12 lean on since map ticket 62, and
`src/containment.rs`, the marker check of build ticket 57. Both are refused at admission and
neither is denied at run time, the same shape as rows 1, 2, 4, 6 and 7. A hiring-panel review of
that day found a patch to either reaching the panel, the third time a mechanism landed outside
`PROTECTED_CODE_PATHS` after build tickets 40 and 66, and the recount of this table is arithmetic
again: fourteen.

**Row 9 reaches the config that points at the script as well, since build ticket 44 on
2026-09-12.** On `cursor-agent` and `agy` the hook config is a file inside the worktree,
`.cursor/hooks.json` and `.agents/hooks.json`, and the worktree is the one directory every writing
policy grants. A run that rewrites it drops the pointer to the script row 9 protects, and the
rewritten file is committed by `hydra reap` for the next run to inherit. So the run-time denial
covers those two, the MCP configs beside them, the project-scope files `claude` reads from cwd
(`.claude/settings.json`, `.claude/settings.local.json`, `.mcp.json`), and the three skill roots,
under every policy and on every provider's gate. Section 7.5 lists them. Not a new row: it is
generated per run like the script and reaped with the worktree, so the admission column would read
the same way row 9's does.

### 15.3 The paragraph for `~/hydra/CLAUDE.md`

The third population in section 4 has no other home. This wording goes in `~/hydra/CLAUDE.md`, and
it is the only one of the three deliveries with nothing enforcing it:

> **Nothing here is enforced.** Hydra's gate applies to runs Hydra launches, not to you. You are an
> agent working on Hydra's own source, which is the one population the harness does not gate: no
> compiled policy, no hook config, no injected skills, no preamble. The five protected configs in
> `docs/harness-spec.md` are protected against Hydra's own runs, and this file is what stands in
> their place for you. Treat them as read-only, and if a change genuinely needs one of them, say so
> and let a human make it.

**Do not reuse the preamble's phrasing here.** The two say similar things and only one is backed by
a mechanism, and a reader who assumes otherwise is exactly the failure this paragraph exists to
avoid.

## 16. Per-run human flags

Some limits lift only when a human types a flag on that run. **The flag is never inheritable, the
delegation policy cannot set it, and it is not on the MCP delegate face.** A flag in argv is chosen
over a blocking prompt, which trains you to hit yes, and over a config toggle, which stays on for
months. It also lands in the run record as an argument, so the record shows why a run was allowed to
do the thing.

Two instances today:

| Flag | Lifts |
|---|---|
| `--allow-billed` | the raw-model plane, one gated `gemini` call on a billed key (D16) |
| `--allow-floor-escape` | the pattern floor in section 7.4 |

The pattern is named once here so a third instance costs a table row rather than a decision. One
candidate has already been considered and declined: proceeding with a degraded MCP set after the
launch check refuses. Three outs already exist, so the lever gets built when the records show it is
wanted.

## 17. Command surface

| Command | Does |
|---|---|
| `hydra run` | section 9 |
| `hydra status [<run-id>]` | reconciles live pids against open run files, harvests dead runs with unharvested bodies, reports abandoned worktrees |
| `hydra logs <run-id> [--follow]` | tails a run file. The only way to watch an unwatched child |
| `hydra show <run-id> [--events]` | the full record a delegation result deliberately omits. Takes an unambiguous run-id prefix since build ticket 39 |
| `hydra resume <run-id>` | hands the provider its own resume flag, mints a fresh linked run id |
| `hydra reap [<run-id>]` | commits a worktree's uncommitted changes to the child's branch, then removes the worktree and the spawn directory. Takes a prefix since build ticket 39 |
| `hydra prune` | moves run files out of `runs/`. Deletes only behind a flag |
| `hydra curate` | invokes rung one as an ordinary recorded run under the `curate` policy |
| `hydra evolve [--task <text>]` | runs one rung-two pass: a proposer, the admission gate, a panel of four, a branch. `--task` hands the proposer a target, and since build ticket 63 the judges see it as evidence that waives nothing |
| `hydra evolve --review <pass-id> --evidence <file>` | a fresh full-panel review of a blocked pass, the same patch under a new pass id with the human's evidence in every judge's prompt; the old block stays. Build ticket 64 |
| `hydra doctor` | the root, the policy path, the four binaries against the measured versions, the policy sha, the protected configs present or missing. Launches nothing; exit 1 when something is missing. Build ticket 67 |
| `hydra export` | section 14 |
| `hydra usage recount` | re-derives token counts from stored text with a better tokenizer |

`hydra reap`, `hydra prune` and `hydra export` are typed by a human and reachable from no run at
either Rungs rung. `hydra curate` and `hydra evolve` are typed by a human too, and
`docs/rungs-spec.md` says what they do.

**`hydra usage recount` has no work to do in v1**, because no run produces `usage_source:
estimated`: the one provider that would have is `unavailable` instead, for the reason in the
contract's section 7.8. The command stays in the surface because the record keeps the text the
recount would read, so the day a provider earns an estimate the tool already exists.

## 18. Non-goals and extension points

Not built, and named so a reader stops looking:

- **A TUI of Hydra's own.** Ruled out by ticket 44, which amended D3. Herdr is the interactive
  layer and Hunk is the diff viewer, both third-party, both driven from outside Hydra: a Herdr
  plugin launches `hydra run` in a pane, a bridge on Herdr's side tails `~/hydra/runs/` and labels
  the pane from the record, and Hunk opens the run's branch against `diff_stat.base_tree`. Hydra's
  core knows neither exists, which is D10. Interactive runs still record thin, since a pane buffer
  is a screen and not a record.
- **The raw-model plane.** LiteLLM with virtual keys is the design, not the build. One gated
  `gemini` path ships.
- **Hermes.** Not routable. `HERMES_HOME` is specced as the config-root lever if it ever returns.
- **Multi-machine use.** Every credential is in this machine's Keychain or environment. You log in
  again per machine.
- **Langfuse**, or any viewer as a source of truth. A viewer tails `runs/`.
- **Training.** Out of scope. Lossless export stays a constraint on the record (D12).
- **Cursor GUI automation.** Ruled out. Headless CLI only.
- **Reading the user's own credential files.** Hydra reads none of them.
- **Interpreting shell commands.** Hydra bounds where a run acts. The user's own guards bound what a
  command means, and section 6.6 explains why that division is deliberate rather than a gap.
- **ACP.** No routable provider speaks it. `claude` and `cursor-agent` never did, and ticket 18
  retired it for Google. The two that did, `gemini` and `hermes`, both left the routable set, which
  is what makes ticket 01's "drive ACP, not print mode" a superseded recommendation rather than a
  wrong one. It is not an extension point, because its headline advantage was answerable permissions
  and the gate replaced those on all three providers with something better for a detached child: a
  refusal decided before the process starts. One watch condition carries it, named by capability
  rather than protocol, so it fires if any routable provider gains a channel that can answer a
  permission request mid-run.
- **A mid-run input channel.** `claude --input-format stream-json` is the only bidirectional stdin
  any provider offers. Specced and unbuilt: observation is read-only, D5 caps the crossings at two,
  and building this is the third channel arriving through a side door.

## 19. Open questions this spec does not close

Each of these is a ticket rather than a paragraph.

### 19.1 Filenames in the spawn directory

Three artifacts share `~/hydra/spawn/<run-id>/` and no closed ticket names any of them. This spec
names them `settings.json`, `mcp.json` and `gate`, and puts Cursor's data root in a
`cursor-data/` subdirectory rather than at the top of the spawn directory.

**That subdirectory is a refinement, and it is flagged rather than buried.** Ticket 29 says Cursor's
data root is the spawn directory itself. Under that reading the vendor writes `projects/<slug>/`
into the same directory that holds the gate script, which weakens "the script lives where nothing
else writes" as a structural argument even though the gate's own rule still denies writes to the
script path. The subdirectory keeps the two concerns apart at no cost. It should be confirmed rather
than assumed.

### 19.2 `denied` against `gated`, closed

Settled by the record contract's section 8.1, which owns the enum: `denied` is dropped and `gated`
stands. Two labels for one state is a bug with a schema, and `denied` also read as a per-tool-call
fact, which is `permission_decision`'s job. Section 12 above was already right and now has a reason
rather than a preference. Kept here as a closed entry so a reader who finds `denied` in ticket 11
knows it was decided rather than dropped by accident.

### 19.3 Three unverified `agy` facts

Carried as caveats rather than claims, and none is on a path Hydra takes automatically: whether
`--effort` does anything to a Claude model id on `agy`, whether `thinking_tokens: 0` means the field
is unwired for Claude ids, and how much of `agy`'s 16,338-token floor a resumed `--conversation` run
recovers from cache.

### 19.4 Nothing reads the policy signal

Ticket 27 says a run that failed on too narrow a policy is a policy bug. So `gated` runs carry a
signal about the delegation policy rather than about a skill, and section 12 records it. No system
in v1 reads it. Rung one is barred from learning from these runs, correctly, and nothing else looks.

This breaks no seam, because D5 requires every field Rungs reads to be one the harness writes
and says nothing about the converse. It is on the map as fog rather than as a ticket, beside the
second-corpus patch, which has the same shape: a signal worth keeping before anyone knows what reads
it.

### 19.5 No stall detector

Liveness is a pid plus a process start time, so a run that is alive and emitting nothing looks
exactly like a run working hard. None of the five failure classes covers it.

Message deltas are the cheap heartbeat and section 6.7 turns them off, which is the right call for
every other reason and closes this door on two providers. `agy` keeps a heartbeat by accident,
emitting `step_update` with `state: ACTIVE` and shipping no flag to stop it. So the detector, when
someone wants it, is either a flag flip on `claude` and `cursor-agent` or a per-provider signal that
exists on exactly one of the three today.

### 19.6 What an interactive run gets from sections 6.5 and 6.6

An ungated interactive run gets no worktree (6.3); a gated one, `claude` only, gets one (build
ticket 28). Two of the three providers read their injected MCP
config out of the worktree (6.5). So on `cursor-agent` and `agy` an interactive run has nowhere to
put one and gets no injected servers, while `claude` gets its config on the command line and does.
The record contract's section 5.6 records that split as it stands.

Two consequences want a decision rather than a note. The fatal Cursor pre-flight in 6.5 checks that
every injected server is ready, and with no injected servers on an interactive run it has nothing to
check, so "fatal interactively too" is currently a promise about an empty set. And `CURSOR_DATA_DIR`
points into the run's spawn directory, which exists per run and is reaped with a worktree that an
interactive run does not have.

Neither breaks anything today. Both are the kind of thing a builder hits in the first hour and
resolves by guessing, which is the reason to write it down.
