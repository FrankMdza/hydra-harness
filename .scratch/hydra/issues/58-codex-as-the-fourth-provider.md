# 58 - Codex CLI as the fourth provider, and what a provider without a hook is

Type: research
Status: resolved
Blocked by:

## Question

The human wants more providers that sign in rather than take a key, and named OpenAI's
Codex CLI as the next one. Its auth fits D4's agent-CLI plane as written: a vendor binary on
a ChatGPT subscription, no API key in any environment Hydra builds.

What is not known, and decides whether it is routable at all: **does Codex CLI expose a
pre-tool hook the gate can compile into?** Every routed provider today does (`claude`'s
`PreToolUse`, Cursor's `preToolUse` and `beforeReadFile`, `agy`'s own), and the gate is a
script registered in that hook. Codex ships approval modes and an OS sandbox of its own
(seatbelt on macOS, landlock on Linux) rather than, as far as the map knows, a hook a third
party can register. If that holds, Hydra has three options and none is free:

1. Route it ungated and record `policy: null`, the way pre-gate records already read. Honest,
   and it makes the fourth provider the one population the design says it does not want.
2. Drive Codex's own sandbox from the policy: translate `standard` into its approval and
   sandbox flags at spawn. A fourth dialect that is not a hook, so the capability table gains
   rows the gate never sees and the record cannot carry a per-call decision.
3. Refuse to route it until it has a hook, and say so in the README's roadmap.

Measure before choosing: install `codex`, read its `--help` and config for anything named
hook, and run one read-only task under its strictest mode to see what its stream carries and
whether per-call decisions appear anywhere Hydra could tee. Ticket 01's rule applies, drive
what the binary offers rather than what the docs promise.

Not a build ticket until the answer is one of the three.

## Answer

Measured 2026-09-12 on `codex-cli 0.154.0`, installed and not logged in, no model called. The
premise was out of date: Codex has a real pre-tool hook, and it is Claude's dialect.

**The hook.** `codex features list` reports `hooks stable true`. The event is `PreToolUse`,
configured in `~/.codex/hooks.json`, `~/.codex/config.toml`, `<repo>/.codex/hooks.json` or
`<repo>/.codex/config.toml`, in the same three-level shape as Claude Code's (event, matcher,
`{type: "command", command, timeout}`), and any `deny` from any layer wins. The payload carries
`cwd`, `hook_event_name`, `session_id`, `tool_name`, `tool_input`, `tool_use_id`,
`transcript_path`, `turn_id`, `permission_mode`. Shell commands report `tool_name: "Bash"` with
`tool_input.command`, patches report `apply_patch`, MCP tools report `mcp__<server>__<tool>`.
The deny answer is `{"hookSpecificOutput": {"hookEventName": "PreToolUse",
"permissionDecision": "deny", "permissionDecisionReason": "..."}}`, which is what
`respond("claude", ...)` in `src/policy/gate.py` already writes. Exit 2 with the reason on
stderr also blocks. Sources: https://learn.chatgpt.com/docs/hooks and the JSON schemas
embedded in the binary.

**Three costs, all real.**

1. *Hook trust.* Codex hashes each hook definition and runs it only after a human trusts that
   hash; a per-spawn gate whose command line changes per run is never trusted and is silently
   skipped. `--dangerously-bypass-hook-trust` exists for automation and is wired into `exec`.
   Hydra has to pass it, and the record has to say so.
2. *No read tool.* Codex reads files through shell, so every read is a `Bash` call. `read-only`
   and `judge` need a decision on what a shell read is, which the other three never needed.
3. *Fail-open, unmeasured.* The doc says a hook that returns `ask` or `approve` "marks the hook
   run as failed, reports the error, and continues the tool call", and never says what a crash
   or timeout does. Assume it fails open like `claude` until a run measures it.

**The rest fits.** `codex exec --json` prints JSONL with `item.completed` items of type
`command_execution` (command, output, exit code), `file_change`, `mcp_tool_call` (server, tool,
arguments, result), `agent_message`, `reasoning`, and `turn.completed` with token usage, so
there is a stream to tee. Auth is a ChatGPT account on any plan or an API key, stored in
`~/.codex/auth.json` by default, which is D4's agent-CLI plane. `--sandbox read-only |
workspace-write | danger-full-access` and `--ask-for-approval never` exist and become belt to
the gate's braces rather than a substitute. `shell_environment_policy.inherit` defaults to the
whole parent environment and does not strip names containing KEY or SECRET, so Hydra's own
allowlist keeps doing that job.

**Verdict: option one, routable with a compiled gate, and the dialect is Claude's rather than a
fourth.** Build ticket 48 carries the work. What it needs from a human first is a logged-in
binary, because the fail-open question and the read-tool question are both measured on a run.
