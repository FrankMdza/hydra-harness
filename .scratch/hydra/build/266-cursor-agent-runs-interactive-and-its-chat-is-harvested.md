# 266 - `cursor-agent` runs interactive, and its chat is harvested

Type: build
Status: resolved
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

Built 2026-10-04 by an Opus run (`01a1073a-63cb-78c0`), reviewed and applied by the session.

`hydra run --interactive --provider cursor-agent` launches the TUI ungated (`--policy`
refused, as for `codex`); `provider/cursor.rs` finds the chat (earliest in the run's cwd created
at or after the process start) and reads `store.db` with the system `sqlite3 -readonly -json`,
since nothing in the repo reads SQLite and `rusqlite` would compile a C library into every build
for one read; `order_chat` orders the messages by the state blob naming the most message ids.
`normalize/cursor.rs` reads a frame with `role` and no `type` as a chat message.
`hydra-copilot` and `masterofpuppets` accept `cursor-agent`. Tests in `tests/harvest.rs` and
`tests/copilot_repo_and_tools.rs` fail on the old provider check.

**The run's guesses, checked by the session on this machine's real chats (no content read):**
- Every blob id is the SHA-256 of its data (19 of 19 on one store).
- Ids sit in the state blobs both behind `0a 20` (24) and otherwise (12); the code searches any
  32-byte run that matches a message id, so it does not depend on the framing. The fixtures use
  only `0a 20`.
- The latest state names the `system` message.
- Tool names across the 40 newest stores: `Read` 1,044, `Grep` 624, `StrReplace` 471, `Shell`
  352, `Glob` 100, `Write` 53, `CallDynamicTool` 11, `GetDynamicTools` 5. The run invented
  `run_terminal_cmd`. The capability table knows `Read`, `Grep`, `Shell` and `Write` for cursor,
  so "every chat tool name becomes an unknown-tool finding" overstates it: `StrReplace`,
  `Glob` and the two dynamic-tool names do.

**No change to existing records:** `hydra doctor --walk` identical before and after, and
`hydra show --events` on all 267 gated `cursor-agent` records identical once the per-call marker
token is masked.

Not done, the session's: one live `hydra run --interactive --provider cursor-agent` and its
harvest. Found by the run and left for the map: `cursor-agent create-chat` returns a new chat's
id, so Hydra could mint the chat up front instead of finding it by cwd and time, and nothing
measured tells a TUI chat from a `--print` chat started in the same cwd. The contract has no
`usage_source: unknown`; a harvested chat records `unavailable`.
