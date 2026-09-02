# 14 - What a failure looks like in the run record

Ticket: [14-ouroboros-rung-one.md](../issues/14-ouroboros-rung-one.md)
Status: findings only. Ticket 14 is a grilling ticket, so nothing here decides anything. No
threshold, no window, no definition of durable, no review policy, no reviewing model. Where the
evidence forces a choice I show the choice and leave it.

Run 2026-09-01 on this machine. `claude` 2.1.257, `cursor-agent` 2026.08.31-4057e58, `agy` 1.1.23,
`hermes` 0.14.0. Everything below is a frame I captured live, a schema string I read out of a
shipped binary, or source I read on disk. Third-party claims are labelled.

## Summary

Ticket 14 asks what counts as a failure worth learning from. Before anyone can answer that, the
record has to be able to see a failure at all. It mostly can, and the two places it cannot are
worse than expected.

**Three of the four providers report failure at the tool call and lie about it at the run level.**
`claude` closed a run with `terminal_reason: "completed"` and `is_error: false` while
`permission_denials` held a real denial. `agy` returned `status: "SUCCESS"`, an empty response and
exit 0 on a run whose only tool call was auto-denied. `cursor-agent` returned
`subtype: "success", is_error: false` on a run where one tool was rejected and another failed. Only
the per-tool frames carry the bad news. Anything that keys off the terminator will file a failed run
as a clean one, on every provider.

**Three of four providers have moved since research 01 and 20, and two of the map's settled facts
are now wrong.** `cursor-agent` 2026.08.31 emits `thinking` frames, per-frame `timestamp_ms`, real
token counts with a cache split, a `retry` frame, a `connection` frame, and a distinct `rejected`
case on tool results. Research 01 says Cursor has none of that, and ticket 09's answer gives Cursor
`usage_source: estimated` on the strength of it. That decision is now built on a fact that expired.

**The permission denial signal is real on all four but the reason behind it is not.** `agy` reports
an auto-deny and a file-not-found with the same `error.type: "TOOL_ERROR"`, separable only by
substring, and both messages contain the word "permission". `claude` has a proper denial taxonomy,
`toolDenialKind`, with `user-rejected` as its own value, and strips it from the stdout stream. It
survives only in `~/.claude/projects/**/*.jsonl`, a file Hydra does not read.

**"A session abandoned" is not derivable from the record as ticket 09 specced it.** The envelope is
`run_id`, `parent_run_id`, `seq`, `ts`, `provider`, `model`. There is no pid, no exit code and no
heartbeat, and no line is written at process exit. An abandoned run and a running run are the same
bytes on disk. A killed run left a valid, silently truncated file; a run killed early enough left
zero bytes.

**Human rejection has no producer.** Ticket 11 put children detached with permissions from a policy
and nothing escalating to a person. On top of that, the only provider that can be asked a permission
question mid-run is `claude`, and its answer carries no field naming who answered. Ticket 09's
`correction` event has no provider that emits it and no code path that writes it.

## Method

Free first, live only for holes.

- String extraction from `/Users/USER/.local/share/claude/versions/2.1.257`, which still embeds its
  Zod schemas with `.describe()` text intact. 354,837 lines.
- String extraction from `/Users/USER/.local/bin/agy` 1.1.23. 823,441 lines. Go string tables, so
  enum members concatenate; treat single names as read, not as a proven wire vocabulary.
- The `cursor-agent` bundle at
  `/Users/USER/.local/share/cursor-agent/versions/2026.08.31-4057e58/`, chunked now rather than one
  file. The print-mode emitter is in `6041.index.js`; the protobuf descriptors are in
  `159.index.js`.
- Hermes source at `/Users/USER/.hermes/hermes-agent/`: `acp_adapter/tools.py`,
  `acp_adapter/permissions.py`, `acp_adapter/server.py`, `hermes_cli/oneshot.py`.
- The user's own `~/.claude/projects/` transcripts, read-only, to see where a denial reason lands
  when it is not on the stream.

Six live probes, each one turn, each bounded by `perl -e 'alarm N; exec @ARGV'` because macOS has no
`timeout`. All writes went to `/private/tmp/hydra-t14/`. No API key was used and `GEMINI_API_KEY` was
never read, so D16 holds.

| probe | command | what it was for |
|---|---|---|
| cursor 1 | `cursor-agent -p ... --output-format stream-json` | headless with no trust flag |
| cursor 2 | same plus `--trust` | a successful edit, usage, thinking |
| cursor 3 | same plus `--trust`, shell plus a missing file | a rejected tool and a failed tool |
| cursor 4/5 | count-to-N, SIGKILL at 6s and at 16s | what an abandoned stream leaves on disk |
| agy 1 | `agy -p=... --output-format stream-json --print-timeout 150s` | the auto-deny frame verbatim |
| agy 2 | same, reading a missing file | a genuine tool error, for comparison |
| claude 1 | `claude -p ... --model haiku --disallowedTools "Write"` | a removed tool against a denied one |
| claude 2 | `claude -p ... --model haiku --permission-mode dontAsk` | a real denial, frame and result array |

One probe was blocked by this session's own classifier when it carried
`--dangerously-skip-permissions`. I did not work around it. The `agy` tool-error case was obtained
under the default `request-review` mode instead, since reading tools are not gated there.

## Gap 1: the signal table

Rows are the six signal classes the ticket names, plus four adjacent rows that gap 2 turns out to
need. Cells name the frame and the field. `-` means the provider emits nothing on the stream for
that signal. Cells marked *unverified* are read from a schema or source and were not observed live.

| signal | `claude` 2.1.257 | `cursor-agent` 2026.08.31 | `agy` 1.1.23 | `hermes` 0.14.0 |
|---|---|---|---|---|
| **tool call error** | `user` frame, content block `{type:"tool_result", is_error:true}`, plus envelope `tool_use_result` | `tool_call/completed`, `tool_call.<kind>.result.error.errorMessage` | `step_update` `state:"ERROR"`, `tool_info.error.{type,message}` | `-z`: nothing. ACP: `tool_call_update` `status:"failed"`, **re-derived by heuristic** |
| **permission denial** | `system/permission_denied` (`tool_name`, `tool_use_id`, `decision_reason_type`, `message`) **and** `result.permission_denials[]` | `tool_call/completed`, `tool_call.<kind>.result.rejected.{command,workingDirectory,reason,isReadonly}` | same as tool error: `state:"ERROR"`, `tool_info.error.type:"TOOL_ERROR"` | `-z`: bypassed, `HERMES_YOLO_MODE=1`. ACP: `session/request_permission`, shell approvals only |
| **denial reason** | `decision_reason_type` (`mode`, `rule`, `classifier`, `asyncAgent`) on the frame; `decision_reason` optional and absent in my run. `toolDenialKind` is **not on the stream** | `rejected.reason`, empty string in my run | free-text `error.message` only | ACP option id echoed back, `deny` or `deny_always` |
| **model refusal** | `system/model_refusal_fallback` and `system/model_refusal_no_fallback`, with `api_refusal_category`, `api_refusal_explanation`, `original_model`. *unverified live* | `-` | `-` | ACP `stop_reason:"refusal"` exists but means **session not found**, not a model refusal |
| **malformed output** | `terminal_reason:"malformed_tool_use_exhausted"`; `result.subtype:"error_max_structured_output_retries"`; `terminal_reason:"structured_output_retry_exhausted"`. *unverified live* | `-` | `error.message` contains `invalid tool call error (invalid_args)` on a bad call | `-` |
| **truncation / context overflow** | `system/compact_boundary` with `trigger`, `pre_tokens`, `post_tokens`, `cumulative_dropped_tokens`; `terminal_reason:"prompt_too_long"`; `stop_reason:"max_tokens"`. *unverified live* | `-` | `-` on the stream. `TRUNCATION_REASON_MAX_TOKEN_LIMIT` and `TRUNCATION_REASON_PROMPT_CACHE_TTL_EXPIRED` exist inside the binary | `-` |
| **retry** | `system/api_retry` (`attempt`, `max_retries`, `retry_delay_ms`, `error_status`), derived from `system/api_error`. In-turn retries (`malformed_tool_use_retry`, `truncated_response_recovery`) are analytics only, **not on the stream**. *unverified live* | `retry/starting` (`attempt`, `is_resume`), `retry/resuming` (`attempt`, `checkpoint_turn_count`), `connection/reconnecting` (`attempt`, `endpoint_url`), `connection/reconnected` | `-` | `-` |
| **rate limit / quota** | `rate_limit_event` every turn: `status`, `rateLimitType`, `resetsAt`, `unifiedWindows.{five_hour,seven_day}.utilization` | `-` | `-` on the stream. The process refreshes a quota object per run and the binary renders `Quota:%.0f%%` | `-` |
| **run-level failure status** | `result.is_error`, `result.subtype`, `result.terminal_reason` (19 values). **All three said clean on a run with a real denial** | `result.is_error` is hardcoded `false`, `subtype` is only ever `"success"` | `result.status`. **Returned `SUCCESS` on a fully denied run with an empty response** | `-z`: exit 0 always. ACP: `stop_reason` only ever `end_turn` or `cancelled` for a real turn |
| **terminator present** | always one `result` | success path only. Failure writes plain text to stderr and exits 1 or 130, **zero JSON** | one `result`. The workspace-trust and no-output explanations go to **stderr, not the stream** | `-z`: none. ACP: `PromptResponse` |

### Per-provider notes, with the frames

**claude.** The denial path, captured under `--permission-mode dontAsk`:

```json
{"type":"system","subtype":"permission_denied","tool_name":"Write",
 "tool_use_id":"toolu_01UJ4esfXYVa9V9bkXzB8PAR","decision_reason_type":"mode",
 "message":"Permission to use Write has been denied because Claude Code is running in don't ask mode. ..."}
```

and on the result frame:

```json
"permission_denials":[{"tool_name":"Write","tool_use_id":"toolu_01UJ...",
                       "tool_input":{"file_path":"...","content":"OK"}}],
"is_error":false,"subtype":"success","terminal_reason":"completed","num_turns":2
```

The array element is `{tool_name, tool_use_id, tool_input}` and nothing else. No reason, no decider,
no timestamp. The frame carries the reason but does not cover every denial. The binary says so
itself, verbatim from the embedded schema:

> Best-effort advisory: in rare races a denial can book without a frame or a frame can lack a
> booking twin. result.permission_denials is the authoritative record. Denials that resolve before
> canUseTool runs, PreToolUse hook denies, and deny-rule overrides of hook allow/ask decisions, are
> not covered here, and neither is the MCP --permission-prompt-tool surface.

So the reason-bearing signal is incomplete and the complete signal is reason-free. Neither is both.

A second finding from the same pair of probes: **`--disallowedTools` is not a denial.** Removing
`Write` produced no `permission_denied` frame, left `permission_denials` empty, and reached the model
as an ordinary tool error, `Error: No such tool available: Write`, with `is_error: true`. A recorder
counting denials will not see tools that were never offered.

The denial taxonomy exists and is hidden. From the binary:

```
FL=["user-rejected","permission-rule","automode-blocked","automode-unavailable",
    "automode-parsing-error","interrupted","cancelled"]
```

`toolDenialKind` carries one of these. Zero occurrences across both of my stdout captures. One
occurrence, `"permission-rule"`, in the transcript of that exact run under
`~/.claude/projects/-private-tmp-hydra-t14-p-claude/`. This user's older transcripts hold
`automode-blocked` and `permission-rule` too. Note also that the frame said
`decision_reason_type: "mode"` for the same event the transcript labelled `permission-rule`, so the
two taxonomies do not agree with each other.

`terminal_reason` is the richest run-level vocabulary of the four, 19 values, and the binary's own
`is_error` mapping splits them:

- error: `blocking_limit`, `rapid_refill_breaker`, `prompt_too_long`, `image_error`, `model_error`,
  `api_error`, `malformed_tool_use_exhausted`, `budget_exhausted`,
  `structured_output_retry_exhausted`, `tool_deferred_unavailable`, `turn_setup_failed`
- not error: `aborted_streaming`, `aborted_tools`, `stop_hook_prevented`, `hook_stopped`,
  `tool_deferred`, `max_turns`, `background_requested`, `completed`

`max_turns` sitting on the non-error side is worth a look from whoever cares about retry loops.

One more frame type that a verbatim recorder has to reckon with: `tombstone`, described as "emitted
when a previously-yielded message is superseded or removed from the transcript... Consumers that
render or persist the stream should remove the referenced message." Ticket 09 discards nothing at
write time, so a run file can hold messages the provider later retracted.

**cursor-agent.** This is the column that changed most, and it changed against research 01 in five
places. A rejected shell call and a failed read, from one run, verbatim:

```json
{"type":"tool_call","subtype":"completed","call_id":"tool_b25a...",
 "tool_call":{"shellToolCall":{"result":{"rejected":{"command":"exit 3",
   "workingDirectory":"/private/tmp/hydra-t14/p-cursor","reason":"","isReadonly":false}}}},
 "timestamp_ms":1788294876215}

{"type":"tool_call","subtype":"completed","call_id":"tool_9c3c...",
 "tool_call":{"readToolCall":{"result":{"error":{"errorMessage":"File not found"}}}},
 "timestamp_ms":1788294876221}
```

The tool result is a protobuf oneof with distinct `success`, `error` and `rejected` cases, confirmed
in the descriptors in `159.index.js`. So Cursor separates a refusal from a failure from a success,
which research 01 says it cannot. `reason` came back empty, so the why is still missing.

The full print-mode frame set at this build, read out of `6041.index.js`: `system/init`, `user`,
`assistant`, `thinking` (`delta`, `completed`), `tool_call` (`started`, `completed`), `result`,
`retry` (`starting`, `resuming`), `connection` (`reconnecting`, `reconnected`),
`interaction_query` (`request`, `response`), and `system/background_shell_timeout`
(`aborted_count`, `timeout_ms`).

`interaction_query` is not a general permission channel. Its auto-answerer is constructed with
`approveWebSearch`, `approveWebFetch`, `askQuestionRejectReason`, `webSearchRejectReason`, so it
covers ask-question, web search, web fetch and mode switching, answered locally with no way in from
a parent.

Two of research 01's traps survive: `permissionMode` is still the hardcoded literal `"default"` (my
run reported it while the real `approvalMode` vocabulary is now `allowlist | auto-review |
unrestricted`), and `duration_api_ms` is still assigned the same value as `duration_ms`, 5344 and
5344 in one run, 8177 and 8177 in the other.

New, and it belongs to ticket 13: **cursor-agent now has a workspace trust gate.** Headless in an
untrusted directory produces a plain-text banner on stderr, exit 1 and an empty stdout:

```
⚠ Workspace Trust Required
  To proceed, you can either:
    • Run 'agent' interactively to decide
    • Pass --trust, --yolo, or -f if you trust this directory
```

`--trust` alone was enough for a file write to go through under the default approval mode, so trust
and force are now separate flags with separate effects. That is Gemini's trap arriving in Cursor.

**agy.** The auto-deny, verbatim, under the default `request-review`:

```json
{"event":"step_update","step_update":{"step_index":2,"state":"ERROR","step_type":"tool",
 "tool_name":"write_to_file","duration_seconds":0.05149,
 "tool_info":{"name":"write_to_file","parameters":{"TargetFile":"/private/tmp/.../probe.txt"},
  "error":{"type":"TOOL_ERROR","message":"permission check failed for write_file \"/private/tmp/.../probe.txt\": user denied permission for write_file(/private/tmp/.../probe.txt)"}}}}
```

A genuine file-not-found, same run shape, different probe:

```json
"error":{"type":"TOOL_ERROR","message":"declaring permissions: cortex tool view_file: convert tool call for permissions: model output error: invalid tool call error (invalid_args) failed to read file: open /private/tmp/.../does-not-exist.txt: no such file or directory"}
```

Identical `state`, identical `error.type`. The only separator is message text, and **both messages
contain the word "permission"**, because `agy` wraps every tool call in a permission-declaration
step and the Go error chain keeps that frame on unrelated failures. A substring match on "permission"
would classify a missing file as a denial. The denial-specific phrase in my capture is
`user denied permission for`. One sample, and the message is not a documented interface.

The run that was entirely denied still ended:

```json
{"event":"result","result":{"status":"SUCCESS","response":"","duration_seconds":2.007579,
 "num_turns":1,"usage":{"input_tokens":14367,"output_tokens":306,"thinking_tokens":229,
 "cache_read_tokens":0,"total_tokens":14673}}}
```

Exit 0. The explanation went to stderr and nowhere else:

```
jetski: no output produced — a tool required the "write_file" permission that headless mode cannot
prompt for, so it was auto-denied.
```

If Hydra tees stdout and drops stderr, that sentence is gone and the record says SUCCESS.

Two more `agy` facts. The denial ended the turn: step 2 errored and there was no step 3, so the
second instruction never ran. And **`init.model` is absent unless `--model` is passed.** Research 20
saw a model field only because it passed one. My default run's init carried `cwd`, `tools` (57 now,
56 in research 20) and `permission_mode`, and nothing else. `conversation_id` sits at the top level
of the init frame, not inside `init`.

`agy`'s internal step-status enum has 13 members (`CORTEX_STEP_STATUS_*`: `CANCELED`, `CLEARED`,
`DONE`, `ERROR`, `GENERATING`, `HALTED`, `INTERRUPTED`, `INVALID`, `PENDING`, `QUEUED`, `RUNNING`,
`UNSPECIFIED`, `WAITING`). I observed three on the stream, `DONE`, `ACTIVE` and `ERROR`. `ACTIVE` is
not in that enum, so the CLI maps rather than passes through, and the mapping is unpublished. The
changelog string in the binary promises "a stable, closed-vocabulary `step_type` discriminator" and
does not print the vocabulary.

**hermes.** `-z` still emits nothing and still returns 0 unconditionally. `hermes_cli/oneshot.py`
redirects the whole call tree's stdout and stderr to `/dev/null`, writes the final string, and ends
`return 0`. An empty answer writes nothing and exits 0.

Over ACP it gains tool calls, thought chunks, plan updates and a permission channel, but two things
narrow what the ACP surface is worth as a failure signal.

The `failed` status is not native. `acp_adapter/tools.py::_tool_result_failed` re-derives it from
the tool's own text, and the docstring says the derivation is deliberately conservative:

> Keep this deliberately conservative. Plain text can contain words like "error" because tests
> failed or a command printed diagnostics; Zed should only receive ACP failed status for structured
> tool-level failures.

It returns true on a `Error executing tool '` prefix, on `success`/`ok` being false, on a non-zero
`exit_code`, and on `{"error": ...}` for a fixed list of polished tools. Everything else, plugin and
unknown tools included, comes back `completed`. So absence of `failed` does not mean the tool
worked.

The permission channel is narrower than "permissions". `acp_adapter/permissions.py` bridges Hermes'
**dangerous-command approval** only, with a 60 second timeout that returns `"deny"`, and any
non-`AllowedOutcome` also returns `"deny"`. It is a shell gate, not a tool gate.

And `stop_reason: "refusal"` is a false friend. `acp_adapter/server.py:1259` returns it when the
session id is not found. The only other values a real turn can end with are `end_turn` and
`cancelled` (server.py:1579). A failed agent turn returns `end_turn`.

One thing research 01 undersells: `PromptResponse` carries a `Usage` object over ACP with
`input_tokens`, `output_tokens`, `total_tokens`, `thought_tokens` and `cached_read_tokens`. Hermes
is only token-blind in `-z`.

## Gap 2: derivability of the four failure classes

Against the record as ticket 09 specced it, which is one file per run, a Hydra envelope of
`run_id`, `parent_run_id`, `seq`, `ts`, `provider`, `model` around the provider's verbatim frame,
and eleven normalized types derived at read time.

### Repeated tool errors: derivable, with one caveat per provider

Every provider that emits tool calls at all emits both the call and its outcome with a join key, so
counting errors against a tool name and its arguments is a read-time derivation over frames already
on disk. Ticket 09's `tool_call` and `tool_result` are the right normalized types and no field is
missing.

The caveats are about what an error means, not whether you can count one.

- `claude`: `is_error: true` covers a failed tool, a denied tool and a tool that does not exist.
  Three causes, one flag.
- `agy`: a denial and a failure share `state: "ERROR"` and `error.type: "TOOL_ERROR"`. Separating
  them needs a substring rule over an undocumented message, and the obvious substring is wrong.
- `cursor-agent`: cleanly separated into `error` and `rejected`. The best of the four.
- `hermes` over `-z`: **not derivable**, there are no tool frames. Over ACP: derivable but
  under-reported, per the conservative heuristic above.

Whoever answers this ticket should know that "repeated tool errors" is one number on Cursor and a
different number on `agy` for the same underlying run, and that the gap is a per-provider adapter
decision, not a threshold.

### A human rejecting a diff: not derivable

Full treatment in gap 4. In short: no provider emits a human rejection on a stream Hydra reads,
ticket 11 removed the human from the loop for children, and ticket 09's `correction` event has no
writer.

### A retry loop: split

Two different things hide under one name.

**Transport and API retries** are derivable on two providers and invisible on two. `claude` gives
`system/api_retry` with `attempt` and `max_retries`. `cursor-agent` gives `retry/starting`,
`retry/resuming` and `connection/reconnecting`, each with `attempt`. `agy` and `hermes` give
nothing, and no field would fix that because the provider does not emit one. Not derivable there,
not fixable by Hydra.

**A model looping on the same call** is derivable everywhere tool arguments are recorded, which is
`claude` (`tool_use.input`), `cursor-agent` (`tool_call.<kind>.args`), `agy`
(`tool_info.parameters`) and `hermes` over ACP (`raw_input`). Not derivable for `hermes -z`.

One retry class is invisible on the best provider. `claude` runs `malformed_tool_use_retry` and
`truncated_response_recovery` transitions inside a turn and reports them only to its own analytics
sink. They never reach the stream, so a turn that silently retried three times looks like a turn
that did not.

Turn-count ceilings are on the record: `num_turns` on `claude`, `cursor-agent` and `agy`, plus
`terminal_reason: "max_turns"` and `result.subtype: "error_max_turns"` on `claude`.

### A session abandoned: not derivable, and this is the sharp one

Ticket 13's reaping problem is still open and this is the same hole seen from the record's side.

What ticket 09 specced gives an abandoned run **nothing that distinguishes it from a running one**.
There is no pid in the envelope, no exit code anywhere, no heartbeat, and no line written at process
exit. "Process exit is the authoritative end of a run" is a rule about authority, not a field. The
`run_end` normalized type has to be derived from something, and for `hermes -z` there is no frame to
derive it from, for `cursor-agent` there is none on the failure path, and for `agy` there is one but
it says `SUCCESS`.

What that looks like on disk, measured. I ran `cursor-agent` under a pipe and sent SIGKILL:

- killed at 6 seconds: **zero bytes**. The process had not reached its `init` frame. An abandoned
  run and a run that never started are byte-identical.
- killed at 16 seconds: 1,140 bytes, every line parses, the file ends in a newline, the last frame
  is `thinking/completed`. A valid file that stops in the middle of a sentence.

So the on-disk shape of abandonment is: a file whose last line is arbitrary, with no terminator, and
no way to tell "still working" from "dead" without going outside the file. Two caveats on that
measurement. My probes wrote to a pipe and Hydra owns a pty (tickets 02 and 13), which changes
buffering, so partial lines may behave differently there. And SIGKILL is the kindest case; a
provider that dies mid-write can leave a partial line, and ticket 09 does not say what Hydra writes
when the provider's output will not parse as a frame.

`hermes -z` deserves its own sentence, because it is the case ticket 01 flagged. It writes no frames
and returns 0 on a failed turn, so an abandoned `hermes` run, a failed `hermes` run and a successful
`hermes` run that produced no text are the same record and the same exit code.

## Gap 3: the fields that would have to exist

Named for ticket 16's seam check, where every field Ouroboros reads has to be one the harness
writes. Each row says which side would write it and why the provider cannot.

| field | what it would answer | who writes it | why the provider cannot |
|---|---|---|---|
| `pid` on `run_start` | is this run's process still alive | harness | no provider prints its own pid |
| a line at process exit, carrying `exit_code` and a wall-clock stamp | did this run end, and how | harness | `hermes` exits 0 on failure, `cursor-agent` writes nothing on failure, `agy` says SUCCESS regardless |
| `last_activity_ts`, or the reader deriving it from the final `ts` | how long since anything happened | harness (already implied by the per-line `ts`) | only `agy` and `cursor-agent` stamp frames, and inconsistently |
| `decider` on `permission_decision` | policy, human, or the provider itself | harness | `claude`'s `permission_denials[]` has no decider field; nobody else has the concept |
| `denial_reason` normalized | told apart a refusal from a failure | harness adapter | `agy` conflates them in free text, `cursor-agent`'s `reason` came back empty, `claude` strips `toolDenialKind` from the stream |
| a stderr channel in the record | the sentence that explains the run | harness | `agy` and `cursor-agent` both put their explanation of a failed run on stderr only |
| `skill_set` and a content hash at `run_start` | which version of a skill this run used | harness | `claude` lists skill names in `init` and no versions; nobody else lists them at all |
| `tool_inventory` at `run_start` | what the run was allowed to attempt | harness, or latched from `init` | `claude` and `agy` list tools in `init`, `cursor-agent` and `hermes` do not |
| `model` when the provider does not state one | which model produced this | harness, by passing `--model` and recording what it passed | `agy` omits `model` from `init` unless asked; `cursor-agent` reports a display name (`"Auto"`) rather than an id |
| a raw-frame parse-failure marker | the provider emitted something unparseable | harness | by definition |

Two of those are not new fields so much as gaps between two closed tickets. Ticket 11's answer
already names `exit_code`, `status`, `status_source` and `error`, but it puts them in the JSON that
`hydra run` returns, not in the run file. Ticket 09's answer does not mention them. If Ouroboros
reads run files rather than CLI output, those four fields currently exist on the wrong side of the
seam. Likewise ticket 11 specifies a `permission_decision` event with `decider: policy`, and ticket
09's event list has `permission_decision` with no fields named at all.

## Gap 4: where a human rejection could physically come from

Given ticket 11, there are four candidate sources and three of them are closed.

**1. A provider emitting a rejection on the stream Hydra tees. Closed.** No provider emits an event
meaning "a person rejected this". `claude` comes closest: it has `toolDenialKind: "user-rejected"`
in its taxonomy, and that value is set both when a person clicks deny and when an `ask` decision has
no prompt surface to reach, which is exactly the headless case. Two very different events, one
label. And it never reaches stdout regardless. `agy` does the same thing in prose, writing
"user denied permission for write_file(...)" on a run with no user anywhere near it.

**2. Hydra answering a permission request on a person's behalf. Physically possible on one provider,
and closed by policy.** `claude` alone runs a two-way control channel:
`control_request/can_use_tool` in, `control_response` with
`{behavior:"deny", message, interrupt?, toolUseID?}` out, over `--input-format stream-json`. That
control channel is the only place in the entire system where an answer can be injected into a
running agent. Ticket 18 proved `agy`'s input stream rejects `permission_response`, ticket 01 found
Cursor has no ACP and no answerable prompt, and Hermes' ACP gate covers only dangerous commands with
a 60 second deny-by-default. Ticket 11 then routed the one live channel to a policy and said nothing
escalates to a person by default. So the channel exists, it is Claude-only, and today nothing human
is on the other end of it.

**3. A person reviewing a child's worktree branch after the fact. Open, and outside every provider
stream.** Ticket 11 gives each child a git worktree on a branch named from the run id. A person
rejecting that work rejects a branch, in git, minutes or hours after the run file closed. No
provider is running at that moment. Whatever records that is Hydra writing into a closed run's file
or into a new record of its own, and ticket 09 specced neither.

**4. A person in an interactive foreground session. Open, and invisible.** In interactive mode each
vendor CLI runs its own TUI, which D3 preserves, and none of them emit `stream-json` in that mode.
`claude` writes the rejection to `~/.claude/projects/**/*.jsonl` with `toolDenialKind` attached, and
I confirmed that field lands there for a denial that also reached stdout, so the transcript is
strictly richer than the stream. Reading it means Hydra reading a second per-provider store that
only one provider has.

Stated plainly, since the ticket asks for that: **as the system is specced today, ticket 09's
`correction` event has no producer.** It was designed for D12's preference pairs, and the pair it
needs cannot be collected from any of the four providers on the path ticket 11 chose. Whether that
means a change to ticket 11, a Hydra-side review step, reading `claude`'s transcript, or accepting
that rung one has no human-rejection class, is exactly the kind of call this ticket exists to grill,
and I am not making it.

## Gap 5: what the record would let you compare across two versions of a skill

The ticket asks how to tell whether a change helped. Deciding that is the human's job. What follows
is only what the record as specced can and cannot support, so the decision is made against real
fields.

### Fields that compare cleanly, within one provider

- **Counts of events.** Tool calls, tool errors, denials, turns (`num_turns` on three of four),
  retries where the provider emits them. These are integers over frames Hydra already stores, and
  they do not depend on pricing, caching or the model's mood.
- **Whether a run reached a terminator at all.** Coarse, but it is the same test on every provider.
- **Which tools were called, and with what arguments.** Present on all four except `hermes -z`.
- **Wall-clock duration**, with the usual noise, and with the note that `cursor-agent`'s
  `duration_api_ms` is a copy of `duration_ms` and carries no API timing.

### Fields that do not compare, with the measurements

**Token counts do not compare across providers**, which research 20 already established with
`agy`'s 16,338 uncached input tokens against `claude`'s 2 billed. Two further reasons found here.
The field names differ (`input_tokens` against `inputTokens`), and so do the semantics:
`cursor-agent` computes its reported input as `max(inputTokens - cacheReadTokens - cacheWriteTokens, 0)`,
subtracting cache from the total, while `agy` reports full input with cache read alongside it.
Normalizing "input tokens" across four providers means picking one of three conventions.

**Token counts do not compare within one provider either.** Two `claude --model haiku` runs, minutes
apart, same machine, near-identical prompts:

| | run 1 | run 2 |
|---|---|---|
| `input_tokens` | 18 | 18 |
| `cache_creation_input_tokens` | 22,179 | 10,189 |
| `cache_read_input_tokens` | 21,787 | 34,257 |
| `total_cost_usd` | 0.0494 | 0.0265 |

Same model, same task family, 1.9x apart on cost, entirely because of cache state. `agy` was worse:
14,367 against 28,962 input tokens for two comparable one-tool prompts, with `cache_read_tokens: 0`
both times.

**Cost does not compare, and on the agent-CLI plane it is not money.** `total_cost_usd` is
Claude-only, it is `costBasis: "list"`, and ticket 08 established the subscription paid. `agy` and
`cursor-agent` report no cost at all.

**Model identity is not stable.** `claude --model opus` resolved to `claude-opus-5` in research 20
and `--model haiku` to `claude-haiku-4-5-20251001` in mine. Both are aliases that can move under a
fixed command line. `cursor-agent` reported `"model": "Auto"`, a display name, not an id. `agy`
omits the field entirely unless `--model` is passed.

**A run can spend on a model the task never asked for.** Research 20's opus run shows
`claude-haiku-4-5` in `modelUsage` alongside opus, 897 input tokens. Per-model attribution is in the
record, so this is visible, but a naive per-run cost or token comparison absorbs it silently.

### What makes two runs incomparable, from the fields on the record

Everything in this list is visible in the record today, or visible only if the harness records it,
and each one is a reason two runs of the same skill are not the same experiment.

- Different provider. Nothing about usage survives the crossing.
- Different model, including an alias that moved between the two runs.
- Different cache state, which is the largest single term in the measured variance above.
- Different permission mode. `claude` reports `permissionMode` in `init` and it changes what the run
  can attempt. `agy` reports `permission_mode`. `cursor-agent` reports a hardcoded lie.
- Different tool inventory. My two `claude` runs had 28 and 29 tools because one flag removed one.
  `agy` offered 57 tools where research 20 saw 56, three hours earlier the same day.
- Different skill set. `claude`'s `init` lists 51 skills by name. No version, no hash, and no other
  provider lists them at all, so "which version of the skill ran" is not on the record for anyone.
- Different provider version. Three of four binaries moved during the life of this map, and two of
  them changed their event vocabulary when they did.
- Different working tree. Ticket 11 gives each child a worktree seeded from the parent's uncommitted
  changes, so two runs of one skill start from different bytes on disk by design.

The one that should worry whoever answers this ticket most is the skill version, because it is the
independent variable of the whole experiment and it is the field nobody records.

## Contradictions with what the map and the closed tickets currently claim

Listed separately because these are amendments other tickets need, not new findings.

1. **Ticket 09's Cursor usage decision rests on an expired fact.** It gives Cursor
   `usage_source: estimated` and a tokenizer picked by model family, because "Cursor emits no token
   counts at all". `cursor-agent` 2026.08.31 emits
   `usage: {inputTokens, outputTokens, cacheReadTokens, cacheWriteTokens}` on the result frame. I
   captured it twice. `usage_source: provider` is now available for Cursor, and the estimator that
   ticket 09 designed may have nothing to do.
2. **Ticket 09's `thinking` line is wrong for Cursor.** It says `thinking` "never" populates from
   Cursor. Cursor now emits `thinking/delta` and `thinking/completed` frames in print mode, no flag
   required, and `result` optionally carries `thinking_blocks`.
3. **Research 01's gap 10 is wrong for Cursor.** It says Cursor stamps no timestamps. Cursor now puts
   `timestamp_ms` on most frames.
4. **Research 01's gap 5 is wrong for Cursor.** It says the Cursor adapter "cannot report a tool
   error distinctly from a tool result". The tool result is a oneof with separate `success`, `error`
   and `rejected` cases.
5. **Research 01's Cursor permission finding is wrong.** It says a denied tool fails "silently. No
   event, no stderr line, nothing in the stream". The `rejected` case is on the stream with the
   command, the working directory and an `isReadonly` flag.
6. **Ticket 11's Cursor conclusion may be reopenable.** It concluded Cursor "cannot be delegated tool
   work under any policy narrower than fully permissive", on the basis of a silent auto-deny. The
   deny is no longer silent, and `--trust` is now a separate, narrower flag than `-f`: my probe wrote
   a file under `--trust` alone. I did not test which tool classes `--trust` covers, so this is a
   reason to re-examine, not a finding that reverses it.
7. **Ticket 13 gains a launch flag.** `cursor-agent` refuses headless in an untrusted directory and
   needs `--trust`, `--yolo` or `-f`. That is a third provider with a mandatory launch flag, next to
   `gemini --skip-trust` and `agy -p=<prompt>`.
8. **Research 20's `agy` init reading generalizes badly.** `init.model` is present only because that
   probe passed `--model`. On a default run there is no model on the record at all.
9. **`utilization` is not monotonic.** Research 20 read `seven_day` at 0.24 at 17:03 UTC. At 20:40
   UTC the same window, same `resetsAt` of 1788415200, read 0.05. Whatever that field measures, it
   goes down as well as up inside one window, so it is not a cumulative counter.
10. **CLAUDE.md is stale on one line.** It says the map carries D1 to D15. The map carries D16, and
    it also lists `agy` where CLAUDE.md still lists `gemini` as one of the four CLIs.

## What stayed unverified

Honest list.

- **Every claude signal I could not trigger without breaking something.** `system/api_retry`,
  `system/api_error`, `model_refusal_fallback`, `model_refusal_no_fallback`, `compact_boundary`,
  `tombstone`, and every `terminal_reason` other than `completed`. All read from the embedded schema
  in 2.1.257, none observed live. The refusal frames in particular would need a prompt designed to
  be refused, which I did not write.
- **Whether `agy` emits anything at all for a model refusal, a rate limit or a context overflow.**
  Nothing in the stream vocabulary suggests it does, and I did not trigger any of them. The
  `TRUNCATION_REASON_*` enum exists in the binary with no evidence it reaches print mode.
- **The rest of `agy`'s `state` vocabulary.** I saw `DONE`, `ACTIVE`, `ERROR`. The internal enum has
  13 members and `ACTIVE` is not one of them, so a mapping exists that I did not read.
- **Whether `agy`'s denial message wording is stable.** One sample. `user denied permission for` is
  the phrase today. It is an error string, not an interface.
- **Cursor's `rejected.reason`.** Empty in my capture. Whether any code path populates it is
  unknown, and it is the difference between a usable denial reason and none.
- **What `--trust` alone permits on Cursor.** A file write went through. Shell did not. The boundary
  between `--trust` and `-f` is undetermined, and ticket 11's conclusion turns on it.
- **Hermes over ACP, live.** Everything in the hermes column past `-z` is read from
  `acp_adapter/*.py` and was not run. `hermes acp` needs a client, and I did not build one.
- **What a partial line looks like.** My SIGKILL tests left whole lines only, over a pipe. Hydra
  runs a pty, and a provider killed mid-write may behave differently. Ticket 09 does not say what
  Hydra writes when a provider's output will not parse.
- **Whether `claude` ever echoes a plain text prompt as a `user` frame.** Research 01 says it does,
  research 20 saw it not happen. My tool-using runs produced `user` frames carrying only
  `tool_result` blocks, which is consistent with research 20 and still does not run the case that
  separates the two.
- **The seam between `~/.claude/projects/` transcripts and the run record.** I confirmed
  `toolDenialKind` lands in the transcript for a run whose stdout lacked it. I did not survey what
  else is in there that the stream drops.

## Housekeeping

No API key was used. `GEMINI_API_KEY` was never read and no raw-model call was made, so D16 holds.
Every probe ran on an already-paid subscription, one turn each. Nothing under `.scratch/hydra/issues/`
was touched and no `Status:` field was set.

Providers wrote their own records, which I left in place rather than delete, since deleting means
writing into their config directories:

- `~/.gemini/antigravity-cli/conversations/` gained two conversations plus their `brain/`,
  `annotations/` and `presence/` siblings.
- `~/.claude/projects/-private-tmp-hydra-t14-p-claude/` holds two transcripts.
- `~/.cursor/chats/` gained entries for five sessions.

My own scratch, including the two string dumps and every captured frame, is under
`/private/tmp/hydra-t14/`. Nothing was written inside `~/hydra` except this file.
