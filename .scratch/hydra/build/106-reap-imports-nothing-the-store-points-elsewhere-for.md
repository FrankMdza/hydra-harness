# 106 - Reap imports only what the run's own store holds

Type: build
Status: resolved
Blocked by: 103

## What

An Opus review of build tickets 100 to 104 as merged, 2026-09-24,
`~/.claude/handoffs/2026-09-23-panel3/review-100-104.md`. Implement its three items under 103 and
the comment fix under 100. Item 1 was confirmed by experiment and comes first.

## Done when

Each item has a test that fails before the fix, checked by reverting it, and the suite passes
outside a seatbelt. Documentation sentences go in the Outcome.

## Outcome

Built on 2026-09-24 by a Claude Code session working on Hydra's source, not through `hydra run`,
on the same branch as build ticket 105. All three items under 103 and the comment under 100 are
done. None needed a decision.

**103.1, the store's `info/`.** `import_run_objects` now calls `scrub_run_store` before it lists
any name. The scrub removes `store/info` whole, as a link if it is one, and never follows it.
Right before `pack-objects` starts, reap checks again, and if `info/` is back it imports nothing.
A process the run left running could still write `info/alternates` between that check and the
moment `pack-objects` reads the store. That window is small and stays open. Closing it means
packing from a copy of the store the run cannot reach.

**103.2, links.** The same walk removes anything at the top of the store that is not a real
directory, and anything in a fan-out directory or in `pack/` that is not a regular file. I went one
step past the review: it also removes a regular file with a second hard link. A hard link to a
loose object of another repository is a regular file, and before this change reap imported it just
as it imported through a link. Git writes each loose object and pack with a single link, so the
check drops nothing the run's own git wrote. Anything dropped is hashed again from the checkout.
The doc comment on `store_object_names` now says the listing skips links but that this alone
never kept `pack-objects` from following one. Build ticket 103's Outcome has the same correction.

**103.3, size.** `RUN_STORE_IMPORT_CAP` is 1 GiB. The scrub sums the sizes of the files it keeps.
A store over the cap imports nothing, and reap commits from the checkout. `index-pack` gets
`--max-input-size=<cap>`, which covers a store that grows after the walk.
`import_run_objects_within(parent, store, cap)` takes the cap as an argument so a test can use a
small one, and `import_run_objects` passes the constant. I picked the number myself. Going over it
costs a rehash of the checkout and nothing else, so no decision rests on it.

**100, the comment.** `src/outcome.rs` now says the two provider-refusal shapes mean "the provider
stopped serving the run, whatever it had done before", and names 01a0cc9d-8f15. A comment has no
behaviour, so no test covers it. I did not do the optional case-insensitive match.

**Tests, each checked by reverting its fix and watching it fail:**

- `tests/cleanup.rs::an_alternates_file_in_the_runs_store_brings_nothing_into_the_parent`. The
  review's own experiment, as a test. The run writes `info/alternates` naming another repository's
  objects, puts a pack index naming one of its blobs in `pack/` with no pack beside it, and stages
  that blob as `leak.txt` with `update-index --cacheinfo`. Without the `info/` removal and the
  second check, reap imported the blob into the parent. After the fix, the blob is missing from the
  parent, `unstage_missing_objects` drops the entry, and the branch holds the run's `work.txt` and
  no `leak.txt`.
- `tests/cleanup.rs::a_linked_fan_out_directory_or_a_hard_link_in_the_runs_store_brings_nothing_in`.
  Two runs, because a name git cannot serve fails the whole `pack-objects` and would hide the other
  case. With links kept, the blob behind a fan-out directory that is a link reached the parent.
  With the hard-link check off, the hard-linked blob reached the parent.
- `tests/cleanup.rs::a_store_over_the_cap_imports_nothing`. A cap one byte under the store's
  size is refused by the walk. A cap of exactly its size gets past the walk and is refused by
  `index-pack`, because the pack stream for one small object, 37 bytes here, is larger than its
  loose file. With the walk's check off, the first case was refused by `index-pack` instead, and
  the message check failed. With the flag off, the second case imported.

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean. Whole suite outside
a seatbelt, `~/.local/bin` off `PATH` so `tests/drift.rs` skips: 29 targets with doc tests, 778
passed, 0 failed. `tests/cleanup.rs` holds 45.

**Proposed documentation sentences.**

`SECURITY.md`, the object-store bullet build ticket 103 proposed, after "which names each object
from its bytes, so a forged name never reaches the parent or a later worktree":

> Before it packs anything, reap removes the store's `info/` directory, where git would read an
> alternates file naming another repository, and every link, every entry that is not a regular
> file, and every file with a second hard link. So `pack-objects` reads only bytes the run's own
> git wrote into its store, never another repository the user can read. A store over 1 GiB imports
> nothing, and reap commits from the checkout instead.

`IMPLEMENTATION.md`, "Git that Hydra runs in a run's checkout, since build ticket 95", the bullet
build ticket 103 proposed, replacing "Reap runs `import_run_objects` and then
`unstage_missing_objects` before it commits.":

> Reap runs `import_run_objects` and then `unstage_missing_objects` before it commits. Since build
> ticket 106 the import first scrubs the store: `info/` goes whole, since git reads
> `info/alternates` there even with the store set by environment, and so does every link, every
> entry that is not a regular file and every file with a second hard link. If `info/` is back by
> the time `pack-objects` would start, nothing is imported. A store over `RUN_STORE_IMPORT_CAP`,
> 1 GiB, imports nothing, and `index-pack` gets `--max-input-size` at the same cap. Whatever is
> not imported is hashed again from the checkout.

`README.md` and `CLAUDE.md`: no file count changes. `tests/cleanup.rs` now also plants an
alternates file, a linked fan-out directory and a hard link in a run's store, and checks the cap.
