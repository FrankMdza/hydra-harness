# 308 - The flood-and-slow-client seats test failed once under nextest's full pool

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session. On the first `cargo nextest run --release --workspace`
after build 307 was applied, `tests/seats.rs::a_flood_and_a_slow_client_do_not_keep_a_request_from_its_answer`
failed in 0.015 s with 988 of 1,382 tests already run; it then passed ten times alone and the
full suite passed on the rerun. 307 touched nothing under `src/` or `tests/`; the failure is
the test's own.

## What

The test opens a seat socket with a 500 ms deadline and a 1 s write timeout, floods it from
one client, has a second client send one byte every 100 ms for three seconds, and asserts that
the flood is refused with `hydra/seat-channel-busy` within 2 s, that no refused connection
wrote a request, and that the slow client's reply comes within 3 s and the request is answered
within 2 s of its start (`tests/seats.rs` lines 883 to 945). It failed in 15 ms, so the
assertion that fired was an early one, not a timeout: most likely the flood's reply or the
"no refused connection wrote" check, raced by the listener under eight test processes
sharing the machine with a release build. The failure output was not captured. That is the
first thing this ticket fixes.

## Do

1. Reproduce under the same conditions: `cargo nextest run --release --workspace` five times
   with a `cargo build` of another worktree running beside it, and the single test two
   hundred times under the same load with
   `cargo nextest run --release --test seats a_flood_and_a_slow_client`. Record the counts and
   the assertion text of every failure. If it never fails, say so, and read the test for the
   window anyway.
2. Name the window: which of the four assertions can see a state the listener has not
   reached yet, and what ordering between the flood thread, the slow thread and the listener's
   accept loop produces it.
3. Close it in the test: wait for the condition each assertion needs (the listener bound and
   accepting before the flood starts; the flood's refusal observed before the slow client's
   reply is read) with a bounded wait that fails loudly, no added sleep, no loosened timeout.
   If the window is in `src/seats.rs` and a real requester could see it, that is a product
   finding: say so and stop.
4. Run the two hundred again under load; the count must be zero. Then the full suite twice.

## Done means

The Outcome names the assertion that fired, the window, both counts, and the change.
`cargo nextest run --release --workspace` and `cargo test --release --workspace --doc` green.

## Outcome

Resolved 2026-10-13. Changed only `tests/seats.rs` and this ticket. No product
code or dependency changed, so `IMPLEMENTATION.md` is unchanged.

The failure reproduced in invocation 178 of the initial 200, in 0.024 s.
None of the four suspected assertions fired. The panic was in `reply_within`,
at the original `tests/seats.rs:810:41`, before it read a reply:

```text
called `Result::unwrap()` on an `Err` value: Os { code: 22, kind: InvalidInput, message: "Invalid argument" }
```

The window is between `UnixStream::connect` and the helper's
`stream.set_read_timeout(Some(wait))`. The listener can accept the flood
connection, send `hydra/seat-channel-busy`, and close its peer before the helper
sets `SO_RCVTIMEO`. On this macOS host, that option then fails with `EINVAL`,
even though the complete reply is buffered and readable. This explains the
early panic; the ticket's hypothesis about a busy-text or record assertion
racing a listener that had not accepted yet was false for the captured failure.
The same helper also reads the slow client's reply after its deadline, so it
must support a peer that has already closed in either phase. The listener's
refusal and close are expected behavior; no product finding required a stop.

A separate `UnixStream::pair` probe confirmed that setting the timeout after
closing the peer returns `EINVAL`, while setting nonblocking mode succeeds
and reads the buffered line. Added
`a_buffered_reply_can_be_read_after_the_peer_has_closed`, which writes a reply,
closes the peer, and checks that the helper still returns that reply. It failed
with the same `EINVAL` against the old helper and passed after the fix.

`reply_within` now reads nonblocking bytes until a line or EOF, preserves partial
bytes across `WouldBlock`, and yields while waiting, bounded by the original
`wait` as a total deadline. It no longer sets an option on the closed peer.
Every existing assertion and timeout remains. The test also observes a bounded
busy reply before starting the ten-connection burst, queues that burst before
reading replies, and observes every refusal before reading the slow reply.
For the closing flood, a channel signals the first observed busy reply;
`recv_timeout(2 s)` replaces the fixed 100 ms setup sleep. No sleep was added.

The same eight `yes` workers writing to `/dev/null`, one per logical core,
stayed alive throughout both measurement batches, all seven full suites,
the doctest command and the release build. They were killed and waited for
before the final format and clippy checks. No other worktree was inspected.
Observed load averages included 60.38 / 31.00 / 15.10 initially and
55.83 / 50.12 / 37.80 during final validation under load.

- Before, full pool: **0 failing suites / 5**, each with 1,382 passed and 4 skipped; all five executions of the flood test passed.
- Before, single test: **1 failure / 200**, invocation 178, the exact panic above. Every invocation's output was captured.
- After, single test: **0 failures / 200**, with the same eight workers.
- After, full pool: **0 failing suites / 2**, each with 1,383 passed and 4 skipped, including the new closed-peer regression.

Commands: `cargo nextest run --release --workspace` for every full suite and
`cargo nextest run --release --test seats a_flood_and_a_slow_client` for every
single-test invocation. Logs are under `target/ticket308/`, including
`before-single-178.log`, `regression-before.log` and `regression-after.log`.

Final validation passed: `cargo test --release --workspace --doc`,
`cargo build --release --workspace`, `cargo fmt --all --check`, and
`cargo clippy --workspace --all-targets -- -D warnings`.
