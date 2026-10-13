# 305 - The suite under nextest, measured against cargo test

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from build 304's Outcome: `cargo test` runs the 76 test
binaries one after another, so the suite's wall time is their sum (95 s of binaries, 85 s of
suite on an idle machine) and not the slowest one. The Hydra list of 2026-10-05 proposed
`nextest` for the same reason and build 296 did without it.

## What

`cargo-nextest` runs every test in its own process and schedules tests from every binary on
one pool, so binaries no longer wait for each other. It also changes what a test may assume:
nothing shared in-process between tests of one binary survives (a `OnceLock`, a static counter,
a `Stubs` directory built per test is fine, one built per binary is not), and a test that
spawns the test binary by name to re-execute itself (`git_config.rs`, `launch_phases.rs`,
`src/run/mod.rs` since 302) must still find the named test under nextest's harness. This
ticket measures whether the gain is real on this machine and whether the suite survives the
change, and decides nothing about CI.

## Do

1. `cargo install cargo-nextest --locked` into `~/.cargo/bin` (a tool install outside the
   repo; say the version in the Outcome). Add nothing to `Cargo.toml`.
2. Measure on the same warm build, three runs each, load average noted before each:
   `cargo test --release --workspace` and `cargo nextest run --release --workspace`. nextest
   does not run doctests; run `cargo test --release --workspace --doc` beside it and add its
   time so the two columns compare the same work.
3. List every test nextest fails or skips that `cargo test` passes, with the reason read from
   the file. The re-exec tests are the ones to watch: nextest runs each test as
   `<binary> --exact <name> --nocapture` itself, so a child that calls the same binary should
   still work, but prove it.
4. If nextest wins by more than noise and nothing fails, write a `.config/nextest.toml` with
   the profile that reproduces the measurement, and one sentence in `CLAUDE.md` under "The
   build" saying both commands exist and which the session runs. Do not touch
   `.github/workflows/ci.yml`; whether CI moves is a line in the Outcome for the human.
5. If a test fails under nextest for a reason that is the test's and not nextest's, fix the
   test only if the fix loses no assertion; otherwise name it and leave it.

## Done means

A table of six timings plus the doctest time, the list from step 3 (empty is a result), and
either the config file and the `CLAUDE.md` sentence or a sentence saying why not. The suite
green under both runners if the config is added.

## Outcome

Measured on 2026-10-13. Installed `cargo-nextest 0.9.148` (commit `cd1d6d546`,
2026-10-07, `aarch64-apple-darwin`) with `cargo install cargo-nextest --locked`.
The network was reachable. No crate dependency was added.

### Method and timings

Warmed the release workspace with `cargo test --release --workspace --no-run` once
(13 min 21 s for this worktree's initial build). Finished the tool installation before
measuring. Perl `Time::HiRes` measured each complete command, including the build check
and nextest discovery, redirecting stdout/stderr to separate logs. Kept the run's inherited
environment unchanged. Alternated the runners on this eight-core machine and recorded
all three load averages before each command. Scripts and raw logs are untracked under
`target/hydra305/`.

Six successful suite timings, in seconds; loads are 1 / 5 / 15 minutes:

| repetition | cargo test | load before cargo | nextest | load before nextest | doctest command | load before doctests | nextest + doctests |
|---|---:|---|---:|---|---:|---|---:|
| 1 | 81.627 | 13.87 / 25.61 / 22.10 | 53.798 | 19.64 / 29.05 / 22.94 | 1.497 | 13.87 / 25.61 / 22.10 | 55.295 |
| 2 | 82.055 | 9.88 / 19.76 / 20.24 | 41.783 | 8.46 / 21.09 / 20.71 | 0.780 | 9.88 / 19.76 / 20.24 | 42.563 |
| 3 | 80.221 | 8.82 / 15.75 / 18.55 | 41.601 | 6.85 / 16.38 / 18.90 | 0.750 | 8.82 / 15.75 / 18.55 | 42.351 |

Commands: `cargo test --release --workspace`, `cargo nextest run --release --workspace`,
and `cargo test --release --workspace --doc`. The last invokes eight crate doctest
harnesses, each currently containing zero tests. Both full runners pass 1,381 active tests
across 76 binaries, with four ignored tests.

Median: cargo test **81.627 s**, nextest plus doctests **42.563 s**, a **47.9%** reduction
(**1.92x** faster). Even the slowest nextest-plus-doc result beats the fastest cargo result
by 24.926 s. The first nextest invocation paid more discovery/startup time (53.798 s wall
versus 44.471 s reported test time) and started at higher load. Later nextest reported test
times were 41.259 s and 41.117 s. The gain exceeds the repetition noise. Build 304's cargo
suite times were 84.7 s and 85.6 s, and its serial sum with four threads per binary was
95.11 s. These cargo measurements are close to that baseline despite the decaying load.

An initial cargo run is excluded: it failed after 78.865 s at load 34.20 / 32.90 / 23.48,
before finishing the workspace. `tests/stop.rs::a_stop_that_fails_pastes_nothing`, line 689,
expected a rendered card in `live` while it still showed `stopping`; the preceding assertion
already saw "was not stopped". `reported` pumps, renders, then checks whether the stop remains
in flight, so completion between rendering and checking is consistent with this failure.
This is an inferred screen/completion race. Left unchanged: it failed under cargo, passed
all three following full cargo runs and every nextest run, and falls outside this ticket's
nextest-only test-fix scope. The failed log is `cargo-1.log`; the table uses `cargo-2`,
`cargo-3` and `cargo-4`. The initially assumed green suite therefore had one intermittent
failure, but three complete successful measurements were obtained for each runner.

### Compatibility

Tests nextest fails or skips while cargo test passes: **none observed**.
No assertion or test changed. Both leave the same four declared ignores:

- `tests/fixture_corpus.rs::write_the_fixture_corpus`: manual fixture rewrite.
- `tests/cockpit.rs::planted_inner`: called by the symlinked-binary parent test.
- `crates/apprentice/src/bank.rs::bank_build_times_a_synthetic_corpus`: manual timing fixture.
- `crates/puppets/tests/differential.rs::write_the_jobs_record_py_runs`: manual comparison input writer.

A directed nextest run captured successful stdout/stderr for nine re-exec candidates
(`reexec.log`). Three in `src/run/mod.rs` executed and passed:
`a_run_with_a_repo_is_handed_its_own_object_directory`,
`inject_skills_does_not_treat_a_codex_copy_as_a_collision_with_itself`, and
`a_failed_injection_leaves_the_names_it_wrote_where_the_failure_record_reads_them`.
Their helpers assert the exact named child's success line, proving child execution.

`tests/launch_phases.rs::a_post_worktree_handoff_failure_through_run_writes_never_started`
returns before re-execution inside the seatbelt (lines 56-59). Captured under both runners,
it prints the same "skip: HYDRA_ROOT is refused from inside the seatbelt". This is a runtime
early return reported as passed, not one of the four declared ignores. Its outside-seatbelt
re-exec branch cannot be proved inside this run; no policy was widened.

### Adopted profile

Added `.config/nextest.toml`: default profile, `test-threads = 8`, `retries = 0`, reproducing
the measured settings. The full workspace passed again with the file present: 1,381 passed,
four skipped, 42.734 s reported test time. Added the sentence under "The build" in
`CLAUDE.md` naming both runners and choosing nextest followed by the doctest command for
sessions. `IMPLEMENTATION.md` records the measurement and clarifies that cargo runs binaries
sequentially, with parallel tests inside each. CI stays unchanged; whether it adopts nextest
is for the human.

All five tests in tests/git_config.rs also passed the directed nextest run, including the FIFO timeout at 10.103 s. The isolated_test helper asserts the exact named child success line, so all five child executions were proved.

Validation: cargo build --release passed; cargo fmt --all --check and cargo clippy --workspace --all-targets -- -D warnings passed clean. Three full cargo suites, three measured nextest suites, the configured nextest suite and all three doctest commands passed. No Rust source, test, crate dependency or CI workflow changed.
