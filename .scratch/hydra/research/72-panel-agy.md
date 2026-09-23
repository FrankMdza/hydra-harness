# Panel 72, agy

## 1. Recommendation
Option 1. Reap commits only on `hydra/<run-id>`. This secures the human's branch and, by narrowing the object store to creation, stops a run from deleting objects.

## 2. Mechanism
In `src/reap.rs`, move the branch check from lines 419-424 to line 409, before `commit_everything` at 410. If `branch != hydra_branch`, return early without committing. Line count: ~6 lines moved.
In `src/sandbox.rs` around line 112, and `src/policy/mod.rs` at `own_write_paths` (lines 625-642), change the carve-out so the objects directory gets `file-write-create` instead of `file-write*`. This requires splitting `own_write_paths` into two functions or returning a tuple of path and permission.
The test `tests/spawn.rs::the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory` proves the carve-out. It would fail if `git add` requires unlink and the profile denies it.

## 3. What it breaks or costs
A legitimate run can no longer commit its work if it pointed `HEAD` at a human's branch. It must be measured whether `git add` works under a profile that grants object creation and denies unlink; `git add` writes temporary files inside `.git/objects/` and renames them, which might require unlink. If `git add` fails, the object carve-out must remain `file-write*`.

## 4. What a hostile run or proposer still gains
After this is built, a hostile run can still bloat the object store by creating garbage objects, because creation remains allowed. It can still alter its own `.git/worktrees/<run-id>/HEAD` to read a different commit, though `reap` will refuse to commit on top of it.

## 5. The strongest argument against your choice
Option 1 introduces complexity in the seatbelt by splitting write permissions, and if `git add` relies on unlinking temporary files inside `objects/`, narrowing to creation will break `git add` entirely for all runs. In that case, we must fall back to Option 3 anyway, making the effort on the seatbelt wasted.

## 6. The other options
Option 2: Rewriting the documents to match the broken code makes the security promises weaker and accepts the risk of losing human work.
Option 3: Simpler than Option 1, but leaves object deletion open, which means a run could still corrupt the human's repository by deleting loose objects or packfiles.

## 7. Something the ticket missed
The ticket claims `file-write*` covers unlink and overwrite, which is true, but it misses that denying the private `HEAD` after the carve-out in the seatbelt might break `git checkout` entirely inside the run, as it needs to update that `HEAD` file.
