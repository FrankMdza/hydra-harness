# 309 - A run's worktree is born with a cloned `target/`: the first build drops from 13 minutes to seconds

Type: build
Status: resolved
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

Resolved 2026-10-13 inside a run, on its own worktree.

`worktree::create` calls `clone_build_cache` after the seed and before the snapshot. On macOS
it clones `debug/`, `release/`, `.rustc_info.json` and `CACHEDIR.TAG` from the parent's
`target/` with `libc::clonefile`, one call per entry, never following a link. `libc` was
already a dependency, so there is no `cp -c -R` and no new crate. libc 0.2.189 has the call but
not `CLONE_NOFOLLOW`, so the flag is a local constant, `0x0001` from `<sys/clonefile.h>`. Any
failure prints one `hydra:` line on stderr and the launch continues. On another platform the
function returns `Ok(())` and does nothing.

`tests/worktree.rs` gained
`the_checkout_gets_the_parents_build_cache_and_nothing_else_under_target`: a parent with
`target/release/marker` and `target/hydra999/junk`; the marker arrives, the junk does not, and
the diff stat is empty. Verified by breaking it twice. With the call removed the test fails on
the marker, and with `hydra999` added to the list it fails on the junk.

Measured with load averages of 9.99 at the start of the cold build, 58.62 at its end, and 27
to 38 for the rest:

- Cold, this worktree created by the old code: `cargo build --release --workspace` compiled
  125 crates in 98.3 s real. That is not 305's 13 min 21 s, which was `cargo test --release
  --workspace --no-run` and also built every crate's test binaries. The ticket's "13 minutes"
  belongs to the test build, not this one.
- New code: a parent repo made from this tree inside the worktree, with this worktree's
  `target/` (413 MB, 1,963 entries, release only) as its cache, and `worktree::create` called
  from a throwaway test. `create` took 0.91 s with the clone and 1.02 s with no `target/`, so
  at this size the clone is below the noise. The first `cargo build --release --workspace` in
  that checkout compiled the 8 workspace crates in 37.7 s, and the second took 0.22 s. The
  first `cargo test --release --workspace --no-run` there compiled the same 8 crates in about
  a minute, read off the load samples rather than timed. A rerun after touching `src/lib.rs`
  took 79.4 s.
- Not measured: the clone of the parent's real 15 GB `target/`, which a run cannot see.
  Directory `clonefile` cost grows with entry count rather than bytes, and the first real
  launch after this lands is the number to read.

Step 5: `target/` is ignored, so `seed_tree`, `diff_stat`, `commit_everything` and
`remove_untracked` see nothing new. The whole suite passed, `cleanup.rs` included: 1383 tests,
4 skipped, doc tests green, fmt and clippy clean. Neither `docs/run-record-contract.md` nor
harness spec 3 or 6.3 says what a checkout holds beyond the seed, so nothing there became
false.

Found on the way: git run from a run's shell writes objects into the run's own store through
`GIT_OBJECT_DIRECTORY`, so a fixture repo made with `git init` and `git commit` from that shell
has no `objects/`. Hydra's own git drops the variable, finds no repository there, walks up to
the parent, and tried to write a ref into `~/hydra/.git`, which the seatbelt refused. The
fixture was rebuilt with both variables unset. The orphaned commit from that first attempt sits
in this run's store, and reap will import it as an unreferenced object.
