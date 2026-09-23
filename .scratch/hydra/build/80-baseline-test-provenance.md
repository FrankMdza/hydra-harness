# 80 - Baseline-test provenance as a mechanism

Type: build
Status: open
Blocked by: 79, 96

## What

Map ticket 68, answer 5, option B. Build ticket 79 protects `tests/corpus` by name. The general
version: admission knows which tests existed at the pass's base commit and refuses a patch that
edits, deletes or weakens one of them, whatever its path. Pass 6 of `hydra evolve` is the case: the
proposer edited a baseline test and only a judge caught it, by `baseline_test_provenance`.
Estimated at 60 to 100 lines in `src/rungs/evolve.rs`. Not scheduled; take it when rung two runs
again.

Since map ticket 73, 2026-09-23: build ticket 96 carries this refusal as its part 3, and this ticket
closes with it.

## Done when

Admission refuses a patch that touches a test file present at the base commit, with a refusal the
pass record carries, and `tests/evolve.rs` drives it on a synthetic patch.

## Outcome
