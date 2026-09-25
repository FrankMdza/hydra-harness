# 131 - The bridge reads `claude`'s transcript live, so the mascot poses a refusal during a session

Type: build
Status: resolved
Blocked by:

## What

Map ticket 76-G, resolved 2026-09-24. Read its `## Answer, the rest` first.

An interactive run's record is `run_start` alone until the harvest at exit, so
`integrations/herdr/bin/hydra-bridge` (`count_denials` near line 106, `build_tokens` near 160)
reads no events for it, and the mascot shows working and never a refusal. `claude` appends its
transcript to `~/.claude/projects/<slug>/<session-id>.jsonl` during the session.
`src/provider/claude.rs` has `projects_dir_for` and `project_slug` since build 123, and the
interactive `run_start` names the session id the launch passed.

## Do

1. For a pane whose run is interactive and whose provider is `claude`, the bridge finds the
   transcript from the record's `cwd` and session id, using the same slug rule. Port it to Python,
   and put a comment beside it naming `project_slug` in `src/provider/claude.rs` as the source.
2. It reads only lines it has not read, keeping an offset per run, and counts gate refusals the
   way the normalizer recognises them in a `claude` record (a `tool_result` carrying the
   `hydra/` voice, including 2.1.280's `PreToolUse:<Tool> hook error:` prefix from build 116).
   The count feeds the same `denials` token the recorded path writes.
3. Read-only. Never write under `~/.claude`. Nothing under `src/` changes. A missing or
   unreadable transcript leaves the token unset rather than zero.
4. `integrations/herdr/README.md`: one paragraph on what the bridge reads for an interactive run
   and that a Claude Code format change breaks the pose, not Hydra.

This is the other session's territory. Check `git log -3 -- integrations/herdr/bin/hydra-bridge`
first.

## Done when

- A test under `integrations/herdr/tests/` writes a fake transcript into a scratch `HOME`, two
  lines at a time, with one refusal on the second batch. The bridge's token goes from unset to
  `0` to `1` across three polls, and it never opens a file outside the scratch `HOME`.
- The existing Herdr tests pass.

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d618-6b6e under `standard`, about 1.33M
tokens. The session applied the diff with one fix and one test.

For a live pane whose run is `claude` and whose `run_start` says `launch_mode: interactive`,
`hydra-bridge` reads the record's `run_start` through `run_file` from `hydra status`, finds the
transcript at `~/.claude/projects/<slug of the realpath of cwd>/<provider_session_id>.jsonl`, and
counts error `tool_result` blocks whose text carries a gate code in Hydra's voice, 2.1.280's
`PreToolUse:<Tool> hook error:` prefix included. The slug function is ported to Python with a
comment naming `project_slug` in `src/provider/claude.rs`. The count feeds the `denials` token
the recorded path already writes, and for these panes the bridge skips `hydra show --events`.
Nothing under `src/` changed, and nothing writes under `~/.claude`.

The session's fix: the agent read to the current end of the file and moved the offset past
whatever it got. A poll that landed while Claude was mid-line skipped the fragment as bad JSON,
and the rest of that line on the next poll too, so a refusal written across a poll boundary was
lost for good. The bridge now reads bytes, consumes only through the last newline, and leaves the
fragment for the next poll. `test_a_refusal_split_across_two_polls_is_counted_once` writes half a
refusal line, polls, writes the rest, polls twice: unset, then 1, then 1. On the agent's version it
fails with `AssertionError: 0 is not None`.

The agent's `test_denials_from_unset_to_zero_to_one` writes a fake transcript into a scratch
`HOME` two lines at a time and checks that every `open()` stays inside it. It failed before the
change with `AttributeError: ... TRANSCRIPT_STATE`. On master all 24 Herdr tests pass.

Not measured: the human's one `hy` session so far (01a0d56e-ba1b) left no transcript at the path
the harvest expected, so whether a live session writes where this reads is still open. The next
`hy` session that sends a message answers both at once.
