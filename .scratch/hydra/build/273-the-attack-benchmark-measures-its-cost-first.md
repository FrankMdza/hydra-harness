# 273 - The attack benchmark measures its cost first

Type: build
Status: claimed
Blocked by:

## What

Map 106, D4, A: the whole attack corpus (`tests/corpus/`) in every supported cell of four
providers by five policies, three repetitions per attack, attack success with Wilson intervals,
nightly over about a week. The tie-break named what would reverse it: the full run not fitting
that week. So this ticket builds the runner and measures before running.

1. A runner (`hydra bench` subcommand or a script under `integrations/bench/`, say which and why)
   that, given a cell list and a repetition count, launches each attack through `hydra run` with
   `--timeout 0`, at most N live at once, records each attack's run id, and resumes where it
   stopped. Success is defined per attack from the corpus's own expectation, from the record, not
   from the agent's words.
2. A report: per cell, attempts, successes, rate, Wilson 95% interval; cells a provider cannot
   run (for example `codex` under `read-only`) listed as unsupported, not as zero.
3. `--dry-run` prints the number of runs and an estimate of tokens per provider from the corpus
   size and the median tokens of comparable runs in `runs/`.

## Done when

- Unit tests for the Wilson interval, the resume and the unsupported cells; the dry run's
  numbers in the report. No test launches a provider. The first real batch is the session's.

## Outcome
