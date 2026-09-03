# 30 - When two hooks answer, does deny win?

Type: research
Status: open
Blocked by: 26

## Question

Ticket 27's composition decision rests entirely on this and nobody has measured it.

Hydra writes a gate config per run. The user's own hooks load too, and Hydra cannot stop them on two
of three providers: `cursor-agent` always reads `~/.cursor/hooks.json` and `~/.claude/settings.json`,
and `agy`'s global and shared config live in the user's home. Ticket 21 ruled out moving `HOME`,
since a wrong one silently logs three providers out. Only `claude --restricted` gives a clean slate.

Ticket 27 accepted composition on the grounds that the user's hooks can only tighten Hydra's floor
and never widen it. **Ticket 26 probed one hook at a time and never two at once, so that grounds is
an assumption.**

Answer for `claude` 2.1.257, `cursor-agent` and `agy`, re-reading every version first since two of
the three self-update:

- **Two hooks on one tool call, one denying and one allowing.** Which answer the provider takes, and
  whether the order they are declared in changes it.
- **Whether scope changes the outcome.** A user-scope allow against a project-scope deny, and the
  reverse. Cursor's loader has seven config locations with a precedence order; find out whether that
  order decides the permission or only the loading.
- **Whether a hook can widen at all.** On `claude` a `PreToolUse` deny beats `bypassPermissions`
  (ticket 26). Find out whether a `PreToolUse` allow beats a denial that would otherwise have come
  from the permission layer, because a hook that can only ever subtract is a different security
  object from one that can add.
- **What Cursor's dedupe does to this.** Ticket 26 found Cursor drops any Claude-format hook whose
  command string matches one already declared in a Cursor-format config. Two hooks that share a
  command string but sit in different scopes may collapse to one, which would decide the question by
  accident.
- **`agy`'s `hook_deny_unless_prior_grant`.** Ticket 26 found `ask` and `force_ask` collapse to deny
  for a detached child. Check whether a prior grant from the user's own config counts as the grant
  that reopens them.

## Why this blocks ticket 16

If deny wins, ticket 27's answer stands as written and the harness spec says the user's hooks
compose with Hydra's.

If an allow can win, ticket 27 inverts. Composition stops being safe, `--restricted` becomes
mandatory on `claude`, and ticket 22's rule takes over for the other two: a provider Hydra cannot
gate is either given a config root Hydra owns or is not routed. That would cut the routable set to
one provider or force a config-root mechanism nobody has designed, which is a spec change rather
than a paragraph.

Read the `## Answer` on ticket 27 and
[research/26-hook-level-action-gate.md](../research/26-hook-level-action-gate.md) first. Ticket 26's
probe rig and its exact commands are in there; this is the probe it did not run.

Output is a findings file at `research/30-hook-precedence-deny-vs-allow.md`. Do not modify the
user's real hook configs; `~/.claude/settings.json` is live and holds their own guards.
