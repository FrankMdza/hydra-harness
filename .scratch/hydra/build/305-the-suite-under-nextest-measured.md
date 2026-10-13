# 305 - The suite under nextest, measured against cargo test

Type: build
Status: claimed
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
