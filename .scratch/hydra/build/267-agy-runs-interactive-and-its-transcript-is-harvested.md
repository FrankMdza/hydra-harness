# 267 - `agy` runs interactive, and its transcript is harvested

Type: build
Status: open
Blocked by: 266

## What

The last of map 100's four. Harness spec 10: `agy` keeps `conversation_summaries.db` and a
`brain/<conversationId>/` directory under `~/.gemini/antigravity-cli/`, and a hook reports the
transcript path (`.system_generated/logs/transcript_full.jsonl`), so take the path from the run
rather than constructing it. Same work as builds 255 and 266, plus `GEMINI.md` as the rendered
file name in `hydra-copilot`.

## Outcome
