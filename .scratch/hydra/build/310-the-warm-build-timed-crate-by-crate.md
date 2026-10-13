# 310 - The warm build timed crate by crate: what holds the 38 seconds a run now pays

Type: build
Status: claimed
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
