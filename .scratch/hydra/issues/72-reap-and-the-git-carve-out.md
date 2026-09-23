# 72 - Which branch `hydra reap` may commit on, and what the git carve-out lets a run write

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-23, by the session, from finding G41 of the second hiring panel (`codex`).

## Question

Build ticket 70 carved two paths out of the denial on the parent's `.git` so that `git add` works
inside a run: the checkout's private git directory, `.git/worktrees/<run-id>/`, whole, and the
shared object store, `.git/objects/`. `src/policy/mod.rs:625-642` adds both to `own_write_paths`,
and `src/sandbox.rs:106-112` turns each into `allow file-write* (subpath ...)`. The gate applies the
same carve-out. `file-write*` covers unlink and overwrite, not only creation.

Two consequences, both read from the code and neither reproduced yet:

- The private directory holds the checkout's own `HEAD`. A run can point it at one of the human's
  branches without naming a ref path. `hydra reap` then runs outside any seatbelt, reads that
  branch name at `src/reap.rs:385`, commits the run's whole tree on top of it at `411`, and only
  afterwards, at `419-424`, says it is "not the branch Hydra made". The human's branch moves, and
  a commit whose tree is the run's can undo work that landed on the branch after the run started.
  The comment at `reap.rs:414-418` treats committing on a human branch as intended.
- A run can delete or replace pack files and loose objects in the human's repository. Content
  addressing stops a run from forging an object under a sha, not from removing one.

`README.md:25-27` says "Your branch never moves". `SECURITY.md:52-56` says `refs/` and `HEAD` stay
denied, which is true of the main `HEAD` and not of the checkout's private one; the test,
`tests/spawn.rs::the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory`, checks only
the main `.git/HEAD`.

The options:

1. **Reap commits only on `hydra/<run-id>`.** Refuse before `commit_everything` when the branch is
   anything else, leave the checkout, and say so. Deny the private `HEAD` after the carve-out, at the
   seatbelt and the gate. Narrow the object store to creation if a measured `git add` under a real
   profile allows it. The README sentence stays true.
2. **A human branch is allowed, and the documents say so.** Keep reap as it is and rewrite
   `README.md:25-27` and `SECURITY.md:52-56`.
3. **Reap refuses off its branch; the carve-out stays.** Closes the damage to the human's branch and
   leaves object deletion open, stated in `SECURITY.md`.

The human asked for a panel of four readers before deciding (`.scratch/hydra/research/72-panel-*.md`).

## Answer

Resolved 2026-09-23 with the human, after a panel of four readers
(`.scratch/hydra/research/72-panel-*.md`). All four chose option 1. Opus found a larger hole the
question did not ask about: Hydra runs `git` outside any seatbelt in a checkout whose git metadata
the run can write, and git reads its configuration from there. That happens in `hydra reap`, in the
diff stat at the end of a run, and in rung two's patch read (`worktree::git` runs with
`current_dir(checkout)` and nothing else, `src/worktree.rs:441`). Git's own behaviour was measured
in a scratch repository, not through Hydra's code paths. A safety classifier stopped Opus's written
answer, so its file holds its closing report only.

The human chose option 1 with Opus's addition:

1. **Reap commits only on the branch Hydra made.** `commit_then_remove` refuses before
   `commit_everything` when the checkout's branch is not `hydra/<run-id>`, or
   `rungs/curate/<run-id>` for a curate checkout, and leaves the checkout in place. The comment
   treating a human branch as intended goes.
2. **Every git call Hydra makes outside the seatbelt in a run's checkout takes its git directory
   from what Hydra recorded, not from the checkout.** `GIT_DIR`, `GIT_COMMON_DIR` and
   `GIT_WORK_TREE` come from the run record or the layout, never from `<checkout>/.git`, in reap, the
   run-end diff stat, rung two's patch read and `commit_pass_branch`. `src/resume.rs:96-98` already
   reads the git directory from the record.
3. **The private git directory's carve-out narrows to what `git add` needs**: the index, its lock and
   `logs/`. `HEAD`, `commondir`, `gitdir` and the rest are denied, at the seatbelt and the gate.
4. **The object store.** Measure whether a profile that allows creation and denies unlink and
   overwrite still lets `git add` work. If it does, use it. If not, the grant stays and `SECURITY.md`
   says a run can delete objects in the parent repository.

`README.md`'s "Your branch never moves" stays true. `codex` asked for the object store to fail
closed; the human kept `git add` inside runs instead. Build ticket 95.
