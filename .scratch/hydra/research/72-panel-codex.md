# Panel 72, codex

## 1. Recommendation

Choose option 1, with a fail-closed condition: do not retain shared object-store writes if creation-only access cannot safely support staging. Refuse an unexpected branch before any cleanup mutation, and preserve the expected `rungs/curate/<run-id>` branch for skill checkouts rather than hard-coding `hydra/` everywhere (`src/reap.rs:204`, `src/reap.rs:225`).

## 2. Mechanism

- In `src/reap.rs`, move branch equality enforcement immediately after `current_branch` in `commit_then_remove`, before ignore removal, staging, and checkout removal (`src/reap.rs:386`, `src/reap.rs:405`, `src/reap.rs:410`, `src/reap.rs:412`). Return an error naming expected and actual branches, leaving the checkout and spawn evidence. Existing callers already stop on error (`src/reap.rs:212`, `src/reap.rs:233`). Approximately 20 changed lines.
- In `src/policy/mod.rs::own_write_paths`, remove the undifferentiated object-store grant and represent Git access separately (`src/policy/mod.rs:618`, `src/policy/mod.rs:635`, `src/policy/mod.rs:642`). In `src/sandbox.rs::compose`, emit private-HEAD denial after the private-directory allowance, including protection against replacement through its parent. The current grant is `file-write*` (`src/sandbox.rs:106`). Compile equivalent explicit-path refusals before the gate's own-path exception, including shell spelling checks (`src/policy/gate.py:247`, `src/policy/gate.py:250`, `src/policy/gate.py:366`). Approximately 80–140 lines across compiler, profile, and gate.
- For objects, require measured protection against overwriting, truncating, unlinking, and renaming over existing loose objects and packs. A create permission alone is not evidence of immutability. If Git requires incompatible operations, deny shared writes and accept unavailable in-run staging pending a separate object-store design. Update the inaccurate content-addressing comment (`src/policy/mod.rs:639`) and documents (`README.md:25`, `SECURITY.md:52`), approximately 20 lines.
- Add roughly 150–220 test lines in `tests/cleanup.rs`, `tests/spawn.rs`, and `tests/gate.rs`. A scratch repository test should redirect private HEAD to a human branch, advance that branch independently, then reap: assert unchanged human tip, retained checkout, and retained files. This fails today because `commit_everything` stages everything and commits through checkout HEAD (`src/worktree.rs:105`, `src/worktree.rs:111`). Cover the curate caller too. Extend the real-profile staging test with private-HEAD replacement and existing-object destruction probes; those denial expectations fail under today's broad grants (`tests/spawn.rs:1445`, `src/sandbox.rs:106`).

## 3. What it breaks or costs

A human who deliberately switches a run onto another branch loses automatic reap-and-commit there. They must save it manually or restore its expected branch. That behavior is explicitly supported today (`src/reap.rs:415`).

Whether `git add` works with object creation allowed and unlink denied is **unconfirmed**. Measure fresh blobs, duplicate blobs, temporary-file cleanup, and index replacement under a real macOS profile outside the enclosing seatbelt. The existing test returns early on `sandbox_apply` failure (`tests/spawn.rs:1508`). Build 70 first claimed private-directory access sufficed, then recorded that object creation was also necessary; it never establishes creation-only compatibility (`.scratch/hydra/build/70-git-index-lock-under-the-seatbelt.md:36`, `.scratch/hydra/build/70-git-index-lock-under-the-seatbelt.md:46`). I ran no tests.

## 4. What a hostile run or proposer still gains

It can delete or corrupt its private index, losing staging state or obstructing reap, wherever the remaining private-directory grant permits it (`src/policy/mod.rs:635`). If shared creation is retained, it can create garbage objects and consume disk. It can also leave malicious source in its checkout for the legitimate snapshot commit; `commit_everything` does not review content (`src/worktree.rs:105`).

The branch comparison is not an atomic ref-update guarantee. `current_branch` and `git commit` are separate operations (`src/reap.rs:479`, `src/worktree.rs:111`); another writer outside the profile could change HEAD between them. Do not advertise protection against that stronger adversary without explicit-ref, compare-and-swap commit publication.

## 5. The strongest argument against your choice

This couples a simple reap fix to an unmeasured filesystem-permission design and may take away ordinary staging. Option 3 could ship quickly and reliably. My reason to accept the cost is that preserving branch names while allowing destruction of their objects does not preserve the human's repository; the current whole-store grant remains the underlying authority problem (`src/policy/mod.rs:642`, `src/sandbox.rs:109`).

## 6. The other options

Option 2 documents surprising behavior but leaves both unwanted commits and destructive object access intact (`src/reap.rs:410`, `src/policy/mod.rs:642`). Documentation cannot authorize each future branch movement.

Option 3 is a useful intermediate patch, not a satisfactory endpoint: it fixes reap while retaining the broad object grant (`src/sandbox.rs:106`).

## 7. Something the ticket missed

Reap is not the only parent-side commit path. `commit_pass_branch` also calls `commit_everything` through the proposer's mutable checkout before publishing a pass branch (`src/rungs/evolve.rs:968`, `src/rungs/evolve.rs:988`). Audit that caller under the same branch-identity rule. Also, repository discovery trusts checkout Git metadata (`src/reap.rs:468`); denying only private HEAD does not establish integrity of every other private metadata file.
