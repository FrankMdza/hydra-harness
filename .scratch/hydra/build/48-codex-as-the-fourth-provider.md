# 48 - Codex CLI as the fourth provider

Type: build
Status: open
Blocked by:

## What

Map ticket 58 measured `codex-cli 0.154.0` and found a `PreToolUse` hook in Claude's dialect,
a JSONL stream under `codex exec --json`, and ChatGPT-account auth. Route it the way the other
three are routed. The pieces, in the order the other providers grew them:

1. `src/provider/codex.rs`: argv for a recorded run (`exec --json`, `--sandbox` and
   `--ask-for-approval never` as belt to the gate, `--dangerously-bypass-hook-trust` because a
   per-spawn hook is never trusted, `-c` overrides to point the hook at Hydra's script), what it
   reads from the worktree, its version probe, and its auth pre-flight (`codex login status`).
2. `src/policy/table.rs`: rows for `Bash`, `apply_patch`, `mcp__<server>__<tool>`,
   `update_plan`, `spawn_agent`; hosted tools such as web search never reach the hook and want
   an `unhooked` row (map ticket 51's kind). The gate's `claude` dialect answers Codex; the
   config writer registers the hook in the shape `docs/hooks` gives.
3. `src/normalize/codex.rs`: `item.*` and `turn.*` events to the contract's tool_call,
   tool_result, message, thinking and usage; outcome derived the way contract 8 says.
4. A decision this ticket must not take alone: Codex has no read tool, so under `read-only` and
   `judge` every read is a shell command. Either those policies grant a bounded shell read on
   Codex only, or Codex is routable under `standard` and `permissive` first. Open a map ticket
   if the second reads wrong.
5. Measure on a logged-in binary before any of this lands: whether a hook crash or timeout
   fails open, and whether `-c hooks...` on the command line is honoured as a hook source.
6. `tests/drift.rs` gains a Codex probe that skips without the binary; `docs/harness-spec.md`
   sections 6.6 and 17 and `IMPLEMENTATION.md` say what is true.

Needs a human logged in first: `codex login` on any ChatGPT plan. Map ticket 58 has the
measurements and their sources.
