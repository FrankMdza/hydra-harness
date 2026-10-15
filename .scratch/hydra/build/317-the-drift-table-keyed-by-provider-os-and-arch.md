# 317 - The drift table keyed by provider, OS and arch, every row with its date and probe

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-15, by the session, from map 119's answer, question 3. `MEASURED_VERSIONS` in
`src/provider/mod.rs` lines 29 to 34 has four rows, one per provider, and no OS, arch, date or
probe. The panel wanted one table keyed by provider, OS and arch, each row saying when and by
which probe it was measured, and `hydra doctor` saying "unmeasured on this platform" where no
row matches, instead of comparing against a version measured on another machine.

## What

Replace the array with a table of rows `{provider, os, arch, version, measured_on, probe}`.
The four existing rows become `macos`, `aarch64`, with the date the version string entered the
file (`git log -S` on the string gives it) and the probe `tests/drift.rs`'s test name. Keep
`measured_version(self)` working for the current platform by looking up `consts::OS` and
`consts::ARCH`, returning `None` where no row matches.

`hydra doctor` already compares the installed CLI versions against the table (read `src/doctor.rs`
for the exact lines). Where the lookup is `None` it prints one line per provider: "unmeasured on
linux x86_64; run tests/drift.rs there", and does not count that as drift.

## Do

1. The table and the lookup in `src/provider/mod.rs`, with unit tests for a matching platform,
   a missing one, and the four rows present.
2. `src/doctor.rs`: the unmeasured line; a test in `tests/` that drives doctor with the table's
   platform forced to one with no row (a `#[cfg(test)]` override or an env var the test sets,
   say which in the Outcome).
3. `tests/drift.rs`: where it compares a probed version against the table, use the lookup, and
   skip with a message rather than fail when the platform has no row.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the drift rule is described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
