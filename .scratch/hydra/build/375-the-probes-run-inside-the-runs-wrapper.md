# 375 - The provider probes run inside the run's own wrapper

Type: build
Status: resolved
Blocked by: 370

## Objective

Before a run starts, Hydra runs the provider's binary to ask it questions, and none of those invocations is wrapped. `inject_into_checkout` at `src/run/mod.rs:679` calls `Provider::resolve`, which runs `claude -p x --model hydra-skill-preflight-not-a-model` in `claude::resolve_in` and `agy -p=/skills` in `agy::slash`. That happens before `write_spawn_config` at line 748 writes the seatbelt or bubblewrap profile. `agy_gate_mounted` runs `agy -p=/hooks` after the profile exists, and it is not wrapped either. `cursor::resolve` and `codex::resolve` spawn nothing.

Build 370 measured what that costs, on 2026-10-28:

- On macOS, Claude Code 2.1.281, the free probe argv in a repository with a committed `.claude/settings.local.json` ran both its `SessionStart` and its `UserPromptSubmit` hooks. Hydra does not touch that file on the host. Inside the wrapper the recorded run reads the same file, but its hooks run under seatbelt or bwrap. The probe runs them under nothing.
- In Docker under bubblewrap, a stand-in `claude` that runs git ran twice per launch: once as the probe, unwrapped, and once as the run, wrapped. The unwrapped invocation committed onto the run's branch in the parent repository's `refs/heads/`, and in an earlier launch it wrote `x` into the private `HEAD`. The wrapped invocation's `git commit` was refused with a read-only file system on the same ref.

The probe has to look at the same machine the run will, which `claude::resolve`'s doc comment already says about the environment. The wrapper is part of that machine.

## Scope

- Every provider invocation a launch makes before or beside the run goes through the same wrapper and profile the run gets: `claude::resolve_in`, `agy::slash` for `/skills` and for `/hooks`. When `sandbox::wraps_launches()` is false, nothing changes.
- So the profile has to exist before the probe. Move the profile write ahead of `inject_into_checkout`, or the probe after it, whichever keeps the record's phases honest. Say which in the Outcome. The probe's transcript lands under `~/.claude/projects/<slug>/`, which the profile already makes writable for the run, so check the probe can still write it under both wrappers.
- `tests/drift.rs` calls `probe_argv` and `resolve_in` under a scratch `HOME`; keep it working.
- A test that fails before the change: a stub `claude` on `PATH` that, when it sees the probe model, tries one write the profile denies, such as a file under the corpus or the parent's `refs/heads/`. Before the change the write lands; after it, it does not and the launch still proceeds. On macOS it can use `/usr/bin/sandbox-exec` the way `tests/spawn.rs` does, and it skips inside a seatbelt like the others.
- Amend `IMPLEMENTATION.md` and the doc comment on `claude::resolve`, which says the probe runs the human's hooks and calls it "the same exposure a recorded run already has". It was not the same, since the run is wrapped.
- `SECURITY.md`: add a bullet saying the probes ran unwrapped before this ticket and what the measurement above found.

## Checks

`cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --release --target x86_64-unknown-linux-gnu`, and `cargo nextest run --release --workspace --no-fail-fast`.

## Outcome

The probe moved after the profile, and the whole spawn config moved with it. `phase_handoff` now links the skills, calls `write_spawn_config`, then probes. Moving only the profile write was not possible on Linux: the bubblewrap profile binds the gate, and compose refuses a deny source that does not exist yet.

What was built:

- `inject_skills` in `src/run/mod.rs` split into `link_skills`, which pins, snapshots and links, and `probe_skills`, which runs `Provider::resolve` and the collision check. `inject_skills` still composes the two with no profile, for `src/interactive.rs` and the two unit tests.
- `provider::probe_command` builds the probe's command through `sandbox::wrap_command` when a profile is given. `claude::resolve`, `claude::resolve_in`, `agy::resolve`, `agy::slash` and `agy::hooks_mounted` take the run's profile. `agy_gate_mounted` passes `handoff.spawn.sandbox_profile`. When `wraps_launches()` is false the profile is `None` and the command is the old bare one.
- `tests/drift.rs` passes `None` to `resolve_in`.
- The doc comment on `claude::resolve` drops "the same exposure a recorded run already has" and says what build 370 measured.
- `tests/launch_phases.rs::the_probe_runs_inside_the_runs_wrapper` launches the built `hydra run` under `standard` with a stub `claude` on `PATH`. On the probe argv the stub writes `probe-ran` into the worktree and tries a file under the corpus. The test asserts the first landed, the second did not, and the run itself started and wrote a `run_end` with no refusal.

What the records now say: every handoff refusal comes after a compiled gate. The skill collision, the codex auth pre-flight and both MCP refusals now carry `scopes`, `gate_sha` and `sandbox` on line 0. A failure before the compile still leaves them absent. A probe error after the compile writes `never_started` with the spawn on it.

Measured: the four checks are green, 1542 tests run, 1542 passed, 4 skipped. The new test skipped. This session runs inside Hydra's seatbelt, where the binary refuses `HYDRA_ROOT`, so the test has not run and neither has its before-and-after. Someone outside a seatbelt has to run it, then once more with the profile argument at `phase_handoff` forced to `None` to see it go red.

Assumed and false: the ticket said "move the profile write ahead", as if the profile stood alone. Under bwrap it depends on the gate file. The ticket also listed only `claude::resolve_in` and `agy::slash`. Two more provider invocations happen before a run and are still bare: `cursor::preflight` runs `cursor-agent mcp enable`, `disable` and `list`, and a gated interactive `claude` session probes before `src/interactive.rs` writes its profile.

Measured by the session on 2026-10-28. On macOS outside a seatbelt the new test passes, and with the profile at `phase_handoff` forced to `None` it fails with "the probe wrote into the corpus". In Docker under bubblewrap, with build 376 applied, a stand-in `claude` ran its probe in its own pid and mount namespace. Its write into the corpus landed in the sandbox's tmpfs and not on the host, and its `git commit` was refused. The two bare invocations named above remain.
