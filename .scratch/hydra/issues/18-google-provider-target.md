# 18 - Which binary is the Google provider?

Type: grilling
Status: resolved
Blocked by: 01

## Question

Ticket 01 found that the plan's Google side does not work. `gemini` 0.42.0 with `oauth-personal` returns `IneligibleTierError: UNSUPPORTED_CLIENT`: Google retired Gemini Code Assist for individuals on this client and points users at Antigravity.

Three options, none clean. Pick one and say what it costs.

1. **`gemini` with a `GEMINI_API_KEY`.** Keeps the best structured output on the Google side: six event types, real token stats, and a full ACP mode carrying thoughts and answerable permissions. Costs the subscription. Google moves to the raw-model plane and every call is per-token spend.
2. **`agy -p`.** Keeps the subscription, since `agy` authenticates by browser OAuth and has a `useG1Credits` setting for Google One AI Premium quotas. Costs everything structured: no events, no session id on stdout, no token counts, no cost, no tool visibility. The same lossy tier as `hermes -z`.
3. **The Antigravity Python SDK.** Better fields, wrong shape, and it needs an API key anyway, so it buys nothing over option 1 while adding a Python sidecar.

Also settle:

- Whether the harness targets `gemini --acp` rather than `gemini -p`, since ACP is the only Google surface carrying thoughts and answerable permissions
- What D4 says about Google now. If the Google provider needs an API key, it sits on the raw-model plane, which contradicts the intent that interactive work rides subscriptions
- Whether it is worth running both: `agy` for interactive work on the subscription, `gemini` on a key for anything that has to be recorded properly
- The watch condition. If `agy` ever grows an `--output-format`, it becomes the only surface pairing the subscription with a structured agent, and this decision reopens

Nobody is logged into Antigravity on this machine, so option 2 needs an `agy` login before it can even be tried.

## Answer

**`agy` is the Google provider, driven as `agy -p=<prompt> --output-format stream-json`.** It rides
the Google AI Pro subscription and needs no API key. `gemini` stays specced as a cold fallback on
the AI Studio key, never used without asking (D16).

The ticket's premise died mid-session. `agy` self-updated from 1.0.10 to **1.1.23** on 2026-09-01
during the login this ticket was waiting for, and 1.1.23 has the flag the map was watching for:
`--output-format text|json|stream-json`. Option 2's entire cost was "everything structured", and
that cost is gone.

### What 1.1.23 emits

NDJSON, one event per line, verified live:

```
init         conversation_id, cwd, tools[56], permission_mode
step_update  step_index, state DONE|ACTIVE|ERROR, step_type user_input|agent_response|tool,
             tool_name, tool_info.parameters, tool_info.error, duration_seconds,
             usage{input_tokens, output_tokens, thinking_tokens, cache_read_tokens, total_tokens}
result       conversation_id, status, response, duration_seconds, num_turns, usage{...}
```

Usage arrives **per step**, with thinking and cache-read tokens separated. `gemini -o stream-json`
on the key gives init, message, result and one aggregate `stats` block, so `agy` is now the richer
Google surface as well as the cheaper one. `conversation_id` is stable and `--conversation <id>`
resumes it, which covers the session identity ticket 13 asks about.

Also new at 1.1.23: `--input-format stream-json`, `--json-schema`, `--agent`, `--mode`,
`--effort low|medium|high`, an `mcp` subcommand, and 56 tools including `define_subagent` and
`invoke_subagent`.

### The sub-questions this ticket listed

**`gemini --acp` or `gemini -p`.** Neither, for now. Google is `agy`, so the ACP work does not get
built. If the fallback is ever promoted, it is `gemini -p -o stream-json`, because ACP's advantage
was answerable permissions and the fallback is not where that matters. Ticket 01's "target ACP, not
print mode" still holds for `claude`, `cursor-agent` and `hermes`. It no longer holds for Google:
`agy`'s print mode carries more than its ACP-less state suggested.

**What D4 says about Google.** Google sits on the agent-CLI plane, subscription, no key. The
raw-model plane keeps one Google AI Studio key that Hydra does not touch by default. D4 is amended
separately: plane names the auth mechanism and nothing else.

**Whether to run both.** Yes, asymmetrically. `agy` always. `gemini` on the key only after Hydra
asks and gets a yes, because that project has billing enabled.

**The watch condition.** It fired. Replaced by two new ones, below.

### What this cost to establish

One `agy` run with three tool calls burned 45,704 tokens of subscription quota. A trivial
`gemini` prompt on the key burned 11,230 input tokens, because the CLI ships its system prompt and
tool schema every call and runs a `gemini-2.5-flash-lite` pass beside the answer model. Neither
number is a blocker. Both belong in any cost model built later.

### Findings other tickets need

- **`gemini` was never blocked by a missing key.** `GEMINI_API_KEY` is exported from `.zshrc` and
  works. `~/.gemini/settings.json` pins `selectedAuthType: oauth-personal`, so the CLI never read
  the key and returned `IneligibleTierError` instead. Proven by running it against an isolated
  `GEMINI_CLI_HOME` with `selectedAuthType: gemini-api-key`: exit 0. Ticket 12.
- **Headless `gemini` needs `--skip-trust`** or `GEMINI_CLI_TRUST_WORKSPACE=true`, otherwise it
  refuses to run in an untrusted directory. Ticket 13.
- **`agy` cannot be asked for permission.** Print mode reports `permission_mode: request-review`
  and auto-denies acting tools mid-run, surfacing the refusal as a tool `ERROR`, never as an
  answerable request. The input stream accepts exactly one message shape,
  `{"event":"user","message":{"content":"..."}}`; `permission_response` comes back as
  `warning: ignoring unsupported stream input message event`. Tickets 11 and 13.
- **`--dangerously-skip-permissions` does execute.** A probe wrote a real file. But `agy` resolved
  the relative path into `~/.gemini/antigravity-cli/scratch/`, not the launch cwd. Ticket 13 should
  not assume cwd.
- **`agy` exposes `claude-opus-4-6-thinking` and `claude-sonnet-4-6`** by id to `--model`, on the
  Google subscription. Graduated as ticket 20.

### New watch conditions

- `agy` gains a permission-response input event. Delegated `agy` runs stop being read-only, and
  ticket 11's policy answerer becomes buildable for Google.
- Google restores Code Assist eligibility for individuals on the `gemini` CLI. The subscription
  returns to `gemini` and the fallback question reopens.
