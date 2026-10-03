# 99 - The gated copilot can still write `~/.claude.json`

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-03, by the session, from build ticket 250's Outcome.

## What the session measured

Build 250 narrowed the sandbox of a gated interactive session (map 98, D10): writes under the
home directory are denied except the worktree, temp, the spawn directory, twelve state
directories under `~/.claude`, and `~/.claude.json*` and `~/.claude/history.jsonl*`. The live
probe refuses `printf x > ~/<file>` and passes it with the deny removed.

`~/.claude.json` was left writable because `claude` rewrites it at every start and the run could
not measure, from inside a seatbelt, whether `claude` starts without it. That file holds
user-scope MCP servers. A gated session can therefore add a server there that the next ungated
`claude` session on the machine starts, with the human's permissions. Nothing measured that path
end to end; it is read off what the file holds.

## Questions for the human

1. Measure first or decide now? A launch of `claude` under the narrowed profile with
   `~/.claude.json` denied, typed by the human, answers whether `claude` runs without it.
2. If it does not run without it: (a) accept the gap and write it in `SECURITY.md`; (b) give the
   gated session its own `CLAUDE_CONFIG_DIR` (or the provider's equivalent) under the spawn
   directory, seeded from the human's, so its writes never reach the real file; (c) compare the
   file's `mcpServers` before and after a gated session and refuse to close the record cleanly if
   it changed.
3. Does the same hold for `history.jsonl`, which a later session reads back as input?

## Answer
