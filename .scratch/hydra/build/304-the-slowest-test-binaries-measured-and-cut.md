# 304 - The slowest test binaries, measured and cut

Type: build
Status: open
Blocked by:
Opened: 2026-10-13, by the session. Build 296 took the suite from 318 s serial to 198 s
parallel in a warm worktree, and the session measured 230 to 284 s on master with another run
on the machine. Nobody has yet measured which binaries hold the time.

## What

`cargo test --release --workspace` builds 84 test binaries and the wall time is whatever the
slowest few take, since 296 made them run in parallel. The suite is the gate every ticket runs
twice, so a minute cut here is a minute per build. The research note
`~/cerebro/research/security-and-compile.md`, section B, proposed `nextest` for per-test
timing; the measurement does not need it, and no new tool is to be added by this ticket.

## Do

1. Measure. `cargo test --release --workspace --no-run` once, then run each test binary under
   `target/release/deps/` with `time` and `--test-threads=4`, serially, and write a table of the
   ten slowest binaries with their wall time into the Outcome. Note the load average before and
   after, so a reader knows what the machine was doing.
2. For each of the five slowest, read the file and say what the time is: process spawns,
   a `sleep`, a timeout waited to its end, a repo built from scratch per test, a build of the
   binary under test inside the test. Name the test functions.
3. Cut what can be cut without losing an assertion. The usual shapes: a fixture built once per
   binary with `std::sync::OnceLock` instead of once per test, a sleep replaced by a wait on
   the condition it was sleeping for, a timeout test that waits 10 s where 1 s proves the same,
   a `git init` repo copied from a template instead of created each time. Do not weaken a
   timing assertion to make it faster; say in the Outcome if one is in the way.
4. Measure again, same method, and put the second table beside the first. The number that
   matters is the slowest binary's time, since that bounds the parallel suite.
5. The seatbelt tests and `drift.rs` launch real processes and may be slow for a reason the
   file states. Leave those alone unless the reason is wrong, and say so.

## Done means

Two tables in the Outcome, before and after, with the slowest binary's time lower and the
suite green twice in parallel. If nothing could be cut safely, the first table alone with the
per-binary reasons is a valid Outcome; the measurement is the point.

## Outcome
