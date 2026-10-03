# 250 - A narrower sandbox for the gated copilot

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-03 by run `01a101a9-6ff9` (`claude` opus 5.5, `standard`, 2.21M tokens),
applied by the session unchanged; the live check was run by the session, since a seatbelt does
not nest inside a run.

Built: `sandbox::copilot_block` and `narrow_for_copilot`. For a gated interactive launch only
(`interactive::finish_policy_launch`), the profile gains, right after the TIOCSTI rule,
`(deny file-write* (subpath "<home>"))` and then allows for the worktree, the temp directories,
the spawn directory, twelve named state directories under `~/.claude` (backups, debug,
file-history, paste-cache, plans, session-env, sessions, shell-snapshots, statsig, tasks,
telemetry, todos), `~/Library/Caches/claude-cli-nodejs`, and `~/.claude.json*` and
`~/.claude/history.jsonl*` by prefix. Hydra's own denies come after the block and still win.
`settings.json`, `hooks/`, `skills/`, `plugins/`, `commands/`, `agents/` and `CLAUDE.md` stay
unwritable. Headless profiles are byte for byte unchanged; `run_start.sandbox` records the
narrowed sha. Tests: four in `tests/sandbox_copilot.rs`, one a live `sandbox-exec` probe that
writes into the worktree as a control and `printf x > ~/<probe>` as the check.

Measured by the session on master, outside any seatbelt: the live probe runs and passes; with the
home deny removed it fails with `a write to home was not refused: rc=0`, and restored it passes.
The run measured `a_gated_interactive_launch_records_the_narrowed_profile` failing without the
`interactive.rs` hunk. `sandbox_copilot` 4, `sandbox_read_only` 3, `sandbox_tty` 2, `spawn` 36,
`--lib` 234; fmt and clippy clean.

Not measured: the list of `claude`'s state directories was read off names and modification times
under `~/.claude`, not traced from a launch under the profile, so step 2's condition is unknown.
Whether a real gated cockpit session still starts and answers is the human's check, after a
release build; the likely failures are a missing `~/.claude` entry or the updater under
`~/.local/share/claude`. Writes to `~/.cargo` or `~/.npm` now fail in a gated session.

Left open, map ticket 99: `~/.claude.json` stays writable because `claude` rewrites it at every
start, and it holds user-scope MCP servers, so a gated session can add a server that the next
ungated session starts.
