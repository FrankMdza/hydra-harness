# 267 - `agy` runs interactive, and its transcript is harvested

Type: build
Status: claimed
Blocked by: 266

## What

The last of map 100's four. Harness spec 10: `agy` keeps `conversation_summaries.db` and a
`brain/<conversationId>/` directory under `~/.gemini/antigravity-cli/`, and a hook reports the
transcript path (`.system_generated/logs/transcript_full.jsonl`), so take the path from the run
rather than constructing it. Same work as builds 255 and 266, plus `GEMINI.md` as the rendered
file name in `hydra-copilot`.


**Measured by the session on 2026-10-04 from this machine's `~/.gemini/antigravity-cli`, shapes
only, no content read or copied** (a run cannot read it; build from this):

- An interactive run has no hook, so the transcript path cannot come from one. Find the
  conversation instead: `conversations/<uuid>.db` (SQLite) has a table `trajectory_metadata_blob
  (id text default "main", data blob)` whose protobuf blob holds the workspace path as plain
  bytes (`/Users/...` or `file:///Users/...`); all 30 newest name a path. Match the run's cwd
  exactly, and the start time from the file's or the transcript's `created_at`. Open read only.
- The uuid of `conversations/<uuid>.db` is the directory `brain/<uuid>/` (139 of 139 match).
  119 of them hold `.system_generated/logs/transcript_full.jsonl` (also `transcript.jsonl` and
  `logs/chunks/...`); a conversation with no transcript yet is possible.
- Transcript lines, over all 119: keys `type`, `source`, `status`, `step_index`, `created_at`,
  plus `content` (string or null), `thinking`, `tool_calls`, `error`. `type` is
  `PLANNER_RESPONSE` (1,435), `GENERIC` (1,373), `USER_INPUT` (122), `SYSTEM_MESSAGE` (56),
  `CHECKPOINT` (16); `source` `MODEL`, `USER_EXPLICIT`, `SYSTEM`; `status` `DONE`, `RUNNING`,
  `ERROR`, `INVALID`.
- A tool call is `{"name", "args"}` inside a `PLANNER_RESPONSE`'s `tool_calls`, with no call id.
  Names seen: `run_command` 781, `view_file` 421, `find_by_name` 68, `write_to_file` 51,
  `replace_file_content` 49, `grep_search` 40, `list_dir` 20, `manage_task` 14, `schedule` 7,
  `call_mcp_tool`, `sed_file`, `notebook_edit` 2 each. Tool output is in
  `.system_generated/steps/<step_index>/output.txt`; a call and its result join by step order,
  never by an id, and the harvest must say so where the order is ambiguous.
- `conversation_summaries.db` would not open read only here; do not depend on it.

Fixtures are built by hand in the test from these shapes. The live check is the session's.

## Outcome
