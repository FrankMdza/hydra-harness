# 35 - An `unhooked` row in the capability table

Type: build
Status: resolved
Blocked by: 29

## Goal

Map ticket 51 measured three Cursor tools the provider runs without asking the hook:
`ReadLints`, `getMcpToolsToolCall` and `awaitToolCall`. Today the table maps the first to
`file_read` on a field the tool does not send, knows nothing of the other two, and the replay
files every call to one as a `GateLeak` that the two corpus walks skip by name. Give the table
a row kind for a tool the provider runs unhooked, record such a call as the provider's own
decision, and take the skips out.

## Where the design says it

- Map ticket 51, `## Answer`, and the three `## Measured` sections above it.
- `docs/run-record-contract.md` section 7.7 for `permission_decision` and its `decider`
  values; the orchestrating session adds `provider` there if it is missing.
- `src/policy/table.rs`: `tool`, `pathed` and the `Tool` struct; the Cursor table.
- `src/normalize/cursor.rs`, the replay loop that derives decisions and pushes `GateLeak`,
  and the raw-name map near `readLintsToolCall`.
- `tests/normalize.rs` and `tests/outcome.rs`, the two corpus walks and their ReadLints skips.
- `IMPLEMENTATION.md`, the Cursor section, the paragraph "Two tool names this build did not
  know arrived on 2026-09-08".

## The slices

**The row.** A third constructor beside `tool` and `pathed`: `unhooked(name)`, a `Tool` whose
capability is a new `Capability::Unhooked` variant, or a flag on the struct if that reads
better; pick the one that keeps `gate.py`'s compiled config unchanged, since the gate never
sees these calls. Three Cursor rows: `ReadLints`, `getMcpToolsToolCall`, `awaitToolCall`. The
raw-name map yields the last two by their raw names, so the normalizer no longer reports
them as `unknown_tool`.

**The replay.** A call to an unhooked tool derives a `permission_decision` with
`decision: allow`, `decider: provider`, and a reason `hydra/unhooked: <tool> is run by
cursor-agent without asking the hook`. No `GateLeak`, no `Disagreement`, no `Ungated`. The
gate script itself is untouched: if the hook ever does fire for one of these, the gate's
own answer stands and the replay reports a `Disagreement` the way it does today.

**The walks.** Remove the ReadLints skips from `tests/normalize.rs` and `tests/outcome.rs`.
Every record on this machine must then pass both walks with no skip.

**Tests.** `tests/normalize.rs`: a synthetic Cursor record with a `readLintsToolCall` carrying
`paths` and a success result derives the provider decision and no finding; the same for
`awaitToolCall`. `tests/gate.rs`: the compiled Cursor gate config carries no entry for the
three, or the same entry it carried before, so nothing at run time changes.

## Done when

- Both corpus walks pass on this machine with the skips gone.
- `hydra show 01a0800f-1fa4-7e92-8637-f6a4220d5d4e --events` shows three provider decisions and
  no gate leak.
- The orchestrating session rewrites the Cursor paragraph in `IMPLEMENTATION.md` with the
  per-tool risk statement and amends contract 7.7.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Cursor Grok 4.6 high fast, run 01a08052-ccdc under
`standard`: 7.71M tokens, seven files, 232 insertions, four tests, fifty minutes. Every slice
as written: `unhooked(name)`, `Capability::Unhooked` left out of the compiled gate config,
`Decider::Provider`, the raw-name map, the replay rule with a `Disagreement` if the hook ever
fires, the skips gone from both walks, and `hydra show 01a0800f-1fa4 --events` showing three
provider decisions and no finding. The model found one thing the ticket got wrong: with the
skips gone, the outcome walk passes on this machine and the leak walk still fails inside a
worktree on record 01a07efb-de49, an `agy` run written before `scopes` existed, whose shell
commands name `~/hydra/worktrees/`. On master the same walk passes. The record carries
`scopes`, so the cause is not the pre-`scopes` re-derivation ticket 15 warned about, and it is
not measured: every agent run since ticket 31 has reported the same record from inside its
worktree, and the launch text tells them to ignore that one test. Whatever the walk reads
differently from a worktree is a rough edge in its own right and waits for someone to run the
walk from one by hand.
