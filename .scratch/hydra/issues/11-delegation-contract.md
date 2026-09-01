# 11 - The hydra run CLI and the MCP delegation face

Type: grilling
Status: open
Blocked by: 01, 09

## Question

D7 fixes the shape: `hydra run --provider <p> --task "..." --parent <session-id>` returns JSON on stdout, with a thin MCP wrapper over the same path so Claude Code gets a typed `delegate` tool. This ticket fills it in.

Settle:

- The full argument surface of `hydra run`, and which arguments the MCP tool exposes
- The result JSON: what an orchestrator gets back, and how much of the child's trajectory rides in it against staying in the run file
- Whether delegation blocks or returns a handle the caller polls
- Concurrency: how many child runs at once, and what happens when one hangs
- Working directory and file conflicts, when parent and child touch the same repo. Worktree per child, shared tree, or the caller's problem
- How a child's permission prompts are answered when no human is watching that pane
- Failure and timeout semantics, and what the parent sees on each
- Depth: is a child allowed to delegate further, and is there a limit

Scenarios: Claude Code delegates the same file to cursor-agent and gemini at once. A child asks for permission to run `rm`. A child runs for forty minutes. The parent session dies while a child is mid-flight.

## What ticket 01 changed

**Target ACP, not print mode.** Across default print modes only five fields are common to all four providers. Driving `gemini --acp` and `hermes acp` instead widens that to streamed text, thought chunks, tool calls with arguments and results, and answerable permissions. Cursor has no ACP and becomes the permanent laggard.

That reshapes this ticket's permission question. In print mode, only Claude offers a two-way channel, its `can_use_tool` control request. Cursor auto-denies silently, Gemini throws, Hermes forces yolo. Over ACP, Gemini and Hermes both gain a real `session/request_permission`. So "how are a child's permission prompts answered when no human is watching" has three different answers depending on provider and mode, and Cursor's answer is that it cannot be asked.

**Session id minting differs per provider**, so `--parent <session-id>` threads Hydra's own run id, never the provider's. See the amendment on ticket 09.

**Claude alone supports bidirectional stdin** through `--input-format stream-json`. If the delegation contract needs to send anything to a running child rather than only reading from it, that is a Claude-only capability today.
