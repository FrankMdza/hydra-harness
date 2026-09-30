# 186 - The cockpit draws while a stop waits

Type: build
Status: resolved
Blocked by: 184

## What

Found by build 184. `m` then `y` in the cockpit calls `stop::stop` on the draw thread, and
`stop::stop` waits for the seat's record to close: up to `GRACE` plus `CLOSE_BUDGET`, 40 s. The
cockpit neither draws nor reads keys meanwhile, and the chat beside it looks dead.

## Do

Tests first in `tests/stop.rs` or `tests/cockpit.rs` on the stub seat those files already use,
each shown failing before the change.

1. `y` starts the stop on a thread (or a child `hydra stop` process; say which and why) and
   returns at once. The card shows `stopping` until the thread reports, then the outcome the stop
   returned; an error shows on the status line.
2. The note is pasted only once the stop reported success, as today; a failed stop pastes nothing.
3. A second `m` on a seat that is stopping says so and asks nothing.
4. A test proves the draw loop keeps running (for example, a key handled) while a stub seat
   ignores SIGTERM for 2 s.
5. `IMPLEMENTATION.md`, the build 184 paragraph.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit`, `--test
  stop` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f330-99db under `standard`. The session
applied it as written. `y` loads the policy set, then runs `stop::stop` on a named thread that
answers over a channel; `poll_stops` reads it on every draw. The card reads `stopping`, then the
outcome; a failed stop says why on the status line and pastes nothing; a second `m` says it is
stopping. A thread, not a child `hydra stop`, because it returns `Stopped` as is and reuses this
cockpit's policy set; the marker and SIGTERM go out before the wait, so quitting mid-stop still
stops the seat. The host pastes the note after each draw, which closed a race an old test caught.
Three tests on a stub that holds SIGTERM for 2 s, each failing first; fmt and clippy clean,
`--test cockpit` 61, `--test stop` 12, `--lib` 224 on master. The chat still takes the keys when
the note lands, up to 40 s after `y`.

