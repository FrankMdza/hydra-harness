# 266 - `cursor-agent` runs interactive, and its chat is harvested

Type: build
Status: open
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

## Outcome
