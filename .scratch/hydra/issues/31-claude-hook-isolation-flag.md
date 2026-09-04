# 31 - Does Hydra load the user's settings on `claude`?

Type: grilling
Status: resolved
Blocked by: 30

## Question

Ticket 27 decided that the user's hooks compose with Hydra's, and it decided that partly on cost. The
only isolation it knew about on `claude` was `--restricted`, which also strips Bash and drops MCP
entirely, so composition was the option that left a delegated child able to do its job.

Ticket 30 found a cheaper one. `--setting-sources project` keeps all 25 tools, `bypassPermissions`
and Hydra's own `--settings` hooks. Isolation on `claude` is now a choice rather than a trade.
Decide whether Hydra takes it.

**Corrected while resolving, 2026-09-03.** The title and the framing above said hooks. The flag
unloads the whole user-scope settings file, so the three `permissions.deny` rules go with the three
`PreToolUse` guards. It is one switch, and a spec saying "hooks" would ship something that drops
deny rules nobody discussed.

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

## Answer, 2026-09-03

**Yes, Hydra loads them and composes, and the reason ticket 27 gave was not the real one.** Settled
in a grilling session with the human on 2026-09-03, after ticket 30 made the cheap isolation flag
available and it turned out not to matter.

### Why composition wins

Hydra's policies bound **where** a run acts. The user's guards bound **what a command means**. Those
are different jobs and only the second stops a force push.

Every Hydra policy is spatial, and `standard` allows reads, writes, shell and network reads inside
the worktree. Every one of the user's guards blocks an action whose effect is entirely outside the
worktree, reached through the network by a shell command `standard` already permits. `git push
--force` from inside a worktree is `standard` behaviour by ticket 27's own definition.

Read on this machine, 2026-09-03. `~/.claude/settings.json` carries three `permissions.deny` rules
and three `PreToolUse` guards totalling 418 lines. `guard_git.py` is 231 of them and blocks force
pushes, `--all` and `--mirror`, a `+` refspec, pushes to protected branches, `branch -D`,
`update-ref`, `symbolic-ref` in write form and `reflog expire`, consuming `-C` and `--git-dir` first
and refusing a subcommand that arrives from a substitution. `guard_aws.py` denies `configure` and
`sso` and covers `s3 cp/mv/rm/sync/rb`. `guard_slack.py` covers webhooks and `chat.*`,
`files.upload` and `conversations.create`, over Bash and over MCP tool names.

**So composition is load-bearing rather than tolerated.** Ticket 27 kept it because `--restricted`
also strips Bash. Isolation is cheap now and the answer holds anyway, because dropping the file
removes protection Hydra has never specced and would have to build to stand still. The harness spec
says plainly that Hydra bounds where a run acts and does not interpret shell commands.

### The pattern floor, and why `agy` forces it

A capability allowlist is not enough on its own, because `shell` swallows everything.

The user's guards reach `claude` and `cursor-agent`, both of which read `~/.claude/settings.json`,
and ticket 30 measured them firing under Cursor. They do not reach `agy`, not because `agy` cannot
read a user-scope hook config but because none has been written, and ticket 30 confirmed
`~/.gemini/config/hooks.json` does not exist. Today a delegated `agy` child under `standard` can
force push and nothing anywhere stops it.

**Hydra ships a floor: argument conditions on the `shell` capability, in the `write_file(<target>)`
shape ticket 27 already permits.** The destructive git forms, `aws configure` and `sso`, Slack
webhooks. On all three providers rather than only where the user's config fails to reach, because a
per-provider floor would make the harness behave differently for a reason living in a home directory
instead of in Hydra. Doubling up costs nothing, since ticket 30 measured that two hooks answering
compose and deny wins.

**It is a floor and never a gate, and the spec says so.** It is a deny list in the one place ticket
27 permits one, and it inherits that shape's failure mode, silent permission on the spelling nobody
wrote down. The evidence sits in the user's own config: two separate rules for `git push --force`
and `git push -f`, because one spelling does not catch the other, and then 231 lines of parser
beside them anyway. On `claude` and `cursor-agent` the guards catch what the floor misses. On `agy`
nothing does.

### Where the floor lives, and when it lifts

**One file in the delegation policy directory**, beside the five policies, rather than a field
repeated in each of them where a policy edited to drop it would be a silent hole. It inherits the
protection the policy set already has: rung one cannot reach it because rung one writes only to the
skill store, and ticket 15 refuses a rung-two patch touching it before review. Protected configs
stay at four rather than growing every time something needs guarding.

**It holds under every policy, `permissive` included.** Ticket 27's one absolute, writes to the
policy directory and the generated script, is justified structurally, and a force push does not
threaten the gate, so that reasoning does not carry across. `agy` decides it instead. Exempting
`permissive` would cost nothing on `claude` and `cursor-agent`, where the user's guards fire
whatever Hydra's policy says, so the exemption would empty the floor on exactly the provider where
it is the only thing present. `permissive` bounds what a delegated agent may do, not what the human
may do, and a human who wants a force push runs it themselves.

**The escape is a flag a human types per run**, in D16's existing shape: typed per run, never
inheritable, unsettable by the delegation policy. That makes two instances of one pattern rather
than a single rule about spend, and the spec names the pattern once.

### `--restricted` keeps one job

`judge`, and nothing else. Ticket 15's panel policy allows nothing at all, so a flag that strips Bash
and drops MCP is the point there rather than a cost, and it makes the narrowest policy structural
instead of script-enforced. Every other policy launches with `--settings` and composes.
`--setting-sources project` is specced as measured and deliberately unused, so a later reader who
finds it in ticket 30 does not assume it was missed.

### The asymmetry, stated rather than built on

The guards reaching two providers of three is a fact about this machine, not a property of Hydra, and
it changes the day a Cursor-native or `agy` hook config is written. The spec states it and builds
nothing on it, which is ticket 22's rule applied to protection the user owns rather than protection
Hydra owns.

One thing left unmeasured on purpose: `guard_slack.py`'s `mcp__.*[Ss]lack.*` matcher passes through
Cursor's format conversion, which ticket 26 found drops `Glob` and collapses `Edit` with `Write`, so
whether the matcher survives is unknown. It needs no ticket, because ticket 27's gate blocks every
MCP call regardless.

### What this changes elsewhere

- **Ticket 27** gains the floor, the real reason composition wins, and `--restricted` narrowed to
  `judge`.
- **Ticket 16** takes the floor's initial patterns and the exact `claude` command line.
- **D16** gains a second instance, so the map's Notes name the per-run human-typed flag as a pattern
  rather than leaving it as a rule about billing.
