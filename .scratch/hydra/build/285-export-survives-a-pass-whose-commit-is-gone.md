# 285 - Export survives a pass whose commit is gone

Type: build
Status: open
Blocked by:

## What

Found on 2026-10-03 while measuring build 277: `hydra export` on the real corpus stops with
`git diff 02a6c80784ba72b845a3c3c02da2b82d329d2dd6 ouroboros/blocked/01a0814a-c730-7e11-847b-fac691b5d57b
failed: fatal: bad object 02a6c80...`. A curate pass's branch names a base commit the object store
no longer has, and one such pass aborts the whole export. Find where the preference export diffs
a pass branch (`src/export_preference.rs`, near the pass branch names), and make a pass whose
base or branch cannot be read count as ineligible with a reason the dry run prints (for example
`pass_unreadable`), so the rest exports. A test with a scratch repository whose pass branch names
a missing commit. `hydra export --dry-run` on the real corpus then lists that pass and nothing
else changes; the session checks that.

## Outcome
