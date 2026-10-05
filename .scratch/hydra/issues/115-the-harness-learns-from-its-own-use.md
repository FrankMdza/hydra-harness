# 115 - The harness learns from its own use

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-04, by the session, at the human's request: Hydra as the daily driver over every
subscription, improving itself through use the way Hermes Agent does, learning to drive the CLIs
better and repairing itself, an evolutionary loop.

## Question

Two rungs exist and neither runs on its own. `hydra curate` (rung one) reads the corpus and
proposes skill edits; `hydra evolve` (rung two) runs a proposer, an admission gate and a panel of
four on a patch to Hydra itself. The apprentice loop trains a small model on the same records.
What is missing is the loop being closed by use rather than by a human typing a command.

1. **What triggers a pass.** (A) a schedule, like the grader's launchd job; (B) an event in the
   records: a provider's CLI changed version (`hydra doctor` sees it), a tool name the capability
   table does not know appeared, a class of run started failing; (C) both.
2. **What a pass may change without the human.** Skills only (rung one's reach today); provider
   facts such as argv, tool names and model lists (the self-repair Frank named); or Hydra's own
   source behind the panel (rung two). Each wider reach needs its own stop.
3. **What stops a bad change.** The panel of four and the admission gate exist for rung two.
   Self-repair of a provider fact needs a check that is cheaper: the drift probes of
   `tests/drift.rs` rerun against the changed fact, and a revert when they fail.
4. **What "better" is measured by**, so the loop selects and does not drift: the grader's
   outcome rate per provider and task class, the fixed goal set of build 274, the attack
   benchmark of map 114 once it runs.
5. **Where the small model fits**: as the proposer, as the judge of last resort, or only as the
   apprentice it is today.

## Answer
