# 250 - A narrower sandbox for the gated copilot

Type: build
Status: claimed
Blocked by:

## What

Map ticket 98, D10 (resolved 2026-10-03), unanimous. On 2026-10-02 a gated session under
`standard` wrote `~/prueba-fuera.txt` with a shell `printf`: the gate does not interpret shell,
and the macOS profile is `(allow default)` with denies for Hydra's own paths (map 67). For a
gated interactive session, the one other people type into, the profile also denies writes
outside the worktree and the temp directory. Agents launched headless keep today's profile; map
67 is not reopened for them.

## Do

1. Read `src/sandbox.rs` (how the profile is composed, lines 80 to 230) and
   `src/interactive.rs` (the gated launch). For a gated interactive launch only, add
   `(deny file-write* (subpath "<home>"))` with allows after it for: the worktree, the temp
   directories the provider and `git` use (`$TMPDIR`, `/private/tmp`, `/private/var/folders`), the
   run's spawn directory, and the provider's own state it must write to run at all. Measure that
   last list on a real `claude` interactive launch rather than guessing (its config and session
   directories under `~/.claude` and `~/.claude.json` are the likely ones); write the measured
   list in the Outcome.
2. If `claude` cannot run with any narrower set than `~/.claude` whole, stop and say so: that is
   a decision about whether the gated copilot may write its own provider's state.
3. Tests in `tests/sandbox_*.rs` style: the composed profile for a gated interactive launch has
   the deny and the allows in that order; a headless launch's profile is unchanged byte for byte.
   One live check under `sandbox-exec` with the composed profile: `printf x > ~/<scratch name>`
   is refused, a write into a scratch worktree is allowed.
4. `SECURITY.md`: the sentence that says shell writes outside the worktree are not blocked gains
   "except in a gated interactive session since build 250". `IMPLEMENTATION.md`: one paragraph.

## Done when

- The live check fails with the deny removed (revert it, run it, restore it). fmt, clippy and
  `cargo test` pass. A real gated cockpit session still starts and answers; the human checks this
  one by hand and the Outcome says it is his.
