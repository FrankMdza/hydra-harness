# 14 - Two scope names, the two-kind deny list, and the reap sweep

Type: build
Status: open
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
