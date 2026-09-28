# 142 - The seats launcher holds under a hostile session

Type: build
Status: open
Blocked by:

## What

An adversarial review of build 140 (session subagent, 2026-09-28, reading only) found no way past
the shapes, the policy containment, the socket scoping or the marking. It found five ways a session
can hurt the launcher or the corpus. The session's own shell reaches `seats.sock` (140's
"Decided by the session"), so each is reachable by the model, not only by a bug. The release
binary is not rebuilt with 140 until this lands.

Line numbers are from the 140 commit; find them by name.

## Do

Tests first, each shown failing before the fix, on stubs:

1. **The record grows without bound.** `src/seats.rs` near 193 to 227 records every request,
   refused ones included, and the cap counts only launches (near 399). With `MAX_REQUEST` at 1 MB
   (near 65) a loop of requests for a missing shape writes gigabytes, and `next_seq`
   (`src/record/mod.rs` near 65) reads the whole file each time. Fix: a per-session cap on all
   requests whatever the decision (60 an hour), refused once spent without writing a line; one
   `seat_request` line that says the cap was hit, once; `MAX_REQUEST` 64 KB.
2. **Connections and threads have no limit.** One thread per connection (near 647 to 666), a
   per-read timeout rather than a deadline for the whole line (near 670), no write timeout (near
   681), and an accept error spins (`let Ok(..) else continue`). Fix: at most 4 connections in
   flight, answer the fifth with a refusal and close; one deadline for the request line;
   `set_write_timeout`; a short sleep after an accept error. Test: a slow client and a flood do not
   stop a well-formed request from being answered, and the harvest still writes `run_end`.
3. **Seats in launch die with the session.** `Served::drop` (near 631 to 644) joins only the accept
   thread; the process exits after the harvest, killing a seat still inside `run::run` before its
   watcher detaches. Fix: count launches in progress and have `stop` wait until each has handed
   off to its detached watcher (bounded wait, then record what did not). Test with a stub launch
   that blocks.
4. **Seat launches print on the human's terminal.** `run::run` prints at `src/run.rs` near 1409,
   1725 and 908 on the launcher's stderr, which is the terminal `claude` draws on. Fix: a quiet
   flag on `Request` the launcher sets; the lines go to the session's record instead or nowhere.
5. **The MCP server scope is a prefix match.** `mcp_server_violation` in `src/policy/gate.py`
   admits any name starting `mcp__hydra__`, so a server named `hydra_` passes. Fix: the tool's
   server part must equal a listed server exactly, per provider naming. Gate test with
   `mcp__hydra___x` refused.

fmt, clippy `--all-targets -D warnings`, `tests/seats.rs`, `tests/gate.rs`, `cargo test --lib`.
`IMPLEMENTATION.md` and `SECURITY.md` in the passages 140 wrote.

## Outcome
