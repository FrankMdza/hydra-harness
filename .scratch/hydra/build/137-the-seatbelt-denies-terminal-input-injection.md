# 137 - The seatbelt denies terminal input injection

Type: build
Status: open
Blocked by:

## What

A validation run for map ticket 82 (`01a0e75a`, claude-fable-5-1, `standard`, 2026-09-28) read
the seatbelt profile Hydra composes and measured that a process inside it can push input into its
controlling terminal. The profile starts from `(allow default)` and adds only file rules
(`src/sandbox.rs`, `compose`, near 79 to 162); nothing in `src/sandbox.rs` or
`src/spawn_config.rs` names an ioctl. The details of the measurement are in that run's
`result_text`, section 2; read them there with `hydra show`, do not copy them into this repo.

Why it matters: in interactive mode the gated `claude` inherits the human's terminal
(`src/interactive.rs`, lines 4 to 7 and near 324 to 336). What the run did not measure, and this
ticket must, is whether a tool shell inside an interactive session keeps that terminal as its
controlling terminal. In the validation run's own recorded (non-interactive) run, `/dev/tty` was
not configured.

Test first.

## Do

1. A test in `tests/spawn.rs` (or a new `tests/sandbox_tty.rs`) that composes the profile for a
   gated launch, runs a tiny probe under `sandbox-exec` with that profile on a pty the test
   creates, and asserts the injection ioctl is refused. It must fail today. Skip on non-macOS.
   The probe touches nothing but the pty the test owns.
2. Add the narrowest rule to `compose` that makes the test pass. If SBPL cannot filter this
   ioctl by request, say so and stop: that is a decision (deny all ioctls on ttys, or interpose a
   pty in interactive mode) and belongs on the map.
3. Measure, without launching a paid provider if possible, whether a shell spawned the way
   `claude` spawns tool shells in interactive mode has the human's terminal as controlling
   terminal. Report what you measured and how.
4. Amend `SECURITY.md` and `IMPLEMENTATION.md` in the passage that lists what the seatbelt
   covers.
5. fmt, clippy `--all-targets -D warnings`, the touched tests and `cargo test --lib`.

## Outcome
