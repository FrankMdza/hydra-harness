# 26 - The hook-level action gate

Ticket: [26 - The hook-level action gate, and whether it reopens ticket 11](../issues/26-hook-level-action-gate.md)
Type: research. This file reports measurements. It proposes mechanisms but decides nothing.

Measured 2026-09-03 on this machine. Written incrementally as each finding landed, so the order
below is roughly the order things were established.

## Versions at the top of this session

Read 2026-09-03 04:40 local, before any probe.

```
claude        2.1.257 (Claude Code)
cursor-agent  2026.09.02-c22c1a3
agy           1.1.24
```

All three match what the map records, so nothing self-updated between ticket 25 and this one.

## Probe hygiene

Everything created for this ticket lives in
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t26/` and is
prefixed `t26-`. No `skills/` directory was touched, because ticket 24 is probing those
concurrently. What was written to a user-scope location, and what was restored, is listed at the
end under "What this probe wrote outside the scratch directory".

## The short version

A dangerous action can be made structurally unreachable inside a child run, on all three providers.
The hook layer is a real gate, not an observer, and it is not the permission layer under another
name. On `claude` a `PreToolUse` deny holds under `--permission-mode bypassPermissions`. On `agy`
it holds while the model is otherwise free to call whatever it likes. The gate covers
shell, file writes, file reads, MCP calls and subagent spawns, all probed.

The config is tamper-proof within a run on all three, and only within a run. Each provider snapshots
its hook config at process start, so a child that rewrites the file mid-run changes nothing. A
child that rewrites it and gets reused in the same directory next time gets an open gate, so Hydra
writes the config at every spawn rather than trusting one it left behind.

Four things that will bite whoever writes the spec:

1. `agy` cannot see a worktree `hooks.json` at all unless the worktree arrives as `--add-dir`, and
   it reports `hooks: []` rather than an error.
2. `cursor-agent` silently refuses a symlinked project hook config and runs the command anyway.
3. `claude` allows the call when a hook answers in a dialect it does not recognize. `agy` blocks
   when its hook fails. Same script, opposite outcomes.
4. All three exit 0 and report success on a run where the gate refused everything.

## cursor-agent: the hook surface is much wider than the ticket assumed

Read from `~/.local/share/cursor-agent/versions/2026.09.02-c22c1a3/index.js`, the bundled
`../hooks/dist/index.js` module, byte offset around 4,975,400.

The ticket lists four events. The binary defines twenty-one:

```
beforeShellExecution   afterShellExecution
beforeMCPExecution     afterMCPExecution
beforeReadFile         afterFileEdit
beforeTabFileRead      afterTabFileEdit
beforeSubmitPrompt     afterAgentResponse    afterAgentThought
preToolUse             postToolUse           postToolUseFailure
subagentStart          subagentStop
sessionStart           sessionEnd            workspaceOpen
stop                   preCompact
```

`preToolUse` and `postToolUse` are the names that matter. Cursor grew a generic tool gate, not only
the four narrow ones.

### Cursor reads Claude Code's hook format

The same module carries a translation table from Claude Code event names to Cursor ones:

```
PreToolUse -> preToolUse            PermissionRequest -> (unsupported)
PostToolUse -> postToolUse          Notification      -> (unsupported)
UserPromptSubmit -> beforeSubmitPrompt
Stop -> stop        SubagentStop -> subagentStop
SessionStart -> sessionStart        SessionEnd -> sessionEnd
PreCompact -> preCompact
```

plus a tool-name table, `Bash -> Shell`, `Read -> Read`, `Write -> Write`, `Edit -> Write`,
`Grep -> Grep`, `WebFetch -> WebFetch`, `WebSearch -> WebSearch`, `Task -> Task`, and `Glob` is
dropped with a warning that it "is not supported in Cursor and will be ignored". A function named
`fp` sniffs a config object and returns `"cursor"`, `"claude-code"` or `"unknown"`, and `sl`
converts the Claude shape into the Cursor shape. So one hook config can in principle serve both
providers, which matters for D5 and for how many shapes Hydra has to author.

### What a Cursor hook is allowed to return

Each event has its own response validator. Only three accept a permission decision:

- `beforeShellExecution` and `beforeMCPExecution` accept `permission` of `allow`, `deny` or `ask`,
  plus `user_message` and `agent_message`.
- `beforeReadFile` accepts `allow` or `deny` only. No `ask`.
- `afterShellExecution`, `afterMCPExecution` and `afterFileEdit` validate the object and ignore any
  permission field. They are observers.

Two constants sit next to the table: a 60 second default hook timeout (`yB:60`) and a
`failClosed:false` default written into every converted Claude-format hook. Fail-closed is a real
knob, and the conversion path sets it off.

### Where Cursor looks for a hook config: seven places, and one of them is Claude's

From `190.index.js` in the same bundle, offset 81,083. The path table is built by a function that
takes the workspace root:

```
enterprise            /Library/Application Support/Cursor/hooks.json   (macOS)
                      C:\ProgramData\Cursor\hooks.json                 (win32)
                      /etc/cursor/hooks.json                           (other)
team                  <workspace>/.cursor/managed/active-team-hooks/hooks.json
user                  ~/.cursor/hooks.json
project               <workspace>/.cursor/hooks.json
claude user           ~/.claude/settings.json
claude project        <workspace>/.claude/settings.json
claude project local  <workspace>/.claude/settings.local.json
```

Three consequences worth stating separately.

**`cursor-agent` already reads this machine's `~/.claude/settings.json`.** That file is live and
holds the user's `PreToolUse` `Bash` guards, `guard_git.py`, `guard_aws.py` and `guard_slack.py`.
Cursor parses it with `parseClaudeConfig`, converts it with the Claude-to-Cursor translator, and
runs it. It also dedupes: any Claude-format hook whose `command:` or `prompt:` string matches one
already declared in the enterprise, team, user or project Cursor configs is dropped with an info
log, so declaring the same hook in both formats runs it once.

**The project config is read from the workspace root, not from cwd.** The paths are joined onto the
resolved workspace, and `cursor-agent` has both `--workspace <path-or-name>` and `--add-dir <path>`.
Ticket 10's warning about `agy` applies here too. A worktree-scoped hook config is read, and the
worktree is the right place to put one, so long as Hydra passes the worktree as the workspace.

**Project-scoped configs refuse to load through a symlink.** The loader calls
`pathContainsSymlink(path, root)` for the four project-scoped entries and pushes
`refusing to load config path that contains a symlink` when any component under the workspace root
is a link. The user and enterprise paths skip the check, because the helper returns no root for a
path outside the workspace. This collides directly with D9 and ticket 10, which inject by symlink.
Hydra must write a real hook file into the worktree, never link one in.

`loadProjectHooks` is a load-time option, default true, and setting it false drops all four
project-scoped entries while keeping enterprise, team and user. So the four in-worktree paths are
the ones a caller can turn off, and the three outside it are not.

## agy: it ships its own hook documentation inside the binary

`strings -a ~/.local/bin/agy` carries the full "Lifecycle Hooks (`hooks.json`)" reference as an
embedded document, so this is first-party documentation read out of the shipped 1.1.24 binary
rather than a guess from symbol names. It also links `https://antigravity.google/docs/hooks`.

Five events, one config file:

```
PreToolUse      before a tool step executes   matcher = tool name    grouped
PostToolUse     after a tool step completes   matcher = tool name    grouped
PreInvocation   before the model is called    no matcher             flat
PostInvocation  after tool calls finish       no matcher             flat
Stop            when the loop terminates      no matcher             flat
```

The config is a single `hooks.json` in the customization root, `.agents/hooks.json`. Handlers are
`type: "command"` only, run through `sh -c`, with the working directory set to the directory
holding `hooks.json` and a 30 second default timeout. Hooks run synchronously and block the agent
loop. Tool names are the step type lowercased with the `CORTEX_STEP_TYPE_` prefix stripped, so
`run_command`, `view_file`, `browser_*`, and the matcher is a regex.

### `PreToolUse` can hard block

Verbatim from the embedded contract. The hook reads a JSON payload on stdin and writes a JSON
object on stdout:

```json
{
  "decision": "ask",
  "reason": "Requires confirmation for test execution.",
  "permissionOverrides": ["command(npm test)"]
}
```

`decision` is required and takes four values:

- `allow` automatically allows the tool execution
- `deny` hard blocks the execution immediately
- `ask` prompts the user, respecting the "Always Allow" cache
- `force_ask` always prompts, ignoring cached permissions

So `agy` does have a deny that fires before the tool runs, and the ticket's guess about
`hook_deny_unless_prior_grant` is close but not exact. See the section below on what those three
symbols actually are.

`permissionOverrides` is an array of temporary grants in a `command(npm test)` shape. That is the
"prior grant" idea, expressed as something a hook hands back rather than something Hydra sets out
of band.

**`overwrite` is the surprising one.** A `PreToolUse` hook may return an `overwrite` object whose
keys are shallow-merged into the tool call's arguments before it runs, and the embedded doc says
"the modified tool call is what actually executes and is recorded". Its own example rewrites
`CommandLine` to `ls -la`. A hook can therefore rewrite a command rather than only refuse it, which
is a bigger capability than any of the other two providers offer, and a bigger hazard.

### The other four events

`PostToolUse` expects an empty `{}` back. It is an observer. `PreInvocation` and `PostInvocation`
can return `injectSteps`, arbitrary `toolCall`, `userMessage` or `ephemeralMessage` entries pushed
into the loop, and `PostInvocation` also takes `terminationBehavior` of `force_continue` or
`terminate`. `Stop` returns `decision: "continue"` to refuse to stop.

`terminate` on `PostInvocation` is worth noting for Hydra: it is a kill switch a hook can pull
without needing to catch the specific tool call.

### What each hook is told

Every payload carries `conversationId`, `workspacePaths`, `transcriptPath`,
`artifactDirectoryPath` and `modelName`, camelCase protojson. The transcript path for the CLI is
`<workspace>/.gemini/antigravity-cli/transcript.jsonl`, which is a second, independent way for
Hydra to reach a run's trajectory and is worth flagging to ticket 13's harvest path.

`PreToolUse` also gets `toolCall.name`, `toolCall.args` and `stepIdx`. `PostToolUse` gets `stepIdx`
and an `error` string when the tool failed.

### `agy` hook config: probed, and `--add-dir` is required again

`agy`'s changelog confirms a set of read-only slash commands answer in print mode "without starting
an agent turn, spending quota or leaving a conversation behind". `/hooks` is one of them. Every
`agy` probe below cost zero tokens, confirmed by the `usage` block coming back all zeros.

From an empty scratch workspace:

```
$ agy -p "/hooks" --output-format json
{"...","usage":{"input_tokens":0,...,"total_tokens":0},"command":{"name":"hooks","data":{"hooks":[]}}}
```

Then with `<scratch>/.agents/hooks.json` written, holding one `PreToolUse` hook named `t26-gate`
with `matcher: "*"`, still run from that directory as cwd:

```
$ agy -p "/hooks" --output-format json
... "command":{"name":"hooks","data":{"hooks":[]}}
```

**Not found.** Then the same command with the directory passed explicitly:

```
$ agy --add-dir "$PWD" -p "/hooks" --output-format json
... "hooks":[{"name":"t26-gate","enabled":true,
    "source":".../t26/.agents/hooks.json",
    "actions":[{"event":"PreToolUse","matcher":"*","type":"command",
                "command":"./t26-agy-deny.sh","timeout_seconds":10}]}]
```

So ticket 10's rule holds for hooks exactly as it holds for skills: **`agy` does not treat cwd as
the workspace, and a worktree-scoped `hooks.json` is invisible unless the worktree arrives as
`--add-dir`.** Hydra already passes `--add-dir <worktree>` for the skill root, so the hook file
rides in on the same flag, but a spec that forgets the flag gets a gate that silently does nothing.
That is the worst failure mode on this ticket: no error, no warning, `hooks: []`.

Workspace trust was not required. `~/.gemini/antigravity-cli/settings.json` was unchanged before
and after, still holding only its one unrelated `trustedWorkspaces` entry, and the hook was listed
anyway. The changelog entry about hooks "not loading after trusting a folder" describes a fixed
reload bug, not a trust precondition.

`/hooks` also reports `source`, the absolute path of the file each hook came from, and `enabled`.
That makes `agy --add-dir <worktree> -p "/hooks" --output-format json` a free preflight check Hydra
can run before every `agy` run to prove the gate is mounted. Worth writing into the harness spec.

### `agy` hooks fail closed, and that was learned the hard way

The first paid `agy` probe had a bug in my hook config, and the bug is the finding. The hook's
`command` was `./t26-agy-deny.sh`, a relative path, and the embedded doc is right that the working
directory is the directory holding `hooks.json`, which is `.agents/`, not the workspace root. The
script sat one level up, so every hook invocation exited 127.

Every tool call in the run was blocked:

```json
{"event":"step_update","step_update":{"step_index":2,"state":"ERROR","step_type":"tool",
 "tool_name":"run_command",
 "tool_info":{"parameters":{"CommandLine":"echo T26MARKER"},
   "error":{"type":"TOOL_ERROR",
     "message":"JSON hook \"jsonhook__t26-gate_PreToolUse_0_0\" failed: command failed: exit status 127, stderr: sh: ./t26-agy-deny.sh: No such file or directory"}}}}
```

`run_command`, then `list_dir`, then `view_file`. All three refused. **A broken `agy` hook denies
everything.** Fail-closed is the default and there is no flag in the config schema to turn it off,
which is the opposite of `cursor-agent`'s converted-Claude default of `failClosed:false`. For a
gate that is the right bias, and it means a Hydra hook script that crashes stops the run rather
than opening it.

Three more facts fell out of the same run.

**The denial is fully visible in `stream-json`.** It arrives as a `step_update` with
`state:"ERROR"`, the tool name, the exact arguments the model tried, and an `error.message` naming
the hook by a generated id, `jsonhook__<hookname>_<event>_<group>_<handler>`. Hydra can record who
blocked what without parsing prose. That answers the ticket's recordability question for `agy`:
yes, and with the offending arguments attached.

**The run still exits 0 and reports `status: "SUCCESS"`.** Ticket 14 predicted this shape and it
holds exactly. A run in which the gate refused every single action is indistinguishable at the
process level from one that did the work. Hydra must derive `denied` from the event stream, never
from the exit code.

**The model narrates the block and then guesses the answer.** Its final message said the command
"could not be executed because tool calls are currently being blocked by a failing `PreToolUse`
hook", and then, unprompted, offered "If executed in a standard shell without the hook error, the
output of `echo T26MARKER` would be: `T26MARKER`". The gate stopped the action. It did not stop the
model from asserting the result of the action it never took. That is worth carrying into the
run-record contract: a denied tool call plus a confident final answer is a shape Ouroboros should
be able to spot.

`init` also reports `permission_mode`, `"request-review"` by default here, alongside the full tool
list. Another free field for the record.

Cost of this probe: four model turns, about 17k cumulative input tokens and 2.6k output on the
`agy` subscription.

### `agy`: a real deny, probed

Second paid probe, hook fixed to an absolute path, `matcher: "run_command"` only, the script
echoing `{"decision":"deny","reason":"t26 gate: blocked by Hydra policy"}`.

The payload the hook received on stdin, verbatim:

```json
{"artifactDirectoryPath":"/Users/USER/.gemini/antigravity-cli/brain/6fe4138d-.../",
 "conversationId":"6fe4138d-33ba-4235-b237-d2f1b9878a7b",
 "modelName":"gemini-3.8-flash-high",
 "stepIdx":2,
 "toolCall":{"name":"run_command",
   "args":{"CommandLine":"echo T26MARKER",
           "Cwd":"/private/tmp/.../t26",
           "WaitMsBeforeAsync":5000,
           "toolAction":"Running command","toolSummary":"Echo marker"}},
 "transcriptPath":"/Users/USER/.gemini/antigravity-cli/brain/6fe4138d-.../.system_generated/logs/transcript_full.jsonl",
 "workspacePaths":["/private/tmp/.../t26"]}
```

The hook sees the full command line and the cwd the tool would run in, so a Hydra gate has enough
to decide on. And the deny landed:

```json
{"state":"ERROR","step_type":"tool","tool_name":"run_command","duration_seconds":0.49,
 "tool_info":{"parameters":{"CommandLine":"echo T26MARKER"},
   "error":{"type":"TOOL_ERROR",
     "message":"tool call denied by pre-tool hook: t26 gate: blocked by Hydra policy"}}}
```

The `reason` string travels verbatim into the tool error and into the final response text, so
Hydra can put a policy identifier in it and read it back out of the record. `status` was `SUCCESS`
and the exit code 0 again.

Two corrections to earlier readings while I am here. `modelName` in the payload is
`gemini-3.8-flash-high`, so `agy` picks the model per run and the hook is told which. And the real
`transcriptPath` is not the `<workspace>/.gemini/antigravity-cli/transcript.jsonl` the embedded doc
shows. It is
`~/.gemini/antigravity-cli/brain/<conversationId>/.system_generated/logs/transcript_full.jsonl`,
under the user's home and keyed by conversation id. Ticket 13's harvest path should use the value
the run reports, not a path built from the workspace.

Cost: one turn, 29,333 input tokens, 619 output.

### `agy` has a declarative permission policy, and headless mode says exactly what it wants

The third probe tried to get the child to overwrite its own `hooks.json`. The write never reached
the hook, because the permission layer stopped it first:

```
"error":{"type":"TOOL_ERROR","message":"permission check failed for write_file
  \".../t26/.agents/hooks.json\": user denied permission for write_file(.../t26/.agents/hooks.json)"}
```

and on stderr:

```
jetski: no output produced — a tool required the "write_file" permission that headless mode cannot
prompt for, so it was auto-denied. Add an allow-rule under permissions.allow in settings.json
(e.g. write_file(<target>)). Alternatively, re-run with --dangerously-skip-permissions to
auto-approve all tools.
```

This matters for ticket 18 independently of hooks. `agy` at 1.1.24 is not a binary with two states,
fully permissive or nothing. It has `permissions.allow` in `settings.json`, matched per tool with
arguments, and headless runs honor it. `agy`'s changelog says so twice: one entry fixes headless
runs "so they now honor persisted `settings.json` policies, including `permissions`, file access,
sandbox mode, auto-execution, and artifact review", and another adds the soft-deny plus the stderr
notice "naming the allow-rule needed to permit them". So `agy` under a narrower policy denies by
default and tells the harness the exact rule string that would have permitted the action. That is a
usable policy engine, not a dead channel.

Note also that the run's `result.response` was empty, `status` was `SUCCESS` and the exit code 0,
with the only signal on stderr. Third time on this ticket. For `agy`, stderr is a channel Hydra has
to capture, not discard.

Cost: one turn, 86,944 input tokens (85,380 of them cache reads), 2,726 output. The expensive probe
of the three, because the model spent fourteen `view_file` calls trying to understand why its write
failed.

### `agy` reads the hook config once per run, so mid-run tamper fails

The permission layer stopped the child from writing `hooks.json`, so I tested the underlying
question directly instead: **is the config re-read between tool calls?** I made the deny script
overwrite `.agents/hooks.json` with `{}` every time it fired, which is the strongest tamper a child
could manage, an attacker with write access to the gate's own config file, executing at exactly the
right moment. Then one prompt asking for two `run_command` calls.

Result: `hooks.json` on disk was `{}` after the run, the hook fired twice, and both calls were
denied.

```
hook fired: 2   ("CommandLine":"echo T26FIRST", "CommandLine":"echo T26SECOND")
step 1: TOOL_ERROR  tool call denied by pre-tool hook: t26 gate: blocked by Hydra policy
step 2: TOOL_ERROR  tool call denied by pre-tool hook: t26 gate: blocked by Hydra policy
```

**`agy` loads its hook config at run start and holds it in memory for the life of the process.**
Wiping the file mid-run changes nothing for that run. The reload the changelog mentions is tied to
"whenever workspaces change", not to a filesystem watch.

The honest limit on that claim: it makes the gate tamper-proof *within* a run and says nothing
about the next one. A child that rewrites `.agents/hooks.json` at minute one leaves a worktree
whose gate is gone at minute twenty when the next run starts in it. For Hydra that is a real
weakness only if a worktree is reused, and ticket 11 already gives each child its own. The
mitigation is to write the hook config at spawn, every time, rather than trusting one left behind.

Cost: one turn, 44,546 input tokens, 738 output.

## claude 2.1.257: `PreToolUse` deny beats `bypassPermissions`

Probed with a Hydra-owned settings file, `--settings <path>`, so nothing in `~/.claude` was
touched. The hook returns the documented shape:

```json
{"hookSpecificOutput":{"hookEventName":"PreToolUse",
  "permissionDecision":"deny",
  "permissionDecisionReason":"t26 gate: blocked by Hydra policy"}}
```

Run with `--permission-mode bypassPermissions`, the most permissive setting Claude Code has:

```
SYSTEM init permissionMode=bypassPermissions
TOOL_USE: Bash {"command":"echo T26MARKER","description":"Echo marker string"}
TOOL_RESULT is_error=True : t26 gate: blocked by Hydra policy
```

**The hook wins over the permission mode.** That is the single most useful fact on this ticket. The
permission surface and the hook surface are not two names for the same gate: a fully permissive
permission policy still runs the hooks, and a hook denial still stops the call. Everything ticket
11 concluded about the permission channel leaves the hook channel untouched.

The reason string arrives verbatim as the tool result, marked `is_error: true`, so Hydra records
the block with its own policy identifier in it.

The payload the hook receives:

```json
{"session_id":"d9ca2399-...","transcript_path":"/Users/USER/.claude/projects/<slug>/<id>.jsonl",
 "cwd":"/private/tmp/.../t26","prompt_id":"2865182a-...",
 "permission_mode":"bypassPermissions","effort":{"level":"xhigh"},
 "hook_event_name":"PreToolUse","tool_name":"Bash",
 "tool_input":{"command":"echo T26MARKER","description":"Echo marker string"},
 "tool_use_id":"toolu_01Eaek..."}
```

`tool_use_id` is the piece `agy` does not give: it ties the denial to the exact tool-use block in
the transcript, so a `permission_decision` event in the run record can point at the call it
refused rather than at a step index.

And again the run reported `subtype: "success"`, `is_error: false`, exit code 0. Three providers,
three zero exits over a blocked action.

### `--restricted` is the flag Hydra should know about

From `claude --help`, not probed:

> Restricted mode: removes the built-in tools that run commands or code (Bash, PowerShell, REPL and
> the other code-running tools) and WebFetch unless `--tools` names them, and ignores user, project
> and local settings files (managed settings and `--settings` still apply; add
> `--strict-mcp-config` to skip MCP servers too). Also confines the file tools to the working
> directories (`--add-dir` included), refuses `bypassPermissions`, and lets only a person or the
> configured permission handler approve writes to settings, git and tool-configuration files.

Three clauses matter to this ticket. It ignores user, project and local settings while honoring
`--settings`, which means Hydra's hook config becomes the only one in force and the child's own
`.claude/settings.json` in the worktree stops counting. It refuses `bypassPermissions`. And it
gates writes to settings and tool-configuration files behind the permission handler, which is the
tamper problem addressed by the vendor rather than by us.

No managed-settings file exists on this machine (`/Library/Application Support/ClaudeCode/` is
absent), so the enterprise tier is available but unused here.

Cost: one turn, 417 output tokens, 10,912 cache-creation and 26,840 cache-read input.

### `claude` also reads its hook config once per run

Same self-wiping-hook test as `agy`. The hook overwrote its own `--settings` file with `{}` on
every fire, and the run asked for two `Bash` calls.

```
settings file after the run: {}
hook fired: 2
TOOL_USE: Bash {"command":"echo T26FIRST"}   TOOL_RESULT is_error=True : t26 gate: blocked by Hydra policy
TOOL_USE: Bash {"command":"echo T26SECOND"}  TOOL_RESULT is_error=True : t26 gate: blocked by Hydra policy
```

Both denied. Claude Code snapshots its hook configuration at session start, so a mid-run rewrite of
the settings file reaches nobody. Same shape as `agy`, same caveat: it says nothing about the next
run in the same directory.

## cursor-agent: `beforeShellExecution` denies, and the worktree config is read

Probe: `<workspace>/.cursor/hooks.json` holding one `beforeShellExecution` command hook, run as
`cursor-agent --workspace "$D" -f -p "..." --output-format stream-json`. The hook returned
`{"permission":"deny","agent_message":"...","user_message":"t26 gate blocked this"}`.

```json
{"type":"tool_call","subtype":"completed",
 "tool_call":{"shellToolCall":{"result":{"rejected":{
   "command":"echo T26MARKER","workingDirectory":"",
   "reason":"Command execution was blocked by a hook: t26 gate blocked this\n\nTo view or modify configured hooks, go to Cursor Settings > Hooks.\n\nAgent note: Do not suggest workarounds to the blocked tool.",
   "isReadonly":false}}}}}
```

The denial is a first-class `rejected` result on the tool call, not an error string, and it carries
the command it refused. Cursor appends its own "Do not suggest workarounds to the blocked tool"
line to the reason, which is a nice touch the other two do not have.

The project-scoped `.cursor/hooks.json` in the workspace was found and used with no extra flag
beyond `--workspace`, unlike `agy`. Exit code 0 again.

One thing to be careful about in how this gets written up. `-p` print mode already "has access to
all tools, including write and shell" per `cursor-agent --help`, and I passed `-f` as well, yet the
`init` frame reported `permissionMode: "default"` and the tool call carried `skipApproval: false`.
So what this probe proves is that **the hook denies in print mode with `--force` passed**. Whether
`--force` was actually in force is not something the stream confirmed, and I did not spend a second
run to isolate it. The `agy` and `claude` results above are the ones that prove a hook beats an
explicitly permissive permission mode; for Cursor, treat it as strongly indicated rather than
isolated.

Also visible in the stream: `hookAdditionalContexts`, an array on every tool call, empty here. That
is where a hook's `additional_context` would ride, so Hydra gets that field for free in the record.

### cursor-agent also reads the hook config once per run

Same self-wiping test. The hook rewrote `.cursor/hooks.json` to `{"version":1,"hooks":{}}` on every
fire; two shell calls were requested.

```
.cursor/hooks.json after the run: {"version":1,"hooks":{}}
hook fired: 2
rejected: echo T26FIRST   ... blocked by a hook: t26 gate blocked this
rejected: echo T26SECOND  ... blocked by a hook: t26 gate blocked this
```

Both denied. **All three providers snapshot their hook configuration at process start.** A child
that rewrites the config mid-run changes nothing about the run it is in. This is a consistent
answer across `claude`, `cursor-agent` and `agy`, reached the same way on each, and it is the
strongest single result on this ticket.

Cursor's `beforeShellExecution` payload, for the record:

```json
{"conversation_id":"8f36ef4b-...","generation_id":"8f36ef4b-...","model":"composer-2.5",
 "command":"echo T26FIRST","cwd":"","sandbox":false,"session_id":"8f36ef4b-...",
 "hook_event_name":"beforeShellExecution","cursor_version":"2026.09.02-c22c1a3",
 "workspace_roots":["/private/tmp/.../t26"],"user_email":"...",
 "transcript_path":"/Users/USER/.cursor/projects/<slug>/agent-transcripts/<id>/<id>.json..."}
```

`sandbox: false` in the payload is useful: a hook can see whether the command was going to run
sandboxed and decide differently. `cursor_version` in every payload is a small gift given how often
this binary moves under us. And `user_email` is in there, which is worth knowing before Hydra pipes
hook payloads anywhere.

### One hook config, two providers: probed

The `.cursor/hooks.json` was deleted and replaced with a project `<workspace>/.claude/settings.json`
in Claude Code's own format, `PreToolUse` with `matcher: "Bash"`. Nothing Cursor-shaped was left on
disk.

Cursor found it, translated it, and denied:

```json
{"rejected":{"command":"echo T26CLAUDEFMT",
  "reason":"t26 claude-format gate blocked this\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "isReadonly":false}}
```

and the hook received a Claude-shaped payload with Cursor's translated names:

```json
{"tool_name":"Shell","tool_input":{"command":"echo T26CLAUDEFMT","cwd":"","timeout":30000},
 "tool_use_id":"de592c7d-...","hook_event_name":"preToolUse",
 "cursor_version":"2026.09.02-c22c1a3","workspace_roots":["/private/tmp/.../t26"]}
```

So **a single `.claude/settings.json` written into the worktree gates both `claude` and
`cursor-agent`**, with `Bash` becoming `Shell` on the way in. That is one hook script serving two
providers, and it means Hydra authors two config shapes rather than three: the Claude one for two
providers, and `agy`'s `.agents/hooks.json` for the third. Worth weighing against writing Cursor's
native format, which buys `beforeReadFile` and the other Cursor-only events that have no Claude
equivalent.

The tool-name translation is where this leaks. `Glob` has no Cursor equivalent and is dropped with
a warning. `Edit` and `Write` both translate to `Write`, so a Claude matcher that treats them
differently stops doing that on Cursor. A shared config is a lowest common denominator.

## The symlink rule splits the three providers

Ticket 10 injects the skill root by symlink. The same question for the hook config has three
different answers, and one of them is a silent gate failure.

**`cursor-agent` refuses a symlinked project hook config, and says nothing about it.** With
`<workspace>/.claude/settings.json` replaced by a symlink to the identical file one directory up,
nothing else changed:

```
hook fired: never
tool result: exitCode 0, stdout "T26SYMLINK\n"
```

The command ran. No error, no warning on stderr, no note in the stream. This is the loader code
quoted earlier doing exactly what it says, `refusing to load config path that contains a symlink`,
except the refusal goes to an internal error list that print mode never surfaces. Hydra writing a
symlinked hook config for Cursor would ship a gate that does not exist and reports success.

**`agy` follows the symlink happily.** `.agents/hooks.json` as a symlink, checked with the free
`/hooks` probe, and the hook is listed with `source` pointing at the symlink path. Zero tokens.

**`claude` follows the symlink too.** Project `.claude/settings.json` as a symlink, hook runs,
`TOOL_RESULT is_error=true : t26 symlink gate`.

So the rule for the harness spec is Cursor's, applied everywhere: write real files, never links, for
hook config. It costs nothing on the other two and it is the difference between a gate and a
placebo on Cursor.

### `claude` fails open on a malformed hook response

Found by accident, worth more than the test it broke. One run left the Cursor-dialect script wired
into Claude's project settings, so the hook printed:

```json
{"permission":"deny","agent_message":"t26 claude-format gate","user_message":"..."}
```

and exited 0. Claude Code ran the hook, wrote nothing to stderr, ignored the response it did not
recognize, and allowed the call:

```
TOOL_RESULT is_error=False : T26CCSYMLINK
```

`agy` blocks when its hook fails. `claude` allows when its hook answers in the wrong dialect.
Cursor sits between the two, with a per-hook `failClosed` flag that defaults to false on the
Claude-conversion path. **The three providers disagree about what a broken gate means**, and a
single shared hook script that emits one dialect would be a real gate on Cursor, a real gate on
`agy`, and an open door on Claude. If Hydra writes one script it has to detect which provider is
calling it, from `hook_event_name` or `cursor_version`, and answer in that provider's shape.

## `--sandbox`: a different gate, composed with hooks rather than replacing them

### `agy --sandbox`

From `agy`'s own embedded documentation, the sandbox is a command-execution confinement, not a
decision layer:

> **Standard Sandbox Mode (BypassSandbox: false)**: By default has read/write access to your
> workspace, but no network access and no access to files outside the workspace unless added
> explicitly by the user.
> **Bypass Sandbox Mode (BypassSandbox: true)**: Disables isolation, allowing network and full
> filesystem access. **Requires manual user approval.**
> The main purpose of the sandbox is to auto-run commands without needing the user's approval.

So the sandbox exists to let commands auto-run, and `BypassSandbox` is a per-call argument the model
sets on `run_command`. A `PreToolUse` hook sees that argument in `toolCall.args` and can refuse it,
which is how the two compose: **the sandbox is the confinement, the hook is what stops the child
asking to leave it.** Nothing in the model's own instructions prevents it trying, and the embedded
guidance actively tells it to retry with `BypassSandbox: true` when a command fails for what looks
like a sandbox reason.

Two hardening entries in the changelog fill in the shape: the terminal sandbox grants read-only
rather than writable access to a repository's `.git`, and `.git` is on the core dangerous-paths
list. There is also a `proceed-in-sandbox` tool execution policy that auto-approves commands running
inside the sandbox and prompts only when one tries to bypass it. For a detached child that cannot be
prompted, `proceed-in-sandbox` plus a hook that denies `BypassSandbox` is a coherent pair.

`agy`'s settings surface, from the same embedded doc, is wider than this map has recorded: a tool
execution policy of `always-proceed`, `request-review`, `strict` or `proceed-in-sandbox`;
non-workspace file access of `allow`, `ask` or `deny`; an internet access policy with the same three
values; permission grants; a command allowlist and denylist; a browser domain allowlist. Project
settings override global ones. `request-review` is the default this machine reported in `init`.

### `cursor-agent --sandbox`

`cursor-agent --sandbox <enabled|disabled>` "explicitly enable or disable sandbox mode (overrides
config)". The implementation is a separate helper binary configured through
`configureSandboxPrereqs({sandboxBinaryPath, workspaceReadEnabled})`, with a read boundary of
`workspace` or `system`, a network allowlist and denylist, and a git switch. All of those come from
`getSandboxingControls()`, which reads **team admin settings from the server**, not from a local
file. A hook payload carries `sandbox: true|false`, so a Cursor hook can also see whether the
command it is judging would have been confined.

Neither sandbox replaces hooks and neither is configured where hooks are. They are two independent
layers, and on both providers the hook is the one Hydra can author per run.

## What `hook_deny_unless_prior_grant` actually is

The three symbols the ticket found, `hook_deny`, `hook_auto_approve` and
`hook_deny_unless_prior_grant`, sit in `agy`'s Go read-only string data next to `force_ask`, and
the binary also carries `permissions.PermissionHookResult` and `permissions.PermissionGrantStore`.
They are internal labels for how the permission manager resolved a call after the hook spoke, not
config keys you can set. The observable contract is the four documented `decision` values.

Mapping them to the documented decisions, `hook_auto_approve` is `allow`, `hook_deny` is `deny`,
and `hook_deny_unless_prior_grant` is what `ask` becomes when no human can answer. That last one is
the interesting case, so I probed it.

**A: hook returns `{"decision":"ask"}`, no overrides.**

```
"error":{"type":"TOOL_ERROR","message":"permission check failed for command \"echo T26ASK\":
  user denied permission to run command:\necho T26ASK"}
stderr: jetski: no output produced — a tool required the "command" permission that headless mode
        cannot prompt for, so it was auto-denied. Add an allow-rule under permissions.allow in
        settings.json (e.g. command(<target>)).
```

Denied, as the name predicts. Note the error says "user denied", which is a lie of convenience:
no user was present. Hydra must not read that string as evidence a human refused.

**B: same hook, plus `"permissionOverrides":["command(echo T26GRANT)"]` matching the exact command.**

Also denied, with the same message. So `permissionOverrides` returned by a hook did **not** satisfy
the check in headless mode. Either the grants are consumed only by the interactive prompt path, or
the override string grammar differs from the `command(<target>)` shape the stderr notice suggests.
I did not spend further runs to separate those two, and it does not change the answer this ticket
needs: **the hook's usable decisions for a detached child are `allow` and `deny`. `ask` and
`force_ask` collapse to deny, and there is no out-of-band grant a hook can issue that reopens
them.**

That is a clean result for Hydra. It matches ticket 11's rule that nothing escalates to a human by
default, and it means an `agy` gate has exactly the two states a policy engine needs.

### `agy`'s declarative policy, read for free

`/permissions` and `/config` also answer in print mode at zero tokens.

```
$ agy --add-dir "$D" -p "/permissions" --output-format json
"permissions":[{"scope":"project"},{"scope":"shared"},{"scope":"global"}]
```

Three scopes, all empty on this machine. And `/config` returns the whole settings surface with
values, including `toolPermission: request-review`, `enableTerminalSandbox: false`,
`allowNonWorkspaceAccess: false`, `artifactReviewPolicy: asks-for-review`, `modelProvider: ""` and
`trustedWorkspaces`.

The asymmetry that matters: **`agy`'s permission scopes are not worktree-scoped.** Global lives in
`~/.gemini/antigravity-cli/settings.json`, project settings live in `~/.gemini/config/projects/<uuid>.json`
keyed by a project id rather than a path, and `~/.gemini/config/hooks.json` is the shared hook file
the `/hooks` TUI writes to. Neither exists on this machine yet. So for `agy` the hook config in
`<worktree>/.agents/hooks.json` is the only gate Hydra can author per run without writing into the
user's home, and D9's rule against writing outside a path Hydra created points straight at the hook.

## A hook does cover file writes, on all three

The ticket's worry was that "a hook on shell execution is not a hook on file writes, and
`afterFileEdit` fires after the edit". True about `afterFileEdit`, and beside the point, because
`preToolUse` covers writes.

Probed on Cursor, the provider where the doubt was strongest. Project `.claude/settings.json`,
`PreToolUse` with `matcher: "Write|Edit"`, asked to create a file and explicitly told not to use
the shell.

The hook received the file path and the full content before anything touched disk:

```json
{"tool_name":"Write",
 "tool_input":{"file_path":"/private/tmp/.../t26/t26-written-by-cursor.txt","content":"HELLO\n"},
 "tool_use_id":"tool_05c0b72b-...","hook_event_name":"preToolUse",...}
```

and the deny landed on the edit tool:

```json
"editToolCall":{"result":{"error":{"path":"",
  "error":"t26 write gate blocked this\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "modelVisibleError":"..."}}}
```

`ls t26-written-by-cursor.txt` after the run: no such file. The write never happened.

Worth noting the model's reply: "The write was blocked by a workspace gate ... If you want, you can
lift that gate or allow writes in this workspace and I can try again." It suggested lifting the gate
to a human rather than trying to lift it itself, which is the behavior you want, and is also a
reminder that the gate's `agent_message` is read by a model that will reason about it.

### Coverage, per provider

| | shell | file write | file read | MCP call | subagent spawn | network |
|---|---|---|---|---|---|---|
| `claude` | `PreToolUse` matcher `Bash` | `PreToolUse` matcher `Write\|Edit` | `PreToolUse` matcher `Read` | `PreToolUse` matcher `mcp__*` | `PreToolUse` matcher `Task` | via `WebFetch`/`WebSearch` matchers |
| `cursor-agent` | `beforeShellExecution` or `preToolUse`/`Shell` | `preToolUse`/`Write` | `beforeReadFile` or `preToolUse`/`Read` | `beforeMCPExecution` | `subagentStart` | `preToolUse`/`WebFetch` |
| `agy` | `PreToolUse` matcher `run_command` | `PreToolUse` matcher on `write_to_file`, `replace_file_content`, `sed_file`, `notebook_edit` | `PreToolUse` matcher `view_file` | `PreToolUse` matcher `call_mcp_tool` | `PreToolUse` matcher `invoke_subagent` or `define_subagent` | `PreToolUse` matcher `read_url_content`, `search_web`, `open_browser_url` |

`agy`'s tool list came back in the `init` frame of every run, 57 names, so the matcher targets are
enumerable per run at no cost. Cursor's `subagentStart` is permission-capable, so a Cursor child can
be stopped from spawning children, which is a second place ticket 11's depth cap could be enforced.

What a hook cannot see, on all three: anything the model does without a tool. Reasoning, the final
answer, and a claim about a result it never obtained. The `agy` probe above produced exactly that
last one.

## What this probe wrote outside the scratch directory

Nothing that needed a backup, because no user-scope config was ever modified. Verified after the
last probe:

```
~/.claude/settings.json            unchanged, mtime Aug 31 22:57, 1510 bytes
~/.gemini/antigravity-cli/settings.json  unchanged, mtime Sep 1 05:28, 82 bytes
~/.cursor/hooks.json               does not exist, was never created
~/.gemini/config/hooks.json        does not exist, was never created
```

Every hook config lived in the scratch workspace or was handed to `claude` with `--settings <path>`
pointing there. No `skills/` directory was touched; `~/.claude/skills` has a recent mtime from the
concurrent ticket 24 agent, not from this one. `find` over `~/.claude`, `~/.cursor` and `~/.gemini`
returns no file named `t26*`.

The three providers did each write their own session records for these runs, which is normal and
not probe residue. Left in place rather than deleted, because removing a provider's own store risks
corrupting its state:

```
~/.claude/projects/-private-tmp-...-scratchpad-t26/
~/.cursor/projects/private-tmp-...-scratchpad-t26/
~/.gemini/antigravity-cli/brain/<one uuid per agy run>/
```

Nothing was written under `~/hydra` outside `.scratch/hydra/`.

## Cost

Free: every `agy` `/hooks`, `/permissions` and `/config` probe, all `strings` and bundle reads, all
version and help output. Confirmed by the `usage` block returning zeros on each.

Paid:

```
agy      6 turns   250,612 input tokens   7,473 output   (subscription)
claude   4 turns    35,402 cache-creation, 154,306 cache-read, 2,004 output
cursor   5 turns    not reported by the provider, see ticket 25
```

The expensive one was `agy` run 3, 86,944 input, where the model spent fourteen `view_file` calls
trying to work out why its write was refused. If a future ticket needs an `agy` probe, tell the
model in the prompt not to investigate a failure.
