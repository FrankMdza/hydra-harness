# 73 - The gated interactive launch runs unwrapped

Type: build
Status: resolved
Blocked by:

## What

`hydra run --interactive --policy <p>` compiles a gate, seeds a worktree, links the snapshot and
writes a full record, and `src/interactive.rs` spawns `claude` bare, with `sandbox: None` and
`gate_sha: None` on its `run_start`. Build ticket 68 wrapped `pty::spawn`, which the interactive
arm does not call. `SECURITY.md` says "every recorded launch" runs under the seatbelt and a
gated interactive run is recorded, so the sentence was false for the launch mode a human uses
by hand until the review of 2026-09-22 named it, and `SECURITY.md` now says it runs unwrapped.

## Done when

The gated interactive arm goes through `sandbox::wrap_command` with the same profile a
recorded run gets, `run_start.sandbox` carries the profile sha and `gate_sha` the gate's, and a
human has run one interactive session under it and written the run id here, because a TUI under
`sandbox-exec` is a thing to measure rather than assume. The ungated arm stays bare: it compiles
no gate and claims nothing.

## Outcome

`src/interactive.rs` gated arm now routes `claude` through `sandbox::wrap_command` with
`prep.spawn.sandbox_profile`, sets `run_start.gate_sha` and `run_start.sandbox` from
`prep.spawn`, and refuses when `/usr/bin/sandbox-exec` is missing on macOS. A public
`gated_exec_argv` lets tests read the wrap without launching. The ungated arm is unchanged.

`tests/spawn.rs::a_gated_interactive_launch_wraps_claude_in_the_seatbelt` asserts the prep
yields a profile path and that the argv starts with `/usr/bin/sandbox-exec` on macOS.
`cargo test --release --test spawn gated_interactive`: 1 passed.
`cargo test --release --test spawn interactive_policy`: 1 passed.
`cargo clippy --all-targets -- -D warnings`: clean.

No interactive session was run in this worktree. A human should run
`hydra run --interactive --policy standard` from a repo, confirm the vendor TUI draws under
the seatbelt, and check the record's `sandbox` and `gate_sha` fields are non-null.

Session note, 2026-09-22: the code and the test are on master. The ticket stays claimed until a
human runs `hydra run --interactive --policy standard`, quits the session, and writes the run id
here with `hydra show <id>` showing non-null `sandbox` and `gate_sha`.

Measured by the human on 2026-09-22 in a Herdr pane: run 01a0c9ca-1591 under `standard`, `claude`
drew its interface under `sandbox-exec`, answered `pong` and exited 0, and `run_start` carries
`launch_mode: interactive`, `sandbox` b4c17cc9 and `gate_sha` d4870f3a; the spawn directory holds
`sandbox.sb` beside the gate. 54.6k tokens.

