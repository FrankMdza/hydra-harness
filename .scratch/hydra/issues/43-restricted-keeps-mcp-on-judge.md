# 43 - `--restricted` no longer drops MCP, so a `judge` run keeps the human's servers

Type: grilling
Status: resolved

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

## Answer

Resolved 2026-09-06 with the human, one round, after the measurement the ticket said needed the
human's config turned out not to. Four decisions and one correction.

**The correction first: the hole is not live.** Build ticket 08 measured `--restricted` keeping a
server passed on `--mcp-config` and inferred that settings-defined servers ride the same path. They
do not. Measured today on 2.1.263 with the free probe (unknown model, no API call, the
`deferred_tools_delta` attachment in the transcript as the signal), a stub user-scope server in a
scratch `HOME`'s `.claude.json` and a stub project-scope server in the cwd's `.mcp.json`:

| Server defined in | no flags | `--restricted` | `--restricted --strict-mcp-config` |
|---|---|---|---|
| `~/.claude.json`, user scope | loads | dropped | dropped |
| `.mcp.json` in the cwd, project scope | loads | dropped | dropped |
| `--mcp-config <file>` | loads | loads | loads |

`--restricted` drops what settings define and keeps what the argv passes. A `judge` run passes
nothing, so it gets an empty set on this version, and the scratch `HOME` made the measurement
without writing into the human's config.

**1. `judge` gains `--strict-mcp-config`, with no `--mcp-config` beside it.** Not to close a hole
but because the vendor's help text ties skipping MCP to `--strict-mcp-config` and not to
`--restricted`, so the argv should carry the flag that names the guarantee rather than rest on how
`--restricted` reads settings this month. 6.7's "which `--restricted` drops anyway" is replaced.

**2. The probe runs under `judge`'s own flags and `mcp.resolved` becomes a measured `[]`.** The
human asked whether an empty answer wastes tokens through the judge. It does not: the probe is free,
the field is on line 0 and no model reads it, and the empty set is the saving the ticket was
worried about losing, since no tool description reaches the judge's context.

**3. `ToolSearch` answers to `skill_invoke`.** The fixture server's tool landed in print mode's
`deferred_tools_delta.addedNames` beside the fifteen deferred built-ins, so MCP tools are deferred
and need `ToolSearch` before a call. `mcp_call` and `provider_native` are both `permissive`-only
today, so nothing observable changes until a policy admits a server without `provider_native`, and
that day it is ticket 39's failure again. Caveat: read off the transcript attachment before any
model turn, which is the same listing the probe already trusts for `mcp.resolved`, not off a model
calling the tool.

**4. The drift note gets one mechanism.** A test that runs the free probe under `judge`'s flags with
stub servers in both settings scopes and asserts the empty set, skipping when `claude` is off the
path. Build ticket work.

**Also re-measured.** 6.7's `--verbose` requirement holds on 2.1.263. `< /dev/null` was re-measured
on 2.1.261 under ticket 16 and was not repeated.

**What moved.** Harness spec 6.5, 6.7 and 7.1; two paragraphs in `IMPLEMENTATION.md`; ticket 39's
decision 5 gets its answer. Build ticket 16 carries the code. No source changed.
