# 11 - Delegation and the MCP delegate face

Type: build
Status: open
Blocked by: 05, 09, 10

## Goal

`--parent`, the depth cap, monotonic narrowing, the result marking, and the narrow MCP
face an orchestrator calls.

## Where the design says it

- Harness spec 9, the full `hydra run` surface and why the MCP face carries four fewer
  arguments.
- 7.3, monotonic narrowing, the partial order over the five, and why an incomparable
  pair is refused rather than resolved.
- 6.1 items 2 and 3, depth and policy containment as pre-flight checks.
- 8, authority per run, why the task string launders, and that the bound is the policy.
- Contract 8.3, a refused spawn is a run record with outcome `gated`.
- Contract 4, `parent_run_id` written by the parent at spawn and never read back.

## Done when

- A child spawns with `--parent`, depth is computed and capped at 2, and a spawn naming
  a wider policy is refused with a run record rather than silently.
- A refused spawn's record carries the requested policy and the parent's, null `pid`,
  and derives to `gated` with `outcome_source: refusal`.
- The child's text comes back inside a boundary whose delimiter is randomised per call,
  picked after the child returned.
- `delegate(provider, task, files?, policy?)` and nothing more on the MCP face.

## Watch for

**A `standard` parent cannot delegate a `curate` child**, because the five are a partial
order rather than a chain. That is an accepted cost, and a curator pass starts at depth 0
from a human.

**Marking is advice.** What Hydra guarantees is that the marker is present. Hydra gates
actions, never beliefs, and the layer that stops a laundered instruction is the one that
never read the text.

## What ticket 08 left you, added 2026-09-06

**Two of contract 6's four refusal codes are written, and the third is yours.**
`hydra::run::refuse` is one function now, shared by 6.1's skill collision and MCP launch checks:
it writes line 0, writes `RunEnd::refused` with your code and remediation, reads the record back
through the normalizer and returns contract 17's projection. `Refusal::PolicyNarrowing` is
already in the enum with no caller. Adding yours is a call rather than a shape.

**A child's MCP set follows its own policy, not its parent's.** `hydra::mcp::plan` takes the
policy name the run was launched under, so a narrower child is handed fewer servers with nothing
extra to wire. Monotonic narrowing over capabilities and admission over policy names are two
different mechanisms that happen to agree here, which is worth knowing before anyone tries to
make one enforce the other.
