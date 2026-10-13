# 304 - The slowest test binaries, measured and cut

Type: build
Status: resolved
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

Resolved on 2026-10-13. The slowest binary went from 96.8 s to 10.1 s, the sum of all 76
from 248.7 s to 95.1 s, and the parallel release suite ran green twice in 84.7 s and 85.6 s.

### Method

`cargo test --release --workspace --no-run` builds 76 test binaries. The 84 the ticket
counts are those plus 8 doctest runs, which `--no-run` does not build. Each binary ran
serially with `--test-threads=4`, from its crate's directory as cargo runs it, under
`env -u GIT_OBJECT_DIRECTORY -u GIT_ALTERNATE_OBJECT_DIRECTORIES` and with
`CARGO_MANIFEST_DIR` set, since two tests assert cargo put that name in their environment.
The wall clock is perl's `Time::HiRes`. The script and the raw lists lived under
`target/hydra304/`, which nothing tracks. A first pass at load 44.6, the other build mid-suite,
ran without `CARGO_MANIFEST_DIR` and failed those two tests; it was thrown away.

### Before and after

Before: load 4.82 at the start, 4.48 at the end. After: load 7.14 at the start, 4.28 at the
end. The after column is the same binary in the second pass.

| # | binary | before | after |
|---|---|---|---|
| 1 | `crates/grader` `differential` | 96.76 s | 8.95 s |
| 2 | `pty_tee` | 52.85 s | 1.06 s |
| 3 | `gate` | 12.42 s | 8.65 s |
| 4 | `git_config` | 10.11 s | 10.10 s |
| 5 | `crates/otel` `otel` | 7.49 s | 0.88 s |
| 6 | `crates/puppets` `differential` | 6.62 s | 6.08 s |
| 7 | `cleanup` | 6.24 s | 5.25 s |
| 8 | `cockpit` | 5.28 s | 5.29 s |
| 9 | `spawn` | 5.04 s | 7.18 s |
| 10 | `sandbox_copilot` | 4.42 s | 4.49 s |
| | all 76 | 248.73 s | 95.11 s |

The ten slowest after:

| # | binary | after |
|---|---|---|
| 1 | `git_config` | 10.10 s |
| 2 | `crates/grader` `differential` | 8.95 s |
| 3 | `gate` | 8.65 s |
| 4 | `spawn` | 7.18 s |
| 5 | `crates/puppets` `differential` | 6.08 s |
| 6 | `cockpit` | 5.29 s |
| 7 | `cleanup` | 5.25 s |
| 8 | `sandbox_copilot` | 4.49 s |
| 9 | `wait` | 4.04 s |
| 10 | `stop` | 4.02 s |

`spawn` moved from 5.0 s to 7.2 s with no change to it, which is the size of the noise the
other build puts on one number.

### The five slowest, and what held the time

1. **`crates/grader` `differential`, 97 s.** `collect::the_labels_are_what_the_python_gave`
   40 s, `collect::collect_writes_what_the_python_wrote` 25 s,
   `judge::the_judge_gives_what_the_python_gave` 13 s, run alone. A sample of the labels test
   showed 4,014 of 4,017 stacks in `Command::output` polling the stub `hydra-puppets`. The
   stub alone takes 8 ms. A fresh copy of the same file takes 175 ms on its first run and
   16 ms after, and a hard link to a file already run takes 17 ms: macOS checks a new file the
   first time it executes, and every case wrote a new stub. Cut: `Stubs` in `differential.rs`
   writes each stub once per test and hard-links it where a case plants it executable;
   `collect.rs` and `judge.rs` use it. Every planter now removes its path before writing, so
   no write reaches a stub through a link. The one labels case that plants the helper
   without the exec bit still gets a fresh file, since a mode change on a link would change
   the shared copy. All 17 tests and every tally pass unchanged.
2. **`pty_tee`, 53 s.** `a_line_past_the_cap_becomes_two_frames_and_is_named_on_run_end`
   was all of it, at 52 s of user CPU in the test process. The cause was in `src/pty.rs`
   rather than the test: `flush_pending` searched `pending` for a newline from the front on
   every read, so a 20 MiB line arriving in pty-sized reads was scanned quadratically. A
   provider printing a frame of several MB paid the same in the watcher. Cut: `tee` keeps
   how far the last search got and `flush_pending` resumes there; bytes only go on the end
   of `pending` between flushes, `persistent_eof` included, so the offset stays true, and it
   goes back to zero whenever a line or a capped frame comes off the front. The test asserts
   what it did and takes 1 s.
3. **`gate`, 12 s.** `a_run_cannot_rewrite_its_own_hook_or_provider_config_under_any_policy`
   9 s alone, then three tests at 2 to 4 s. Each is a run of `/usr/bin/python3` on the
   compiled gate per payload, 260 of them in the slowest, and that run is what the test
   asserts on. The interpreter is the file exec'd, so the first-run check does not apply.
   `batch_ask` would answer the 260 in a few processes, but the hook calls the single entry,
   and the batch-equals-single test does not cover `codex`, so the test would stop proving
   what it proves. Cut instead: each provider asks on its own scoped thread. 9 s became
   2.45 s. Planting a wrong expectation made three of the threads panic and the test fail,
   so an assertion in a thread still fails it.
4. **`git_config`, 10 s.** `a_global_config_that_includes_a_fifo_does_not_hang_hydras_git`
   was 10.07 s of it: `worktree::identity` and `repo_root` each wait out
   `CONFIG_READ_TIMEOUT`, 5 s, on a fifo nobody writes. A timeout waited to its end, but the
   timeout is the product's constant and the test proves that constant holds. Left alone;
   shortening it means a test-only knob on `src/worktree.rs` or a shorter timeout for real
   runs, and either is a decision.
5. **`crates/otel` `otel`, 7.5 s.** `otel_gives_what_the_python_gave` alone, 65 cases, 37
   of them planting the same stub `bin/hydra` as a new file. Same cause as 1. Cut: a `Stubs`
   in `otel.rs` keyed by content, hard-linked into each case. 0.9 s.

Neither the seatbelt tests nor `drift.rs` came near the top five: `sandbox_copilot` 4.4 s,
`drift` 0.9 s, the other three `sandbox_*` under 0.1 s. Nothing there was touched.

### What the ticket assumed that is false

"The wall time is whatever the slowest few take, since 296 made them run in parallel."
`cargo test` runs the binaries one after another. 296 let each binary's tests share its
threads. The suite's wall time is close to the sum of the binaries plus the build check:
95 s of binaries, 85 s of suite with cargo's default thread count against `--test-threads=4`
here. So every binary cut counts, not just the slowest. The slowest is now
`git_config`, and its 10 s is the timeout in item 4.

The first-run cost is not specific to these tests. Any test that writes a new executable
and runs it pays about 170 ms per file on this machine; the grader and otel ones were the
ones planting by the hundred.
