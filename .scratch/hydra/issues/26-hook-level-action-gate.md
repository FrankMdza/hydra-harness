# 26 - The hook-level action gate, and whether it reopens ticket 11

Type: research
Status: resolved

## Question

Ticket 05's lesson is that a guarantee written in a prompt is not a guarantee. Tickets 10 and 17
applied it at Hydra's own tool-dispatch layer, which covers what Hydra's curator does. It does not
cover what a **child provider** does inside a run, and that is the larger surface.

Ticket 11 settled that a child's permissions come from a policy rather than a human, and concluded
that `cursor-agent` cannot be delegated tool work under any policy narrower than fully permissive.
Ticket 18 concluded `agy` cannot be asked for permission at all, since it auto-denies mid-run and
surfaces the refusal as a tool error rather than an answerable request. Both conclusions were
reached against the **permission-prompt** surface.

All three providers also have a **hook** surface, which does not ask and can refuse. Read off the
installed binaries on 2026-09-02, not probed:

- `cursor-agent` 2026.09.02-c22c1a3: `hooks.json`, with `beforeShellExecution`, `beforeReadFile`,
  `beforeMCPExecution` and `afterFileEdit`.
- `agy` 1.1.24: `hook_deny`, `hook_auto_approve`, `hook_deny_unless_prior_grant`, `hook_args`,
  `hook_caller`, and a `/hooks` command. Also `--sandbox`, described as terminal restrictions.
- `claude` 2.1.257: `PreToolUse` hooks, which can deny a call. Documented and in daily use here.

Settle, per provider:

- **Where the hook config lives**, and whether a worktree-scoped file is read. Ticket 10 found `agy`
  ignores its own workspace skill root unless the directory arrives as `--add-dir`, and ticket 23
  is asking the same question of MCP config. Do not assume cwd is the project. Test it.
- **What a hook can actually deny**, and what it cannot see. A hook on shell execution is not a hook
  on file writes, and `afterFileEdit` fires after the edit.
- **Whether a denial is reported in the output stream** in a way Hydra can record. Ticket 14 found
  three of four providers report failure at the tool level and success at the run level, so a
  silently denied action that still exits zero is the expected hazard, not a surprise.
- **Whether the hook itself can be tampered with from inside the run.** A hook config the child can
  rewrite is not a gate. Ticket 21 found `NODE_OPTIONS` and `PYTHONPATH` execute attacker-named code
  inside `cursor-agent`, so assume the child is hostile and check.
- **Whether `hook_deny_unless_prior_grant` means what it sounds like.** If `agy` can be told to
  refuse an action class unless a grant was issued out of band, that is the mechanism this ticket
  exists to find, and it changes ticket 18's conclusion.
- **What `--sandbox` restricts** on `agy` and on `cursor-agent`, and whether it composes with hooks
  or replaces them.

Then answer the question the whole ticket is for: **can Hydra make a dangerous action structurally
unreachable inside a child run, or only observable after the fact?** If the answer is different per
provider, say so per provider, because ticket 22 already set the rule that a provider Hydra cannot
gate is either given a config root Hydra owns or is not routed.

## What this reopens

If hooks deny reliably, ticket 11's "Cursor cannot be delegated tool work under any policy narrower
than fully permissive" is too strong, and so is ticket 18's "`agy` cannot be asked for permission".
Neither is wrong about the prompt surface. Both may be wrong about the gate. Amend them rather than
editing their answers, and say which claim changed.

Output: `research/26-hook-level-action-gate.md`, plus a recommendation on whether the remaining
decision needs its own grilling ticket.

## Answer

**Yes, on all three, and the gate is not the permission layer under another name.** A `PreToolUse`
hook denial on `claude` 2.1.257 holds under `--permission-mode bypassPermissions`, and on `agy`
1.1.24 it holds while the model is otherwise free. `cursor-agent` 2026.09.02-c22c1a3 rejects the
same way in print mode with `--force` passed. Measurements in
[research/26-hook-level-action-gate.md](../research/26-hook-level-action-gate.md), all probed live
on 2026-09-03, not read off `strings`.

Coverage is wider than the ticket assumed. Cursor defines twenty-one hook events, not four, and six
of them can return a permission decision: `beforeShellExecution`, `beforeMCPExecution`,
`beforeReadFile`, `beforeTabFileRead`, `subagentStart` and `preToolUse`. A hook blocks a file write
before it reaches disk on all three, probed on Cursor with the file path and content in hand and no
file afterwards. `afterFileEdit` firing late is true and irrelevant.

**The config is tamper-proof within a run, and only within a run.** All three snapshot their hook
configuration at process start. Tested the hard way, with a hook that overwrote its own config file
every time it fired: the second tool call was still denied on `claude`, on `agy` and on
`cursor-agent`. Nothing re-reads the file mid-process. The residual hole is the next run in the same
directory, so Hydra writes the hook config at every spawn rather than trusting one left behind, and
ticket 11's worktree-per-child already keeps that hole small.

Four mechanisms this hands the harness spec.

**Where the config goes, per provider.** `agy` reads `<worktree>/.agents/hooks.json` and cannot see
it unless the worktree also arrives as `--add-dir`, exactly as ticket 10 found for skills, and it
reports `hooks: []` rather than an error when the flag is missing. `cursor-agent` reads seven
locations including `<workspace>/.claude/settings.json`, and it **refuses a symlinked project config
silently and runs the command anyway**, which collides with D9's symlink injection. `claude` takes
`--settings <path>`, so its config need not live in the worktree at all. Rule: real files, never
links, written fresh per spawn.

**One config can serve two providers.** Cursor reads Claude Code's hook format and translates it,
`PreToolUse`/`Bash` becoming `preToolUse`/`Shell`. Probed with a project `.claude/settings.json` and
nothing Cursor-shaped on disk, and the deny landed. So Hydra authors two shapes, not three, at the
cost of a lowest-common-denominator matcher set: `Glob` is dropped with a warning, `Edit` and
`Write` collapse together.

**The three disagree about what a broken gate means.** `agy` fails closed, and a hook that exits
non-zero denies every tool. `claude` fails open on a response in the wrong dialect: it ran the hook,
ignored `{"permission":"deny"}`, and allowed the call. Cursor has a per-hook `failClosed` flag that
the Claude-conversion path sets to false. One shared script has to detect the caller and answer in
that provider's shape, or it is a real gate on two providers and an open door on the third.

**A denial is recordable on all three, and the exit code is not.** `claude` gives a tool result with
`is_error: true`, the reason verbatim, and a `tool_use_id`. `agy` gives a `step_update` with
`state: "ERROR"`, the attempted arguments, and the reason inside a hook-named error string.
`cursor-agent` gives a first-class `rejected` result carrying the command. All three then exit 0 and
report success, which is ticket 14's hazard confirmed a fourth time. `permission_decision` with
decider `hook` derives from the stream, never from the exit status.

Three findings worth carrying elsewhere. `agy`'s `PreToolUse` can return `overwrite`, which
shallow-merges into the tool call's arguments so the modified call is what runs and what gets
recorded, a rewrite capability the other two lack. `claude --restricted` ignores user, project and
local settings while honoring `--settings`, refuses `bypassPermissions`, and gates writes to
settings and tool-configuration files, which is the vendor solving the tamper problem for us.
And `agy`'s `hook_deny_unless_prior_grant` means what it sounds like: `ask` and `force_ask` collapse
to deny for a detached child, and `permissionOverrides` did not reopen them, so a hook's usable
decisions are `allow` and `deny`.

### Recommendation

**Yes, this needs a grilling ticket.** Research settles that the gate exists and how each provider
spells it. It cannot settle what Hydra puts in the gate, and that is now the biggest unforced
decision left on this map: a policy engine that was scoped for two providers turns out to be
buildable for four surfaces across three, and the gate can also rewrite commands rather than only
refuse them.

The sharp question: **now that every provider can be gated, is Hydra's delegation policy a deny
list Hydra authors once and injects everywhere, or a per-run allowlist the caller passes, given that
the same hook script must answer in three dialects and fails open on `claude` if it gets one
wrong?**

