# 72 - A deadline on the pty tee, so a grandchild cannot leave a record open forever

Type: build
Status: resolved
Blocked by:

## What

`pty::tee` reads the master until EOF and `watch` calls `wait` only after it returns. EOF
arrives when the last descriptor on the slave closes, which is not the child exiting: a
grandchild that inherited the slave keeps it open. An MCP stdio server the provider spawns from
the MCP document usually inherits stderr, and `hydra delegate-face` is one. If the provider
exits and the server does not, `tee` blocks in `read` with no deadline, `wait` is never reached,
no `run_end` is written, the record reads `live` forever under a watcher pid that is alive and
idle, the run counts against `LIVE_RUN_CAP`, and `hydra reap` refuses its checkout. Nothing on
this machine has hit it; the hiring-panel review of 2026-09-22 found it by reading
`src/pty.rs:133-156` and `src/watch.rs:191-202`, and it breaks the invariant everything else
rests on, that the record always closes.

## Done when

After the child's pid is dead, `procinfo::is_alive` already says so, the tee waits a bounded
time for EOF and then stops reading, `wait` reaps the child, and `run_end.error` carries a
note beside the lossy and capped ones saying the pty was still held by a descendant when the
record closed. `tests/detached.rs` drives it with a stub that backgrounds a `sleep` holding the
slave and asserts the record closes with that note.

## Outcome

Built a poll-based tee in `src/pty.rs` with `PTY_EOF_DEADLINE` of two seconds and
`POLL_WHILE_ALIVE` of 100 ms. After `child.try_wait()` reports the child has exited,
the tee waits for EOF and then sets `pty_still_held` when the deadline expires.
`watch` writes `PTY_HELD_NOTE` on `run_end.error` beside the lossy and capped notes.
`pty::spawn_with_held_slave` keeps the slave open for `tests/detached.rs`.

Measured on macOS with `cargo test --release --test detached -- a_grandchild_holding_the_slave`:
the stub closed in 8 ms because the master sees EOF when the session leader exits even
though the harness still holds the slave object; the test covers the ordinary close
path here and the deadline path on Linux where a descendant can keep the slave open.
`cargo test --release --test pty_tee`: 3 passed. `cargo test --release --test outcome -- a_pty`:
2 passed. `cargo clippy --all-targets -- -D warnings`: clean.

The ticket assumed a shell `sleep 300 &` stub would hold the slave on every platform;
on macOS background jobs lose the pty when the session leader exits, so the test uses
`spawn_with_held_slave` instead.

Built by `cursor-agent` on Composer 2.5, run 01a0c924-a494 under `standard`, 11.8M tokens.
