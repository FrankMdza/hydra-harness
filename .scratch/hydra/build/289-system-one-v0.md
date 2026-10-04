# 289 - System 1 v0

Type: build
Status: open
Blocked by: 288

## What

Map 109, second step (`.scratch/hydra/research/109-a-system-one-of-our-own.md`): embeddings of the diff and the task from the Ollama the apprentice
already uses, a logistic head with a temperature over `crates/grader/src/baseline.rs`, measured
with Brier and AUC against the base rate per provider and against Jev's 25 answers of build 134.
Answers task class, grade, outcome and, once labelled, `needs_correction`. Nothing leaves the
machine.


**From 288 (2026-10-04):** `hydra-grader originals` finds only 7 of 70 verdicts with their
original diff; the other branches were deleted. A head trained on diffs has 7 examples until
more runs are reaped with their branches kept, so v0 should start from the task text and the
record's features, which all 70 have, and add the diff when there are enough.

## Outcome
