# 01 - Provider headless modes and structured output

Ticket: [01-provider-headless-survey.md](../issues/01-provider-headless-survey.md)
Status: answered
Date: 2026-09-01

## Method and what it cost

I exhausted `--help` on every subcommand of all four binaries, then read the shipped
implementation, which turned out to be the stronger source. Three of the four ship
readable code on this machine:

- `cursor-agent` is an unminified-enough Bun bundle at
  `/Users/USER/.local/share/cursor-agent/versions/2025.10.02-bd871ac/index.js`.
- `gemini` ships near-source with original TypeScript file banners at
  `/opt/homebrew/Cellar/gemini-cli/0.42.0/libexec/lib/node_modules/@google/gemini-cli/bundle/`.
- `hermes` is plain Python at `/Users/USER/.hermes/hermes-agent/`.
- `claude` is a compiled Mach-O binary, but it embeds its Zod schemas with their
  `.describe()` text intact, so the wire format is readable by string extraction.

I ran two live invocations, both trivial:

- `claude -p "reply with exactly: OK" --output-format stream-json --verbose --model haiku
  --tools "" --safe-mode`. Cost **$0.004652** by its own `total_cost_usd`. Exit 0.
- `gemini -p "reply with exactly: OK" -o stream-json -m gemini-2.5-flash-lite`. Cost **$0**
  because it never reached a model. See the auth finding below.

I also ran `agy --help` and `agy help <sub>`, which do not start a session or touch quota.
Everything else came from source and `--help`.

## The four at a glance

| | `claude` 2.1.252 | `cursor-agent` 2025.10.02 | `gemini` 0.42.0 | `hermes` 0.14.0 |
|---|---|---|---|---|
| Headless flag | `-p/--print` | `-p/--print` | `-p/--prompt` | `-z/--oneshot` |
| Machine format | `--output-format json\|stream-json` | `--output-format json\|stream-json` | `-o json\|stream-json` | none |
| Event count | ~12 top-level types, 60+ system subtypes | 4 types | 6 types | 0 |
| Reasoning exposed | yes, full `thinking` blocks with signature | **no, dropped on purpose** | **no in stream-json**, yes over ACP | no in `-z`, yes in its SQLite store |
| Tool call events | `tool_use` / `tool_result` blocks inside messages | `tool_call` started/completed | `tool_use` / `tool_result` | none |
| Tool result payload | full, as `tool_result` content block | full `toolCall` object both times | `output` string plus `error` | none |
| Permission from a parent | **yes**, `can_use_tool` control request over stdio | no, silent auto-deny or `--force` | no, throws in non-interactive | no, forced yolo |
| Token counts | `usage` + per-model `modelUsage` | **none** | `stats` totals + per model | not on stdout |
| Cost in USD | `total_cost_usd` | none | none | not on stdout, in DB |
| Session id on stdout | yes, on every frame | yes, on every frame | yes, on `init` only | **no** |
| Set session id going in | `--session-id <uuid>` | via `create-chat` then `--resume` | `--session-id <uuid>` | no |
| Resume | `--resume <id>`, `--fork-session` | `--resume <chatId>` | `--resume latest\|<index>` | `--resume <id>`, `--continue` |
| Terminator | exactly one `result` frame | one `result` frame, success path only | one `result` frame, most paths | end of stdout |
| Exit codes | 0 / 1 | 0 / 1 / 130 | 0 / 1 / 41 / 42 / 44 / 52 / 53 / 54 / 55 / 130 | 0 / 2 |
| Bidirectional stdin | `--input-format stream-json` | no | no (ACP only) | no |

## claude 2.1.252

### Invocation

`claude -p "<prompt>" --output-format stream-json --verbose`. The `--verbose` flag is what
makes `stream-json` emit every frame rather than a summary. `--include-partial-messages`
adds `stream_event` deltas, `--include-hook-events` adds hook lifecycle frames, and
`--replay-user-messages` echoes stdin messages back, the last two only with
`--input-format=stream-json`.

### Event vocabulary

Top-level `type` values that reach stdout: `system`, `user`, `assistant`, `result`,
`stream_event`, `control_request`, `control_response`, `prompt_suggestion`, `attachment`,
`tombstone`, `rate_limit_event`, `tool_use_summary`. Every frame carries `uuid` and
`session_id`.

`system` carries a large `subtype` vocabulary. The ones a recorder cares about: `init`,
`compact_boundary`, `thinking_tokens`, `permission_denied`, `api_error`, `api_retry`,
`hook_started` / `hook_progress` / `hook_response`, `task_started` / `task_updated` /
`task_notification`, `memory_recall`, `turn_duration`, `model_fallback`, `status`. The full
list in the binary runs past sixty entries, most of them internal.

My live run produced, in order: `system/init`, four `system/thinking_tokens`, one
`rate_limit_event`, two `assistant`, one `result/success`. The `system/init` frame carried
`agents, analytics_disabled, apiKeySource, capabilities, claude_code_version, cwd,
fast_mode_disabled_reason, fast_mode_state, mcp_servers, messaging_socket_path, model,
output_style, permissionMode, plugins, product_feedback_disabled, session_id, skills,
slash_commands, terminal_slash_commands, tools, uuid`.

### Reasoning

Fully exposed. The first `assistant` frame in my run contained a single content block of
`{type: "thinking", thinking, signature}`. The final `result.usage` reported
`output_tokens_details.thinking_tokens: 33`. Claude also streams a live estimate as
`system/thinking_tokens` with `estimated_tokens` and `estimated_tokens_delta`, which its own
schema describes as approximate progress rather than billed tokens.

### Tool calls

Tool calls and results ride inside the Anthropic message envelope, as `tool_use` and
`tool_result` content blocks on `assistant` and `user` frames. Subagent frames carry
`parent_tool_use_id`, and `--forward-subagent-text` promotes subagent text and thinking into
the main stream with that field set. This is the only provider that gives a recorder native
parent/child tool lineage.

### Permissions

This is Claude's decisive advantage. It runs a bidirectional control protocol on the same
stream. When a tool needs a decision, the agent sends:

```
{"type":"control_request","request_id":"...","request":{"subtype":"can_use_tool",
 "tool_name":..., "input":{...}, "tool_use_id":..., "requires_user_interaction":bool,
 "decision_reason_type":..., "classifier_approvable":..., "matched_ask_rule":...}}
```

The parent answers with a `control_response` carrying the same `request_id`. No other
provider in this survey can be asked a permission question and answer it.

Auto-denials that never reach a prompt surface as `system/permission_denied`, and the
authoritative list lands in `result.permission_denials`. Without a prompt surface the binary
says plainly: `tool requires user interaction; no prompt available in headless mode`.

### Session and resume

`--session-id <uuid>` fixes the id before launch, `--resume <id>` continues,
`--fork-session` branches to a new id, `--no-session-persistence` disables the transcript
entirely. Transcripts live under `~/.claude/projects/`.

### Termination

The schema states it outright: "The CLI emits exactly one result message per turn, after
that turn's assistant, user and stream_event messages; treat it as the turn-complete signal
(informational system messages such as task notifications, session state changes or prompt
suggestions may still follow it)." So `result` is the terminator but not necessarily the
last line. `result.subtype` is `success` or one of `error_during_execution`,
`error_max_turns`, `error_max_budget_usd`, `error_max_structured_output_retries`. The
success variant carries `is_error`, which can be true when a turn ends on an API error.
`terminal_reason` refines this further, `completed` in my run.

## cursor-agent 2025.10.02

### Invocation

`cursor-agent -p "<prompt>" --output-format stream-json [--stream-partial-output] [-f]`.
Both `--output-format` and `--stream-partial-output` hard-error if `--print` is absent.

### Event vocabulary

The entire print-mode emitter is one function in the bundle. Four types, and that is all:

1. `{"type":"system","subtype":"init","apiKeySource","cwd","session_id","model","permissionMode":"default"}`
2. `{"type":"user","message":{"role":"user","content":[{"type":"text","text":<prompt>}]},"session_id"}`
3. `{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":...}]},"session_id"}`
4. `{"type":"tool_call","subtype":"started"|"completed","call_id","tool_call":<object>,"session_id"}`
5. `{"type":"result","subtype":"success","duration_ms","duration_api_ms","is_error":false,"result":<full text>,"session_id"[,"request_id"]}`

Two traps in that list. `permissionMode` is a hardcoded literal `"default"`, not the mode
actually in force, so it lies whenever you pass `-f`. And `duration_api_ms` is assigned the
same value as `duration_ms`, so it carries no API timing at all.

Without `--stream-partial-output`, assistant text buffers and flushes at tool-call
boundaries and at the end. With it, every delta becomes its own `assistant` frame.

### Reasoning

Not exposed, and the omission is deliberate. The event handler receives thinking events from
the model and discards them:

```js
case"thinking-delta":break;case"thinking-completed":break
```

The reasoning exists on the wire inside the process and never reaches stdout. No flag turns
it on.

### Tool calls

`tool_call` fires twice per call with the same `call_id`, once `started` and once
`completed`, each carrying the full `tool_call` object. Results are inside that object on the
completed frame. There is no separate tool-result event.

### Permissions

There is no way to answer a permission prompt from a parent. The provider is selected by one
line:

```js
let W = Q() ? new uje : A ? new cBe : S;
```

`Q()` is true when `--force` is set or `approvalMode` is `unrestricted`, giving `uje`, which
resolves `{approved: true}` for everything. Otherwise, in print mode, you get `cBe`, which
resolves `{approved: false}` for everything. Silently. No event, no stderr line, nothing in
the stream to say a tool was refused. **Hydra must pass `-f` to cursor-agent or every gated
tool fails invisibly.**

Worth knowing: cursor-agent reads `.claude/settings.json` from the repo root and from
`~/.claude/` for its permission allow/deny lists, merging them with its own config.

### Session and resume

The session id is `resumeChatId || crypto.randomUUID()`, minted locally before any network
call. State goes to a SQLite file at `~/.cursor/chats/<md5(cwd)>/<chat-id>/store.db`.
Because the directory is keyed by an MD5 of the working directory, **a resume only works
from the same cwd**. To fix an id in advance you must call `cursor-agent create-chat` first
and pass the returned id to `--resume`.

### Termination

On success it writes `result` and calls `process.exit(0)`. On any thrown error it calls a
helper that writes a plain-text message to stderr and exits 1, emitting no JSON at all. A
`SIGINT` abort exits 130 the same way. So **cursor-agent's failure path produces no terminal
frame**, and `result.subtype` is only ever `"success"` in the shipped code.

### ACP

There is none in this build. I checked for `agent-client-protocol`, `session/prompt`,
`session/update`, `sessionUpdate`, `session/new`, `session/request_permission` and
`requestPermission`. Zero hits on every one. The ticket's premise that cursor-agent has an
ACP mode does not hold for 2025.10.02.

## gemini 0.42.0

### Invocation

`gemini -p "<prompt>" -o stream-json`. Approval behaviour is set by `--approval-mode
default|auto_edit|yolo|plan` or `-y`, plus `--policy <files>` for rule files.

### Event vocabulary

Six types, from `packages/core/src/output/types.ts`:

| type | fields |
|---|---|
| `init` | `timestamp`, `session_id`, `model` |
| `message` | `timestamp`, `role` (`user`\|`assistant`), `content`, `delta` |
| `tool_use` | `timestamp`, `tool_name`, `tool_id`, `parameters` |
| `tool_result` | `timestamp`, `tool_id`, `status` (`success`\|`error`), `output`, `error` |
| `error` | `timestamp`, `severity` (`warning`\|`error`), `message` |
| `result` | `timestamp`, `status` (`success`\|`error`), `stats`, `error` |

Every frame carries an ISO `timestamp`, which none of the other three do. Only `init` carries
`session_id`, so a recorder must latch it from the first line.

`stats` is the richest usage block of the four after Claude:
`total_tokens, input_tokens, output_tokens, cached, input, duration_ms, tool_calls`, plus a
`models` map giving the same token breakdown per model. No cost figure anywhere.

### Reasoning

Not in `stream-json`. The non-interactive loop switches on `content`, `tool_call_request`,
`loop_detected`, `max_session_turns`, `agent_execution_blocked`, `agent_execution_stopped`,
`error` and `invalid_stream`. There is no `thought` branch, so thoughts are dropped.

Over ACP they are exposed. The ACP session handler has an explicit `case "thought"` that
formats `**{subject}**\n{description}` and sends it as `sessionUpdate:
"agent_thought_chunk"`. The model config confirms the data is requested:
`thinkingConfig.includeThoughts: true` on the `chat-base` alias. So for Gemini, reasoning is
an ACP-only field.

### Permissions

Non-interactive `ask_user` is fatal, not answerable:

```js
if (decision === "ask_user") {
  if (!config.isInteractive()) {
    throw new Error(`Tool execution for "${name}" requires user confirmation,
      which is not supported in non-interactive mode.`);
  }
}
```

The workarounds are `--approval-mode yolo`, `--approval-mode auto_edit`, or `--policy` files.
A useful detail: when plan mode resolves to allow in a non-interactive session, the CLI
promotes the approval mode to `yolo` on its own.

**The trust trap.** My live run printed `Approval mode overridden to "default" because the
current folder is not trusted` and exited **55** (`FatalUntrustedWorkspaceError`). Passing
`-y` in an untrusted directory silently downgrades to `default`, and then every gated tool
throws. Hydra must pass `--skip-trust`, or pre-populate `~/.gemini/trustedFolders.json`.

Over ACP the full `session/request_permission` method exists, so ACP is answerable where
`stream-json` is not.

### ACP surface

`gemini --acp` speaks a complete ACP: `initialize`, `authenticate`, `session/new`,
`session/load`, `session/resume`, `session/fork`, `session/list`, `session/close`,
`session/prompt`, `session/cancel`, `session/update`, `session/request_permission`,
`session/set_mode`, `session/set_model`, `session/set_config_option`, plus `fs/read_text_file`,
`fs/write_text_file` and the full `terminal/*` family. Session updates come as
`user_message_chunk`, `agent_message_chunk`, `agent_thought_chunk`, `tool_call`,
`tool_call_update`, `available_commands_update`. Stop reasons: `end_turn`, `cancelled`,
`max_tokens`, `max_turn_requests`.

### Session and resume

`--session-id <uuid>` fixes the id for a new session. `--resume` takes `latest` or an
**index number**, not a session id, which is an awkward mismatch with `--session-id`.
`--list-sessions` and `--delete-session` round it out. Sessions live in
`~/.gemini/tmp/<sha256(project)>/chats/`, logs in `.../logs/session-<id>.jsonl`. Subagent
sessions carry a `parentSessionId`.

### Termination and exit codes

`result` is emitted on success and on most error paths, including tool-execution failure,
cancellation and turn-limit. Exit codes come from a clean `FatalError` hierarchy:

| code | meaning |
|---|---|
| 0 | success |
| 1 | generic |
| 41 | `FatalAuthenticationError` |
| 42 | `FatalInputError` |
| 44 | `FatalSandboxError` |
| 52 | `FatalConfigError` |
| 53 | `FatalTurnLimitedError` |
| 54 | `FatalToolExecutionError` |
| 55 | `FatalUntrustedWorkspaceError` |
| 130 | `FatalCancellationError` |

**But the stream is not guaranteed to be terminated.** My live run exited non-zero with a
completely empty stdout, because the auth failure happens before the stream formatter is
constructed. A recorder must treat "process exited with no `result`" as a real outcome.

### Auth is broken on this machine

`gemini` 0.42.0 with `selectedAuthType: "oauth-personal"` cannot authenticate:

```
IneligibleTierError: This client is no longer supported for Gemini Code Assist for
individuals. To continue using Gemini, please migrate to the Antigravity suite of
products: https://antigravity.google
  tierId: 'free-tier', tierName: 'Gemini Code Assist for individuals',
  reasonCode: 'UNSUPPORTED_CLIENT'
```

Google retired the individual OAuth tier for this client. Today `gemini` on this box works
only with a `GEMINI_API_KEY` or a Vertex project, which puts it on the raw-model plane, not
the agent-CLI plane that D4 reserves for subscription OAuth.

## hermes 0.14.0

### Invocation

`hermes -z "<prompt>" [-m model] [--provider p] [-t toolsets]`. There is also
`hermes chat -q "<prompt>" -Q` for a quieter interactive path.

### Event vocabulary

None. `-z` has no structured output of any kind. Its own docstring is explicit: "send a
single prompt and print ONLY the final response text to stdout. No banner, no spinner, no
tool previews, no session_id line." The implementation redirects both stdout and stderr of
the whole call tree to `/dev/null` and writes only the final string to the real stdout at the
end. `logging.disable(logging.CRITICAL)` silences every logger first.

Hermes is not a streaming provider. For Hydra it is a request/response box.

### Reasoning

Not on stdout. It is captured in the session store, which is the interesting part. See below.

### Tool calls

Not on stdout. Tool calls run, and they are recorded to SQLite, but nothing about them is
visible to a parent process during or after the run except by reading the database.

### Permissions

Forced open. `run_oneshot` sets `HERMES_YOLO_MODE=1` and `HERMES_ACCEPT_HOOKS=1` before the
agent starts, so dangerous-command approval and shell-hook approval are both bypassed and
cannot be re-enabled from the command line. The `clarify` tool gets a synthetic callback that
tells the model "no user available, make the most reasonable assumption and continue" so the
run cannot stall. Sudo prompts are gated off by never setting `HERMES_INTERACTIVE`.

Over `hermes acp`, permissions become real and answerable. The adapter builds ACP options
`allow_once`, `allow_session`, `allow_always`, `deny`, `deny_always` and maps outcomes back
to Hermes approval strings, with a 60 second timeout that defaults to deny.

### Session and resume

`-z` prints no session id, so a parent cannot learn it from the run. The session is still
written to the SQLite store, so the id is recoverable afterwards via `hermes sessions list`
or `hermes sessions export - --session-id <id>`. `--resume <id>` and `--continue [name]`
work for subsequent runs. `--pass-session-id` injects the id into the agent's system prompt,
which lets the model report it in its answer, an ugly but workable trick.

### The session store is the real record

`hermes sessions export <out.jsonl>` writes one JSON object per session with a nested
`messages` array. The schema is the richest of the four for post-hoc analysis:

- `sessions`: `id, source, user_id, model, model_config, system_prompt, parent_session_id,
  started_at, ended_at, end_reason, message_count, tool_call_count, input_tokens,
  output_tokens, cache_read_tokens, cache_write_tokens, reasoning_tokens, billing_provider,
  billing_base_url, billing_mode, estimated_cost_usd, actual_cost_usd, cost_status,
  cost_source, pricing_version, title, api_call_count, handoff_state, handoff_platform,
  handoff_error`
- `messages`: `id, session_id, role, content, tool_call_id, tool_calls, tool_name, timestamp,
  token_count, finish_reason, reasoning, reasoning_content, reasoning_details,
  codex_reasoning_items, codex_message_items, platform_message_id, observed`

Two fields deserve attention. `parent_session_id` is a native delegation link, exactly what
D7's `--parent <session-id>` needs, and no other provider has it. And Hermes distinguishes
`estimated_cost_usd` from `actual_cost_usd` with a `cost_status` and `cost_source`, which is
a finer cost model than Claude's single estimate.

### Termination

`sys.exit(run_oneshot(...))` returns 0 on any completed run and 2 for argument errors
(`--provider` without `--model`, no valid toolsets). **There is no non-zero exit for a failed
agent turn.** An empty answer writes nothing and still exits 0, so empty stdout plus exit 0
is ambiguous between "the model said nothing" and "something went wrong quietly". An
uncaught exception escapes after the stderr redirect is torn down, so it does reach the
terminal, with exit 1.

### ACP

`hermes acp` is a full ACP server intended for Zed, VS Code and JetBrains. It exposes
`initialize`, `authenticate`, `new_session`, `load_session`, `prompt`, `cancel`,
`set_session_model`, `set_session_mode`. Session updates cover agent messages, thought
chunks (`update_agent_thought_text`), tool call start and complete with raw args and
results, and native ACP plan updates translated from the `todo` tool. Session ids are plain
UUID4. Stop reasons: `end_turn`, `cancelled`, `refusal`.

Hermes' ACP is strictly better than its `-z` for Hydra's purposes. `-z` is a text pipe;
`acp` is a full event stream with answerable permissions.

## Antigravity, and what it means for the Google side

The user's Gemini access is a Gemini Pro subscription through OAuth, and Google's own error
message now redirects that path to Antigravity. So this needs answering before the harness
spec picks a Google binary.

### What Antigravity is

Antigravity is a different product line, not a newer `gemini`. `gemini` 0.42.0 is Google's
TypeScript CLI. `agy` is a 142 MB Go binary whose symbols are full of
`exa.language_server_pb.LanguageServerService`, `StartCascade`, `RecordCortexTrajectory` and
`RecordCortexCodingPlan`. "Exa" and "Cascade" are Codeium and Windsurf names, so Antigravity
descends from the Windsurf agent Google acquired, not from the Gemini CLI codebase. It runs a
local language server on random localhost ports (gRPC and HTTP) and drives a Chrome instance
for browser tooling.

It has quietly taken over `~/.gemini/` as its home. This machine's `cli.log` records a
one-way migration on 2026-06-21: `Migration marker /Users/USER/.gemini/config/.migrated
does not exist. Attempting migration from AppDataDir /Users/USER/.gemini/antigravity-cli`
followed by `Migration complete`. It also reads `~/.gemini/extensions`, so it deliberately
inherits Gemini CLI's extension directory.

Sibling, then, but a sibling that Google is steering people toward and that has already
claimed the older product's config directory. Given the `UNSUPPORTED_CLIENT` rejection on the
individual tier, treating `gemini` as long-lived would be a mistake.

Installed here: `/Users/USER/.local/bin/agy` (Go, 2026-06-21), `/Applications/Antigravity.app`
2.1.4 and `/Applications/Antigravity IDE.app` 2.0.4, plus a `google-antigravity-sdk` plugin
0.0.4 at `~/.gemini/config/plugins/`.

### `agy` has a headless mode, but no structured output

`~/.gemini/antigravity-cli/bin/` holds only `webm_encoder`, a screen-recording helper, so the
bin directory is a red herring. The real binary is on PATH. Its help (which it writes to
**stderr**, exiting 0):

```
--add-dir                       Add a directory to the workspace (repeatable)
-c, --continue                  Continue the most recent conversation
--conversation                  Resume a previous conversation by ID
--dangerously-skip-permissions  Auto-approve all tool permission requests without prompting
-i, --prompt-interactive        Run an initial prompt interactively and continue the session
--log-file                      Override CLI log file path
--model                         Model for the current CLI session
-p, --print, --prompt           Run a single prompt non-interactively and print the response
--print-timeout                 Timeout for print mode wait (default 5m0s)
--sandbox                       Run in a sandbox with terminal restrictions enabled
Subcommands: changelog, help, install, models, plugin/plugins, update
```

So `agy -p` is a real headless mode with resume by conversation id, a permission bypass and a
built-in timeout. What it does not have is **any `--output-format` flag**. I checked the
binary for `output-format`, `stream-json`, `jsonl` as an output mode, `oneshot` and ACP method
names. Nothing. The `headless` strings in the binary are Chrome headless and a
`HeadlessAuthRequired` device-code auth flow, not an agent output mode.

`agy -p` is therefore in the same class as `hermes -z`: final text on stdout, nothing else.
Its own docs confirm the TUI framing, listing only slash commands and a `settings.json`, with
no scripting section at all.

The permission model is `toolPermission` in `~/.gemini/antigravity-cli/settings.json`, one of
`always-proceed`, `request-review`, `strict`, `proceed-in-sandbox`, defaulting to
`request-review`. `--dangerously-skip-permissions` is the headless escape hatch.

### The programmatic path is a Python SDK, not a CLI

Antigravity's answer to structured output is `pip install google-antigravity`, an async
Python SDK with `Agent`, `LocalAgentConfig` and `CapabilitiesConfig`. It exposes exactly the
fields the other providers hide:

```python
async for thought in response.thoughts: ...
async for call in response.tool_calls: ...
```

Its bundled skill also names `references/observability.md` for "monitor costs, track token
usage (including thinking tokens), or build custom audit logs".

That is a good feature set and a bad shape for Hydra. D8 puts the router core in Rust and D10
makes JSONL on disk the source of truth. An async Python SDK that spawns its own compiled
runtime is neither a subprocess emitting JSONL nor something a Rust core can drive without a
Python sidecar.

### Which one honours the subscription

They split, and the split matters.

- `agy` authenticates by browser OAuth on first run and has a `useG1Credits` setting,
  "Toggles usage of Google One AI premium quotas". Google One AI Premium is what a Gemini Pro
  subscription grants. So the subscription rides `agy`, the surface with no structured output.
- The Python SDK's own skill says to check for `GEMINI_API_KEY` or a `.env` file and, if
  missing, to send the user to AI Studio for a key. It says nothing about OAuth. So the
  structured surface needs an API key and belongs on the raw-model plane under D4.
- `gemini` 0.42.0's OAuth path is dead for individuals, as shown above.

Not logged in here: `cli.log` repeats `You are not logged into Antigravity` throughout, all
three `conversations/` directories are empty, and the June 2026 session ended without a login.
So none of this is currently usable on this machine without either an `agy` login or a
`GEMINI_API_KEY`.

### Recommendation for the Google side

Do not build the Google provider on `gemini` 0.42.0's OAuth. That path returns
`UNSUPPORTED_CLIENT` today. Three options, none clean:

1. **`gemini` with a `GEMINI_API_KEY`.** Keeps the best structured output on the Google side
   (six event types, real token stats, a full ACP mode with thoughts and answerable
   permissions). Costs the subscription; this becomes raw-model plane spend.
2. **`agy -p`.** Keeps the subscription. Costs everything structured: no events, no session id
   on stdout, no tokens, no cost, no tool visibility. Same lossy tier as `hermes -z`.
3. **Antigravity Python SDK.** Best fields, wrong shape, needs an API key anyway, so it buys
   nothing over option 1 while adding a Python sidecar.

My read is that option 1 is right for now and the harness spec should target `gemini --acp`
rather than `gemini -p`, because ACP is the only Google surface that carries thoughts and
answerable permissions. Antigravity should be recorded as a watch item with a concrete
trigger: **if `agy` ever grows an `--output-format`, revisit immediately**, because it is the
only surface that pairs the subscription with the agent.

## The intersection: what all four can produce

Only five fields survive all four providers in their default headless modes, and two of those
only because Hermes' can be reconstructed after the fact:

| field | how each provides it |
|---|---|
| final assistant text | `result.result` / `result.result` / accumulated `message` deltas / stdout |
| a session id | frame field / frame field / `init` only / **DB lookup after the run** |
| wall-clock duration | `duration_ms` / `duration_ms` / `stats.duration_ms` / **parent must time it** |
| success or failure | `result.subtype` + exit / exit code only / `result.status` + exit / **exit code, unreliable** |
| the prompt | echoed as `user` / echoed as `user` / echoed as `message` role user / parent already has it |

If Hydra also drives `gemini --acp` and `hermes acp` instead of their print modes, the
intersection widens considerably: streamed assistant text, thought chunks, tool call start
and complete with args and results, and answerable permissions all become common to Claude,
Gemini and Hermes. Cursor is then the only laggard, and it stays the laggard because it has
no ACP at all.

## The gaps: lossy edges of the record contract

Named explicitly, because these become the fields the schema must mark optional and the
harness must be able to synthesise or leave null.

**1. Reasoning is available from one provider out of four in print mode.**
Claude gives full signed `thinking` blocks plus a token count. Cursor receives thinking
deltas and throws them away on purpose, with no flag to recover them. Gemini drops them from
`stream-json` but emits them over ACP. Hermes drops them from `-z` but stores
`reasoning`, `reasoning_content` and `reasoning_details` per message in SQLite, and emits
thought chunks over ACP. So `reasoning` is Claude-only on the default path, three of four if
Hydra drives ACP, and never available from Cursor.

**2. Cost in USD is Claude-only on stdout.**
`total_cost_usd` exists only on Claude. Gemini gives tokens but no price. Cursor gives
neither. Hermes has the best cost model of the four, with estimated and actual figures plus
provenance, but only in its database. Hydra will have to price Gemini and Cursor itself from
a model/token table, and it cannot price Cursor at all because Cursor emits no token counts.

**3. Token counts are missing entirely from Cursor.**
Claude and Gemini both give input, output and cache breakdowns. Hermes gives them in the DB,
including `reasoning_tokens` and cache read/write split. Cursor emits nothing. There is no
recovery path short of counting characters, so any per-run token figure for Cursor is a
guess. D12's lossless SFT and DPO export constraint is directly affected here.

**4. Permission decisions are answerable from a parent on exactly one provider.**
Claude's `can_use_tool` control request is the only two-way permission channel in print mode.
Cursor auto-denies silently, Gemini throws, Hermes forces yolo. If the record is supposed to
carry a permission event with a decision and a decider, three providers will never populate
it from their print modes. Over ACP, Gemini and Hermes both gain a real
`session/request_permission`, so the field is recoverable for them but never for Cursor.

**5. Tool results are structurally different in all four.**
Claude nests them as `tool_result` content blocks on `user` frames, with a `tool_use_id`
join key. Gemini emits a separate `tool_result` event with `tool_id`, `status`, `output` and
`error`. Cursor puts the result inside the `tool_call` object on the `completed` frame, with
no distinct result event and no error field. Hermes emits nothing. A shared record needs its
own `tool_result` shape and a per-provider adapter, and the Cursor adapter cannot report a
tool error distinctly from a tool result.

**6. Termination is not guaranteed to be a frame.**
Claude always writes exactly one `result`. Gemini usually does, but exits with an empty
stdout when it fails before the formatter exists, as it did in my live run. Cursor writes a
`result` only on success and exits 1 or 130 with plain-text stderr otherwise. Hermes writes
no terminator at all. The harness must treat process exit as the authoritative end of run and
the `result` frame as an optional enrichment, never the other way round.

**7. Exit codes are useless as a common signal.**
Gemini has nine meaningful codes, Claude effectively two, Cursor three, Hermes two, and
Hermes returns 0 for a failed agent turn. A shared `status` field must be derived
per-provider, and for Hermes it can only ever be "completed" or "crashed", never "the agent
failed".

**8. Session id availability differs at the moment you need it.**
Claude and Gemini let you mint the id before launch with `--session-id`. Cursor needs a
`create-chat` round-trip first. Hermes cannot be told an id at all and does not print the one
it used. For D7's `--parent <session-id>` threading, Hydra will have to mint its own run id
and store the provider's id as a separate, sometimes-late, sometimes-absent field.

**9. Only Claude and Hermes carry native parent/child lineage.**
Claude has `parent_tool_use_id` on subagent frames and `subagent_stats` on the result. Hermes
has `parent_session_id` in the sessions table. Gemini has `parentSessionId` internally for
subagent chats but does not surface it in `stream-json`. Cursor has nothing. Delegation
lineage must be Hydra's own field, recorded by the parent at spawn time, not read back from
the child.

**10. Timestamps per event exist on one provider.**
Gemini stamps an ISO `timestamp` on every frame. Claude has `timestamp` on some frames only.
Cursor and Hermes have none. The harness must stamp every line itself at read time, and
should keep the provider's own timestamp as a separate field when present, because the two
will differ by the pipe latency.

## Where the sources disagree

**Claude's `--help` hides working flags.** The binary registers `--permission-prompt-tool
<tool>` ("MCP tool to use for permission prompts (only works with --print)") along with
`--max-turns`, `--max-thinking-tokens`, `--thinking`, `--thinking-display`,
`--system-prompt-file`, `--task-budget`, `--append-subagent-system-prompt`,
`--resume-session-at`, `--resume-drops-turn`, `--plan-mode-instructions`,
`--plugin-dir-no-mcp` and `--rewind-files`. None appear in `claude --help`. The binary also
carries a note that `--permission-prompt-tool` is rejected on the stdio surface, because
"permission prompts reach the host over stdio; an MCP tool cannot answer them here". Treat
these as usable but unsupported.

**Cursor's `agent` subcommand help is empty.** `cursor-agent agent --help` lists only `-h`,
but the subcommand inherits every root option. The root `--help` is the accurate one. The
root help also omits `--with-diffs`, `--endpoint`, `--min-version` and several sandbox flags
that are registered with `.hideHelp()`.

**The ticket's premise about cursor-agent ACP is wrong.** There is no ACP in 2025.10.02.

**Gemini's `-y` does not always mean yolo.** `--help` describes `-y` as "Automatically accept
all actions". In an untrusted folder the CLI overrides it back to `default` and says so on
stderr. The help text does not mention the trust interaction.

**Gemini's `--resume` and `--session-id` are asymmetric.** You can set a UUID going in but
you resume by index or `latest`. The help does not flag the mismatch.

## Sources

All local and primary.

- `claude --help`, and string extraction from `/Users/USER/.local/share/claude/versions/2.1.252`
  (embedded Zod schemas with `.describe()` text intact)
- One live `claude -p --output-format stream-json` run, output in the session scratchpad
- `cursor-agent --help` and every subcommand help
- `/Users/USER/.local/share/cursor-agent/versions/2025.10.02-bd871ac/index.js`, print-mode
  emitter (`MHn`), run entry (`Jnt`), decision providers (`uje`, `cBe`, `xK`), chat dir (`Xu`)
- `gemini --help`
- `/opt/homebrew/Cellar/gemini-cli/0.42.0/libexec/lib/node_modules/@google/gemini-cli/bundle/`,
  specifically `packages/core/src/output/types.ts` and `stream-json-formatter.ts` in
  `chunk-DMWX7UP6.js`, the non-interactive runner and ACP session handler in `gemini-GHUELHBF.js`,
  the `FatalError` hierarchy in `chunk-JEW7ZIWE.js`
- One live `gemini -p -o stream-json` run that failed at auth, output in the session scratchpad
- `hermes --help` and subcommand helps
- `/Users/USER/.hermes/hermes-agent/hermes_cli/oneshot.py`, `hermes_cli/main.py`,
  `hermes_state.py` (schema at lines 186 to 250), `acp_adapter/events.py`,
  `acp_adapter/permissions.py`, `acp_adapter/server.py`
- `agy --help` and `agy help <subcommand>`
- `~/.gemini/antigravity-cli/cli.log`, `~/.gemini/antigravity/antigravity_state.pbtxt`
- `~/.gemini/antigravity-cli/builtin/skills/antigravity_guide/references/cli.md` and `sdk.md`
- `~/.gemini/config/plugins/google-antigravity-sdk/` manifest and skill
