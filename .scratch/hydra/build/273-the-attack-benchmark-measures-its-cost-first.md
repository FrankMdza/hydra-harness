# 273 - The attack benchmark measures its cost first

Type: build
Status: resolved
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

Built 2026-10-04 by a Grok 4.6 run (`01a106e2-8075`), reviewed and amended by the session.

`hydra bench` in `src/bench.rs`: parses the corpus table, expands cells by attacks by
repetitions, drops `not applicable` rows, lists `codex:read-only` as unsupported rather than a
zero, keeps `bench/attack-state.json` for resume, prints Wilson 95% intervals, and `--dry-run`
prints the plan and a token estimate. 11 tests in `tests/bench.rs` and four unit tests; none
launches a provider.

**The dry run on the real corpus**, by the session: 741 launches (13 classes, 19 supported cells,
3 repetitions). Estimated tokens: `claude` 310 M, `cursor-agent` 225 M, `agy` 85 M, `codex` 60 M.
These take the median `total_tokens` of every run of the provider, build runs and cache reads
included, so they are an upper bound and not the number map 106 needs.

**What the session did not accept: the scoring.** A run that ends `completed` counts as an attack
that got through, and each run gets a one-line task. An agent that declines ends `completed`, so
the benchmark would report breaches that never happened. The corpus has no per-class success
test and no attack prompts. That is a design question, not an implementation, so it went to the
map as ticket 114, and `main.rs` now refuses `hydra bench` without `--dry-run`, naming map 114.
A test drives the binary and checks the refusal and that no state is written; without the guard
the same test tried to launch and was killed by its timeout, and nothing was left running.
