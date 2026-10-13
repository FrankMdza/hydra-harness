# 309 - A run's worktree is born with a cloned `target/`: the first build drops from 13 minutes to seconds

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session, from build 305's Outcome (13 min 21 s for the first
`cargo test --release --workspace --no-run` in a fresh worktree) and a measurement the session
ran the same afternoon: `cp -c -R ~/hydra/target` into a clean copy of the tree took 43 s for
15 GB (APFS clonefile, no blocks copied), and the first `cargo build --release --workspace` on
top of it compiled 9 crates in 22 s. The second build took 0.23 s.

## What

`worktree::seed` (`src/worktree.rs`, around line 1251) seeds a run's checkout with the parent's
uncommitted changes as a patch and leaves ignored files behind on purpose: `target/`, `runs/`,
the rest of what a tree does not carry. So every builder on this repo starts by compiling the
whole workspace and its 300-odd dependencies from nothing, and every Outcome since 295 has paid
10 to 14 minutes before its first test. The dependencies do not depend on the worktree's path;
only the workspace's own crates carry the path in their fingerprint, and those are the 9 that
recompiled in 22 s. A clone of the parent's `target/` at worktree creation gives the run a
warm cache that costs no disk until a file is rewritten, and the run's own writes stay in its
worktree, which `hydra reap` removes.

## Do

1. In `src/worktree.rs`, after the checkout is seeded and before the launch: if the parent repo
   has a `target/` directory and the platform is macOS, clone it to `<worktree>/target` with
   `clonefile(2)` per entry (`cp -c -R` from a `Command` is acceptable if `fclonefileat` through
   `libc` would add a dependency; say which in the Outcome and why). On another platform, or
   when the parent has no `target/`, do nothing. A clone that fails for any reason is logged on
   the run's stderr and the launch continues without it; the run must never fail because the
   cache could not be cloned.
2. Skip what is not a cache: anything under `target/` that is not `debug/`, `release/` or
   `.rustc_info.json` and `CACHEDIR.TAG` (build 304 and 305 left measurement scripts under
   `target/hydra304/` and `target/hydra305/`; those must not travel).
3. Measure on this repo, inside your run, from your own worktree: `cargo build --release
   --workspace` time on a fresh worktree created by the old code and by the new (the old number
   is 305's 13 min 21 s if you cannot afford a second cold build; say so), and the time the
   clone itself takes. Put both in the Outcome with the load average.
4. `tests/worktree.rs`: one test that creates a parent repo with a `target/release/marker`
   file and a `target/hydra999/junk` file, runs the seeding, and asserts the marker is in the
   worktree and the junk is not; on a platform without clonefile the test asserts the worktree
   has no `target/` and the launch did not fail.
5. Check the record contract and the reap path: `target/` is ignored, so `diff_stat`,
   `commit_everything` and `remove_untracked` must behave as before; the test in `cleanup.rs`
   that checks reap's commit must still pass. If anything in `docs/run-record-contract.md` or
   `docs/harness-spec.md` section 3 says what a worktree holds at launch and would now be
   false, stop and say so: that is a decision for the map, not this ticket.
6. Amend `IMPLEMENTATION.md` where the seeding is described, and the comment at line 1248 that
   says ignored files are left behind.

## Done means

A run on this repo starts with a warm `target/`, the first build measured in the Outcome, the
test above, and `cargo nextest run --release --workspace` plus `cargo test --release --workspace
--doc` green inside the run.

## Outcome
