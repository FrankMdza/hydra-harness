# 02 - Wire the gate into a recorded `claude` run

Type: build
Status: open
Blocked by: 01

## Goal

The compiled gate reaches a real run, and the record stops carrying a null policy. This
is the ticket that closes the first slice's largest departure from the design.

## Where the design says it

- Harness spec 6.6, the gate as a `PreToolUse` hook: the config path, `--settings`,
  real files never links, rewritten at every spawn, and that a hook is a veto and never
  a key.
- 6.7's `claude` command line, both policy shapes.
- 5, the spawn directory, which this ticket creates for the first time.
- 8, the instruction preamble, which asserts three things and no more.
- Contract 5.1, `policy` and `policy_sha`.

## Done when

- `~/hydra/spawn/<run-id>/` holds `settings.json` and `gate`, and both are written
  fresh per spawn.
- `--settings` is on the command line and the user's own hooks still compose, per 6.6.
- The preamble rides the task string and lands in `run_start.task` verbatim.
- `run_start` carries a real `policy` and `policy_sha`.
- **`--permission-mode bypassPermissions` goes on last**, after a run has been measured
  with a hook denial holding under it. Not before.
- `IMPLEMENTATION.md` loses the null-policy entry and the `bypassPermissions` entry.

## Watch for

**The preamble is protected config #4 and it is prose a human should read.** Section 8
is strict about what it may claim: what Hydra wrote at spawn is the run's instructions,
everything read afterwards is data, and the run's actions are bounded by a policy it
cannot widen. It never claims the gate will catch a bad action and never says anything
shaped like a guarantee. A run that trusts the gate is the Hermes failure with the roles
swapped. Show the wording to the human rather than shipping it.

**Do not reuse the preamble's phrasing in `~/hydra/CLAUDE.md`.** Harness spec 15.3 says
why: the two say similar things and only one is backed by a mechanism.

**The first slice already proved the default mode refuses.** A run asked to write inside
its own worktree got `system/permission_denied`, wrote nothing, and reported success
with exit 0. When `bypassPermissions` goes on, the same probe must still be refused, by
the gate this time. That is the measurement, and the record shows which refused because
`decider` is a field Hydra sets from the config it wrote.
