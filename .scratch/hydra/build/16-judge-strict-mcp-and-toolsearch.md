# 16 - `judge` gets `--strict-mcp-config`, the probe follows, and `ToolSearch` moves

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-06. All four slices built, plus one the ticket did not see coming.

**The argv.** `claude::mcp_flags` is the one reading of what a launch shape does about
`--restricted`, `--mcp-config` and `--strict-mcp-config`, and `claude::argv` and the new
`claude::probe_argv` both write their command line from it. So `judge` carries
`--restricted --strict-mcp-config` and no file on the launch and on the probe, and the two
cannot drift apart without a test failing. `mcp_undeliverable` still answers on the policy
name and its reason string now names the argv rather than the flag.

**The probe.** `crate::mcp::Probe` was `Strict(&Path)` or `Unanswerable` and is now the policy
name plus the optional file, which is what the flags come from. `judge` stopped being the
unanswerable shape. `McpFlags::answers_for_servers()` is the strict flag, a `judge` probe
carries it, and `mcp.resolved` on a `judge` record is a measured `[]`. `tests/drift.rs` is
what proves the producer writes it; `tests/mcp.rs` checks the argv Hydra hands the probe and,
separately, that an empty array survives a record round trip as an empty array.

**`ToolSearch`.** One row moved to `Capability::SkillInvoke`. No policy file changed and
`policy_sha` did not move. `tests/gate.rs` compiles the real gate and gets allow under
`read-only`, `standard`, `curate` and `permissive`, and `hydra/capability` under `judge`.

**The drift test.** `tests/drift.rs`, two tests, the only two in the suite that launch a
provider. Both are free: the probe passes a model no catalog holds, so the binary refuses
before it calls the API and still writes the transcript. The first drives the launch's own
producer, `claude::resolve_in`, with a `judge` `Probe` and asserts the `Resolved` it hands
back carries `mcp: Some([])` and `skills: None`. The second is the control, the same stubs
with the flags off. Every `claude` call runs under a scratch `HOME` and a scratch cwd, the
version check included, so the human's own `~/.claude.json` is neither read nor written, and
a passing test deletes both directories. They skip with a printed reason when `claude` is off
the PATH, checked by running the compiled binary with a stripped PATH.

`claude::resolve_in` is `resolve` with the home and the environment named rather than
discovered, and that test is the reason it exists. `ChildEnv::build` drops a `HOME` that
disagrees with the passwd entry, which is the right rule for a launch and the wrong one for
measuring the binary.

### What was measured, `claude` 2.1.263, 2026-09-06

Probe under `--restricted --strict-mcp-config`, one stub server in each settings scope: no
`mcp__` name in `deferred_tools_delta.addedNames`, so `mcp.resolved` is `[]`. The same stubs
with no flags: `mcp__hydra-user-scope__ping` and `mcp__hydra-project-scope__ping` both there.
Ticket 43's matrix holds on this build. The two tests run in parallel and the binary finishes
in 0.7s to 1.3s across five runs.

### What the ticket assumed that turned out false

**A restricted probe answers nothing about skills either, and that would have refused every
`judge` run.** Measured the same evening: a scratch `HOME` with one user-scope skill and a cwd
with one project-scope skill gave a `skill_listing` of 14 names with no flags and the binary's
own 12 under `--restricted`. Filesystem-discovered skills are dropped along with the servers.
The launch-time collision check calls a linked name the provider did not list a collision, so
the probe's new flags would have refused every `judge` run on a machine with a non-empty store,
with a remediation telling the human to rename a skill. `claude::resolve` now answers `None`
for `skills.resolved` when the probe ran restricted, which is contract 5.3's own stance on an
unanswered listing, and prints one line on stderr saying so, because a failed probe already
prints and the two nulls would otherwise read the same on the way past. **Underneath it is a
real finding no field carries: a `judge` run reads none of the skills Hydra linked into its
checkout.** Nothing is lost today, since `judge` grants no `skill_invoke`, and whether that
policy should be linked skills at all is a decision for the map rather than for this commit.

**The transcript path needed the cwd canonicalized.** Claude Code slugs the path it resolved:
a probe run in `/tmp/x` on macOS writes to `~/.claude/projects/-private-tmp-x`. A run's own
worktree crosses no link so nothing changed there, and without it the drift test could not find
its own transcript.

### Three sentences in `docs/`, authorized after review

Harness spec 7.1's "one of the twenty tools that answer to `provider_native`" is nineteen.
Contract 5.3's "identical on a recorded `claude` run" now says it holds for all five policies,
since `judge` passes the strict flag with no file and both fields are empty and measured.
Contract 5.3 scoped an absent `skills.resolved` to `cursor-agent`, and a `claude` `judge` run
produces one too, for the restricted-probe reason above. One clause each and nothing else in
`docs/` moved.

### Counts

`cargo test` 425 passed before, 432 after, 0 failed, and `cargo clippy --all-targets` is clean.
Seven tests added: three in `src/provider/claude.rs`, one in `tests/gate.rs`, one in
`tests/mcp.rs`, two in `tests/drift.rs`. `CLAUDE.md` still says `tests/` holds thirteen files
and it now holds fourteen, left for whoever merges the three parallel tickets.

A two-axis review landed after the first commit. What it changed: the drift test drives
`claude::resolve_in` rather than asserting off a hand-written record, and its version check and
scratch cleanup moved under the scratch `HOME`; `tests/mcp.rs`'s two tests say which half they
check; `claims_the_set` became `answers_for_servers`; the restricted path prints on stderr; the
three `docs/` clauses above went in; and the drift timing here is measured rather than carried
over from a run that included a compile.
