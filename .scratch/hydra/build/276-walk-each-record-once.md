# 276 - Walk each record once

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 1 of the Rust review (finding 1): `hydra doctor --walk` normalizes every record nine times. Memoize `read` in `walk::walk_corpus` (`src/walk.rs:68-84`) with a `HashMap<PathBuf, Result<Run, String>>`, and run `check_normalizing_writes_nothing` first so its snapshot still brackets the first read. Behaviour must not change. Net: `tests/doctor.rs`, `tests/record_contract.rs`. Measure `hydra doctor --walk` wall time on the real corpus before and after is the session's (a run cannot read it).

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-03 by run `01a10517-84b9` (`cursor-agent` cursor-grok-4.6-high, `standard`,
0.97M tokens), applied without changes.

Built: `walk_paths` serves every check from a per-path cache of the first `read`, and runs the
write-nothing check first so its snapshot brackets that read. Eight normalizes per record become
one.

Measured by the session on the real corpus: `hydra doctor --walk` takes 46 s against 356 s with
master's binary, and prints the same findings, plus one from a record that did not exist during
the first walk. A first comparison with the binary built inside the run's worktree lost four
`GateLeak` findings: that binary sits under `worktrees/<id>/` and resolves a different root, so
the replay's protected path moved. Built in master's checkout, the two agree. A walk compares
binaries only when both are built where the human runs them.
