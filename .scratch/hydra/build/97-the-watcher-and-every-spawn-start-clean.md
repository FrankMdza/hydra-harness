# 97 - The watcher and every spawn Hydra starts get a cleared environment

Type: build
Status: resolved
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 1 and 9, findings H3 and H46.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/watch.rs, src/provider/codex.rs (the login probe only) and any other `Command::new` in `src/` that starts a provider binary or proposer code.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

A test that reads the running watcher's environment and fails without the clear. The Outcome lists every `Command::new` in `src/` with its environment policy. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome

Built on 2026-09-23.

**The watcher starts from the child's environment.** `watch::detach` takes a `&ChildEnv` and
starts `hydra watch` with `env_clear` and that environment: the provider's allowlist plus what
Hydra set. It is the least the watcher can rebuild the child's environment from, and it needs
nothing else, because every path it reads is on the spec. `hydra run` hands it
`machine.child_env`. The watcher no longer holds the launching shell, so a same-uid `ps -E`
against it reads the allowlist and nothing more.

**The `codex` login probe gets the allowlist.** `login_status` builds `ChildEnv` for `codex`
and clears before it, like the other four probes. A `CODEX_HOME` in the shell no longer changes
which login the probe answers about.

**The containment marker never reached a recorded child, and now it does.** The ticket did not
name this; the watcher work found it. `hydra run` set `HYDRA_RUN_ID` on its own copy of the
child's environment, which is what `run_start.env_allowlist` reports. What the child gets is
built by the watcher: `ChildEnv::build` over the watcher's own environment, which drops every
name outside the allowlist, plus the spec's `hydra_env`. Only `CURSOR_DATA_DIR` was in
`hydra_env`. So since build ticket 57 every record has named `HYDRA_RUN_ID` and no recorded
child has carried it. Before build 88 the pty inherited the watcher whole, and the watcher's
environment was the launcher's, which never holds its own run id either. The effect is that
ticket 57's containment check read a recorded run's relaunch of `hydra` as a human at depth 0.
Ticket 57's measurement planted the marker by hand, and `tests/delegation.rs` does the same, so
nothing measured delivery. Interactive mode sets the marker straight on the child's `Command`,
so it was never affected. The fix is `compose_child_env` in `src/run.rs`: it puts the marker and
`CURSOR_DATA_DIR` in `hydra_env` and then copies that map onto the child's environment, so a
name cannot reach line 0 without also reaching the spec. Not measured on a real provider,
because this ticket launches none. A check for the human: `hydra run --provider claude` with a
task that runs `echo $HYDRA_RUN_ID`.

Tests, each one checked by reverting its fix and watching it fail:

- `tests/detached.rs::the_watcher_holds_the_childs_environment_and_nothing_the_launcher_carried`
  starts a real watcher on a stub, waits for line 0 so the watcher has exec'd, and reads its
  environment from the kernel with `sysctl(KERN_PROCARGS2)`, the bytes `ps -E` prints
  (`/proc/<pid>/environ` on Linux). It asserts the names equal the handed `ChildEnv`'s exactly.
  With `env_clear` and `envs` removed from `detach` it failed, listing `CARGO`,
  `CARGO_MANIFEST_DIR` and the rest of the test process's environment. It first asserts the test
  process carries a name outside the allowlist, so a clean environment cannot make it pass by
  accident.
- `src/provider/codex.rs::tests::the_login_probe_gets_the_allowlist_and_nothing_the_launcher_carries`
  points the probe at a stub `codex` that dumps its environment and answers logged in. With the
  clear removed it failed, listing `CLAUDE_CODE_MESSAGING_TOKEN`, `MANPATH`, `TERM_PROGRAM` and
  others. `login_status` is now a wrapper over `login_status_of(binary)` so the stub can stand in.
- `src/run.rs::tests::every_name_hydra_sets_travels_to_the_detached_half` calls
  `compose_child_env` for all four providers on a scratch `Layout::at` and asserts the marker is
  in `hydra_env`, and that every name in `child_env.names()` is either rebuilt by the allowlist or
  carried in `hydra_env`. With the marker's insert removed it failed on `claude`.

Measured: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean. The
whole suite passed outside a seatbelt, 735 tests. `tests/pty_tee.rs` took 405 seconds under the
load of two other builds on the machine, and passed.

**Every `Command::new` in `src/`, with its environment policy.** Line numbers are after this
change.

| Site | Starts | Environment |
|---|---|---|
| `watch.rs:122` `detach` | `hydra watch`, the detached half | Cleared, then the child's `ChildEnv`. Inherited before this ticket |
| `pty.rs:115` `CommandBuilder::new` | the recorded provider, under `sandbox-exec` | Cleared, then the watcher's rebuilt `ChildEnv` plus `hydra_env`. Build 88 |
| `provider/mod.rs:75` `version` | `<provider> --version` | Cleared, `ChildEnv` for that provider |
| `provider/claude.rs:460` | `claude` skill listing probe | Cleared, the run's `ChildEnv` |
| `provider/cursor.rs:383` | `cursor-agent mcp ...` | Cleared, the run's `ChildEnv` |
| `provider/agy.rs:273` | `agy -p=/<command>` probe | Cleared, `ChildEnv` for `agy` |
| `provider/codex.rs:283` `login_status_of` | `codex login status` | Cleared, `ChildEnv` for `codex`. Inherited before this ticket |
| `interactive.rs:255` | the interactive provider | Cleared, `ChildEnv` plus the marker |
| `interactive.rs:373` | `claude` for the interactive harvest | Cleared, `ChildEnv` |
| `rungs/evolve.rs:840`, `844` | rung two's admission steps, proposer code, with and without the seatbelt | Cleared, `admission_env` plus an empty `HYDRA_ROOT`, one shared tail |
| `policy/replay.rs:212` | `/usr/bin/python3` running Hydra's own compiled gate at read time | Inherited, plus `PWD`. Not a provider and not proposer code. The gate expands `~` and variables from its environment, so a replay under a different environment can disagree with the run, which the module doc already says. Left alone |
| `worktree.rs:105` `GitLocation::command`, `160`, `182`, `575`; `reap.rs:468` | `git` on the human's repo and on run checkouts | Inherited. Not a provider and not proposer code. Build ticket 99 sets `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_NOSYSTEM` on the checkout path |
| `run.rs:2153`, `live.rs:608`, `procinfo.rs:112`, `131`, `rungs/evolve.rs:1531` | `git`, `sleep`, `ps`, `env` | Test code only |

**Proposed documentation sentences, for build ticket 102.**

For `SECURITY.md`, beside its first sentence, which H3 says overclaims:

> The allowlist bounds what the child inherits, and since build ticket 97 it also bounds the
> watcher, the Hydra process that owns the child's pty and sits outside the seatbelt for the
> whole run: `hydra run` starts it from the child's environment rather than its own. That does
> not keep secrets away from a run under `standard`. A shell command can still read a dotfile
> on disk, and a same-uid `ps -E` still reads the human's own terminals.

For `IMPLEMENTATION.md`, in the paragraph on build ticket 57 and the marker:

> Until build ticket 97 the marker reached no recorded child. The launcher set it on its copy
> of the child's environment, which is what `env_allowlist` reports, but the watcher rebuilds the
> child's environment from the allowlist and the marker was not on the launch spec. Ticket 57's
> measurement and `tests/delegation.rs` planted it by hand, so nothing caught it. Records written
> before build 97 name `HYDRA_RUN_ID` in `env_allowlist` for a child that never had it.
> `src/run.rs::compose_child_env` now puts every name Hydra sets on the spec, and
> `every_name_hydra_sets_travels_to_the_detached_half` fails when one is left off.

For `IMPLEMENTATION.md`, wherever the environment allowlist is described:

> Every process Hydra starts that runs a provider binary or proposer code starts with
> `env_clear`: the watcher, the pty child, the five probes including the `codex` login check,
> both interactive launches and rung two's admission steps. `git` and the read-time gate replay
> still inherit, and build ticket 97's Outcome has the table.

Measured by the session on master after the merge: `claude` haiku under `standard`, run
01a0d1fa-c301, ran `echo $HYDRA_RUN_ID` and printed its own run id.
