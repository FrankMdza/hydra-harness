# 23 - MCP config across providers, against the ticket 10 shape

Type: research
Status: resolved

## Question

Graduated from the map's fog, which parked this until the skill injection strategy closed. Ticket 10
closed it: per-run injection into the worktree, first-party root per provider, self-ignoring
directory, links into an immutable snapshot, nothing written outside a path Hydra owns.

The question is whether MCP server config can ride the same shape, and what it costs if it cannot.

Answer these for each of the three routable providers, `claude` 2.1.257, `cursor-agent`
2026.08.31-4057e58 and `agy` 1.1.23. Re-read every version before trusting a claim, since two of
the three self-updated mid-map.

- **Where the config lives.** Every path scanned, at user scope and at project scope, and whether
  a project-scope file merges with the user one or replaces it.
- **Whether a worktree-scoped file is read at all.** This is the load-bearing question. Ticket 10
  found `agy` ignores its own workspace skill root unless the directory arrives as `--add-dir`, so
  do not assume cwd is the project for any provider. Test it, do not read it out of a doc.
- **Whether a symlinked config file is followed**, since ticket 10's injection links rather than
  copies, and Gemini was once thought to ignore a symlinked `SKILL.md` before that turned out to be
  a dangling relative link.
- **What the format is** per provider, and whether one canonical document could generate all three
  or whether the shapes genuinely differ.
- **What happens to a server the user already configured.** Merge, shadow, or conflict, and whether
  the provider says anything when two definitions share a name. Ticket 10 had to fail a run on a
  skill name collision; find out whether MCP has the same hazard.
- **Whether a server can be scoped off.** Hydra's delegation policy (ticket 11) decides a child's
  permissions, and an MCP server is a tool surface. If a child inherits every server the human
  configured, say so plainly, because that is a hole in ticket 11 rather than a detail here.

`agy` has an `mcp` subcommand and `cursor-agent` has `--approve-mcps`, so both have a surface worth
probing. `agy -p "/skills" --output-format json` costs zero tokens and returns resolved paths;
check whether an equivalent read-only slash command exists for MCP before spending quota.

Output is a findings file at `research/23-mcp-config-injection.md` plus a recommendation on whether
this becomes a decision ticket or folds into the harness spec as written.

## Partial progress, 2026-09-02, agent killed by a rate limit

The agent died on a session rate limit before writing a findings file or reaching a conclusion. Only
its narration survived, so what follows is where it had got to, not what it found. Nothing here is
established. Its probe rig may still be at
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t23/`, which
is session scratch and will not survive long. It contains string dumps it had already pulled from
the `agy` and `claude` binaries, which are the expensive part to redo.

Nothing of the user's MCP config was modified. `~/.gemini/config/mcp_config.json` is still the
zero-byte file it was in June, and `~/.cursor/mcp.json` does not exist.

Where it got to:

- Located the installed bundles for all three providers and read the user's existing MCP config.
- Found Cursor's MCP paths in the bundle, along with how it computes the project root and where
  approvals are stored.
- Pulled MCP config paths out of the `agy` binary with `strings`.
- **Claude reads the worktree's `.mcp.json`.** This is the one substantive claim it made and the
  most useful one, since it is the load-bearing question for whether MCP rides ticket 10's shape.
  It is a claim from reading the binary, not a live probe. Verify it.
- Was starting the symlink test and the user-against-project collision test under an isolated
  `HOME` when it died.

Whoever resumes should read ticket 24's partial results first. Its precedence findings landed after
this ticket was written and they change what to look for: the three providers resolve a name
collision in three different ways, and they do not even agree on which string is a skill's identity.
Expect MCP server names to have the same problem and test for it directly.

## Answer, 2026-09-03

Findings with every command in
[research/23-mcp-config-injection.md](../research/23-mcp-config-injection.md), 744 lines, probed
live against `claude` 2.1.257, `cursor-agent` 2026.09.02-c22c1a3 and `agy` 1.1.25.

**MCP rides ticket 10's shape on all three, and each wants a different launch flag.** A
worktree-scoped config is read by every provider, verified with a spawn-logging probe server rather
than a listing, so these are connections. `claude` needs cwd to be the worktree and ignores
`--add-dir`. `cursor-agent` takes cwd or `--workspace` and also ignores `--add-dir`. `agy` requires
`--add-dir <worktree>` and cwd alone loads nothing, reproducing ticket 10's rule exactly. The probe
also found a third `agy` path its own embedded documentation does not name,
`<worktree>/.agents/mcp_config.json`.

**Symlinks are followed on all three**, absolute and relative, into a snapshot directory, confirmed
in a paid Cursor run. Worth flagging beside ticket 26: Cursor follows a symlinked MCP config while
refusing a symlinked hook config. Two loaders, two rules, so "write real files" is a hook rule and
not a general one. A dangling link is silence and exit 0.

**One canonical document can generate all three, and cannot be all three.** The stdio server body is
byte-identical across the providers, loaded unchanged in each. File name, directory and the remote
key all differ (`type`+`url`, `url`, `serverUrl`), and `agy` refuses `command` and `serverUrl`
together.

**Collision behaviour splits three ways, and `claude` is the only surface on this map that reports
one.** A project server wins and `claude mcp list` prints a diagnostic naming both endpoints, free.
`agy` collapses duplicates silently. Cursor is the hazard: an unapproved project entry does not
shadow the human's same-named server, it fails to replace it, so the listing says "not loaded (needs
approval)" while the human's server process starts.

**Claude's approval gate does not apply in print mode.** `claude mcp list` reports pending approval
and `claude -p` connects anyway. The settings that do approve are user-scope only, never the
worktree's `.claude/settings.json`.

### The delegation hole is real, and it is exposure rather than execution

Probed directly on `claude` against a copy of `~/.claude.json`: a user-scope server appears in a
headless child's tool list inside an isolated worktree, and its process runs. Cursor and `agy`
follow from their loaders with nothing left to probe, and neither has a per-run off switch. Only
`claude` can scope MCP off per run, through `--strict-mcp-config`, `--restricted` (which drops all
MCP despite the help text saying otherwise) or `disabledMcpjsonServers` in `--settings`, and all
three kill the server process rather than only hiding the tool.

Ticket 26's gate blocks the calls on all three, so a child cannot use an inherited server. What
Hydra cannot prevent on two of three is the server starting and its tools reaching the model's
context. Ticket 11 says a child's permissions come from a policy, and a policy that cannot say "this
child sees no MCP" is not saying what that sentence claims. Ticket 27 owns what to do about it.

### Two places ticket 10's shape does not survive

`.mcp.json` sits at the worktree root and cannot self-ignore the way ticket 10's skill roots do, so
`git status` shows it and `git add -A` stages it. `.cursor/mcp.json` and `.agents/mcp_config.json`
both can. And approving a Cursor server writes state outside any path Hydra created, which is D9's
rule broken by the vendor rather than by us.

### Recommendation, taken

Split, as the findings recommend. The mechanics fold into the harness spec as written. Ticket 29
takes the two choices that are genuinely about MCP config. The third, whether a delegated child gets
MCP at all, belongs to ticket 11 and is being decided in ticket 27, which is in flight.

**Cost.** `claude` free, using a `deferred_tools_delta` transcript oracle the agent built for it.
`cursor-agent` one paid run. `agy` four paid turns, 60,383 tokens on `gemini-3.8-flash-low`, and it
is the only provider with no free route to its resolved MCP set.
