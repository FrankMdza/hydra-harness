# 43 - `--restricted` no longer drops MCP, so a `judge` run keeps the human's servers

Type: grilling
Status: open

## Question

Build ticket 08 wired MCP injection on `claude` and measured one of the design's premises
out from under it. Harness spec 6.7 leaves both MCP flags off the `judge` argv with the
words "No `--mcp-config`, which `--restricted` drops anyway", and 6.5 says `claude` closes
ticket 27's inherited-server residue. Both rest on ticket 23's measurement of 2.1.257.

**On 2.1.263, `--restricted` keeps MCP.** Measured 2026-09-06 with the free pre-flight
probe, no API call: `claude --restricted --mcp-config <file>` resolved the server named in
the file, and 2.1.263's own `--help` now says "add --strict-mcp-config to skip MCP servers
too". So a `judge` run, which carries neither flag, starts whatever servers the human has in
`~/.claude.json`, and their tool descriptions reach the judge's context. The gate refuses
every `mcp__` call under `judge`, so nothing executes; what is left is exactly the residue
6.5 says this provider is free of.

**Does the `judge` argv gain `--strict-mcp-config`, with no `--mcp-config` beside it?** That
is one flag and it would give `judge` the empty server set the other four policies get
through a file. Ticket 08 did not take it, because 6.7 writes that argv and the rule is that
a build does not edit the design.

The inheritance itself was not observed: nobody has a user-scope server configured on this
machine, and writing one into the human's own config is theirs to do. What was observed is
`--restricted` keeping a server passed on `--mcp-config`, which is the same code path.

## Settle

- **Whether `judge` gets `--strict-mcp-config`.** If yes, 6.7's `judge` block changes by
  one line and its justification sentence goes; if no, 6.5's claim that `claude` closes the
  residue needs a `judge` exception written into it.
- **What `mcp.resolved` becomes on a `judge` run.** Today it is null, because the probe is
  not `--restricted` and cannot reproduce the run's shape. With the flag on both, the probe
  could answer, and 5.3's "identical on a recorded `claude` run" would hold for all five
  policies rather than four.
- **Whether the measurement gets finished.** One user-scope server in `~/.claude.json`, one
  `judge` run, and the init frame's `mcp_servers` says whether the residue is real on this
  version. About a minute, and it needs the human's config.
- **What else in 6.5 and 6.7 rests on 2.1.257.** Ticket 08 found this one by accident. The
  build plan's note about drift says re-read at the top of every session, and this is the
  first flag claim to expire under it.

## What it costs to leave open

Nothing runs `judge` today: rung two's panel is not built. The day it is, three judges
share a subscription and one of them loads the human's own servers into its context before
weighing a patch. Small until it is not.

## Where the evidence is

`IMPLEMENTATION.md`, the section "`--restricted` no longer drops MCP, so a `judge` run
inherits the human's servers". The module comment at the top of `src/provider/claude.rs`,
and `claude::mcp_undeliverable`, which answers on the policy name rather than on what the
flag does, for this reason. Build ticket 08's `## Outcome`. Design ticket 23's `## Answer`
for the original measurement.

## What ticket 39 left you, added 2026-09-06

**Measure whether MCP tools arrive deferred, while you are measuring this.** Ticket 39 left
`ToolSearch` in `provider_native`, which only `permissive` grants. In an interactive 2.1.263 session
the MCP tools load deferred and need `ToolSearch` before they can be called. If print mode does the
same on a run with `--mcp-config`, an injected server's tools are unreachable under every policy but
`permissive`, whatever the `init` frame lists, and `ToolSearch` moves to `skill_invoke`'s side for
the same reason `Skill` did. One run, and look at the `init` frame's `tools` for `mcp__` names.
