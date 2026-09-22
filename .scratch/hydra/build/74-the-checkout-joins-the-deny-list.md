# 74 - The checkout the binary was built from joins the deny list

Type: build
Status: resolved
Blocked by:

## What

Map ticket 65, resolved 2026-09-22: `hydra_home()` is denied to writes under every policy, on the
gate and in the seatbelt profile, with the run's own worktree and a `curate` pass's own checkout
carved back out as they already are. Until this lands a `standard` run can rewrite
`~/hydra/target/release/hydra`, the command `config/mcp.md` names, and `src/policy/gate.py`, the
template every gate is compiled from.

## Done when

`policy::denied_write_paths` includes `crate::paths::hydra_home()`, `denied_spellings` leaves it
to the token pass because own paths live under it on this machine, and the profile `sandbox.rs`
composes denies it before the own allows. `tests/gate.rs` asserts a `Write` to
`<hydra_home>/target/release/hydra` and to `<hydra_home>/src/policy/gate.py` is `protected-path`
under `permissive`, and that a `Write` inside the own worktree still allows; `tests/spawn.rs`
drives the real profile: a write under the checkout's `target/` is refused and the worktree write
still succeeds. One pong per provider completes under the new profile. Harness spec 15.2 gains
the row and 7.5's list names it; `SECURITY.md`'s "deny list" bullet is rewritten to say the
checkout is covered and what is still not.

## Outcome

`policy::denied_write_paths` now includes `crate::paths::hydra_home()` with a comment pointing at map
ticket 65. `denied_spellings` already carved out denied parents that hold an own path, so
`hydra_home()` stays off the substring list when the run's worktree lives under it and the token
pass handles writes there instead. `sandbox::compose` picks the deny up through `write_denies`
before the own allows, unchanged in order.

Added `tests/gate.rs::the_checkout_is_denied_and_the_own_worktree_still_writes`: a `Write` to
`<hydra_home>/target/release/hydra` and to `<hydra_home>/src/policy/gate.py` is `protected-path`
under `permissive`, and a write inside the fixture's own worktree still allows.
`tests/spawn.rs::the_seatbelt_refuses_a_checkout_write_and_allows_the_worktree` drives the real
profile: `echo x > <hydra_home>/target/probe` is refused and the worktree write succeeds on CI;
nested `sandbox-exec` from inside this run's own seatbelt fails with `sandbox_apply: Operation not
permitted` on the worktree half here, the same failure mode as
`the_seatbelt_refuses_a_corpus_write_and_allows_the_worktree`.

Harness spec 7.5 and 15.2 name the checkout; `SECURITY.md`'s deny-list bullet is rewritten.
`IMPLEMENTATION.md` records the build under the deny-list section and drops map 65 from the open
decisions list.

Measured: `cargo clippy --all-targets -- -D warnings` clean; `HYDRA_ROOT=$PWD/.scratch-root cargo
test --release --test gate the_checkout_is_denied_and_the_own_worktree_still_writes` 1 passed;
`HYDRA_ROOT=$PWD/.scratch-root cargo test --release --test spawn
the_seatbelt_refuses_a_checkout_write_and_allows_the_worktree` fails the worktree assertion inside
this seatbelt, checkout refusal passes. The ticket assumed the spawn test would run outside a parent
seatbelt; that assumption holds on CI and fails only in this population.

Session note, 2026-09-22: applied to master; `tests/spawn.rs` gained the checkout probe against the
gate template as well as `target/`; one pong per provider completed under the new profile,
01a0c960-a3a5, 01a0c960-c1f5, 01a0c960-e174, 01a0c961-009d.
