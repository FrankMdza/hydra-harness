# 75 - Admission runs under the proposer's seatbelt

Type: build
Status: resolved
Blocked by:

## What

Map ticket 66, resolved 2026-09-22: `rungs::evolve::admission_gate` runs `cargo fmt --check`,
`cargo clippy`, `cargo build` and `cargo test` in the proposer's worktree from the human's shell.
They are to run through `/usr/bin/sandbox-exec -f <profile>` with the profile the proposer's spawn
already wrote at `spawn/<proposer-run-id>/sandbox.sb`, and with `HYDRA_ROOT` pointed at an empty
scratch root so the corpus walks do not read a corpus the profile denies. The proposer's profile
is the right one because it carves out exactly that worktree.

## Done when

`admission_gate` takes the profile path and the scratch root, wraps each step on macOS and runs
it bare on Linux, and the pass's harness record names the profile sha the steps ran under and
the root. `tests/evolve.rs` plants a test in a fixture worktree that writes into the layout's
`runs/` and asserts admission reports the step failing rather than the file appearing; on a Mac
without `sandbox-exec` the test returns early the way the seatbelt tests do. `IMPLEMENTATION.md`'s
rung two section says what changed and that a hostile test can still read the home directory;
`SECURITY.md`'s sentence on admission moves from "outside the seatbelt" to "under the proposer's
profile, which bounds Hydra and not the machine".

## Outcome

`admission_gate` in `src/rungs/evolve.rs` now takes the proposer's `sandbox.sb` path and a scratch
root under `spawn/<pass-id>/admission-root/`. On macOS each of the four cargo steps runs as
`/usr/bin/sandbox-exec -f <profile> cargo ...` with `HYDRA_ROOT` set; on Linux it runs bare with
the same variable. A missing profile or missing `/usr/bin/sandbox-exec` on macOS fails closed naming
the path or binary rather than running bare. The pass harness `run_end` carries `admission_sandbox`
and `admission_hydra_root` beside the usual fields; `src/record/validate.rs` type-checks both when
present. `tests/evolve.rs::admission_runs_under_the_proposers_seatbelt_and_refuses_a_hostile_test`
plants `tests/zz_hostile.rs` in a minimal scratch worktree, asserts `cargo test` fails and
`<layout>/runs/hostile` does not appear, and returns early when `exec_available()` is false or
`inside_seatbelt()` is true, because nested `sandbox-exec` is refused on this machine. Measured with
`cargo test --release --test evolve`: 25 passed. `cargo clippy --all-targets -- -D warnings` exited 0.
The ticket assumed the hostile test could run in a Hydra-launched worktree; nested `sandbox-exec`
inside the proposer seatbelt is refused, so the test skips there.

Built by `cursor-agent` on Composer 2.5, run 01a0c956-462b under `standard`, 2.8M tokens.
