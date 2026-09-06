# 13 - `skill_invoke` and the `skill_snapshot` scope

Type: build
Status: resolved
Blocked by: 09

## Goal

Make a linked skill usable under every policy but `judge`: the `Skill` tool answers to a capability
of its own, and the run's snapshot is a read scope of its own. Map ticket 39 decided both on
2026-09-06 and measured what decides them.

## Where the design says it

- Map ticket 39's `## Answer`, and run `01a07886-a450-7a21-92cf-799b26358a05` for the measurement:
  the body arrives on the `Skill` call with no read the gate sees, and a sibling arrives through
  `Read` on the link path, which the gate resolves into the snapshot.
- Harness spec 6.4, the paragraph "A run may invoke what it was linked and may read it"; 7.1 on
  `skill_invoke`; 7.2's table and the paragraph under it.
- Contract 7.6: `tool_call` carries the capability Hydra's table maps the name to and enumerates
  none, so a new name costs no schema bump. Contract 7.7: re-derivation at read time needs the
  scope recomputable from the record, and `hydra.skills.store_sha` is the field.

## Authorized

Frank authorized on 2026-09-06, map ticket 39 question 6, the one edit to protected config #1
that `CLAUDE.md` otherwise forbids an agent: `policies/read-only.json`, `standard.json`,
`curate.json` and `permissive.json` gain `skill_invoke` and the `file_read` on `skill_snapshot`
entries the answer names, and `policies/README.md` gains the capability and the scope in its lists.
Nothing else under `policies/` moves. `policy_sha` moves with it and 13 of the 17 records on this
machine then read as `StalePolicy` to rung one; accepted.

## Done when

- `Skill` maps to `skill_invoke` in `src/policy/table.rs`; `read-only`, `standard`, `curate` and
  `permissive` grant it and `judge` does not; `Policy::contains` still has `standard` containing
  `read-only`.
- `Scope::SkillSnapshot` exists, resolves per run from the pinned sha, and reaches the compiled
  gate as `file_read` on it for `read-only`, `standard` and `curate`, never on `file_write`.
- `tests/gate.rs`: a `Read` through the link under `standard` allows with `hydra/allow` and a
  resolved path inside the snapshot; a `Write` through the same link is still `hydra/out-of-scope`;
  a symlink out of the worktree that is not the snapshot is still denied, so the existing test
  keeps its name and passes.
- `src/policy/replay.rs` recomputes the scope from `skills.store_sha` and re-derives the same
  decision on the measured run's record.
- One recorded `standard` run on `haiku` with a marker skill reports both sentinels. Run
  `01a07886` is the template with the policy changed; the marker skill is two files and the store
  commits itself at launch.
- `IMPLEMENTATION.md`'s section "A linked skill is unreachable under four of the five policies" is
  rewritten to describe the binary, and `policies/README.md` carries the new row.

## Watch for

**Four `Spawn` literals and eleven `write_spawn_config` callers.** The brief on map ticket 39
counted them: `src/run.rs`, `src/policy/mod.rs` tests, `src/normalize/claude.rs`, `tests/gate.rs`
for the literals; one caller in `src/run.rs` and ten across `tests/spawn.rs` and `tests/curate.rs`.
`inject_skills` already computes the snapshot path and has to return it; the ordering in `run.rs`
is already right, injection before the gate compiles.

**`the_provider_native_tools_belong_to_permissive_alone` asserts `Skill` is denied under
`standard`.** Replace the assertion rather than the test.

**Ticket 40 edits `Scope` and `Spawn` after you.** Leave the seam clean: one arm, one `as_str` row,
one entry in `compile`'s scope map, and nothing that assumes the fourth scope is the last.

**`cursor-agent` and `agy` have no `Skill` tool the specs know of.** `skill_invoke` maps to nothing
in their tables until one appears; the snapshot scope applies to them unchanged, because the gate
resolves paths the same way on all three.

**The store is empty and the 1.8 second listing probe runs on every launch anyway.** Not this
ticket's, but you will see it in every run you make.

## Outcome

Resolved 2026-09-06. Built, and measured on one real run.

**What was built.** `Capability::SkillInvoke` and `Scope::SkillSnapshot` in `src/policy/mod.rs`,
`Skill` moved to the first in `src/policy/table.rs`, and the second reaching the compiled gate as
one more entry in the `scopes` map. `policies/read-only.json`, `standard.json`, `curate.json` and
`permissive.json` gained the capability and the three that scope their reads gained
`file_read` on `skill_snapshot`; `policies/README.md` gained both. Nothing else under `policies/`
moved.

**The scope is `Layout::skill_snapshot_scope`, and both sides call it.** The launch passes the sha
`inject_skills` pinned, contract 7.7's re-evaluation passes the sha it reads off
`hydra.skills.store_sha`, and the two cannot drift because there is one function. That is the shape
`skill_store_scope` already had. `inject_skills` returns the sha rather than the snapshot path the
ticket expected: the path is `layout.snapshot(sha)` either way, and returning the sha is what puts
the derivation on one side of the seam instead of two.

**A run with no sha gets `<spawn dir>/no-snapshot`**, a path nothing creates, for the reason a run
with no repo gets `no-worktree`. Pointing at `skills-snapshots/` would hand every run every other
run's snapshot, and the tests check that.

**Measured.** Run `01a078bc-420f-7b23-8e67-2dfc9f98f859`, `claude` 2.1.263 on `haiku`, `standard`,
with the same two-file marker skill ticket 39 used and new sentinel strings. Both sentinels came
back. The gate answered `hydra/allow: policy standard allows skill_invoke.` on the `Skill` call and
`hydra/allow: policy standard allows file_read.` on a `Read` of the sibling through the link path.
Reading the record back derives both again and reports no gate leak. The probe skill was removed
from the store afterwards, the way ticket 39's was.

**Tests.** `tests/gate.rs` gained six: the invoke under four policies and its `hydra/capability`
refusal under `judge`, the sibling read through the link under `read-only`, `standard` and
`curate`, the write through the same link still `hydra/out-of-scope`, a sibling snapshot directory
still out of scope, a link to somewhere neither protected nor pinned still out of scope, and
`judge` reading no snapshot. `a_symlink_out_of_the_worktree_does_not_widen_the_scope` keeps its name
and passes. `tests/normalize.rs` gained the re-derivation, plus its mirror image: the same record
without the sha derives a deny and reports a gate leak, which is what proves the scope did the
work. `src/policy/mod.rs` gained `standard_contains_read_only` and a check that reads and invokes
travel together and that no policy names the snapshot on `file_write`. 359 pass, none fail.

**`the_provider_native_tools_belong_to_permissive_alone` kept its name**, with `Skill` taken out of
its list and a comment saying where it went.

### What the ticket assumed that turned out otherwise

**18 of 22 records read as `StalePolicy`, not 13 of 17.** The corpus grew by five between map
ticket 39's decision and this build. Three of the 22 carry no `policy_sha` at all, which is what a
harness run writes. Accepted, same as before.

**`write_spawn_config` took an eighth argument rather than a struct.** Eleven callers, one line
each, and `#[allow(clippy::too_many_arguments)]` beside it, which `run_start` in the same file
already carries. A struct is the better shape and it is ticket 40's to make if 40 needs it.

**A curate pass never has a snapshot.** It launches with no worktree, so nothing is injected and
its `skill_snapshot` resolves to `no-snapshot`. The `file_read` entry in `curate.json` is what 7.2
says and what 7.3's containment needs; it does nothing on a curate run today, and it will the day a
curate pass gets a checkout.

**`ToolSearch` is the half still denied under `standard`.** Ticket 43 decides it, unchanged here.

### Found on the way, not decided here

**`shell` reaches the snapshot and no policy stops it.** Under `standard`,
`echo x > <worktree>/.claude/skills/<name>/SKILL.md` is allowed as `shell`, measured on the
compiled gate. `shell` is unscoped by design and the snapshot is not one of 7.5's seven denied
paths, so this was true before this ticket too. What this ticket changed is that the gap now
reads as a contradiction: `skill_snapshot` is kept off every policy's `file_write` on purpose,
and a run can still edit its own instructions through the capability that carries no path.
Snapshots are shared per sha, so the run it corrupts may be a live one.

Closing it is a decision and not an implementation: 7.5 lists seven denied paths and this would
be an eighth, and denying `~/hydra/skills-snapshots/` whole would also close the door a curate
pass may one day need. It belongs on the map beside ticket 40. `IMPLEMENTATION.md` carries it.

### What the review found, and what changed because of it

`/code-review high` on the working tree, four findings worth acting on. All four are fixed here.

**The policy edit manufactured a false `Disagreement`, and rung one was reading it.** Run
`01a07512-d36e-7651-b8d9-e9e279e5730b` is a `standard` run whose `Skill` call the old gate
correctly refused. Today's `standard` allows `skill_invoke`, so the replay says allow, and the
normalizer reported the gate and the derivation contradicting each other. `hydra curate --dry-run`
ranked that run second and put the contradiction above its own `StalePolicy` line, so rung one was
handed a hard signal that was an artifact of this commit. Contract 7.7 calls a disagreement the one
place two independent sources answer **the same question**, and a moved `policy_sha` means they did
not, so `normalize::policy_moved` now downgrades all three disagreement sites to a `Limit` that says
which two policies answered. `StalePolicy` still fires beside it and the signal is gone from the
dry run. This is the first policy edit since records existed, so it is the first time the hole could
fire at all.

**`GateLeak` has the same exposure and is left alone.** A policy edit that *narrows* something would
make the replay deny a call the old gate allowed, which reads as a leak that never happened. Nothing
fires today, because this edit only widened: `no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`
still passes over all 22 records. Suppressing a leak on a moved sha is the unsafe direction of the
same fix, so it is written down rather than done. It belongs beside ticket 42.

**The test fixture wrote `skills: null`, which the contract forbids.** `check_run_start` flags a
null `skills`, `mcp` or `auth` group by name, and `src/run.rs` leaves the group absent for exactly
that reason. Nothing caught it because `tests/record_contract.rs` builds its own records, so every
synthetic record in three suites was a shape the validator would have rejected.
`the_fixture_writes_a_run_start_the_contract_accepts` now runs the fixture's own output through
`validate::check`, with and without a sha.

**A `store_sha` off a record was joined unchecked.** `PathBuf::join` with an absolute path replaces
everything before it, so a record carrying `store_sha: "/"` would compile a replay gate whose
`skill_snapshot` root is `/`, and every `file_read` denial under `read-only`, `standard` and
`curate` would re-derive as an allow, swallowing real `GateLeak` findings. Only a malformed record
reaches it, but contract 7.7 is an audit path over evidence. `skill_snapshot_scope` now requires hex
and falls back to `no-snapshot` otherwise, which fails toward a run that could read nothing.

**The snapshot-write assertion claimed more than it checked.** It looked only for an explicit
`skill_snapshot` on `file_write`, so `permissive` passed while reaching the snapshot through
`anywhere`. The message now says what it tests, and `IMPLEMENTATION.md` carries both write paths,
`shell` and `file_write anywhere`, in one place with the reason they are the same hole.

`docs/status.html` is untracked, unrelated and left out of the commit.
