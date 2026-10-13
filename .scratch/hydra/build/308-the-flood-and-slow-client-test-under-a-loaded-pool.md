# 308 - The flood-and-slow-client seats test failed once under nextest's full pool

Type: build
Status: claimed
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
