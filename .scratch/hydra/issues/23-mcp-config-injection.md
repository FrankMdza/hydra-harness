# 23 - MCP config across providers, against the ticket 10 shape

Type: research
Status: open

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
