# 317 - The drift table keyed by provider, OS and arch, every row with its date and probe

Type: build
Status: resolved
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

## Outcome

`MEASURED_VERSIONS` is a table of `MeasuredVersion { provider, os, arch, version, measured_on,
probe }`. The four rows are macos aarch64: `claude` 2.1.268 and `cursor-agent` 2026.09.18-9a7762b
and `agy` 1.2.8 on 2026-09-22 (`git log -S` on `5c00b28`), `codex` 0.154.0 on 2026-09-21
(`80b3be6`). `measured_version` looks up `measured_platform()`, which reads `consts::OS` and
`consts::ARCH` unless both `HYDRA_DRIFT_OS` and `HYDRA_DRIFT_ARCH` are set, and returns `None`
when no row matches. `hydra doctor` then prints `{binary} unmeasured on {os} {arch}; run
tests/drift.rs there` and does not set `missing`. `tests/doctor.rs::doctor_prints_unmeasured_when_the_platform_has_no_row`
drives the binary with those two env vars set to `linux` and `x86_64`. `tests/drift.rs` skips
with that same message when the lookup is `None`.

Measured: the three provider unit tests and the doctor unmeasured test passed (0.012 s, 0.013 s,
0.014 s, 0.380 s); the judge drift probe still ran on this macos aarch64 machine (1.175 s).
`cargo nextest run --release --workspace`: 1392 passed, 4 skipped, 47.446 s. Clippy 34.85 s,
no warnings. Doc-tests: 0 tests, ok.

False: `tests/drift.rs` did not compare a probed `--version` against the table, it only put the
probed string in panic messages, so the change is a skip on a missing row rather than a version
equality assert (that would go red on self-update). `cursor-agent` and `agy` have no test in
that file; those two `probe` fields are empty. Doctor never counted a version mismatch as
`missing` either; the unmeasured line replaces the comparison print, it does not change an
exit-code rule that was not there.
