# 79 - Should an unhooked row keep the tool's real capability?

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-24, by the session, from the fourth panel's `claude` reader (weakness 3 and "Before publishing" item 5 in `docs/reviews/2026-09-24-fourth-panel-claude.md`) on build ticket 113.

## Question

Map ticket 51 made a tool Cursor runs without asking the hook an `unhooked(name)` row:
`Capability::Unhooked` replaces the tool's real capability, the compiler leaves the row out of
the gate, and the Cursor reader files every such call as `Allow`, decided by the provider, with
no finding under any policy (`src/normalize/cursor.rs` near 535 to 561). Three rows were
harmless that way: `ReadLints`, `getMcpToolsToolCall`, `awaitToolCall` read the run's own data.

Build ticket 113 added a fourth the same way, `WebSearch`, after `composer-2.5` searched the web
on a `standard` run and the corpus walk filed the unknown name as a leak. The reader's point:
`standard` allows `network_read`, so that run was inside policy, but a Cursor run under `judge`
or `read-only`, which grant no network, can now search the web and the record shows an
ordinary allowed call. The row turned the walk green by renaming the leak.

The alternative is a row that keeps the real capability and carries unhooked as a flag: the
compiler still leaves it out of the gate (the hook never fires, nothing to compile), but the
replay evaluates the policy against the capability and, where the policy denies it, files a
finding that names the provider as the one who ran it. On `standard` the four rows read as they
do today. Under `judge` a web search becomes a finding, which is what the walk is for.

What it costs: `Capability` stops being one enum value per row (`src/policy/table.rs`, the
`Tool` struct near line 60), the Cursor reader's unhooked arm changes, the three existing rows
need a capability each (`ReadLints` is a file read of the run's own data; the other two are
provider-native), and the fixture corpus gets a `judge`-run twin. One or two build tickets.

Options:

1. Keep 51 as it is and document the gap in `SECURITY.md`: an unhooked Cursor tool is outside
   every policy, and `judge` on Cursor is weaker than `judge` on `claude` by exactly the list of
   unhooked rows.
2. Change the row shape as above, so the replay judges unhooked calls by capability.

The session's recommendation is 2: it is the smaller of the two in prose and it is what the
walk already claims to check.

## Answer

Resolved 2026-09-24 by the human, option 2.

An unhooked row keeps the tool's real capability, and "unhooked" becomes a flag on the row
instead of a capability. The compiler leaves a flagged row out of the gate, as it does today,
because the hook never fires for it. The replay evaluates the policy against the row's real
capability. Where the policy allows it, the call reads as it does now, allowed and decided by
the provider. Where the policy denies it, the replay files a finding that names the provider as
the one who ran the tool. Under `standard` all four Cursor rows read as they do today. Under
`judge` or `read-only` a web search becomes a finding.

This covers every unhooked row, not only Cursor's. `CODEX` in `src/policy/table.rs` has
`unhooked("WebSearch")` too, and the same reasoning applies to it.

The three older Cursor rows each need a capability. `ReadLints` reads the run's own data.
`getMcpToolsToolCall` and `awaitToolCall` are provider-native. The build ticket checks each
against its measured record before committing to that.

Build ticket 124.
