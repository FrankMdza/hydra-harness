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

## Findings

(appended as they land)

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
