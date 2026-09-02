# 20 - Claude models on the Google subscription, at source

Status: findings only. Ticket 20 is a grilling ticket, so nothing here decides anything. No routing
policy is recommended. Where the two surfaces differ I show the difference and leave the call to
the human.

Run 2026-09-01 on this machine. `agy` 1.1.23, `claude` 2.1.252 (Claude Code). Everything below is
either a frame I captured live, a string I read out of a shipped binary, or a third-party report
that I label as third-party.

## Summary

Both Claude ids are really there at 1.1.23 and the route works. One trivial prompt through
`agy --model claude-opus-4-6-thinking` returned a clean `SUCCESS` result frame with per-step token
counts. It cost 16,338 input tokens on the Google subscription for a two-word answer, because the
Antigravity system prompt and 56 tool schemas ship on every call.

Three things the ticket did not anticipate.

**The two Opus surfaces are not the same model.** `agy` offers Claude Opus 4.6. `claude --model
opus` on this machine resolved to `claude-opus-5` with a one-million-token context window. The
ticket frames this as one model reachable two ways. It is two model generations reachable one way
each.

**Claude Code already emits the five-hour signal, every turn, before the limit is anywhere near
hit.** `rate_limit_event` carries `rateLimitType: "five_hour"`, a `status` enum, a `resetsAt` epoch
and a `utilization` fraction per window. Ticket 20 asks for a detectable signal instead of a guess.
The signal is a first-class frame on the stream and nothing has to break for it to appear. I did
not trigger anything.

**Attribution is asymmetric and it is not a `provider` field on either side.** Claude Code answers
the question in three places on the stream. `agy` answers it nowhere on the stream, though its
process knows the answer and writes it to a log file.

---

## Gap 1: the model list

Command:

```
agy models
```

Exit 0. `Fetching available models...` goes to stderr, the list to stdout. Verbatim stdout, tab
separated, in the order printed:

```
gemini-3.7-flash-high	Gemini 3.7 Flash (High)
gemini-3.7-flash-medium	Gemini 3.7 Flash (Medium)
gemini-3.7-flash-low	Gemini 3.7 Flash (Low)
gemini-3.6-flash-high	Gemini 3.6 Flash (High)
gemini-3.6-flash-medium	Gemini 3.6 Flash (Medium)
gemini-3.6-flash-low	Gemini 3.6 Flash (Low)
gemini-3.1-pro-high	Gemini 3.1 Pro (High)
gemini-3.1-pro-low	Gemini 3.1 Pro (Low)
claude-sonnet-4-6	Claude Sonnet 4.6 (Thinking)
claude-opus-4-6-thinking	Claude Opus 4.6 (Thinking)
gpt-oss-120b-medium	GPT-OSS 120B (Medium)
```

Eleven ids. Both Claude ids ticket 18 reported are present and spelled exactly as the ticket has
them.

Two details worth noticing. `claude-sonnet-4-6` has no `-thinking` suffix in its id but its display
name is `Claude Sonnet 4.6 (Thinking)`, so the suffix is not the thinking marker it looks like.
And the list is not Google-only. `gpt-oss-120b-medium` is on the same subscription, which widens
the overlap question past Anthropic if anyone wants it widened.

The Gemini ids carry their reasoning effort in the id (`-high`, `-medium`, `-low`). The two Claude
ids do not. `agy --help` also registers a separate `--effort low|medium|high` flag. Whether
`--effort` does anything to a Claude id is untested, see the unverified section.

## Gap 2: does the route work

Command, run from an empty scratch directory so no `CLAUDE.md` or project config could load:

```
agy -p="Reply with exactly: OK" \
    --model claude-opus-4-6-thinking \
    --output-format stream-json \
    --print-timeout 120s
```

Exit 0. Empty stderr. Five NDJSON lines on stdout. The init frame, verbatim, wrapped for reading
(the 56 tool names are one array on one line in the original):

```json
{"event":"init","conversation_id":"eb70018a-fef5-46dc-899f-241902c4fcf8","init":{
 "model":"claude-opus-4-6-thinking",
 "cwd":"/private/tmp/claude-501/.../scratchpad",
 "tools":["ask_custom_permission","ask_permission","ask_question","browser_click_element",
 "browser_drag_pixel_to_pixel","browser_get_dom","browser_get_network_request","browser_input",
 "browser_list_network_requests","browser_mouse_down","browser_mouse_up","browser_move_mouse",
 "browser_press_key","browser_refresh_page","browser_resize_window","browser_scroll",
 "browser_scroll_dom","browser_select_option","browser_subagent","call_mcp_tool",
 "capture_browser_console_logs","capture_browser_screenshot","click_browser_pixel",
 "command_status","define_subagent","delete_knowledge","execute_browser_javascript",
 "find_by_name","finish","generate_image","grep_search","invoke_subagent","list_browser_pages",
 "list_dir","list_permissions","list_resources","manage_inbox","manage_subagents","manage_task",
 "multi_replace_file_content","notebook_edit","notebook_execution","open_browser_url",
 "read_browser_page","read_resource","read_url_content","replace_file_content","run_command",
 "schedule","search_web","sed_file","send_command_input","send_message","view_file","wait",
 "wait_5_seconds","write_to_file"],
 "permission_mode":"request-review"}}
```

The three middle frames and the result frame, verbatim:

```json
{"event":"step_update","step_update":{"conversation_id":"eb70018a-fef5-46dc-899f-241902c4fcf8","step_index":0,"state":"DONE","step_type":"user_input"}}
{"event":"step_update","step_update":{"conversation_id":"eb70018a-fef5-46dc-899f-241902c4fcf8","step_index":1,"state":"ACTIVE","step_type":"agent_response","text_delta":"OK"}}
{"event":"step_update","step_update":{"conversation_id":"eb70018a-fef5-46dc-899f-241902c4fcf8","step_index":1,"state":"DONE","step_type":"agent_response","text_delta":"\n","duration_seconds":2.740282,"usage":{"input_tokens":16338,"output_tokens":13,"thinking_tokens":0,"cache_read_tokens":0,"total_tokens":16351}}}
{"event":"result","result":{"conversation_id":"eb70018a-fef5-46dc-899f-241902c4fcf8","status":"SUCCESS","response":"OK\n","duration_seconds":2.815272,"num_turns":1,"usage":{"input_tokens":16338,"output_tokens":13,"thinking_tokens":0,"cache_read_tokens":0,"total_tokens":16351}}}
```

What the frames report.

**Model.** Echoed once, in `init.model`, exactly as passed. The result frame has no model field, so
a recorder must latch the model from init or lose it. This matches ticket 18's finding that only
init carries the session-level fields.

**Permission mode.** `request-review`, unchanged by the model choice. Ticket 18 established that
this mode auto-denies acting tools mid-run and surfaces the refusal as a tool `ERROR` rather than
an answerable request. My probe called no tools, so I did not re-test that behaviour on a Claude
id. It is a reasonable assumption that permission handling is a property of the harness and not the
model, but I did not verify it here.

**Token counts.** Present twice, identical on the last `step_update` and on `result`. Four
components plus a total. 16,338 input tokens for a four-word prompt. Nothing was cached
(`cache_read_tokens: 0`), which is expected on a fresh conversation and means a repeated probe
would not cost the same.

**Thinking tokens.** Reported as `0` on a model whose display name is `Claude Opus 4.6 (Thinking)`.
Either the trivial prompt produced no reasoning, or the field is not wired for Claude ids. One run
cannot tell those apart.

**Account, tier or quota.** Nothing. No field on any of the five frames names an account, a plan, a
tier, a quota or a remaining balance. See gap 4.

## Gap 3: the comparison

Command, same prompt, same empty directory:

```
claude -p "Reply with exactly: OK" --model opus --output-format stream-json --verbose
```

Exit 0. Five frames: `system/init`, `rate_limit_event`, `assistant`, `rate_limit_event`,
`result/success`. One stderr line, `Warning: no stdin data received in 3s, proceeding without it`,
which is a harness artifact of running it without redirecting stdin and not a finding.

The assistant frame, verbatim:

```json
{"type":"assistant","message":{"id":"msg_011Ced5hTJujRmkhBau6CjzY","type":"message","role":"assistant","model":"claude-opus-5","content":[{"type":"text","text":"OK"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"context_management":null,"diagnostics":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":10558,"cache_read_input_tokens":8144,"output_tokens":1,"service_tier":"standard","inference_geo":"not_available","cache_creation":{"ephemeral_1h_input_tokens":10558,"ephemeral_5m_input_tokens":0}}},"parent_tool_use_id":null,"session_id":"d8e52cff-f783-4340-9848-3002bb21ac0b","request_id":"req_011Ced5hSMN2LcgWC1Ghi8xV","uuid":"9939831f-4b99-44fe-b0a8-c977c6250b7e","timestamp":"2026-09-01T17:03:58.391Z","type":"assistant"}
```

The result frame, verbatim, formatted:

```json
{"type":"result","subtype":"success","is_error":false,"api_error_status":null,
 "result":"OK","stop_reason":"end_turn","terminal_reason":"completed","num_turns":1,
 "duration_ms":1699,"duration_api_ms":2495,"ttft_ms":1676,"ttft_stream_ms":1010,
 "time_to_request_ms":47,"queued_turn_count":0,
 "total_cost_usd":0.110709,
 "fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required",
 "permission_denials":[],
 "modelUsage":{
  "claude-opus-5":{"canonicalModel":"claude-opus-5","provider":"firstParty","costBasis":"list",
   "costUSD":0.109762,"contextWindow":1000000,"maxOutputTokens":64000,
   "inputTokens":2,"outputTokens":4,"cacheCreationInputTokens":10558,"cacheReadInputTokens":8144,
   "webSearchRequests":0},
  "claude-haiku-4-5-20251001":{"canonicalModel":"claude-haiku-4-5","provider":"firstParty",
   "costBasis":"list","costUSD":0.000947,"contextWindow":200000,"maxOutputTokens":32000,
   "inputTokens":897,"outputTokens":10,"cacheCreationInputTokens":0,"cacheReadInputTokens":0,
   "webSearchRequests":0}},
 "usage":{"input_tokens":2,"output_tokens":4,"cache_creation_input_tokens":10558,
  "cache_read_input_tokens":8144,"service_tier":"standard","speed":"standard",
  "inference_geo":"not_available","output_tokens_details":{"thinking_tokens":0},
  "server_tool_use":{"web_fetch_requests":0,"web_search_requests":0},
  "cache_creation":{"ephemeral_1h_input_tokens":10558,"ephemeral_5m_input_tokens":0},
  "iterations":[{"type":"message","input_tokens":2,"output_tokens":4,
   "cache_creation_input_tokens":10558,"cache_read_input_tokens":8144,
   "cache_creation":{"ephemeral_1h_input_tokens":10558,"ephemeral_5m_input_tokens":0}}]},
 "subagent_stats":{"spawned":0,"completed":0,"failed":0,"max_depth":0,
  "requested":{"foreground":0,"background":0,"unset":0},
  "killed":{"parent":0,"system":0,"user":0},
  "refused":{"budget":0,"concurrency_limit":0,"depth_limit":0},
  "started_in_background":0,"spawned_by_subagents":0,"by_type":{}},
 "session_id":"d8e52cff-f783-4340-9848-3002bb21ac0b",
 "uuid":"c9e05220-c4c4-4bfd-8022-4876e4f7858c"}
```

Init frame top-level keys, with the long arrays reduced to counts:

```
type=system  subtype=init  session_id  uuid
model="claude-opus-5"          claude_code_version="2.1.252"
cwd                            permissionMode="default"
apiKeySource="none"            output_style="default"
tools[25]                      skills[51]                slash_commands[83]
terminal_slash_commands[2]     agents[5]                 plugins[1]
mcp_servers=[]                 capabilities=["interrupt_receipt_v1",
                                             "interrupt_cancel_queued_v1","msg_lifecycle_v1"]
memory_paths={"auto":"~/.claude/projects/<slug>/memory/"}
messaging_socket_path="/tmp/cc-socks/29274.sock"
fast_mode_state="off"          fast_mode_disabled_reason="sdk_opt_in_required"
analytics_disabled=false       product_feedback_disabled=false
```

The 25 tools, verbatim:

```
Task, Bash, CronCreate, CronDelete, CronList, DesignSync, Edit, EnterWorktree, ExitWorktree,
ListAgents, Monitor, NotebookEdit, PushNotification, Read, RemoteTrigger, ReportFindings,
ScheduleWakeup, SendMessage, Skill, TaskOutput, TaskStop, ToolSearch, WebFetch, WebSearch, Write
```

### Side by side

| | `agy --model claude-opus-4-6-thinking` | `claude -p --model opus` |
|---|---|---|
| Model actually served | `claude-opus-4-6-thinking` | `claude-opus-5` |
| Context window reported | not reported | 1,000,000 |
| Max output reported | not reported | 64,000 |
| Frames this run | 5: init, 3x step_update, result | 5: system/init, 2x rate_limit_event, assistant, result/success |
| Frame envelope | `event` discriminator, id nested under a per-type key | `type` discriminator, flat, `uuid` + `session_id` on every frame |
| Per-frame uuid | no | yes |
| Per-frame timestamp | no | on `assistant`, not on all |
| Session id | `conversation_id`, on every frame | `session_id`, on every frame |
| Settable before launch | no, minted in init | yes, `--session-id <uuid>` |
| Resume | `--conversation <id>` | `--resume <id>`, `--fork-session` |
| Tool count offered | 56 | 25 |
| Tool character | browser and IDE heavy: 14 `browser_*`, subagent define/invoke, `generate_image`, `schedule`, `manage_inbox` | file, shell, web, delegation, scheduling. No browser tools in this session |
| Permission mode field | `permission_mode: "request-review"` | `permissionMode: "default"` |
| Permission from a parent | no. Ticket 18 proved the input stream rejects `permission_response` and acting tools auto-deny as tool `ERROR` | yes, `control_request/can_use_tool` over the same stream |
| Denials on the record | tool `ERROR` only | `result.permission_denials[]`, empty here |
| Reasoning exposed | `thinking_tokens` count only, `0` here | `output_tokens_details.thinking_tokens`, plus `thinking` content blocks when the model reasons |
| Token counts | input, output, thinking, cache_read, total. Per step and on result | input, output, cache_creation, cache_read, thinking, split per model, plus a per-iteration array |
| Cache write tracked | no | yes, `cache_creation_input_tokens`, split 1h and 5m |
| Cost in USD | none | `total_cost_usd` and per-model `costUSD` with `costBasis` |
| Second model in the run | none visible | yes, `claude-haiku-4-5` ran alongside, 897 input tokens |
| Input tokens, this prompt | 16,338 | 2 billed input, 10,558 cache write, 8,144 cache read |
| Subagent lineage | `define_subagent`/`invoke_subagent` tools exist, nothing on the stream | `parent_tool_use_id` per frame, `subagent_stats` on result |
| Terminator | one `result` with `status: "SUCCESS"` | one `result` with `subtype` and `terminal_reason` |
| Timing detail | `duration_seconds` | `duration_ms`, `duration_api_ms`, `ttft_ms`, `ttft_stream_ms`, `time_to_request_ms` |

The token comparison is not like for like and should not be read as one. `agy` shipped 16,338
tokens of system prompt and 56 tool schemas uncached. Claude Code shipped a comparable bulk but
wrote 10,558 of it to a one-hour cache and read 8,144 from an existing one, so its billed input was
2. The `total_cost_usd` of $0.110709 is a list-price figure Claude Code computes for a run that a
subscription paid for, not money that moved.

### One discrepancy with research/01

Research 01's intersection table says the prompt comes back "echoed as `user`" on Claude. This run
emitted no `user` frame at all. `user` frames carry the `tool_result` blocks answering `tool_use`
blocks, and this run used no tools, so there was nothing to echo. Whether a plain text prompt is
ever echoed as a `user` frame is worth rechecking before the record contract leans on it.

## Gap 4: attribution

The ticket says `provider` alone stops being enough to say which subscription paid. Here is what
each stream can actually answer.

### Claude Code answers it, in three independent places

**1. `apiKeySource` on the init frame.** Value here is `"none"`, meaning no API key was in play and
the session ran on OAuth. Ticket 08 already established there is no Anthropic API key on this
machine, so `apiKeySource: "none"` plus a successful run is a subscription run.

**2. `modelUsage[*].provider` on the result frame.** Value here is `"firstParty"` on both models.
Alongside it, `costBasis: "list"`, which says the cost figure is list price rather than anything
negotiated or metered.

**3. The presence of `rate_limit_event` at all.** This is the strongest one and it comes from the
binary's own schema text, not from inference. The embedded `@internal` description of the
`unifiedWindows` field, read verbatim out of `/Users/USER/.local/share/claude/versions/2.1.252`:

> @internal Per-window usage for the session (5-hour), weekly (7-day), and overage-included weekly
> (per-model bucket; present only for accounts whose responses carry that window) subscription
> rate-limit windows, as read from the anthropic-ratelimit-unified-* response headers. utilization
> is the fraction of the window used (usually 0-1, same scale as the top-level utilization field;
> values above 1 occur when usage legitimately runs past a window's cap, e.g. lower-priority
> episodes past the 5-hour limit); resetsAt is unix epoch seconds. Unlike the top-level
> status/utilization fields, which describe the currently limiting window, both windows are tracked
> on every observation, and events are emitted when a window's rounded percentage or reset time
> moves, not only on status transitions. Windows absent from the account state are absent here.
> Absent until the first response carrying these headers is observed, and always absent for
> API-key, Bedrock, and Vertex sessions, whose responses do not carry them.

The last clause is the finding. `unifiedWindows` is present only on a claude.ai subscription
session and absent on API-key, Bedrock and Vertex sessions. A second embedded description confirms
the type is scoped that way: `Rate limit information for claude.ai subscription users.`

So on the Claude side the record can carry the answer, and it can carry it three ways. None of the
three names the account. They classify the auth plane, not the human.

Command to reproduce the schema reads:

```
strings -n 6 /Users/USER/.local/share/claude/versions/2.1.252 > claude-strings.txt
grep -n "Rate limit information for claude.ai" claude-strings.txt
grep -n "always absent for API-key, Bedrock, and Vertex" claude-strings.txt
```

### agy does not answer it on the stream

No field on any of the five frames names an account, plan, tier or quota. The process knows the
answer and keeps it off the stream. Two pieces of evidence.

**The binary carries a tier vocabulary.** From `strings -n 5 /Users/USER/.local/bin/agy`:

```
G1_PRO_TIER   G1_ULTRA_TIER   LEGACY_TIER   STANDARD_TIER   FREE_TIER
GCP_ENTERPRISE_TIER   GCP_GE_EDU_PRO_TIER
userTierId    useG1Credits    "Models & Quota"    "Quota:%.0f%%"
AGY_CLI_HIDE_ACCOUNT_INFO     "Account ineligible: %s"
```

`G1_PRO_TIER` is the Google One AI Premium bucket that ticket 18 tied to the subscription.
`AGY_CLI_HIDE_ACCOUNT_INFO` implies account info is shown somewhere by default, and `Models &
Quota` and `Quota:%.0f%%` imply an interactive view of it. None of it reached print mode here.

**The log carries the identity, the stream does not.** `~/.gemini/antigravity-cli/cli.log` recorded
this line at the moment my probe authenticated, quoted with the address redacted:

```
applyAuthResult: email=<the Google account address>, authMethod=consumer, quotaProject=
```

Followed within two seconds by three `quota_manager.go:45] doRefreshQuota: starting reload
(force=true)` lines. So `agy` resolves an account, classifies it as `consumer`, and refreshes a
quota object per run. Command to reproduce, read-only:

```
grep -iE "tier|G1_|quota|useG1|account|subscription" ~/.gemini/antigravity-cli/cli.log | tail -20
```

For anyone weighing whether Hydra could recover attribution from `agy` some other way: the
per-conversation store at `~/.gemini/antigravity-cli/conversations/<conversation_id>.db` is
SQLite with protobuf blobs. Its tables are `trajectory_meta`, `steps`, `gen_metadata`,
`executor_metadata`, `parent_references`, `trajectory_metadata_blob`, `battle_mode_infos`. I copied
the `.db` out and read it. The only readable strings are the conversation id, a `cascade_id`, a
second uuid and the literal `default-cli-project`. No model, no tier, no account. That is weak
evidence rather than proof, because I copied the `.db` without its `-wal`, so step payloads may not
have been in the file I read.

### The shape of the gap

| question | `claude` | `agy` |
|---|---|---|
| Which auth plane paid | yes, `apiKeySource` and `modelUsage[*].provider` | no |
| Which subscription tier | implied by `rate_limit_event` existing at all | no, though `userTierId` exists in the process |
| Which account | no, not on the stream | no, but the CLI log names the email |
| Which model | yes, on `assistant` and in `modelUsage` | init only |

## Gap 5: the five-hour limit signal

I did not trigger anything. Every claim here comes from a live frame observed at normal
utilization, from the binary's embedded schema, or from a third-party bug report that I label as
such.

### The signal already exists on the stream, at rest

Both `rate_limit_event` frames from the gap 3 run, verbatim:

```json
{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"five_hour","resetsAt":1788292800,"isUsingOverage":false,"overageStatus":"rejected","overageDisabledReason":"org_level_disabled","unifiedWindows":{"five_hour":{"utilization":0.36,"resetsAt":1788292800},"seven_day":{"utilization":0.24,"resetsAt":1788415200}}},"session_id":"d8e52cff-f783-4340-9848-3002bb21ac0b","uuid":"21d595d2-15d7-46a4-8577-64b70ba38d76"}
{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"five_hour","resetsAt":1788292800,"isUsingOverage":false,"overageStatus":"rejected","overageDisabledReason":"org_level_disabled","unifiedWindows":{"five_hour":{"utilization":0.37,"resetsAt":1788292800},"seven_day":{"utilization":0.24,"resetsAt":1788415200}}},"session_id":"d8e52cff-f783-4340-9848-3002bb21ac0b","uuid":"f6b113a9-2eda-4446-9a0f-0bfd5d5537e3"}
```

Read: the five-hour window was 36% used at the start of a two-second run and 37% at the end,
resetting at epoch 1788292800, which is 2026-09-01 15:00 local. The seven-day window sat at 24%,
resetting 2026-09-03 01:00 local. Two frames fired inside one trivial turn, because the schema says
events fire when a window's rounded percentage moves, not only when status changes.

So the fallback condition ticket 20 wanted does not need the limit to be hit. `utilization` is a
continuous reading available on every turn. `resetsAt` gives the window boundary.

### The vocabulary

`status`, three values. All three appear as literals in the binary and all three are named in one
embedded `@internal` description, quoted verbatim:

> While set, overageStatus allowed / allowed_warning means paid extra usage covers the overflow
> (nothing will be cut off). Stays true at hard exhaustion (status rejected); combine with status
> instead of reading this field alone as "still usable".

So: `allowed`, `allowed_warning`, `rejected`. The last is hard exhaustion.

`rateLimitType`, observed once as `five_hour`. These literals sit in the binary near the rate-limit
code and match what a third-party issue reports as the type union: `five_hour`, `seven_day`,
`seven_day_opus`, `seven_day_sonnet`, `seven_day_oauth_apps`, `seven_day_overage_included`. I saw
only `five_hour` live, so treat the rest as read from strings rather than observed.

`overageDisabledReason`, observed as `org_level_disabled`. `out_of_credits` also exists in the
binary.

The whole thing derives from response headers, named in the binary:
`anthropic-ratelimit-unified-status`, `-utilization`, `-reset`, `-surpassed-threshold`,
`-overage-status`, `-overage-disabled-reason`, `-upgrade-paths`, `-fallback`,
`-representative-claim`.

Commands:

```
grep -n -x -E "allowed|allowed_warning|rejected|five_hour|seven_day|seven_day_opus|seven_day_sonnet|seven_day_overage_included" claude-strings.txt
grep -n "anthropic-ratelimit-unified" claude-strings.txt
```

### The exhaustion shape, third-party

I could not observe `status: "rejected"` without triggering the limit, and I did not. This shape
comes from anthropics/claude-code issue 57096, where a user hit it in `claude -p` and pasted the
output. Treat it as a report, not as something I verified:

```json
{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1778193600,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"out_of_credits","isUsingOverage":false}}

{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"You're out of extra usage · resets 6:40pm (America/New_York)"}]},"error":"rate_limit","isApiErrorMessage":true,"apiErrorStatus":429}

{"type":"result","subtype":"success","is_error":true,"api_error_status":429,"duration_ms":400}
```

Every field name in that report exists in the local 2.1.252 binary, which is as far as local
corroboration goes. Occurrence counts from `claude-strings.txt`: `api_error_status` 9,
`isApiErrorMessage` 62, `apiErrorStatus` 10, `<synthetic>` 2, `out_of_credits` 10,
`org_level_disabled` 13, `overageDisabledReason` 8.

The trap in that shape, if it holds: `subtype` stays `"success"` while `is_error` is `true`. Local
schema text agrees, describing the success variant as carrying an `is_error` that can be true when
a turn ends on an API error. A recorder keying off `subtype` alone would file an exhausted run as a
clean one. `api_error_status: 429` and `is_error: true` are the fields that separate them.

That reporter's case was an overage exhaustion (`out_of_credits`), not a plain five-hour
exhaustion, so `overageDisabledReason` will differ on this machine. This machine reports
`org_level_disabled` with `overageStatus: "rejected"` today, meaning paid overage is not available
here at all and a five-hour exhaustion would go straight to hard refusal with nothing to spill
into.

### One more surface, not on the stream

The binary carries a whole interactive auto-resume machine around this: `quota-auto-resume`,
`quota-auto-resume-credits`, `quota-auto-resume-cap`, `quota-auto-resume-cancelled`,
`quota_auto_resume_fired`, `quota_auto_resume_stale`, `quota_auto_resume_disabled`, plus strings
like `Usage limit reached, continuing automatically at`, `Automatic continue stopped after repeated
usage-limit hits`, and a `/rate-limit-options` command. Whether any of that reaches `-p` is
untested. It reads as TUI behaviour.

### One version caveat

anthropics/claude-code issue 50518 asks for per-bucket utilization to be exposed to headless SDK
consumers and was closed as not planned, on the grounds that `utilization` was missing from
`rate_limit_event` in normal operation. On 2.1.252 it is present, inside `unifiedWindows`. So that
issue is stale against this build, and the field's availability is version-sensitive enough that
anything built on it should pin a version.

Separately, anthropics/claude-agent-sdk-python issue 599 reports that the Python SDK's
`message_parser.py` raised `MessageParseError` on `rate_limit_event` and crashed consumers even at
`status: "allowed"`. Hydra parses the stream itself under D8 and D10, so it is not exposed to that,
but it is a warning that the frame is new enough for parsers to trip on.

### agy has no equivalent

Nothing in the `agy` stream carries a quota reading, a limit type, a reset time or a utilization
figure. The process refreshes a quota object per run (`quota_manager.go` in the log) and the binary
has a `Quota:%.0f%%` renderer, so the number exists. It does not reach print mode.

## What stayed unverified

Honest list of what I could not run, and why.

**The exhausted-limit shape on this machine.** Not attempted, by instruction and by preference. The
frames in gap 5 for `status: "rejected"` are a third-party report corroborated only by field names
existing in the local binary. Nobody should build a fallback trigger on that shape without seeing
it once, and seeing it once means being rate limited.

**Whether `agy`'s permission behaviour is the same on a Claude id.** My probe called no tools, so I
did not re-observe the auto-deny that ticket 18 documented on a Gemini id. Assumed to be a harness
property, not verified.

**Whether `thinking_tokens: 0` means the field is unwired for Claude ids.** One trivial prompt is
not evidence either way. A prompt that forces reasoning would settle it in one run.

**Whether `--effort low|medium|high` does anything to a Claude id on `agy`.** The Gemini ids encode
effort in the id and the Claude ids do not, so the flag may be the only lever, or may be ignored.
Untested.

**Whether `--output-format json` on `agy` carries fields `stream-json` drops.** Untested, one more
probe would answer it.

**Cached cost on `agy`.** My run reported `cache_read_tokens: 0` on a fresh conversation. Whether a
resumed `--conversation` run reads cache, and how much of the 16,338 tokens is recoverable, is
unknown. The 16,338 figure is a cold-start number and should not be treated as the per-turn cost.

**Whether Claude Code echoes a plain text prompt as a `user` frame.** It did not here. Research 01
says it does. Neither of us ran the case that would separate a genuine echo from a `tool_result`
carrier.

**Which model `claude --model opus` resolves to over time.** It gave `claude-opus-5` today. That is
an alias, not a pin, so the comparison in gap 3 has a moving part on the Anthropic side and a fixed
id on the Google side.

**The `agy` conversation store's contents.** I read a copy of the `.db` without its `-wal`, so the
absence of model and account strings there is suggestive rather than conclusive.

## Housekeeping

Ticket 18 warned that `agy` writes relative paths into `~/.gemini/antigravity-cli/scratch/` rather
than the launch cwd. I snapshotted that directory before and after. It was empty before and is
empty after, so nothing leaked. My probe called no tools, which is the likeliest reason.

`agy` did create its normal per-run records, which I left in place rather than delete, because
deleting them means writing to `~/.gemini`:

```
~/.gemini/antigravity-cli/conversations/eb70018a-fef5-46dc-899f-241902c4fcf8.db (+ -wal, -shm)
~/.gemini/antigravity-cli/annotations/eb70018a-fef5-46dc-899f-241902c4fcf8.pbtxt
~/.gemini/antigravity-cli/presence/eb70018a-fef5-46dc-899f-241902c4fcf8.lock
~/.gemini/antigravity-cli/brain/eb70018a-fef5-46dc-899f-241902c4fcf8/
```

Worth one note for ticket 13: `agy` made a per-conversation `brain/<conversation_id>/scratch/`
directory on this run. If relative paths now resolve there rather than into the shared
`antigravity-cli/scratch/`, ticket 18's warning has moved. I did not write a file, so I did not
confirm which one wins.

Claude Code wrote its usual transcript under `~/.claude/projects/`. Nothing under `~/.claude`,
`~/.gemini` or `~/.hermes` was modified by me.

No API key was used. `GEMINI_API_KEY` was never read and no raw-model call was made (D16). Both
probes ran on already-paid subscriptions, one turn each.

## Sources

Local and primary except where marked.

- `agy models`, `agy --help`, `agy help models` on 1.1.23
- One live `agy -p --model claude-opus-4-6-thinking --output-format stream-json` run
- One live `claude -p --model opus --output-format stream-json --verbose` run on 2.1.252
- String extraction from `/Users/USER/.local/share/claude/versions/2.1.252`, embedded Zod
  `.describe()` text
- String extraction from `/Users/USER/.local/bin/agy`
- `~/.gemini/antigravity-cli/cli.log`, `~/.gemini/antigravity-cli/settings.json`, read-only
- A copy of `~/.gemini/antigravity-cli/conversations/<id>.db`, read with sqlite3
- Third party, labelled as such above: [anthropics/claude-code issue
  57096](https://github.com/anthropics/claude-code/issues/57096), [anthropics/claude-code issue
  50518](https://github.com/anthropics/claude-code/issues/50518),
  [anthropics/claude-agent-sdk-python issue
  599](https://github.com/anthropics/claude-agent-sdk-python/issues/599)
