# 274 - A fixed goal set measures the orchestrator

Type: build
Status: resolved
Blocked by:

## What

Map 107, D5, C: a fixed set of five goals in three sizes, each run three times through Master of
Puppets, orchestrator cost against its agents per run, median and spread; real sessions reported
beside it as a check that the set is typical. Write the goal set as files under
`integrations/bench/goals/` against a scratch copy of a small repository, and a report command
that, given the orchestrator runs, finds each one's agents (started after it, in its repository)
and prints the ratio per run and the median. Running it is the session's, one goal a night.

## Outcome

Built 2026-10-04 by a Grok 4.6 run (`01a1073a-63cb-7e63`), reviewed and applied by the session.

`hydra cost <orch-id>... [--check <session-id>...]` in `src/cost.rs`: each orchestrator run's
`total_tokens` over the sum of the runs that started after it in the same repository, until the
next named orchestrator there; then n, median, min and max, and the same block for real sessions
under `--check`. It launches nothing. Six tests in `tests/cost.rs` on hand-built records, each
failing against a stub that returns nothing.

`integrations/bench/goals/` holds five goals (two small, two medium, one large) against a tiny
`tally` crate in `integrations/bench/repo/`, copied fresh for each repetition, and the README
says one goal a night, then `hydra cost`. The ticket named no goals; the run wrote them. Running
them is the session's and has not started.
