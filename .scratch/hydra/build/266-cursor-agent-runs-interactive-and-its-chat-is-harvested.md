# 266 - `cursor-agent` runs interactive, and its chat is harvested

Type: build
Status: claimed
Blocked by:

## What

Map 100: a provider becomes an orchestrator once Hydra can read its transcript back. Build 255
did `codex`; this is `cursor-agent`, the same shape. Harness spec 10 says it keeps per-chat
directories under `~/.cursor/chats`. Measure the layout and line shapes from directories already
on this machine (copy no user content), launch the TUI ungated from `hydra run --interactive
--provider cursor-agent`, find its chat after exit by the run's cwd and start time, and harvest
messages, tool calls and usage. Then `masterofpuppets <repo> <model> cursor-agent` and the
copilot's provider check accept it, and `COPILOT.md` renders where `cursor-agent` reads
instructions (`AGENTS.md` per build 154). Build 255's Outcome is the model, including its
correction: filter on what the TUI itself writes, measured on more than one file.


**Measured by the session on 2026-10-04 from this machine's `~/.cursor/chats`, shapes only, no
content read or copied** (a run under `standard` cannot read `~/.cursor` with its file tools, so
build from this):

- `~/.cursor/chats/<32-hex workspace>/<chat uuid>/` holds `meta.json` and `store.db` (plus the
  SQLite `-wal` and `-shm` while open). 296 `store.db` and 309 `meta.json` on this machine.
- `meta.json`: `{"schemaVersion": int, "createdAtMs": int, "hasConversation": bool,
  "updatedAtMs": int, "cwd": str}`. `cwd` and `createdAtMs` find a run's chat; open the db read
  only (`mode=ro`), since the TUI may still hold it.
- `store.db`: `CREATE TABLE blobs (id TEXT PRIMARY KEY, data BLOB)` and
  `CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)`. Blob ids are 64-hex SHA-256.
- Message blobs are JSON `{"role", "content", "id"?, "providerOptions"?}` with role `system`,
  `user`, `assistant`, `tool`. `content` is a string or a list of parts:
  assistant `{"type": "tool-call", "toolCallId", "toolName", "args"}`,
  `{"type": "text", "text"}`, `{"type": "redacted-reasoning", "data", "providerOptions"}`;
  tool `{"type": "tool-result", "toolCallId", "toolName", "result", "experimental_content"}`.
  On one chat: 47 calls and 47 results joined on `toolCallId`.
- The order is not in the JSON. Protobuf blobs whose first byte is `0x0a` embed the raw 32-byte
  ids of message blobs, and they grow: each lists the ids of the conversation so far, in order,
  so the one naming the most ids is the latest state. Other protobuf blobs (first bytes `0x12`,
  `0x1a`, `0x23`, `0x2f`) name no message ids; leave them unread and say so in the record.
- No usage figures were found in the JSON blobs; if the TUI writes tokens anywhere, it is in the
  protobuf, and the harvest records `usage_source: unknown` rather than guess.

Fixtures are built by hand in the test from these shapes, never copied from `~/.cursor`. The live
check (a real interactive run, harvested) is the session's after the code lands.

## Outcome
