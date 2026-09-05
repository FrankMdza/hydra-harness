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
