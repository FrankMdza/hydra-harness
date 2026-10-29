# 375 - The provider probes run inside the run's own wrapper

Type: build
Status: open
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

## Outcome
