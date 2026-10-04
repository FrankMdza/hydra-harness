# 288 - The set of original diffs

Type: build
Status: open
Blocked by:

## What

Map 109, first step (`.scratch/hydra/research/109-a-system-one-of-our-own.md`): every human verdict in build 143's queue joined with the diff exactly
as the agent returned it, read from the run's branch `hydra/<run-id>`, written to a local jsonl.
It is the bottleneck of every System 1 that follows. A command (where it lives is part of the
work: `hydra-grader` is the likely home). Tests on synthetic repositories; a run cannot read
`~/hydra/runs`.

## Outcome
