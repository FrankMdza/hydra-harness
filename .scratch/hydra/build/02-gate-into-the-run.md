# 02 - Wire the gate into a recorded `claude` run

Type: build
Status: resolved
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

## Outcome

Built, measured on `claude` 2.1.261 on 2026-09-05, and `IMPLEMENTATION.md` carries the
numbers and the run ids.

**What landed.** `~/hydra/spawn/<run-id>/` holds `gate` at `0o700` and `settings.json`,
both written fresh per spawn by `hydra::run::write_spawn_config`. `--settings` is on the
command line and 6.7's two policy shapes are both there: `judge` takes `--restricted`
and no `--permission-mode`, the other four take `--permission-mode bypassPermissions`.
`run_start` carries a real `policy` and `policy_sha`. The preamble rides the task string
and lands in `run_start.task` verbatim. `hydra run` gained `--policy`, defaulting to
`standard`, and `--allow-floor-escape`.

**The measurement, in the order it was taken.** The flag went on last, as the ticket
asked. First the same probe with `bypassPermissions` held back, which proved the hook
was mounted and the `*` matcher matched while the provider's own permission layer was
still there. Then the flag, and the denial held:
`hydra/capability: policy read-only does not allow file_write (Write).` reached the
model verbatim with `is_error: true`, no file was written, and the run closed
`subtype: success`, exit 0. Then a `standard` run of the same probe, which the gate
allowed and which wrote the file, because two denials prove only that something says no.

**What the ticket assumed that turned out otherwise.**

- **`standard` did not hit the `ToolSearch` and `Skill` denial.** The ticket expected
  the first gated `standard` run to notice it. A one-tool probe never reached for
  either. The capability loss is still there and still real, and it wants a run with
  ordinary coding shape to surface it. Ticket 03 or 07 will get there first.
- **`hydra_version` stopped separating gated records from ungated ones.** The first
  slice's note said any `0.1.x` record predates the gate. Three gated records were
  written under `0.1.0` during this ticket's own measurement, before the version moved
  to `0.2.0`. A null `policy` is the test that holds, and `IMPLEMENTATION.md` says so
  now.
- **The `PreToolUse` matcher was not settled by any research.** Ticket 26 and ticket 30
  both probed `claude` with a named matcher, `"Bash"`. The gate needs every tool, `"*"`
  is documented rather than measured, and a matcher that fails to match fails open on
  this provider. Measured here instead: the gate fired on `PreToolUse:Write` on all
  three probes.

**One decision this ticket declined to make.** Nothing checks at launch that the gate
script actually runs. On `claude` a hook that fails to start fails open, so a broken
interpreter or a mangled shebang is silent. A launch-time smoke test of the compiled
script would be an eighth entry in harness spec 6.1's pre-flight list, and 6.1 is a
closed design decision with seven. That belongs on the map rather than in this commit.
`IMPLEMENTATION.md` records the risk under rough edges.

## What the code review found, added 2026-09-05

Two axes, standards and spec, both against the finished change.

**The spec axis found a live hole and it is not closed.** `shell` walks past all seven
of 7.5's path denials, because the deny list is consulted only for `file_write` and
`Bash` carries no path. That reaches protected config #1, the run corpus, and the run's
own gate script, and an emptied gate script makes `claude` fail open, so two `Bash`
calls under `standard` end the gate for the rest of the run. `bypassPermissions`, which
this ticket put on the command line, is what removed the layer that had been refusing
those calls. `IMPLEMENTATION.md` opens with the finding and
`tests/gate.rs::shell_walks_past_the_protected_path_denials` pinned it.

**Closed in the commit after this ticket's own**, on the human's call. A shell command
naming any of the seven is now refused whole, measured on run
`01a07340-67a3-7c41-8c69-8d1e28bb58a7`. What remains is a path the command never spells,
built from a variable, which is 7.4's own failure mode and which needs a mechanism that
does not read argv. That part is still a decision and still belongs on the map.
`IMPLEMENTATION.md` carries both halves.

**Smaller spec findings, all recorded rather than fixed.** `--allow-floor-escape` is not
on the run record, and harness spec 16 says a per-run human flag lands there; the field
does not exist in contract 5.1, so adding it is a contract change. `~/hydra/spawn/` is
denied only as this run's own script rather than as a directory, which a detached
concurrent run would make reachable. Both are in `IMPLEMENTATION.md`.

**Spec findings that were fixed.** The reason given for holding `--strict-mcp-config`
back was true of `--mcp-config` and not of it: 6.5 says the strict flag stops the human's
inherited servers starting at all. Measured instead, and all three runs came back
`mcp_servers: []`, so nothing was inherited on this machine. That is an observation
rather than the guarantee the flag makes, and `src/provider/claude.rs` now says so. The
flag count in two headers said nine optional flags with two built; section 9 lists eight
and three are built. A sentence in `IMPLEMENTATION.md` claimed a no-worktree run "can
read and shell out and write nowhere", and `standard` scopes reads to the worktree too,
so it reads nothing either.

**Standards axis.** `cargo fmt` had never been run against the new files and three hunks
were unformatted. Three comments still spoke of ticket 02 in the future tense, and one
in `src/policy/mod.rs` still counted four protected configs as fileless after this ticket
wrote the preamble. `src/record/lines.rs` still said no preamble rides the task string.
All fixed. Its judgement calls, left alone with reasons: the policy name travels as a
`&str` where the repo's idiom for a closed set is an enum, `write_spawn_config` takes six
parameters where `policy::Spawn` is the bundle that already exists, and the temp-root
scaffolding in `tests/spawn.rs` repeats `tests/gate.rs`. None of the three is worth a
refactor inside this ticket, and all three are cheap later.
