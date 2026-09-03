# 26 - The hook-level action gate, and whether it reopens ticket 11

Type: research
Status: claimed

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
