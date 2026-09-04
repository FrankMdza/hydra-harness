# 31 - Does Hydra drop the user's hooks on `claude`?

Type: grilling
Status: open
Blocked by: 30

## Question

Ticket 27 decided that the user's hooks compose with Hydra's, and it decided that partly on cost. The
only isolation it knew about on `claude` was `--restricted`, which also strips Bash and drops MCP
entirely, so composition was the option that left a delegated child able to do its job.

Ticket 30 found a cheaper one. `--setting-sources project` drops the user's user-scope hooks while
keeping all 25 tools, `bypassPermissions` and Hydra's own `--settings` hooks. Isolation on `claude`
is now a choice rather than a trade. Decide whether Hydra takes it.

- **Whether composition was ever wanted or only tolerated.** Ticket 30 measured that the user's hooks
  can only subtract, so composition is safe. Safe is not the same as desirable. A user hook denying
  something Hydra's policy allows makes a run fail for a reason the policy does not explain, and
  ticket 27's `gated` outcome cannot say whose gate fired.
- **The user's guards are real, and they may be doing uncounted work.** `~/.claude/settings.json`
  holds `guard_git.py`, `guard_aws.py` and `guard_slack.py`. Dropping them inside a Hydra-launched
  child leaves Hydra's five policies as the only thing between a delegated agent and a `git push`.
  Decide whether those policies actually cover what the guards cover, or whether composition has been
  carrying weight nobody added up.
- **What it does to the three-provider story.** Isolation on one provider of three is the shape
  ticket 22 refused to claim. Taking it on `claude` alone means the harness spec describes two
  different worlds, and the same task moved between providers behaves differently for a reason
  nobody sees. Weigh that against leaving a cheap protection unused.
- **Interaction with ticket 28's decider field.** Ticket 28 gave ticket 09 a `permission_decision`
  decider field Hydra sets. Dropping the user's hooks on `claude` makes that field trivially correct
  there while it stays hard on the other two.
- **Whether `--restricted` stays specced for anything.** It strips Bash and drops MCP. Decide if it
  keeps a job, such as ticket 15's `judge` panel, or whether `--setting-sources project` replaces it
  everywhere.

## Why this blocks ticket 16

The harness spec has to state the exact command line Hydra builds for `claude`. This is a flag on it,
and it changes what the spec can promise about isolation.

## Where this came from

Ticket 30, 2026-09-03, while measuring whether a deny beats an allow. The measurement went ticket
27's way and this fell out of the same session. Read ticket 27's "Whose hooks run in a delegated
child" and finding C11 in
[research/30-hook-precedence-deny-vs-allow.md](../research/30-hook-precedence-deny-vs-allow.md).
