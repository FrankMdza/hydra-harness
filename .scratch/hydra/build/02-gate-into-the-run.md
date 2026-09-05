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

## What ticket 01 left you, added 2026-09-05

**The API you need is `hydra::policy::write_gate(&PolicySet, &Spawn)`.** It compiles
and writes the script, sets the exec bit to `0o700`, and creates the spawn directory's
parent. `PolicySet::discover()` loads `policies/` and `PolicySet::sha()` is the
`policy_sha` the record wants. `Spawn` carries the run's worktree, the skill store,
`~/hydra/runs/`, the gate script's own path and the `--allow-floor-escape` flag. The
gate script denies writes to itself, so its path has to be the one it will really live
at, not a placeholder.

**The dialect is argv[1] and there is no default.** Write the hook command as
`<spawn-dir>/gate claude`. With no argument the script denies in two shapes and exits
2, which is the safe answer and a useless one on a real run. This is the one place a
typo fails closed loudly rather than open quietly, so it is worth a first-run check.

**The script's shebang is `/usr/bin/python3`, absolute.** Do not "fix" it to
`/usr/bin/env python3`. `env` goes through the child's inherited PATH and a hook that
fails to start fails open on this provider.

**`standard` denies `ToolSearch` and `Skill`**, along with seventeen other tools that
answer to no capability the five policies speak in. See `IMPLEMENTATION.md`. Expect the
first gated `standard` run to hit it. That is 7.1's chosen direction rather than a bug,
and the fix, if the records ask for one, is an edit to `policies/standard.json`.

**Every refusal reason starts with a stable `hydra/<code>` prefix**: `protected-path`,
`capability`, `out-of-scope`, `floor`, `unknown-tool`, `no-path`, `no-tool-name`,
`bad-payload`, and `hydra/allow` on the way through. The reason string arrives verbatim
as the tool result with `is_error: true`, so it lands in the record and ticket 03 can
key on the prefix rather than parse prose. Do not reword them without checking 03.

**An explicit `"allow"` is emitted rather than silence.** 6.6 measured that a hook is a
veto and never a key, so on `claude` the two are the same thing, and saying it out loud
is what lets a caller tell a decision from a crashed script. A crash is the case that
fails open, so it is worth being able to see.

**`tests/gate.rs` is the pattern to extend.** It compiles a real gate per policy and
feeds it payloads in the shape ticket 26 measured. When the provider is in the loop, the
probe that matters is the one `IMPLEMENTATION.md` already records: a write to `probe.txt`
inside the run's own worktree, refused under `bypassPermissions`, by the gate this time
rather than by the provider's default mode.
