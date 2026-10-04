# 285 - Export survives a pass whose commit is gone

Type: build
Status: resolved
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

Resolved 2026-10-04 by run `01a1051f-e4e1` (`cursor-agent` cursor-grok-4.6-high, `standard`,
3.52M tokens), applied without changes.

Built: a blocked pass whose base or branch cannot be read is `JudgeDisposition::Unreadable`, and
`--dry-run` now walks preference pairs too and prints it as `pass_unreadable`; the rest exports.
Test `a_pass_whose_base_commit_is_gone_is_unreadable` failed with the catch reverted. The ticket
called it a curate pass; it is an evolve pass (`ouroboros/blocked/`).

Measured by the session on the real corpus: the dry run lists six `pass_unreadable` evolve passes
(`01a0814a`, `01a08169`, `01a0822e`, `01a08241`, `01a084d3`, `01a08684`) and is otherwise the
same as before but for two runs that did not exist then; a real `hydra export` exits 0 with 350
lines, where it stopped at the first of those passes. The export is what the first training round
reads.
