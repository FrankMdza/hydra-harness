# 29 - Where MCP config goes, and what to do about Cursor's approval state

Type: grilling
Status: open
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
