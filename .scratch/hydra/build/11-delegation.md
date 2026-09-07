# 11 - Delegation and the MCP delegate face

Type: build
Status: claimed
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

## What ticket 09 left you, added 2026-09-06

**`delegate(provider, ...)` has two providers to name now, and one of them needs a repo.**
A `cursor-agent` run reads its gate, its MCP config and its skill links out of the worktree,
so `hydra::run` refuses it before a run id exists when the caller is not inside a git repo,
and again when that repo already carries `.cursor/hooks.json` or `.cursor/mcp.json`. Both
refusals are errors rather than records, the way a wrong PATH is. A delegate face that runs
from outside a repo can therefore delegate to `claude` and not to `cursor-agent`, and a
child's worktree is created from the repo the face stands in, not the parent run's checkout.

**`--shared-tree` on `cursor-agent` is `mcp::Checkout::None`**, which makes every admitted
server undeliverable and refuses the run with `mcp_unavailable` when the policy admits one.
The flag itself is still not built; today `Checkout::None` is only ever a run outside a repo.
Harness spec 9 keeps `--shared-tree` off the MCP face, so nothing here reaches you through
`delegate`.

**`hydra::run::refuse` is provider-agnostic and `tests/mcp.rs` drives it with both.** Your
`Refusal::PolicyNarrowing` record is the same three steps whichever provider the child named.

**Per-run data directories do not collide.** Each `cursor-agent` child gets
`~/hydra/spawn/<run-id>/cursor-data/`, so eight children approving servers at once write
eight approval files and none of them touches `~/.cursor/`. The pre-flight's `mcp enable`
and `mcp list` run per child with cwd in that child's worktree.

**The stderr sentences a refused `cursor-agent` child prints are on its record**, as noise
lines, and two of them derive `rate_limited` and `auth_failed`. A parent reading a child's
contract 17 result gets those outcomes rather than `failed`, which is the difference between
a retry loop and a lesson.

## What ticket 10 left you, added 2026-09-06

**`delegate(provider, ...)` names three providers now, and two of the three need a repo.**
`agy` reads its gate, its MCP config and its skill links out of the worktree the same way
`cursor-agent` does, and it is stricter about it: the directory has to arrive as
`--add-dir` or none of the three loads and the provider says nothing about it. So
`run::worktree_preconditions` is one function for both, refusing before a run id exists
when the caller is outside a git repo and again when that repo already carries the
provider's own two files. A delegate face standing outside a repo can delegate to `claude`
alone.

**One departure the spec does not ask for sits between your refusal codes and a spawn.**
An `agy` launch runs a free `agy -p="/hooks"` after writing its hook config and compares
the listing's `source` against the file Hydra wrote. A failure writes a record through
`never_started` and derives `failed`, deliberately not dressed as one of contract 6's four
refusals. If delegation grows a fifth code, that check is a candidate to move under it, and
the reason it is not one today is that nothing in the contract covers "the gate did not
mount".

**Depth and narrowing are provider-blind and nothing here changed that.** The gate is
compiled per provider, so a child on a different binary than its parent gets its own tool
table and its own dialect, and the policy it was launched under is the only thing 7.3
compares. A `standard` parent delegating a `read-only` child to another provider is the
same check it is within one.

**`Provider::Agy` is the third arm of thirteen match sites**, and the compiler lists them
for you if a fourth ever arrives: `binary`, `dialect`, `skill_root`, `user_skill_roots`,
`resolve`, `subscription`, `observe`, `FromStr`, `ChildEnv::build`, `mcp::undeliverable`,
`mcp::write_config`, `run::write_spawn_config` and the argv match in `run::run`.
