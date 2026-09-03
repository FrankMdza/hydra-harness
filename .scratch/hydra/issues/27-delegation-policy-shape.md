# 27 - The shape of the delegation policy, now that every provider can be gated

Type: grilling
Status: open
Blocked by: 26

## Question

Ticket 26 changed the ground under ticket 11. When ticket 11 wrote the delegation contract, two of
three providers looked ungatable, so the policy could only be a preference. It is now a mechanism:
`claude`, `cursor-agent` and `agy` all hard-block a tool call from a config Hydra owns, before the
tool runs, and all three snapshot that config at process start so a running child cannot rewrite it.

The sharp question ticket 26 leaves:

**Is the delegation policy a deny list Hydra authors once and injects everywhere, or a per-run
allowlist the caller passes, given that the same hook script has to answer in three dialects and
fails open on `claude` if it gets one wrong?**

Settle:

- **Deny list or allowlist.** A deny list is one artifact Hydra maintains and every run inherits. An
  allowlist is per run, narrower by default, and puts the burden on whoever calls `hydra run`.
  Ticket 11 caps depth at 2 and says permissions come from a policy rather than a human, which
  leaves both readings open.
- **One script or three.** The gate must answer in each provider's dialect. One script that branches
  is one thing to keep correct; three scripts are three things that can drift. `claude` fails open
  on a response it does not recognise, so a dialect bug on that provider is a silent hole rather
  than an error.
- **What the policy is allowed to say.** Ticket 14 rules that rung one may not edit config, and the
  delegation policy is exactly the config it meant. Decide whether the policy is human-authored
  only, and whether that is a rule enforced in code or a convention.
- **Whether `agy`'s `overwrite` is used.** Its `PreToolUse` can rewrite a tool call's arguments
  before it runs, rather than only allowing or denying. That is more power than the other two have
  and it is not obviously a good idea.
- **What a gated run reports.** All three exit 0 and report success on a run where the gate refused
  everything. Ticket 14's failure taxonomy has a class for exactly this, the clean-looking failed
  run, so decide what the record carries when the gate fired.
- **The symlink collision.** Ticket 10 injects skills by symlink. `cursor-agent` silently refuses a
  symlinked hook config and runs the command anyway. So hook config is copied, not linked, at least
  for Cursor, and probably everywhere so one rule covers all three.
- **Whether one `.claude/settings.json` should gate two providers.** Cursor reads Claude Code's hook
  format. Convenient, and it means a Cursor gate silently depends on a Cursor behaviour that could
  change. Ticket 10 rejected the same convenience for skills, for the same reason.

Read [research/26-hook-level-action-gate.md](../research/26-hook-level-action-gate.md) before
grilling. It is 862 lines and it has the per-provider detail with commands.

## What this does not reopen

Ticket 11's answer stands except where ticket 26 amended it. Detached children, one worktree each,
depth capped at 2, results kept small on purpose. This ticket is about the gate, not the contract
around it.
