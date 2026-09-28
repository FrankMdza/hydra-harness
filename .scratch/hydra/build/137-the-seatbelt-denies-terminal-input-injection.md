# 137 - The seatbelt denies terminal input injection

Type: build
Status: resolved
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

Run `01a0e768` (claude opus, `standard`) wrote the test, the SECURITY and IMPLEMENTATION passages
and step 3's measurement, then stopped mid-work: it started a build in the background, yielded
to wait for it, and print mode ended the process. Its record says `completed` with no rule in
`src/sandbox.rs`, a probe that ignored its profile (`let _ = profile`, no `sandbox-exec`), a
leftover debugging test, and docs describing a rule that was not there. The session finished it
on master. The probe hung at exit under the first real run: with echo on, the injected bytes
went to an output queue nobody drained, so the test now puts the pty in raw mode. Red on master
without the rule (`tiocsti rc=0` under the spawn's profile), green with
`(deny file-ioctl (ioctl-command TIOCSTI))`; the bare-profile control still injects. Step 3, as the
run measured it from inside its own non-interactive run: `claude` starts each tool shell in a
session of its own, no controlling terminal, stdin on `/dev/null`, so a tool shell could not
reach the human's terminal this way; `claude` itself, and anything it starts without a new
session, could. Measured the same night inside a live interactive session (`01a0e78d`, claude sonnet, `standard`, launched in a Herdr pane with the release binary carrying the rule): the tool shell runs in a session of its own, fds 0 to 2 are not terminals and `/dev/tty` answers ENXIO, so there was nothing to inject into. The rule is defence in depth for `claude` itself and what it starts without a new session. fmt, clippy, lib 211, gate 99,
spawn 34, sandbox_tty 2, all on master.
