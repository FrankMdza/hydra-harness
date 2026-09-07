# 14 - Two scope names, the two-kind deny list, and the reap sweep

Type: build
Status: resolved
Blocked by:

## Goal

Land the code half of map ticket 40, decided on 2026-09-06. Three things: `skill_checkout` becomes
a scope of its own and the policy-name match in `src/paths.rs` goes; the deny list gains a second
kind of entry, so a run can write only inside what Hydra created for it; and `hydra reap` sweeps a
leftover curate checkout the way it sweeps a worktree, commit first.

## Where the design says it

- Map ticket 40's `## Answer`, decisions 4, 5, 6, 9, 10 and 11, and its data-loss matrix.
- Harness spec 5, the full table; 7.2, the `curate` row and the paragraph under it; 7.4, the git
  row and the two paragraphs after the table; 7.5, "Denied outright" and "Carved out of those
  denials, per spawn".
- `docs/ouroboros-spec.md` 3.6 and 9.1.

## Authorized

Frank authorized on 2026-09-06, map ticket 40 questions 4 and 10 and this ticket's follow-up, the
two edits to protected config #1 that `CLAUDE.md` otherwise forbids an agent:

- `policies/curate.json`: the `file_write` entry's scope moves from `skill_store` to
  `skill_checkout`. The `file_read` entry on `skill_store` stays.
- `policies/floor.json`: `git.forbidden` gains `gc` and `prune` as one-element entries, and
  `worktree` in every form but `list`, which needs a special case in `check_git` beside the one
  `clean` has, because the entry shape cannot say "except".

Frank authorized a third line during the build, on 2026-09-06, after slice A measured the
alternative: `policies/curate.json` gains `file_read` on `skill_checkout`. Without it, the ten
`Read` calls a curator makes inside its own checkout re-derive as `out-of-scope` on
`01a07513-baa3-7cf0-b20a-ded11ede6a38`, the one real `curate` record, and the corpus walk in
`tests/normalize.rs` reports ten `GateLeak` findings that never happened. With the line, that same
walk reports zero across all 27 records on this machine. A curator that cannot read what it wrote
cannot propose from it, so the missing line was a gap in the ticket rather than a reason to leave
it closed.

Nothing else under `policies/` moves. `policies/README.md` gains `skill_checkout` in its scope
list. `policy_sha` moves with it; 18 of the 22 records on this machine already report
`StalePolicy` and the one current record joins them.

**Both lines land in the same commit as the code and not before.** `skill_checkout` is not a
`Scope` variant today, so a `curate.json` naming it fails `PolicySet::discover()` and with it every
test and every `hydra run` until the variant exists.

## The slices

**The scope.** `Scope` gains `SkillCheckout` and an `as_str` arm. `Spawn` gains `skill_checkout:
Option<&Path>`, `None` on every policy but `curate`, and `skill_store` goes back to meaning
`~/hydra/skills/` on every spawn. `compile()` emits the entry when it is `Some`. `src/paths.rs`
loses `skill_store_scope`'s match on the string `"curate"` and gains `skill_checkout(run_id)`, a
pure function of `Layout` and the run id. `src/run.rs`, `src/ouroboros/curate.rs` and the three
normalizers follow. `gate.py` needs nothing for this slice: `scopes_for` already looks names up in
a dict.

**The deny list.** `denied_write_paths` grows from seven entries to the list 7.5 names, and
`Spawn` gains `parent_git_dir: Option<&Path>`, which `src/run.rs:132` already has as `repo`. A
second list, `own_write_paths`, carries the run's worktree and its `skill_checkout` when it has
one. `gate.py`'s two checks, the `file_write` path test and `protected_path_named_by`, become
"inside a denied path and not inside an own path". The comments in `src/policy/mod.rs`,
`src/paths.rs` and `gate.py` that say "seven" or "`curate` writes here, outside its own worktree"
all move.

**The reap sweep.** `src/reap.rs` learns `Layout::skill_checkout`. For each checkout present:
commit to `ouroboros/curate/<run-id>`, then `git worktree remove` without `--force`, so a commit
that did not happen stops the removal. `remove_checkout` in `src/ouroboros/curate.rs` is the
existing shape. Reap skips a checkout whose run is live, the same test it applies to a worktree.

**The tests.** Ticket 40's loss matrix, 32 probes per policy, becomes a test in `tests/gate.rs`
rather than a probe that was deleted: every row that read `ALLOW` under `standard` or `permissive`
on 2026-09-06 asserts `protected-path` now, and the own-worktree write that must stay allowed
asserts `allow`. `tests/gate.rs` gains a `curate` gate compiled from a real
`Layout::skill_checkout` path that allows a write in the checkout and refuses one in the store;
the existing `curate_writes_inside_the_skill_store_and_nowhere_else` passes a directory it named
`skills` and never calls the layout, so it passes under either reading. `tests/cleanup.rs` drives
reap against a checkout with an uncommitted `proposal.md` and checks the commit reaches the branch
before the directory goes. `tests/normalize.rs`'s corpus walk must still pass on run
`01a07513-baa3-7cf0-b20a-ded11ede6a38`.

## Done when

- `cargo test` passes, the matrix test included.
- `IMPLEMENTATION.md`'s section "What the gate does about `shell`, and what it still does not"
  says the two-kind list is built and which rows the 2026-09-06 matrix closed. Its two paragraphs
  under "Ouroboros rung one" beginning "Where that scope points" and "The checkout lives at" say
  what the scope is now called, and the "Rough edges" entry pointing at tickets 40 and 42 loses
  the half that was 40's.
- Map ticket 41 can be taken: its answer needs `skill_checkout` to exist.

## Where the evidence is

Map ticket 40, `## Prepared for the human` for the pre-decision measurements and `## Answer` for
the loss matrix. `src/policy/mod.rs`, `denied_write_paths` and `Spawn`. `src/policy/gate.py`,
`evaluate` and `protected_path_named_by`. `src/paths.rs`. `src/reap.rs`. `src/worktree.rs:28`
for how the parent repo is found.

## Outcome

**Built.** `src/policy/mod.rs`: `Scope::SkillCheckout`; `Spawn` carries `layout: &Layout`,
`skill_checkout: Option<&Path>` and `parent_git_dir: Option<&Path>`; `denied_write_paths` grew to
7.5's two-kind list and `own_write_paths` is the new carve-out list, both fed to `gate.py` through
`compile()`. `src/paths.rs`: `skill_store_scope` is gone, `skill_checkout(run_id)` is a pure
function of the layout and the run id, and nothing in the file matches on a policy name any more.
`src/policy/gate.py`: `protected_root` decides both the `Write` check and the shell check off the
same two lists, and `check_git` refuses every `git worktree` form but `list`, plus `gc` and
`prune`. `policies/curate.json` and `policies/floor.json` carry the three authorized lines above.
`src/run.rs`, `src/ouroboros/curate.rs` and the three normalizers derive `skill_checkout` from the
run's policy name and `parent_git_dir` from `git rev-parse --git-common-dir`, through one
`crate::worktree::git_common_dir`, the launch asking from the parent and the read side from the
record's worktree. `ReplaySpawn` in `src/normalize/mod.rs` rebuilds the launch once for all three
adapters, and its no-worktree fallback mirrors the launch's instead of falling back to the record's
`cwd`. `own_write_paths` carves a worktree out only when it is not under `~/hydra/spawn/`, so a run
outside a repo writes nowhere. `src/reap.rs`:
`commit_then_remove` generalizes the worktree's commit-then-remove shape to the curate checkout,
its commit half is `commit_everything` in `src/worktree.rs`, shared with `propose`, `Done` is
`worktree`, `store_checkout` and `spawn_removed` with one `Checkout` each, and `orphan_checkouts` sweeps
`skill-worktrees/` the way `orphan_spawns` sweeps `spawn/`. `tests/gate.rs` gained the two matrix
tests and a real curate-checkout test; `tests/cleanup.rs` gained five reap tests for the checkout;
`tests/spawn.rs` and `tests/curate.rs` followed the new signatures.

**Measured.** `cargo test`, 0 failed throughout, 425 passed at the end: lib 136 to 139, `cleanup`
20 to 25, `gate` 54 to 57, `spawn` 15 to 16, `worktree` 4 to 5, every other binary unchanged
(`curate` 17, `detached` 6, `mcp` 13, `normalize` 61, `outcome` 37, `pty_tee` 2,
`record_contract` 16, `skills` 19, `status` 12). `cargo clippy --all-targets` silent. The matrix is `tests/gate.rs`'s
`a_write_the_matrix_allowed_on_2026_09_06_is_protected_now` and
`a_shell_command_the_matrix_allowed_is_refused_now_unless_it_is_the_runs_own_scratch` now: 13 write probes across five policies plus a shell table for
`standard` and `permissive`, and every row that read `ALLOW` under either policy on 2026-09-06
reads `protected-path` now, except the five rows decision 11 of ticket 40 keeps allowed: `git
checkout -- .`, `git stash drop`, `git clean -f`, `git worktree list`, and `rm -rf` on the run's
own worktree. `policies/curate.json`'s third line, measured both ways: without `file_read` on
`skill_checkout`, `tests/normalize.rs`'s corpus walk reports ten `GateLeak` findings on the one
real `curate` record, `01a07513-baa3-7cf0-b20a-ded11ede6a38`; with it, zero findings across all 27
records on this machine. `policy_sha` moved to
`1a3ca23f6abd163e50feb024c3a73f985442fae650fc520b678bb22e109f4b1f`, and all 27 records on this
machine now read `StalePolicy`.

**What the ticket got wrong.** The two authorized lines were not enough. A curator that cannot
read what it wrote cannot propose from it, and the third line, `file_read` on `skill_checkout`,
was missing from the plan and had to be authorized mid-build. `policy_sha` was predicted to move
18 of 22 records to `StalePolicy`; the corpus grew to 27 records between the prediction and the
build, and all 27 moved. The expected matrix had `capability` for every shell probe under `judge`,
`read-only` and `curate`; the real gate answers `protected-path` instead whenever the command
names a denied path, because the path denial runs before the policy is even consulted, which the
pre-ticket-40 matrix had already shown for `~/hydra/runs/` and the new one now shows everywhere. A
sentence in `IMPLEMENTATION.md` claiming that denying `~/hydra/skills-snapshots/` whole would also
stop `curate` from ever writing a proposal was wrong. The checkout lives at
`~/hydra/skill-worktrees/<run-id>/`, a sibling of the snapshots directory rather than something
inside it, so denying one leaves the other alone.

Two more came out of the review rather than the build. The plan derived the parent's `.git` at read
time from `repo_root` on the record's `cwd`, and that `cwd` is the run's worktree, where
`--show-toplevel` answers with the worktree itself, so the first version replayed every worktree run
with a `.git` the carve-out then owned and the parent's denial never existed at read time. Both
sides ask `git rev-parse --git-common-dir` now, and a reaped worktree leaves the read side without
that path, which is one more key build ticket 15 has to store. And the first version carved
`spawn/<run-id>/no-worktree` out of the denied spawn directory on a run outside a repo, a hair off
7.5's sentence that the run's own spawn directory is not carved out; `own_write_paths` skips a
worktree under `~/hydra/spawn/` now and a test pins it.

Map ticket 42 closed the same evening and build ticket 15 carries its code. This ticket added
`parent_git_dir` to the keys 15 stores and says why there.
