# 27 - The shape of the delegation policy, now that every provider can be gated

Type: grilling
Status: resolved
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

## A case ticket 15 hands you, 2026-09-03

**A judge run is a `hydra run` with every tool denied.** Rung two's panel needs no tools at all: the
diff arrives in the prompt and one JSON answer comes back. That is the narrowest policy the gate
will ever be asked to express, and it is a useful test of the shape. If "no tools at all" is awkward
to say in the policy, the policy is the wrong shape.

It also tests the deny-list reading directly. A deny list Hydra authors once and injects everywhere
has to be able to say "everything" for this one run without that meaning "everything, except the
tools the list forgot".

## Answer, 2026-09-03

**A named allowlist, expressed in capabilities, compiled into a per-run script that lives outside
the worktree it guards.** Ticket 26 turned the policy from a preference into a mechanism; this is
what Hydra puts in it.

### Deny list or allowlist

Named allowlists, deny by default, and the `denied` list from ticket 11 goes away.

The providers self-update, `agy` moved four times in the life of this map, so a deny list authored
today knows nothing about a tool shipped next week and its failure mode is silent permission. An
allowlist fails toward a run that cannot do its job, which someone notices. The `denied` list goes
because with deny-by-default it can only subtract from the allow list, and a policy where two rules
can disagree is a bug with a schema. Argument conditions live inside an allow entry instead, which
is the shape `agy` already uses in the rule strings it prints, `write_file(<target>)`.

**The caller passes a name, never a list.** Ticket 11 put `policy` on the MCP face as an optional
argument, which means a model composes it. A name is a thing a human wrote once; a list is security
assembled at call time by the least accountable party in the system.

### What the policy names

Capabilities, with a mapping table Hydra owns, never provider tool names. Ticket 24 found the three
providers share no notion of a skill's identity, and ticket 23 found the same for MCP servers, so a
policy written in one provider's vocabulary does not survive being handed to another. Capabilities
are the only thing all three actually share.

Then check the table against the provider at run time rather than trusting it. `agy` returns its 57
tool names in the `init` frame of every run at no cost, so Hydra compares what the provider says it
has against what the table knows. **A tool name the table does not recognize is denied and
recorded.** A self-update that renames a tool then breaks the next run loudly instead of quietly
widening what a child may do. The cost is accepted in that direction on purpose.

### The five policies

| Name | What it allows |
|---|---|
| `judge` | nothing |
| `curate` | reads the corpus, writes only inside the skill store |
| `read-only` | reads inside the worktree, no shell, no writes |
| `standard` | reads, writes, shell and network reads inside the worktree |
| `permissive` | everything, and the only policy that may spawn in-process subagents |

Five because every one of them is a use case this map already has, and a sixth should arrive with a
reason rather than a hunch. `judge` is ticket 15's panel and it doubles as the test of the shape:
the narrowest policy is one word rather than an enumeration of everything to forbid.

`standard` allows network reads. Denying `WebFetch`, `search_web` and `read_url_content` would make
a child's inputs fully reconstructable from the record, and it would also break ordinary coding
work, at which point people route around `standard` and the strictness buys nothing.

`curate` writes to `~/hydra/skills/`, which is outside its own worktree. It is the second policy
that breaks the worktree-write assumption, and both exceptions are handled below.

### Three config artifacts, one script

Cursor reads Claude Code's hook format, so one file could gate two providers. Ticket 10 rejected the
same convenience for skills and this rejects it again, for a sharper reason: Cursor's conversion path
writes `failClosed:false` into every converted hook, and only Cursor's own format can say otherwise.
Depending on a translation layer for a security gate, on the provider that self-updates most, trades
the wrong thing away.

So `claude` gets `--settings` (its config need not sit in the worktree at all), `cursor-agent` gets a
native `.cursor/hooks.json` with `failClosed` true, and `agy` gets `<worktree>/.agents/hooks.json`
plus `--add-dir`, without which it reports `hooks: []` rather than an error.

One script, because policy evaluation is the part that has to be correct and the dialect is a thin
layer at the end. Three scripts would triple what must stay right in order to vary the one part that
differs.

**The dialect is passed, never inferred.** `claude` ignores a response in a shape it does not
recognize and allows the call, so a script that guesses wrong is an open door on exactly one
provider. Hydra writes the config, so Hydra writes the provider onto the command line as an explicit
argument. The script is told which dialect to answer in, and denies when the argument is absent.

### Whose hooks run in a delegated child

Both Hydra's and the user's, and the record says so.

A clean slate is available on one provider and not the others. `claude --restricted` ignores user,
project and local settings while honoring `--settings`, and it also gates writes to settings and
tool-configuration files, so Hydra uses it. `cursor-agent` always reads `~/.cursor/hooks.json` and
`~/.claude/settings.json`, and this machine's copy of the latter is live with the user's own
`guard_git.py`, `guard_aws.py` and `guard_slack.py`. `agy`'s global and shared config live in the
user's home and are not worktree-scoped. Hydra cannot isolate by moving `HOME`, because ticket 21
found a wrong `HOME` silently logs three providers out.

Claiming isolation Hydra can deliver one time in three is the claim ticket 22 refused. Composition
is safe here only because the user's hooks can tighten Hydra's floor and never widen it, **and that
is the one load-bearing fact on this ticket that nobody has measured.** See below.

### What the script is, and where it lives

Generated per spawn with the policy compiled into it, written outside the worktree it guards. Not a
script that reads a policy file at fire time, because anything read at fire time is a file that may
have changed since the previous tool call.

Ticket 26 proved all three providers snapshot their hook *config* at process start, tested by
overwriting the config file every time the hook fired. It never tested the script the config points
at, and that file is read fresh on every invocation. The config snapshot protects the pointer and
not the target.

Putting the script outside the worktree closes that by construction. The child's write access is
scoped to the worktree by the very policy the script enforces, so the first call is checked by the
original script, which is what denies the write that would have replaced it. The induction holds for
three of the five policies and breaks for `permissive` and `curate`, both of which write outside
their worktree. The carve-out below is their base case.

**Calling back into `hydra` per tool call is ruled out.** The small reason is a subprocess on every
call inside Cursor's 60 second hook budget. The large one is that a gate depending on the parent
still running turns a dead parent into a fail-open on the provider that already fails open.

### In-process subagents

Every policy but `permissive` denies them.

Ticket 11 caps delegation depth at 2, computed at spawn from `parent_run_id`. All three providers can
spawn a subagent inside a run without going through `hydra run`: Claude's `Task`, `agy`'s
`invoke_subagent` and `define_subagent`, Cursor's `subagentStart`. None of those gets a run id, none
writes a run file, and none counts against the cap. D10 makes the record the source of truth, and an
in-process subagent is work with no record at all, on the exact trajectory Ouroboros learns from.

Ticket 26 confirmed all three surfaces are gateable, so this is enforceable rather than aspirational.
The cost belongs in the spec: a delegated Claude child cannot use `Task`, which is a real capability
loss on coding runs.

### MCP

**Deny means no call succeeds, not that the child sees nothing.** Ticket 23 found a delegated child
inherits the human's user-scope servers on all three providers and that only `claude` can scope them
off per run. Ticket 26's gate blocks the calls everywhere, so nothing executes, and ticket 22's rule
is satisfied because Hydra can gate what a child does.

The residue gets written down rather than buried. An inherited server's tool descriptions reach the
model's context, which is the same class of problem ticket 24 found when injection stripped
descriptions off skills the user already had. And a server process that starts may connect somewhere
or hold a credential before any call is attempted, so "nothing executed" is true of the agent and not
of the machine. The record carries which servers were visible to the run, in the shape ticket 09 uses
for `skills.linked`.

`--restricted` on Claude drops MCP entirely, so the clean version comes free on the one provider that
offers it.

### `agy`'s `overwrite` is forbidden

Its `PreToolUse` can shallow-merge into a tool call's arguments so a rewritten call is what runs. The
deciding reason is the record: a rewritten call means the record no longer shows what the model
actually asked for, and rung one learns from what agents tried, so a silently corrected trajectory
teaches the curator that the agent got it right. The second reason is the map's standing pattern,
that a capability on one of three providers makes any policy using it mean something different on
`agy`.

### What a gated run records

**A run outcome of `gated`**, in the shape ticket 12 gave auth failure and ticket 10 gave skill
collisions. All three providers exit 0 and report success on a run where the gate refused everything,
which is ticket 14's clean-looking failed run confirmed a fourth time.

The outcome exists so the curator does not rewrite a skill over it. Ticket 14's rule for auth is the
precedent: a skill does not need evolving because a token expired, and it does not need evolving
because a tool was denied either. A run that failed on too narrow a policy is a policy bug.

The record also carries the policy's name and a hash of it, since a denial only means something
against the policy that produced it and named policies are editable by definition. `permission_decision`
with decider `hook` derives from the event stream, never from the exit status, per ticket 26.

### Who may author a policy, and the one carve-out

Human-authored, enforced with what already exists.

The policy directory joins ticket 15's protected paths, so a rung-two patch touching it is refused
before review, and rung one cannot reach it because its write surface is the skill store alone.
Ticket 19 put the export allowlist in the same category, so Hydra now has three protected configs:
the delegation policy set, the environment allowlist and the export allowlist.

The run-time half is one rule. **The gate denies writes to the policy directory and to the generated
script under every policy, `permissive` and `curate` included.** That is the single place `permissive`
stops being permissive, and it earns the exception twice over: without it a child under `permissive`
can rewrite the policy constraining its siblings, and the script-protection induction above has no
base case.

### What is not verified, and what it would change

One fact holds up the composition decision: that a deny beats an allow when Hydra's hook and the
user's own hook both answer on the same tool call. Ticket 26 probed one hook at a time and never two
at once.

If it turns out an allow can win, this answer inverts. Composition stops being safe, `--restricted`
becomes mandatory where it exists, and ticket 22's rule takes over for the two providers where it
does not, meaning Hydra either owns the config root or does not route them. That is a harness spec
change rather than a paragraph, so ticket 30 measures it and blocks ticket 16.

### What this changes elsewhere

- **Ticket 11** gains the policy's shape. Its `allowed tools, denied tools` becomes a named
  capability allowlist, and its `policy?` argument on the MCP face takes a name.
- **Ticket 14** gains the run-time half of its config rule. Barring rung one from config was a write
  surface restriction; the gate now enforces it against every run, not only the curator's.
- **Ticket 15** gains two protected paths, the policy directory and the generated gate script.
- **Ticket 28** inherits its answer's floor. The scenario it poses, a child's result asking the
  orchestrator to grant write access to the skill store, is refused by the carve-out above regardless
  of what any model believes about the text.
- **Ticket 29** has its third question answered here rather than there.
- **Ticket 16** takes the policy engine into the harness spec, and waits on ticket 30.

## Confirmed by ticket 30, 2026-09-03

**The unverified fact holds.** A deny beats an allow on all three providers, and the stronger result
is that a hook cannot grant at all, so the user's hooks can only tighten Hydra's floor. This answer
stands as written. `--restricted` does not become mandatory, ticket 22's rule is not triggered, and
the routable set stays at three.

Two details to fold in. `agy` names one action `write_to_file` as a step and `write_file` as a
permission rule, so the capability table needs both. And Cursor accepts Claude's response dialect on
`preToolUse`, so the shared script answers in two shapes rather than three, though passing the
dialect explicitly stays right for `agy` and for claude's fail-open.

**A cheaper isolation option appeared that this answer did not have.** `--setting-sources project`
drops the user's hooks on `claude` while keeping all 25 tools, `bypassPermissions` and Hydra's own
`--settings` hooks. This answer chose composition partly because `--restricted` also strips Bash.
Whether Hydra takes the cheaper flag is ticket 31.

## Extended by ticket 28, 2026-09-03

**A child's policy may never be wider than its parent's.** This answer settled that the caller passes
a policy name rather than a list. It did not say who may pass which name, and the MCP face's caller
is a model, so a poisoned parent can name `permissive`. Ticket 11's depth cap bounds recursion, not
escalation.

**Wider means capability set containment** over the five named allowlists, which needs nothing new
since they are already capability sets. The five are a partial order rather than a chain, because
`curate` writes to the store and not to a worktree while `standard` is the reverse, so an
incomparable pair is refused rather than resolved. A `standard` parent therefore cannot delegate a
`curate` child, and a curator pass starts at depth 0 from a human.

A spawn refused this way still writes a run record, with outcome `gated` and no child process behind
it, so an attempted escalation is countable rather than invisible.

## Amended by ticket 31, 2026-09-03

**Composition stands, and for a better reason than this answer gave.** This answer kept the user's
hooks partly because the only isolation it knew, `--restricted`, also strips Bash. Ticket 30 found
`--setting-sources project`, which isolates without that cost, and composition still wins. Hydra's
policies bound where a run acts, the user's guards bound what a shell command means, and only the
second stops a `git push --force` that `standard` otherwise permits.

**A pattern floor joins the policy set.** Argument conditions on the `shell` capability, in the
`write_file(<target>)` shape this answer already allows, covering the destructive git forms, `aws
configure` and `sso`, and Slack webhooks. It lives as one file in the policy directory rather than a
field repeated across five policies, so it inherits the protection the set already has and the
protected config count stays at four. It applies on all three providers and under every policy
including `permissive`, because the user's own guards cover `claude` and `cursor-agent` whatever
Hydra does, so an exemption would empty the floor on `agy` alone. A human lifts it per run with a
flag in D16's shape.

It is a floor and not a gate. A deny list fails toward silent permission, which is why this answer
refused one for tools, and saying so out loud is the price of covering `agy` at all.

**`--restricted` narrows to `judge`.** Stripping Bash and dropping MCP is the point for a policy that
allows nothing and a cost everywhere else, so every other policy launches with `--settings`.
`--setting-sources project` is measured and deliberately unused.
