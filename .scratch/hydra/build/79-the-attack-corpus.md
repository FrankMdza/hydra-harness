# 79 - The attack corpus lives in `tests/corpus`, where rung two cannot edit it

Type: build
Status: resolved
Blocked by: 77

## What

Map ticket 68, answers 5 and 6, resolved 2026-09-22, and the corpus shape proposed in
`.scratch/hydra/research/68-attack-corpus-opus.md`, section "The corpus as a mechanism".
`tests/gate.rs` and `tests/spawn.rs` each carry their own copy of the helpers that compile a gate
into a scratch layout, feed it a payload, read the answer back and run a command under the real
profile (`Gate`, `raw_answer`, `under_seatbelt`, `refused` and whatever they depend on).

1. Move those helpers into `tests/corpus/mod.rs` and have both files use it. No behaviour change;
   every existing test keeps its name.
2. `tests/corpus/README.md`: one table, one row per attack class the research document lists,
   with the mechanism that answers it, the test that probes it, or the reason it cannot apply to
   Hydra. A class with no probe and no reason is not a row. No empty tests.
3. `tests/corpus` joins `PROTECTED_CODE_PATHS` in `src/rungs/evolve.rs:447`, so rung two cannot edit
   the corpus its patches are judged against. Extend the test that covers that list.

## Done when

The three items above. `cargo test --release --test gate --test spawn --test evolve` green; the
full count of tests in `gate` and `spawn` unchanged. Clippy and fmt clean. Edits `tests/` and
`src/rungs/evolve.rs` only.

## Outcome

`tests/corpus/mod.rs` holds `Gate`, `Decision`, `raw_answer`, `under_seatbelt`, `refused` and the
constants both files used; `tests/gate.rs` and `tests/spawn.rs` import it and keep every test name
and count (gate 90, spawn 30). `tests/corpus/README.md` has fourteen rows. `tests/corpus` is on
`PROTECTED_CODE_PATHS`, and `tests/evolve.rs`'s protected-row test now walks every row of that list.
The run marked A3 and A5 `not applicable`; they are classes Hydra does not answer, not classes that
cannot reach it, so the session changed them to `Open`, A1's probe to the policy matrix, and added
both to `SECURITY.md`. Measured on master with the full suite and an empty `HYDRA_ROOT`: every
target green, clippy and fmt clean. Built by `cursor-agent` on Composer 2.5 under `standard`, run
01a0cc3f-1e0f, 2.0M tokens.
