# 131 - The bridge reads `claude`'s transcript live, so the mascot poses a refusal during a session

Type: build
Status: open
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
