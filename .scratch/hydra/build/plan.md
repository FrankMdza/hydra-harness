# Hydra build

Label: build:index

The companion to `../map.md`. That map charted the design and closed at 38 of 38; its
destination was three documents and D1 forbade production code while it was open. It
reached that destination on 2026-09-05 and the code started after. **This file indexes
the build, and nothing in it reopens a design decision.** A ticket here that finds
itself needing a decision rather than an implementation has found the edge of the
design, and it should say so and stop rather than decide.

## Destination

`docs/` implemented: one router that launches all three providers on a gate it
compiled, records them per the contract, and hands Ouroboros a corpus worth reading.

## Notes

**Read in this order.** `docs/` for what to build, `IMPLEMENTATION.md` for what is
already built and every departure from the design with its reason, then this file,
then your ticket. Amend `IMPLEMENTATION.md` in the same change that narrows the gap;
a ticket is not done while that file still claims the gap is open.

**Re-read all three provider versions at the top of every session** and date every
provider-specific claim you write. Two of the three self-update without asking, `agy`
moved four times in four days, and `claude` moved between two tickets on this map's
predecessor. A finding has been invalidated by drift four times so far.

**Measure, do not read.** Ticket 34 on the design map verified a field by reading the
ternary that gated it, and ticket 35 found the field empty every time a human used it.
A field's presence in a conditional says the branch exists, not that anything writes
it.

**Prose follows `~/.claude/skills/unslop/SKILL.md`.** No em dashes.

## The tickets

- [01 - The policy set, the capability table, and the gate compiler](01-policy-set-and-gate-compiler.md)
- [02 - Wire the gate into a recorded `claude` run](02-gate-into-the-run.md)
- [03 - Normalize a `claude` run into the ten event types](03-normalizer-event-types.md)
- [04 - Derive the outcome and complete what `hydra run` returns](04-outcome-derivation.md)
- [05 - Detached spawn, the timeout, and `hydra status`](05-detached-spawn-and-status.md)
- [06 - `hydra reap` and `hydra prune`](06-reap-and-prune.md)
- [07 - The skill store, the snapshot, and per-run injection](07-skill-store-and-injection.md)
- [08 - MCP injection on `claude`](08-mcp-injection-claude.md)
- [09 - Route `cursor-agent`](09-route-cursor-agent.md)
- [10 - Route `agy`](10-route-agy.md)
- [11 - Delegation and the MCP delegate face](11-delegation.md)
- [12 - Ouroboros rung one](12-ouroboros-rung-one.md)
- [13 - `skill_invoke` and the `skill_snapshot` scope](13-skill-invoke-and-snapshot-scope.md)
- [14 - Two scope names, the two-kind deny list, and the reap sweep](14-two-scope-names-and-the-deny-list.md)
- [15 - `scopes` and `floor_lifted` on the record](15-scopes-and-floor-lifted-on-the-record.md)
- [16 - `judge` gets `--strict-mcp-config`, the probe follows, and `ToolSearch` moves](16-judge-strict-mcp-and-toolsearch.md)
- [17 - `claude` runs record their thinking text again](17-thinking-text-on-claude-records.md)
- [18 - `hydra show <run-id> [--events]`](18-hydra-show.md)
- [19 - The Herdr bridge, as a plugin on Herdr's side](19-herdr-bridge-plugin.md)
- [20 - A run's sequence as a graph in the terminal](20-hydra-graph-in-the-terminal.md)
- [21 - A smoke test for the bridge, against a running Herdr](21-bridge-smoke-test.md)
- [22 - Join a gate answer to its call when the provider pipelines calls](22-join-gate-answers-under-pipelined-calls.md)
- [23 - `base_tree` on the record](23-base-tree-on-the-record.md)
- [24 - The pane says what the record knows](24-the-pane-says-what-the-record-knows.md)
- [25 - No skill injection on `judge`](25-no-skills-on-judge.md)
- [26 - The face gets its parent, and the document admits it](26-the-face-gets-its-parent.md)
- [27 - `hydra run --interactive` on `claude`, and the harvest](27-interactive-claude-and-harvest.md)
- [28 - `hydra run --interactive --policy <name>` on `claude`](28-interactive-with-a-policy.md)

## Out of scope

Everything `docs/harness-spec.md` section 18 lists as a non-goal, unchanged: the TUI,
the raw-model plane beyond one gated `gemini` path, hermes, multi-machine use,
Langfuse, training, Cursor GUI automation, reading the user's credential files,
interpreting shell commands, ACP, and a mid-run input channel.

Ouroboros rung two is out of scope for this file until rung one has run. `docs/`
specifies it and nothing stops someone building it; it gets a ticket when the corpus
exists to justify one.
