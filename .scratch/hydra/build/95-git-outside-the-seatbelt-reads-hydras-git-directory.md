# 95 - Git outside the seatbelt reads Hydra's git directory, and reap commits only on its own branch

Type: build
Status: resolved
Blocked by: 89

## What

Map ticket 72, resolved 2026-09-23. Four parts; read the ticket's `## Answer` and
`.scratch/hydra/research/72-panel-codex.md` and `72-panel-opus.md` first.

1. **Git directory from the record.** `worktree::git` (`src/worktree.rs` ~441) and `git_with_index`
   run `git` with `current_dir(checkout)` and nothing else, so git finds its repository through
   `<checkout>/.git`, a file inside the run's worktree that the run can rewrite, and reads its
   configuration from wherever that file points. Hydra runs these calls outside any seatbelt. Every
   such call made in a run's checkout must set `GIT_DIR`, `GIT_COMMON_DIR` and `GIT_WORK_TREE`
   explicitly from what Hydra knows independently of the checkout: the run record's worktree block,
   or the parent repository's git directory and the run id (`.git/worktrees/<run-id>`).
   `src/resume.rs` ~96-98 already reads the git directory from the record. The callers: `hydra reap`
   (`src/reap.rs`, `commit_then_remove` and what it calls, `current_branch`, `main_worktree`), the
   run-end diff stat (`src/run.rs` ~1436, and `worktree::diff_stat` / `seed_tree`), rung two's patch
   read (`read_proposer_patch`, which since build 89 also reads the checkout's `HEAD`, and the
   `seed_tree` it calls, in `src/rungs/evolve.rs`), and rung one's `curate::propose` (its
   `rev-parse HEAD` and `commit_everything`). `commit_pass_branch` already commits in the parent
   repository since build 89 and needs nothing; the first run of this ticket, 01a0cfa7-7c3e,
   stopped on that, which was not a reason to stop. Find any other call in a run's checkout by
   grepping for `git(` and `Command::new("git")`. A cheap belt on top: pass `-c core.hooksPath=/dev/null`
   and `-c core.fsmonitor=false` on those calls.
2. **Reap commits only on its own branch.** In `commit_then_remove` (`src/reap.rs` ~374), when the
   checkout's branch is not the `hydra_branch` argument, refuse before `commit_everything`, leave the
   checkout and the spawn directory, and say which branch it found. Remove the comment at ~414-418
   that treats a human branch as intended. The same for a curate checkout and `rungs/curate/<run-id>`.
3. **The private carve-out narrows.** `own_write_paths` (`src/policy/mod.rs` ~625-642) grants
   `.git/worktrees/<run-id>/` whole. Narrow it to what `git status` and `git add` write there: the
   `index`, `index.lock` and `logs/`. Measure it: `tests/spawn.rs::the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory`
   runs `git add` under the real profile. Deny writes to that directory's `HEAD`, `commondir` and
   `gitdir` at the seatbelt (`src/sandbox.rs` `compose`) and the gate (the same `own_write_paths`
   list reaches `gate.py`). Extend the test to check a write to the private `HEAD` is refused.
4. **The object store.** Measure whether a profile that allows `file-write-create` and denies
   `file-write-unlink` and overwrite on `.git/objects/` still lets `git add` of a new file and of a
   file whose blob already exists work. Record exactly what you measured in the Outcome. If it works,
   use it. If not, keep today's grant and stop there; the session updates `SECURITY.md`.

## Done when

- `tests/cleanup.rs`: a run's checkout whose `.git` file was rewritten to point at a decoy git
  directory inside the worktree, where the decoy configures something git would act on and that
  leaves a marker file if it runs; `hydra reap` commits to the real `hydra/<run-id>` and the marker
  never appears. It fails before part 1. The same shape for the run-end diff stat, in whichever test
  file drives it.
- `tests/cleanup.rs`: a checkout whose branch was switched to another branch of the throwaway repo,
  which then gains a commit of its own; reap refuses, that branch's tip is unchanged and the checkout
  is still there. It fails before part 2.
- `tests/spawn.rs`: the private `HEAD` write is refused, `git add` still works.
- `README.md` "Your branch never moves" is true, and `SECURITY.md` ~52-56 describes the narrowed
  carve-out and the git directory rule.
- One real run on `claude` haiku under `standard` that runs `git add` and then `hydra reap` from
  outside, both still work. Say the run id.

## Notes for the run

Tests take a scratch `Layout` and a throwaway repository, never `Layout::discover()`. Ignore
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`. The seatbelt tests return
early inside a seatbelt (`tests/spawn.rs` ~1509), and you are inside one: say which tests could not
measure in your worktree, and the session runs them outside. Run `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` before you stop.

## Outcome

Built by `codex` under `standard`, run 01a0cfae-6b3a, 7.7M tokens; the first run, 01a0cfa7-7c3e,
stopped on a detail an earlier ticket had already fixed, and the task text now says a stale detail
is not a reason to stop. Then fixed by the session after an Opus review that ran the seatbelt tests
outside a seatbelt: cleanup 38 and spawn 32 passed there.

Parts 1 to 3 as asked: `worktree::CheckoutGit` pins `GIT_DIR`, `GIT_COMMON_DIR` and
`GIT_WORK_TREE` from the record's `scopes.parent_git_dir` or a sidecar saved at launch under the
deny list, on reap, the run-end diff stat, status's seed index, rung two's patch read and rung
one's proposal. Reap and curate refuse a foreign branch before staging. The private carve-out is
the index and its lock.

What the review changed:
- The agent kept `logs/` writable. Neither `git status` nor `git add` needs it, and a run that
  replaced it with a link would have had reap's commit append a reflog line, outside the seatbelt,
  to the link's target; the review measured it. The grant is gone, and reap and curate remove the
  private `logs/` before committing, for checkouts launched earlier.
  `a_link_planted_as_the_private_logs_takes_no_reflog_line_to_its_target` fails without that.
- The decoy tests used `core.fsmonitor` as their marker, which the `-c core.fsmonitor=false` belt
  defeats on its own. The decoy now also defines a clean filter the checkout selects. With the
  three environment variables removed from `CheckoutGit::command`, both decoy tests fail.
- The hooks and fsmonitor belt applied to every git Hydra runs, parent repositories included, so
  `git worktree add` stopped running the human's own post-checkout hook. It is scoped to checkouts.
- A pruned run's reap reads its record from `pruned/`; the missing-metadata error says how to
  recover by hand.

Part 4, the object store, found its premise false. The session measured, outside a seatbelt,
that `file-write-create` alone lets `git add` of new and existing blobs work with a clean
`git fsck` and 23 stray `tmp_obj_*` files, and refuses delete, overwrite, `chmod` and rename of an
existing object. The review then measured that a run can still create a pack mapping an existing
sha to other bytes and git serves it. Creation-only stops deletion, not substitution, so the
grant stays and map ticket 74 holds the question. `SECURITY.md` states it, with the
`~/.gitconfig` residual the review named: `standard` leaves home writable by shell, so a run can
define a filter reap's git will run.

Measured on master: cleanup 39, spawn 32, evolve 26, curate 22, status and outcome green, lib
200; fmt and clippy clean. `tests/normalize.rs`'s corpus walk fails on master before and after
this change, on record 01a0cc56-c15b, an `Edit` to a mistyped `worktrades/` path with no hook
decision; not caused here. Fixed the same day: Claude Code refused the call on its own `stat`
before any hook, a sixth spelling for `src/normalize/claude.rs`'s list, and the Opus panel run
01a0cf89-8b3d added a seventh, a call a safety classifier withheld. The walk is green over every
record on this machine.

The real run: `claude` haiku under `standard`, run 01a0cfcb-4fd1, ran `git add` inside the new
profile and got `A  probe95.txt`; `hydra reap` from outside committed it, removed the checkout and
deleted the branch.
