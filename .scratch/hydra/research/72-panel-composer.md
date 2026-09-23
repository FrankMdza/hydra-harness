# Panel 72, composer

## 1. Recommendation

**Option 1.** Reap must commit only on the branch Hydra created (`hydra/<run-id>` or `rungs/curate/<run-id>`), refuse earlier when the checkout is on anything else, and close the private `HEAD` hole the carve-out opened. `README.md:25-27` promises the human's branch never moves; the code at `reap.rs:410-423` can violate that today without any ref write under the main `.git/HEAD`.

## 2. Mechanism

**`src/reap.rs` (~25 lines).** In `commit_then_remove` (`reap.rs:374-434`), after `current_branch` (`reap.rs:386-392`) and before `commit_everything` (`reap.rs:410`), bail when `branch != hydra_branch`. Remove or replace the branch at `reap.rs:415-423` that treats a human branch as success. `one` already passes `format!("hydra/{run_id}")` (`reap.rs:207-208`) and `format!("rungs/curate/{run_id}")` (`reap.rs:228-229`).

**`src/policy/mod.rs` (~15 lines).** In `own_write_paths` (`mod.rs:618-648`), stop granting the whole `git_dir.join("worktrees").join(run_id)` tree (`mod.rs:635`). Carve out only `index`, `index.lock`, and `logs/` under the private directory, not `HEAD` or `gitdir` files a run could use to repoint the checkout. Keep `git_dir.join("objects")` (`mod.rs:642`) only until a measured profile proves create-without-unlink works; ticket 70's outcome (`70-git-index-lock-under-the-seatbelt.md:46-52`) shows `git add` needs the object store.

**`src/sandbox.rs` (~10 lines).** `compose` (`sandbox.rs:78-137`) mirrors `own_write_paths` at `sandbox.rs:106-112`; any narrowing there must match policy.

**`src/policy/gate.py` (~5 lines).** `protected_write_path` (`gate.py:247-256`) uses the same `own_write_paths` JSON from `mod.rs:851-854`; no separate logic, but a gate test should assert a write to the private `HEAD` is refused.

**Tests (~40 lines).** Extend `tests/cleanup.rs` (reap commits to `hydra/<run-id>` at `cleanup.rs:252-288`): seed a checkout on `main`, call `commit_then_remove` path via `reap_one`, expect `Reaped::Failed` and no new commit on `main`. Extend `tests/spawn.rs::the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory` (`spawn.rs:1445-1543`): after `git add` succeeds (`spawn.rs:1523-1524`), assert writing `worktrees/r1/HEAD` is refused. Both tests fail on current code.

## 3. What it breaks or costs

A run that legitimately checked out a human branch (nothing in `worktree::create` forbids it; `worktree.rs:242-257` only names `hydra/<run-id>` at launch) keeps its checkout until the human fixes the branch or deletes it; reap no longer silently commits there. **`git add` may break** if `objects/` is narrowed before measurement: ticket 70 found add needs `parent_git_dir/objects/` (`mod.rs:636-641`, `70-git-index-lock-under-the-seatbelt.md:48-51`). SBPL `file-write*` (`sandbox.rs:82-84`, `sandbox.rs:109`) does not separate create from unlink; that split must be measured on a real profile before landing.

## 4. What a hostile run or proposer still gains

Until `objects/` is narrowed, a run can **delete or overwrite pack files and loose objects** under `.git/objects/` (`mod.rs:642`, `sandbox.rs:106-112`). Content addressing blocks forging a sha, not removing blobs another ref needs. Under `standard`, **shell can still reach paths the gate does not substring-match** (`SECURITY.md:40-46`). Reap runs outside the seatbelt (`reap.rs:1-2`, no sandbox wrap), so these are run-time holes, not reap holes.

## 5. The strongest argument against your choice

Option 2's comment at `reap.rs:415-418` reflects a real workflow: a human who deliberately checks a run onto their branch may want reap to preserve work there. Option 1 turns that into a hard failure and forces manual recovery from uncommitted files. That is safer for the default case Hydra documents, but it punishes an advanced use the code once treated as benign. If the human wants that workflow, they should opt in explicitly, not discover it through a moved `main`.

## 6. The other options

**Option 2** keeps the branch-move and object-deletion surfaces and only rewrites docs; it makes `README.md:27` and `SECURITY.md:52-56` false where they claim branches do not move from inside a run. **Option 3** fixes the reap side but leaves object deletion (`mod.rs:642`) and the README contradiction on branch safety; it is half a fix.

## 7. Something the ticket missed

`SECURITY.md:52-56` says `refs/` and `HEAD` stay denied and "no branch moves from inside a run," but the spawn test only probes the **parent** `.git/HEAD` (`spawn.rs:1542-1543`), not `worktrees/<run-id>/HEAD`, which the carve-out grants whole (`mod.rs:635`). The ticket is right that reap is the commit path, but the doc overstates what the seatbelt test proves. The same `commit_then_remove` logic applies to the skill-store checkout (`reap.rs:224-230`); any branch guard must cover `rungs/curate/<run-id>` as well as `hydra/<run-id>`.
