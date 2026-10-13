# 310 - The warm build timed crate by crate: what holds the 38 seconds a run now pays

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from build 309's Outcome: with the parent's `target/`
cloned, the first `cargo build --release --workspace` in a run compiles the 8 workspace crates
in 37.7 s, and a rebuild after touching `src/lib.rs` took 79.4 s. That is now the time every
builder pays before its first test, and again after every edit to the main crate.

## What

The dependencies are paid for once and cloned. What remains is the workspace's own crates: the
`hydra` library and binary, `hydra-puppets`, `hydra-grader`, `hydra-pyjson`, `hydra-records`,
`hydra-otel`, `hydra-apprentice`, `hydra-local-llm`, and 76 test binaries when the suite builds.
Nobody has measured which of them holds the critical path, how much is codegen against
linking, or what the release profile's defaults (`codegen-units`, `lto`, `debug`) cost on an
edit-build-test loop that runs in a worktree dozens of times a day. This ticket measures,
then changes only what the numbers justify.

## Do

1. You are the first run launched with a cloned `target/`. Before anything else, time
   `cargo build --release --workspace` in your worktree as it is and write that number down
   with the load average: it is the real first-build figure 309 could not measure.
2. `cargo build --release --workspace --timings` after `touch src/lib.rs`, and after
   `touch crates/pyjson/src/lib.rs`. Read the HTML's unit table (it is under
   `target/cargo-timings/`) and put the ten longest units in the Outcome with their time,
   and the critical path. The same for `cargo nextest run --release --workspace --no-run`
   after `touch src/lib.rs`, which is what a builder pays after an edit: say how much of it is
   the 76 test binaries linking.
3. Measure the profile knobs one at a time, each on the `touch src/lib.rs` build, three runs
   each, load noted: `codegen-units = 16` against the default for release, `lto = "off"`
   against the default (`false` is thin-local), and `debug = false` if the release profile
   carries any. Measure linking with the system linker against `-C link-arg=-fuse-ld=lld` only
   if `lld` is already on this machine (`which ld64.lld`); do not install anything.
4. Apply in `Cargo.toml` only a knob that saves more than 10 percent on the edit build, in a
   `[profile.release]` block with a comment naming this ticket and the measured delta, and
   confirm the suite is green with it and `target/release/hydra` still passes
   `tests/record_contract.rs`. If nothing clears 10 percent, change nothing and say so; the
   tables are the deliverable.
5. Amend `CLAUDE.md`'s sentence under "The build" about the dev profile if the release
   profile now carries a knob worth knowing, and `IMPLEMENTATION.md` beside the 295 and 309
   paragraphs.

## Done means

The first-build figure, the two unit tables with their critical paths, the knob table, and
either a `Cargo.toml` change with its measured delta or a sentence saying nothing cleared the
bar. `cargo nextest run --release --workspace` and `cargo test --release --workspace --doc`
green.

## Outcome

Resolved on 2026-10-13 inside the launched run, using only this worktree. No dependency or
tool was installed. The ticket was already claimed at entry.

### Method and the inherited target

The worktree **did arrive with an inherited, cloned `target/`**. Before the first build,
`ls target` printed `CACHEDIR.TAG`, `debug`, and `release`. No parent directory was inspected.
The first build compiled the eight workspace crates and reused the dependency cache.
It took **22.008 s**, with load averages **4.86 / 10.31 / 20.66** before and
**6.68 / 10.31 / 20.36** after. This is the real launched-worktree figure missing from 309.
309's synthetic seeded checkout took 37.7 s at one-minute loads between 27 and 38;
its 79.4 s rebuild is not reproduced here. The lower load prevents attributing the difference
solely to the clone. The clone's own duration cannot be recovered from inside this run.

Perl `Time::HiRes` timed that first command. Subsequent commands used Python's monotonic
clock around the whole process, including Cargo startup and nextest discovery. Each ran
serially, with inherited environment unchanged except for the single measured profile variable
or the explicitly identified linker instrument. Rust 1.98.1, Cargo 1.98.1; the installed
nextest from 305 was used. HTML unit durations and starts are rounded to hundredths by Cargo.
Loads throughout this Outcome are 1 / 5 / 15 minutes. Raw logs, HTML copies, parsed unit tables,
and measurement scripts remain untracked under `target/hydra310/`.

### Build commands and loads

`hydra-units` and `pyjson-units` mean `cargo build --release --workspace --timings` after
respectively touching `src/lib.rs` and `crates/pyjson/src/lib.rs`. `test-units` is the first
`cargo nextest run --release --workspace --no-run --timings` after touching `src/lib.rs`;
`test-edit-units` repeats it after all test artifacts have been warmed, without instrumentation.

| measurement | whole command, s | load before | load after |
|---|---:|---|---|
| hydra-units | 15.696 | 6.35 / 10.12 / 20.18 | 6.88 / 10.04 / 19.97 |
| pyjson-units | 7.679 | 6.10 / 9.71 / 19.68 | 6.57 / 9.75 / 19.63 |
| test-units | 69.832 | 6.02 / 9.52 / 19.44 | 10.29 / 10.36 / 18.99 |
| test-edit-units | 51.678 | 16.45 / 14.56 / 19.52 | 13.33 / 14.08 / 19.03 |

### The ten longest units after touching hydra

This build has only two rebuilt units. The next eight in duration order are cache hits,
represented as zero by Cargo, not eight instantaneous compilations. An empty target suffix
means a library throughout these tables. All rows share the hydra-units loads above.

| unit | duration, s | start, s | finish, s |
|---|---:|---:|---:|
| hydra library | 14.66 | 0.21 | 14.87 |
| hydra binary | 0.76 | 14.88 | 15.64 |
| allocator-api2 (cached) | 0.00 | 0.00 | 0.00 |
| anstream (cached) | 0.00 | 0.00 | 0.00 |
| anstyle (cached) | 0.00 | 0.00 | 0.00 |
| anstyle-parse (cached) | 0.00 | 0.00 | 0.00 |
| anstyle-query (cached) | 0.00 | 0.00 | 0.00 |
| anyhow library (cached) | 0.00 | 0.00 | 0.00 |
| anyhow build-script (cached) | 0.00 | 0.00 | 0.00 |
| anyhow build-script run (cached) | 0.00 | 0.00 | 0.00 |

Critical path: Cargo startup to 0.21 s, hydra library to 14.87 s, hydra binary to 15.64 s.
The library's recorded sections are **4.14 s frontend and 10.52 s codegen**. Cargo does not
split the binary's 0.76 s into codegen and link; that entire unit must not be called link time.
Only two units rebuild, not the whole workspace after every main-crate edit.

### The ten longest units after touching pyjson

Twelve units rebuild. All rows share the pyjson-units loads above.

| unit | duration, s | start, s | finish, s |
|---|---:|---:|---:|
| hydra-grader library | 6.45 | 0.98 | 7.43 |
| hydra-puppets library | 4.44 | 0.41 | 4.85 |
| hydra-puppets binary | 2.25 | 4.85 | 7.10 |
| hydra-apprentice library | 1.87 | 0.73 | 2.60 |
| hydra-otel library | 1.66 | 0.91 | 2.57 |
| hydra-pyjson library | 1.52 | 0.19 | 1.71 |
| hydra-records library | 1.01 | 0.41 | 1.42 |
| hydra-apprentice binary | 0.37 | 2.60 | 2.97 |
| hydra-otel binary | 0.34 | 2.57 | 2.91 |
| hydra-local-llm binary | 0.31 | 1.71 | 2.02 |

Critical path follows metadata pipelining: pyjson frontend 0.19-0.41 s, records frontend
0.41-0.73 s, grader scheduled at 0.98 s, grader library through 7.43 s, grader binary
through 7.66 s (0.23 s, outside the top ten). Grader spends 2.83 s in frontend and 3.62 s
in codegen. The overlapping puppets branch finishes at 7.10 s. Summing the twelve unit
durations, 20.69 s, would overstate the 7.679 s wall clock.

### Test build: ten longest units, first and warm edit

The first test build refreshes workspace test artifacts still fingerprinted for the parent's
checkout and artifacts invalidated by the preceding pyjson touch: 79 rebuilt units, including
77 Cargo test-harness units. The final nextest suite reports 77 binaries; the ticket and 305
counted 76. Cargo finished in 53.22 s; whole nextest no-run was
69.832 s, leaving 16.612 s for command overhead and discovery. These are not test execution
seconds. All rows in the first column share the test-units loads above.

| first test-build unit | duration, s | start, s | finish, s |
|---|---:|---:|---:|
| hydra library | 28.86 | 0.13 | 28.99 |
| hydra library tests | 23.30 | 0.13 | 23.43 |
| hydra gate tests | 12.24 | 29.35 | 41.59 |
| hydra-grader differential tests | 11.39 | 0.13 | 11.52 |
| hydra seats tests | 10.43 | 36.10 | 46.53 |
| hydra-puppets differential tests | 9.61 | 2.04 | 11.65 |
| hydra cockpit tests | 9.35 | 43.88 | 53.23 |
| hydra-puppets library tests | 8.80 | 3.79 | 12.59 |
| hydra normalize tests | 8.05 | 41.20 | 49.25 |
| hydra harvest tests | 7.12 | 28.99 | 36.11 |

Observed critical chain: hydra library 0.13-28.99 s (12.77 s frontend, 16.09 s codegen),
job-slot scheduling before cockpit starts at 43.88 s, then cockpit through 53.23 s.
Cockpit's 14.89 s wait after its dependency is ready is resource scheduling, not a dependency
on every test that happened to run first. Test-unit durations sum to 246.44 s and overlap.

The warm uninstrumented edit rebuilds **45 units: the hydra library, its binary, and 43 Cargo
test-harness units**. Tests of other workspace crates stay cached. Cargo takes 43.67 s;
whole nextest no-run takes 51.678 s, with 8.008 s overhead/discovery. All rows below share
the test-edit-units loads above.

| warm edit unit | duration, s | start, s | finish, s |
|---|---:|---:|---:|
| hydra library | 20.30 | 0.09 | 20.39 |
| hydra library tests | 15.22 | 0.09 | 15.31 |
| hydra gate tests | 12.08 | 20.40 | 32.48 |
| hydra cockpit tests | 11.22 | 32.45 | 43.67 |
| hydra seats tests | 9.78 | 28.25 | 38.03 |
| hydra normalize tests | 8.71 | 31.46 | 40.17 |
| hydra harvest tests | 6.94 | 20.40 | 27.34 |
| hydra outcome tests | 6.55 | 28.01 | 34.56 |
| hydra spawn tests | 5.05 | 29.29 | 34.34 |
| hydra evolve tests | 4.86 | 34.47 | 39.33 |

Observed critical chain: hydra library 0.09-20.39 s (4.38 s frontend, 15.92 s codegen),
12.06 s of resource scheduling before cockpit at 32.45 s, then cockpit through 43.67 s.

### What is actually linking

Cargo records no phase sections for its test-harness units. A supplementary warm edit used a
local `RUSTC_WRAPPER` to select a local linker wrapper that calls the same system `cc`, timing
each subprocess. Nothing was installed or persisted in project configuration. The first
instrument attempt, 59.568 s at loads 9.02 / 10.09 / 18.79 before and
25.80 / 15.05 / 20.06 after, lost jobserver descriptors; it was discarded. The revised wrapper
preserved those descriptors. It took 55.037 s, loads 24.06 / 14.87 / 19.97 before and
17.71 / 14.77 / 19.62 after; Cargo took 46.12 s.

The instrumented rebuild made 44 successful `cc` calls totaling **5.168 s**. The ordinary
hydra binary used 0.124 s, so the 43 test-harness links totaled **5.044 s**; those test units
as a whole totaled 172.59 s. These are sums across concurrent units, not a wall-clock
subtraction from 55.037 s. The slowest call, models, was 0.184 s at load
21.82 / 15.36 / 19.91 before and after. Each call's load is in `linkcalls.jsonl`.

Limitation: explicitly selecting a linker makes this toolchain try external `rust-objcopy`
for debuginfo stripping, which warns that its LLVM dylib is missing. The successful `cc`
subprocess durations are a direct measurement of linking, but this instrumented whole-build
figure is excluded from the knob comparison. The clean uninstrumented build was repeated
immediately afterward. Thus this is evidence that compilation dominates the warm edit,
not a precise estimate of every original build's link phase. The task's "76 test binaries
linking after an edit" assumption is false: only hydra's 43 Cargo harnesses relink on this edit.

### Profile knobs, one at a time

Each measured repetition touches `src/lib.rs` then runs `cargo build --release --workspace`.
The initial repetitions used `--verbose` to retain the effective compiler flags; confirmations
omit verbosity. Variables were command-local, never combined. The first explicit codegen-units
build changed dependency fingerprints and compiled 100 crates, so its 32.219 s is **warm-up**,
not an edit-build datum. Its loads were 11.87 / 13.57 / 18.56 before and
18.40 / 15.22 / 19.00 after. A fourth repetition supplies the third warm datum. The lto-off
warm-up took 30.688 s, loads 15.62 / 14.82 / 18.72 before and 18.91 / 15.74 / 18.92 after.

| profile | repetition | edit build, s | load before | load after |
|---|---|---:|---|---|
| defaults | 1 | 15.411 | 13.33 / 14.08 / 19.03 | 12.39 / 13.81 / 18.82 |
| defaults | 2 | 15.307 | 12.39 / 13.81 / 18.82 | 12.43 / 13.75 / 18.71 |
| defaults | 3 | 15.313 | 12.43 / 13.75 / 18.71 | 11.87 / 13.57 / 18.56 |
| codegen-units = 16 | 1 (raw 2) | 15.328 | 18.40 / 15.22 / 19.00 | 16.82 / 15.02 / 18.86 |
| codegen-units = 16 | 2 (raw 3) | 15.447 | 16.82 / 15.02 / 18.86 | 15.62 / 14.82 / 18.72 |
| codegen-units = 16 | 3 (raw 4) | 16.255 | 12.00 / 14.38 / 18.12 | 10.93 / 13.98 / 17.89 |
| lto = "off" | 1 | 12.342 | 18.91 / 15.74 / 18.92 | 16.88 / 15.44 / 18.75 |
| lto = "off" | 2 | 12.107 | 16.88 / 15.44 / 18.75 | 15.87 / 15.26 / 18.65 |
| lto = "off" | 3 | 12.724 | 15.87 / 15.26 / 18.65 | 17.82 / 15.67 / 18.74 |

| knob | default median, s | knob median, s | saving | decision |
|---|---:|---:|---:|---|
| codegen-units = 16 | 15.313 | 15.447 | -0.9% | leave implicit; release already defaults to 16 |
| lto = "off" | 15.313 | 12.342 | 19.4% | clears the 10% bar |
| debug = false | already false | not repeated | none available | no release debuginfo to remove |
| system linker vs lld | system only | not run | not measured | `which ld64.lld` returned not found; nothing installed |

The default's implicit `lto = false` permits local thin LTO; `"off"` disables it. Even the
slowest off repetition beats the fastest default by 2.583 s while running at higher load.
The codegen-units spelling changes Cargo's fingerprints but does not improve compilation.
No debug or linker setting was added.

Restored defaults, warmed their artifacts, then repeated a touched build; restored off,
warmed its artifacts, then repeated the same touch. Confirmation:

| profile | edit build, s | load before | load after |
|---|---:|---|---|
| restored defaults | 15.822 | 10.42 / 13.72 / 17.72 | 9.86 / 13.42 / 17.55 |
| restored off | 12.369 | 9.67 / 13.26 / 17.44 | 9.48 / 13.04 / 17.29 |

That pair saves 21.8%, supporting the first three-run result. Transition warm-ups were
16.562 s for explicit codegen-units (loads 12.61 / 14.62 / 18.27 to
12.00 / 14.38 / 18.12), 16.196 s for defaults (10.93 / 13.98 / 17.89 to
10.42 / 13.72 / 17.72), and 12.472 s for off (9.86 / 13.42 / 17.55 to
9.67 / 13.26 / 17.44); none enters an edit-build median.

Added only `lto = "off"` in a root `[profile.release]` block with the ticket and measured
19.4% delta in its comment. Updated the dev-profile sentence in `CLAUDE.md` to include
this release setting, and documented the measurement beside builds 295 and 309 in
`IMPLEMENTATION.md`. No Rust source, test assertions, runner configuration, or dependency
changed. Touching source files changed timestamps only. Runtime throughput was not benchmarked;
this ticket measures compilation and checks behavior with the suite.

With that setting actually in Cargo.toml and no profile variables, a touched
`cargo build --release --workspace --timings` took **12.104 s**, loads
7.19 / 11.94 / 16.66 before and 8.22 / 11.99 / 16.63 after. The library unit took 11.38 s,
with 4.13 s frontend and **7.25 s codegen**, followed by the 0.61 s binary unit.
This checks that the adopted configuration reproduces the command-local experiment.
The near-identical frontend against the initial 4.14 s and the shorter codegen against
10.52 s support the compilation-phase explanation, subject to the recorded load difference.

Build 295's dev change remains a disk result (1.3 GB to 1.1 GB, 23 s to 22 s at a concurrent
run's unreported exact load), not evidence for a release timing improvement. Build 304's
suite reductions, 84.7/85.6 s whole-suite and 95.11 s summed binaries at loads 7.14 to 4.28,
and build 305's 42.563 s median nextest-plus-doc against 81.627 s cargo-test median, with
their per-run load triples in those Outcomes, concern executing already-built tests.
They must not be compared directly with this ticket's 69.832 s or 51.678 s no-run builds.
The cold 13 min 21 s from 305 also included building tests; the real inherited-target build
here is the separate 22.008 s workspace binary build reported above.

### Validation on the adopted profile

All commands below ran with the profile in Cargo.toml, without the experimental environment
variables or wrappers. Loads are again 1 / 5 / 15 minutes.

| command | whole command, s | load before | load after | result |
|---|---:|---|---|---|
| `cargo nextest run --release --workspace` | 90.207 | 7.11 / 11.63 / 16.44 | 13.14 / 12.73 / 16.40 | 1,384 passed, 4 skipped, 77 binaries |
| `cargo test --release --workspace --doc` | 0.931 | 12.41 / 12.58 / 16.33 | 11.97 / 12.49 / 16.27 | eight crate doctest harnesses passed, zero doctests |
| `cargo test --release --test record_contract` | 0.352 | 11.97 / 12.49 / 16.27 | 11.97 / 12.49 / 16.27 | all 20 passed |
| `cargo fmt --all --check` | 0.820 | 10.06 / 12.04 / 16.05 | 10.06 / 12.04 / 16.05 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 22.257 | 10.06 / 12.04 / 16.05 | 9.41 / 11.70 / 15.81 | clean |

The nextest command included 30.26 s of Cargo compilation for the newly adopted profile and
42.659 s of reported test execution; the remaining 17.288 s was startup/discovery overhead.
Those phases share the full command's recorded loads above; phase-boundary loads were not
sampled. There was no suite failure and no record-contract failure. The final suite has
77 binaries and 1,384 active tests, rather than the ticket's 76 and 305's 1,381.
`target/release/hydra` was rebuilt with off and the contract tests passed both within nextest
and under the direct Cargo command. Those tests validate Hydra's library writer/validator and
fixture corpus; they do not themselves launch the executable. The old skill's claim of an
unavoidable worktree corpus failure did not hold in this run. No test was weakened or edited.

The meaningful gain is the 19.4% warm edit-build median reduction, confirmed by restoring both
profiles and by a build from the committed-to-file setting. The first-build, unit tables,
critical chains, linker limitations, profile repetitions, and all loads are recorded above.
