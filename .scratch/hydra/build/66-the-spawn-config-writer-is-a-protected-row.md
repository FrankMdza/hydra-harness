# 66 - The spawn-config writer is its own file and a protected row

Type: build
Status: resolved
Blocked by:

## What

Map ticket 60, resolved 2026-09-21. `write_spawn_config`, `SpawnFiles` and everything only they
use move from `src/run.rs` to `src/spawn_config.rs`, with `pub use` from `run.rs` so
`tests/spawn.rs`, `src/interactive.rs` and every call site keep their path. The new file joins
`PROTECTED_CODE_PATHS` in `src/rungs/evolve.rs` as harness spec 15.2's row 4, and the doc
comment there stops mapping `src/policy/` to both rows 4 and 9. `tests/evolve.rs` refuses a diff
to the new file at admission and keeps `src/run.rs` on the list that passes.

Do not edit `docs/`, `IMPLEMENTATION.md` or `.scratch/`. Launch no provider.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` and `cargo test --release --test evolve --test spawn`. Do not run the whole suite.

## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c5dd-bf97 under `standard`, 469k
tokens: `src/spawn_config.rs` holds `SpawnFiles` and `write_spawn_config`, `run.rs` re-exports both
so every call site keeps `hydra::run::write_spawn_config`, the file is in `PROTECTED_CODE_PATHS`
as row 4, the doc comment maps `src/policy/` to row 9 alone, and `tests/evolve.rs` refuses the new
path at admission with `src/run.rs` still on the passing list. Applied unchanged. `evolve` 16 and
`spawn` 19 in the worktree and on master. Nothing the ticket assumed turned out false.
