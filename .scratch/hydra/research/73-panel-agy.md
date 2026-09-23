# Panel 73, agy

## 1. Recommendation
Option 1 (An allowlist), but modified to allow adding new tests.

## 2. Mechanism
In `src/rungs/evolve.rs:448-465`, replace `PROTECTED_CODE_PATHS` with `ALLOWED_CODE_PATHS` containing only the allowed paths (e.g., `src/provider/`, `src/normalize/`).
In `src/rungs/evolve.rs`, update `protected_path_hit` (lines 468-480) to return the path if it does not start with any prefix in `ALLOWED_CODE_PATHS`. Line count: ~20 lines changed.
The test `tests/evolve.rs::a_diff_outside_the_protected_rows_passes_admission` proves the current denylist. This test would fail without changes because the paths it expects to pass (like `src/run.rs`) would be blocked by the new allowlist.

## 3. What it breaks or costs
A legitimate rung-two proposer can no longer edit arbitrary mechanisms or tests outside the allowlist. Every widening requires a human edit. Past passes (like `rungs/01a0c4b4-81ec-7752-ab39-451ff5dfc144` which touched `tests/gate.rs`) would have been refused by the allowlist.

## 4. What a hostile run or proposer still gains
A hostile proposer can still subtly weaken or break the mechanisms inside the allowlist (like `src/normalize/` or `src/provider/`), potentially opening loopholes in how tool calls are parsed or how providers interact with the gate.

## 5. The strongest argument against your choice
An allowlist severely stifles the autonomous improvement loop. If the AI cannot touch `src/run.rs` or `src/sandbox.rs` to fix bugs or optimize execution, Rung Two degrades from a self-improving system to self-improving provider configs. The cost in human intervention to constantly update the allowlist negates the benefit of autonomous evolution.

## 6. The other options
Option 2: A wider denylist fails as soon as a fifth mechanism lands outside it. It is a reactive game of whack-a-mole.
Option 3: Refusing all of `tests/` means proposals arrive without tests, violating engineering practices and forcing humans to write them.

## 7. Something the ticket missed
Yes, there is a better option. An allowlist for `src/` combined with allowing the creation of *new* files in a dedicated `tests/proposals/` directory. This stops proposers from rewriting existing probes in `tests/gate.rs` or `tests/spawn.rs`, but still allows them to deliver tests for their own patches, solving the flaw in Option 3 without the extreme restriction of Option 1.
