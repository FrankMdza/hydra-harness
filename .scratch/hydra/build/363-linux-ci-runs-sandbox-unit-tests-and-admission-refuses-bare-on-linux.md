# 363 - Linux CI runs `sandbox::tests` and `hydra evolve` refuses bare admission on Linux

Status: resolved

## Objective

Close two Linux gaps identified by the review panel on `2e99a25` (`01a17e09-b543-77e3-990c-5f0f95d696aa`):
1. `.github/workflows/ci.yml` passes `--skip sandbox` on the `ubuntu-latest` job, which skips all 12 pure unit tests in `src/sandbox.rs` (`sandbox::tests::*`), including the new bubblewrap composer and witness tests from builds 318a and 318b.
2. `src/rungs/evolve.rs` (`run_admission_step`) refuses on Linux when `wraps_launches()` is true, but falls through to `Command::new(bin)` (running proposer code bare) on Linux when `wraps_launches()` is false (`bwrap` absent or user namespaces disabled).

## Problem

1. In `.github/workflows/ci.yml`:
   - Lines 44 to 48 still say there is no Linux wrapper and that `--skip sandbox` drops `src/sandbox.rs` because it describes a wrapper Linux does not apply.
   - None of the 12 unit tests in `src/sandbox.rs` invokes `/usr/bin/sandbox-exec` or `bwrap`; they are pure cross-platform tests of `compose`, `compose_bwrap`, `rules_of`, `linux_userns_enabled_in`, `linux_inside_bwrap_from`, and `forbids_vendor_sandbox`. Only the integration tests in `tests/spawn.rs` invoke `/usr/bin/sandbox-exec`.
2. In `src/rungs/evolve.rs`:
   - Lines 1253 to 1281 have three branches: macOS seatbelt (`kind() == KIND_SEATBELT && wraps_launches()`), `else if crate::sandbox::wraps_launches()` (refusing because no bubblewrap admission profile exists yet), and `else { Command::new(bin) }`.
   - Under Decision 1 of build 318, proposer code must never run bare when no admission wrapper is available. Replacing the `else if` + `else` with a single `else` branch that returns the refusal error guarantees `run_admission_step` never runs proposer code bare on Linux whether `bwrap` is present or not.

## Cases

1. `cargo test --release --lib sandbox::tests::` runs and passes all 12 unit tests in `src/sandbox.rs`.
2. In `src/rungs/evolve.rs`, when `kind != KIND_SEATBELT` (for example `KIND_BUBBLEWRAP`, whether `wraps_launches` is `true` or `false`), the admission step refuses with `"admission gate: no bubblewrap profile for admission yet, so it refuses to run proposer code bare"` and never spawns `bin` bare.
3. On macOS with `KIND_SEATBELT` and `wraps_launches() == true`, `run_admission_step` still wraps `bin` in `/usr/bin/sandbox-exec -f <profile>`.

## Do

1. In `src/rungs/evolve.rs`:
   - Extract a pure helper `admission_wrapper_check(kind: &str, wraps_launches: bool, profile_exists: bool, exec_available: bool) -> Result<(), String>` (or equivalent) and call it in `run_admission_step`, removing the bare `Command::new(bin)` fallback so any non-seatbelt platform returns `Err("admission gate: no bubblewrap profile for admission yet, so it refuses to run proposer code bare".into())`.
   - Add a unit test in `src/rungs/evolve.rs` testing `admission_wrapper_check` across `(KIND_SEATBELT, true, true, true)`, `(KIND_BUBBLEWRAP, true, true, true)`, and `(KIND_BUBBLEWRAP, false, false, false)`.
2. In `.github/workflows/ci.yml`:
   - Update the comment at lines 44 to 48 so it accurately states that `src/sandbox.rs` unit tests run on Linux while `--skip sandbox` on the workspace test step skips the `/usr/bin/sandbox-exec` integration tests in `tests/spawn.rs`.
   - Add `- run: cargo test --release --lib sandbox::tests::` to the `linux` job before the workspace `cargo test` step.
3. Update `IMPLEMENTATION.md` around the build 318 admission note to state that `run_admission_step` refuses on Linux both with and without `bwrap`, and that Linux CI runs `sandbox::tests::`.
4. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo test --release --lib sandbox::tests::`, and `cargo test --release --lib rungs::evolve::`.

## Outcome

Built:

- `src/rungs/evolve.rs` has `admission_wrapper_check`, a pure helper. Only a seatbelt that wraps launches, with its profile on disk and `sandbox-exec` present, returns `Ok`. Every other case returns an error before any spawn. `run_admission_step` calls it and then always wraps `bin` in `/usr/bin/sandbox-exec -f`. The bare `Command::new(bin)` branch is gone.
- A new unit test, `admission_wrapper_check_refuses_every_platform_but_a_wrapped_seatbelt`, covers the three cases in the ticket.
- `.github/workflows/ci.yml`: the `linux` job comment now says the `sandbox` skip drops only the `tests/spawn.rs` integration tests. A new step runs `cargo test --release --lib sandbox::tests::` before the workspace test step.
- `IMPLEMENTATION.md`: the build 318 admission note and the admission paragraph in the evolve section now say admission refuses on Linux with and without bwrap, and that Linux CI runs `sandbox::tests::`.

Measured on macOS, where this was built:

- `cargo test --release --lib sandbox::tests::`: 12 passed.
- `cargo nextest run --release -p hydra --lib rungs::evolve::`: 5 passed, including the new test.
- `cargo fmt --all --check`: clean.
- `cargo clippy --release --workspace --all-targets -- -D warnings`: clean.

Not measured: the Linux job. No Linux host ran these steps. I checked by reading that the 12 tests need no macOS path, since `system_sha256` falls back to `sha256sum`. The `tests/evolve.rs` admission tests that call the gate already skip when `exec_available()` is false, so a Linux host without bwrap skips them. A host with bwrap already refused before this change.

Departures from the ticket's wording:

- The missing-profile error no longer names the path. The helper takes `profile_exists` as a bool, so the path is not in its error. The path is still known to the caller if a later change needs it.
- A seatbelt that does not wrap launches gets its own message. The ticket's message names bubblewrap, which would be wrong on macOS.
