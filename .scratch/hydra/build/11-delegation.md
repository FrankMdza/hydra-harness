# 11 - Delegation and the MCP delegate face

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-07. Two slices, both green with no provider launched.

**Slice 1, lineage and the two pre-flight checks.** `Request` gains `parent`, `hydra run`
gains `--parent <run-id>`, and `run::Lineage` is the seam: `Lineage::of` reads `depth` and
`policy` off line 0 of the parent's record through a new `live::opening_line`, the twin of
`closing_line`, and `Lineage::narrowing` compares the two policies through the
`Policy::contains` that was already there. Both run ahead of the environment build, the
provider version read and the two repo-file checks, so an attempted escalation is recorded
even when the child was never going to start, and the record carries no provider fields:
contract 8.3's phrase, a null `provider_version` and an absent `env_allowlist`. Three shapes are refused before a run id exists,
all of them typed errors and none of them a record: a parent with no record, a parent whose
`policy` is null, and a child that would land past `live::MAX_DEPTH`, which is 2 beside
`LIVE_RUN_CAP`. The narrowing refusal runs after the mint and before the worktree, so it
writes `run_start`, `run_end` and nothing else on disk, through the same `run::refuse`
tickets 07 and 08 built. `parent_run_id` now travels through the launch spec into
`hydra watch` and through `refuse`, so it is on every line of a child's file.

**Slice 2, the marking and the face.** `hydra delegate-face [--parent <run-id>]` is a
hidden subcommand and a hand-written stdio JSON-RPC server in `src/face.rs`: `initialize`
answers the version the client sent, `notifications/initialized` is ignored, `tools/list`
returns one tool, `tools/call` runs it, and anything else is `-32601`. No dependency added.
The tool is `delegate(provider, task, files?, policy?)` and nothing else; an argument the
face does not take is refused by name. `face::mark` wraps the child's text in
`<<<hydra-result TOKEN>>>` and `<<<end hydra-result TOKEN>>>`, the token being a UUIDv7
with its six clock bytes dropped, 20 hex characters, drawn after the child returned.
`hydra run`'s own JSON is unchanged.

**Measured.** 19 new tests: 7 in `tests/delegation.rs`, 7 in `tests/face.rs` and 5 unit
tests in `src/face.rs`. 444 pass and 0 fail on the commit this was written against, which
had 425; 451 pass after rebasing onto build ticket 16, which brought 432 of its own.
`cargo clippy --all-targets` is clean. Nothing launches or even asks for a provider:
`tests/face.rs` drives the built binary over a pipe, and its end-to-end test comes back
`outcome: gated` with the record on disk on a machine where no agent CLI is installed.

**What the ticket assumed that turned out false or unstated.**

- **Contract 5.1 had no field for the parent's policy and 8.3 requires one.** `parent_policy`
  is new on `run_start`, absent on a run with no parent, and `docs/run-record-contract.md`
  section 5.1's lineage table gains the row with a pointer to 8.3. That is the one doc edit
  this ticket made.
- **The depth cap is a constant, not configuration.** Harness spec 9 says "capped at 2,
  configurable" and names no mechanism. A flag would let a child raise its own cap and a
  protected config for one integer would be a sixth of those, so it is `live::MAX_DEPTH` and
  moving it means a rebuild. `IMPLEMENTATION.md` says so.
- **The version read had been hoisted above every check, and that made an escalation
  invisible.** `run::run` read `provider.version()` and built the child's environment before
  anything about the request was judged, so a child naming a wider policy on a provider that
  is not installed errored out with no record. 6.1 lists neither among its seven checks, so
  the order was an accident rather than a reading. Items 2 and 3 now run first, which is
  what makes contract 8.3's "no provider fields" true of the record and what lets the face
  test pass with no agent CLI on the machine.
- **The face needed a way to point the binary at a throwaway `~/hydra`.** Every other test
  builds a `Layout::at` a temp directory; this one drives the binary as a subprocess.
  `HYDRA_ROOT` overrides `Layout::discover`, is read in `src/paths.rs` and nowhere else, and
  never reaches a child provider because 6.2's allowlist does not carry the name. What is
  left of it is in `IMPLEMENTATION.md`'s rough edges: a run that can shell out can export
  the variable itself and record its own children somewhere the corpus does not look, which
  is 7.5's "the gate cannot keep a run away from a binary reached through `shell`" in a new
  spelling.
- **Nothing injects `--parent <run-id>` into the face's own arguments per run.** The MCP
  document is copied into a run's config as written, so a face spawned from it has no
  `--parent` and its children are depth 0. Closing that means a per-server substitution rule
  the document format has no field for, which is a document change and a code change
  together. It is written up in `IMPLEMENTATION.md` and left.
- **`files` is a decision.** Harness spec 9 names it and defines it nowhere, so the paths are
  rendered under one heading at the end of the task string. They grant nothing.
- **A `judge` parent can delegate nothing but `judge`**, which the ticket did not say but
  `policy::contains` already made true, and it is worth knowing before someone gives a
  panel member a helper.
