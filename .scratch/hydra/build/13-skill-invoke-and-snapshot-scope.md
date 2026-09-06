# 13 - `skill_invoke` and the `skill_snapshot` scope

Type: build
Status: open
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
