# 73 - The gated interactive launch runs unwrapped

Type: build
Status: open
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
