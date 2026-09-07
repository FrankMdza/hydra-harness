# 16 - `judge` gets `--strict-mcp-config`, the probe follows, and `ToolSearch` moves

Type: build
Status: open
Blocked by:

## Goal

Land the code half of map ticket 43, decided on 2026-09-06. Four things: the `judge` argv gains
`--strict-mcp-config` with no `--mcp-config` beside it; the pre-flight probe carries `judge`'s own
flags so `mcp.resolved` is a measured `[]` on that policy; `ToolSearch` answers to `skill_invoke`
rather than `provider_native`; and one test runs the free probe under `judge`'s flags and asserts
the empty set, skipping when `claude` is not on the path.

## Where the design says it

- Map ticket 43's `## Answer`, the matrix and decisions 1 to 4.
- Harness spec 6.5, the `claude` row and the residue paragraph; 6.7, the `--restricted` paragraph
  and the `judge` argv block; 7.1, the `ToolSearch` sentence.

## The slices

**The argv.** `src/provider/claude.rs` pushes `--strict-mcp-config` on `judge`. `mcp_undeliverable`
still answers on the policy name and its reason string moves from "`--restricted` drops it" to
"`judge` passes no config and `--strict-mcp-config` beside it".

**The probe.** `resolve` in `src/provider/claude.rs` takes the run's flags rather than deciding
them from whether a config file exists: on `judge`, `--restricted --strict-mcp-config` and no
`--mcp-config`. `mcp.resolved` on a `judge` record becomes `[]`. Contract 5.3's "identical on a
recorded `claude` run" then holds for five policies and its sentence can say so.

**`ToolSearch`.** `src/policy/table.rs` moves the row to `Capability::SkillInvoke`. No policy file
changes, because every policy that grants `skill_invoke` already does. `tests/gate.rs` gains the
row: `ToolSearch` allowed under `read-only`, `standard` and `curate`, denied under `judge`.

**The drift test.** One test in `tests/spawn.rs` or a new `tests/drift.rs`: skip unless `claude`
is on the path, run the probe with `judge`'s flags in a directory holding a stub `.mcp.json` and
under a scratch `HOME` holding a stub user-scope server, and assert no `mcp__` name in the
transcript's `deferred_tools_delta`. Measured shape is in ticket 43's answer. Record the version the
probe reported in the assertion message so the day it flips the failure says which build.

## Done when

- A `judge` spawn's argv carries the flag and the contract test reads `mcp.resolved: []` off a
  record written through a stub.
- `ToolSearch` derives allow under `standard` in `tests/gate.rs`.
- The drift test passes on this machine and skips on one without `claude`.
- `IMPLEMENTATION.md`'s "`--restricted` no longer drops MCP" section and its `ToolSearch` paragraph
  say built rather than pending.
