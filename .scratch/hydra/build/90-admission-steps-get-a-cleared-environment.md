# 90 - Rung two's admission steps start from a cleared environment

Type: build
Status: claimed
Blocked by:

## What

Finding G43 of the second hiring panel, 2026-09-23, raised by `codex`. It is G1's class on the one
path build ticket 88 did not touch. `run_admission_step` (`src/rungs/evolve.rs` ~563) builds a
`Command` in both branches, seatbelt and bare, with only `.env("HYDRA_ROOT", ...)` and no
`env_clear`. So `cargo build` and `cargo test` of a proposer's patch run with the human's whole
shell environment, API keys and agent sockets included, under a profile that leaves the network
open and home readable.

Clear the environment in both branches and pass an explicit allowlist of what `cargo` and `rustc`
need. Start from `PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `CARGO_HOME`, `RUSTUP_HOME` and
`RUSTUP_TOOLCHAIN`, drop what a real admission run proves unnecessary, and add only what it proves
necessary. Put the list beside the provider allowlist in `src/env.rs` if it fits there, as its own
named constant; `src/env.rs` is a protected rung-two path, which is where this belongs.

## Done when

- A test plants a variable (for example `HYDRA_ADMISSION_CANARY=planted`) in the test process and
  drives an admission step whose `bin` prints its environment (`/usr/bin/env`); the output does not
  contain the canary and does contain `HYDRA_ROOT`. Make `run_admission_step` reachable from the
  test the smallest way (a `pub(crate)` unit test in `evolve.rs` is fine). Run it without the clear
  and say in the Outcome that it failed.
- A real admission, `cargo build` and `cargo test` on this repository in a scratch worktree through
  the same function, still passes. Say in the Outcome which variables it needed.
- `SECURITY.md` lines 40 to 50, where the admission gate is described, say the steps get a cleared
  environment and name the allowlist.

## Notes for the run

Tests take a scratch `Layout`, never `Layout::discover()`. Ignore
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`. Run `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` before you stop. Build ticket 89 edits other functions
in `evolve.rs` at the same time; keep your change inside `run_admission_step`, the new constant and
the test.
