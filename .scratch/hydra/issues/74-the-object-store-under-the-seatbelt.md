# 74 - What a run may write in the parent's object store

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-23, by the session, from build ticket 95 part 4, whose premise turned out false.

## Question

`git add` inside a run writes blobs into the parent repository's shared `.git/objects/`, so build
ticket 70 granted the run full write access there, and it still has it. Map ticket 72 asked for
creation-only if a measured `git add` allowed it, on the assumption that creation-only gives
integrity. Measured 2026-09-23, outside any seatbelt, under profiles that deny `file-write*` on
`objects/` and allow only `file-write-create`:

- `git add` of a new blob and of one already stored exits 0 with a correct index and a clean
  `git fsck`. Each object leaves one `tmp_obj_*` behind, because git cannot unlink its temporary.
- Deleting, overwriting, `chmod`, renaming over, hard-linking over an existing object and renaming
  a fan-out directory are all refused.
- A run can still create `objects/info/alternates`, symlinks, and a pack plus index that maps an
  existing sha to other bytes. With the object present only in the forged pack, `git cat-file`
  and `git checkout` returned the forged content. A forged pack racing the real one, where git
  prefers newer packs, was not measured.
- A run can also create a loose object under the sha of content the human has not written yet;
  git then skips its own write of that object. Not measured.

So creation-only stops deletion and does not stop substitution. The options:

1. **Creation-only.** Strictly better than today on deletion, same on substitution; costs stray
   `tmp_obj_*` files that `git gc` removes.
2. **No object store at all.** Runs lose `git add`, which build ticket 70 was built to give them;
   reap stages from outside anyway.
3. **A private object store per run.** `GIT_OBJECT_DIRECTORY` pointed at a directory under the
   run's spawn, with the parent's store as an alternate for reading; reap copies or repacks the
   run's objects into the parent from outside. The run can corrupt only its own store.
4. **Leave it and say so.** Today's grant, stated in `SECURITY.md`, which it now is.

Measurements: build ticket 95's Outcome and its Opus review.

## Answer

Resolved 2026-09-23 with the human, after the third hiring panel ranked this among its most serious
open findings (H1). Option 3, a private object store per run: the run writes objects only into its
own directory under its spawn and reads the parent's store as an alternate; reap moves the run's
objects into the parent from outside the seatbelt after git has checked them; the parent's
`objects/` goes back to fully denied. Build ticket 103.
