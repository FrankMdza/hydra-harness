# 109 - `sheet.py`: a spec sheet per agent, and two rankings from the corpus

Type: build
Status: resolved
Blocked by:

## What

The ranking half of Master of Puppets, map ticket 75 ("Rankings and the spec sheet" in
`.scratch/hydra/research/75-master-of-puppets-panel.md`, plus the judge's corrections). A Python
module, `integrations/puppets/sheet.py`, standard library only, that builds one JSON document per
agent (a provider and model pair) from the run records, merges hand-kept vendor facts, and ranks
agents per provider and per task class. Nothing under `src/` changes. `puppets.py` (ticket 108,
being written at the same time by another run) will import `rank()`; do not write `puppets.py`.

Read first: `docs/run-record-contract.md` sections 4 to 6 (line 0's envelope: `provider`,
`model`, `run_id`, `parent_run_id`, `ts`; `run_start`'s `policy`, `subscription`,
`provider_version`, `launch_mode`; `run_end`'s `ended_at` and `usage`), `src/outcome.rs` lines
539 to 615 (the usage projection: `raw_input`, `output_tokens`, `cache_read_tokens`,
`usage_source`), `hydra show --help` and one record's `hydra show` output shape as
`src/show.rs` prints it, and `src/rungs/review.rs` around `parse_judge_answer` for what a judge
run's text looks like.

**Sources, per record.** Provider, model and timestamps from line 0; policy, subscription and
provider version from `run_start`; duration from line 0's `ts` to `run_end.ended_at`; outcome
from `hydra show <id>` (never the exit code; cache the answers by run id in
`<home>/sheet-cache.json` so a rebuild costs one `hydra show` per new record); tokens from the
usage projection, with `usage_source` counted; denials as the count of `permission_decision`
events with `decision: deny` in `hydra show <id> --events`; `judge` runs as those with
`policy: judge`, and whether their text parsed. A record with a null model rolls up to the
provider. Harness runs (`command` other than `run`) are excluded. Resumed runs are not
independent successes: a record with `parent_run_id` set counts under its parent's task.

**The vendor file.** `integrations/puppets/vendors.json`, hand-kept, one entry per agent with
`as_of`, `source_url`, `context_tokens`, `thinking_levels`, `plan`, `windows` and
`window_budget_tokens`; every value may be null and a null stays null in the sheet. Ship it
with the four providers and the models named in `IMPLEMENTATION.md` and `README.md`, values
null except what those two files state.

**Cost is window share**, not dollars. For `claude`, the change in `seven_day.utilization`
between a run's first and last `rate_limit_event`, p50 across runs; for the other three, `raw_input
* cache_read_weight + output_tokens` over the vendor `window_budget_tokens`, null when the budget
is null. Report the sample size beside every statistic.

**Rankings.** `rank(sheet, task_class, grade)` filters agents by policy compatibility and
availability, then orders by expected cost divided by the lower bound of a beta posterior on
first-pass acceptance, `Beta(a + applied, b + fixed + rejected)`, with priors from
`integrations/puppets/priors.json`, which you seed from the memory notes quoted here: composer-2.5
is good at scripts and medium Rust and needs a mechanism review (a 4, b 2); haiku suits small
tools with a pasted check and not logic (a 3, b 3); `agy` gemini flash is for one-question probes
only (a 1, b 4, except `probe` a 4, b 1); sonnet writes good Rust at 7M to 55M tokens (a 5, b 1);
`codex` is the hardest reviewer and the cheapest judge (a 5, b 1 on `security_review`, a 3, b 2
elsewhere). A grade needs the lower bound to clear 0.5 for `low`, 0.6 for `medium`, 0.7 for
`high`; `agi` returns nobody. The `human_verdict` events of a goal log (ticket 108's format:
`{task, run_id, result}`) update the counts when `rank` is given a list of them.

**The CLI.** `python3 sheet.py build [--runs DIR] [--home DIR]` writes
`<home>/agents/<provider>--<model>.json`; `sheet.py rank --class C --grade G` prints the ordered
agents with their bound, cost and sample size; `sheet.py show <provider> [<model>]` prints one
sheet.

## Done when

`integrations/puppets/tests/test_sheet.py` runs green under `python3 -m unittest` against a
temporary runs directory of synthetic records you write (at least: two providers, one null
model, one `judge` run, one resumed run, one run with no `run_end`) and a stub `hydra` script
answering `show` from a table, and covers the exclusions, the roll-up, the cost null when the
budget is null, the prior-only ranking when no verdicts exist, and a ranking that changes after
two `applied` verdicts. You cannot read `~/hydra/runs/` from your worktree; the session runs
`build` against the real corpus after review. Do not run cargo. No em dashes in the README
section you add to `integrations/puppets/README.md` (create it if ticket 108's run has not).

## Outcome

Built 2026-09-24 by `codex` on run 01a0d36f-4447 (1.56M tokens): `sheet.py`, `vendors.json`,
`priors.json`, `tests/test_sheet.py`, seven tests green, all applied as written. Codex narrowed the
ticket's "exclude every command but `run`" to keep `resume` records under their root task, and
read `parse_judge_answer`'s serde structs to accept extra keys, both right. Built over the real
corpus by the session: 26 provider and model pairs from 380 records, `hydra show --events` once
per record, about four minutes cold. What the first ranking showed: with the ticket's priors,
composer-2.5 (a 4, b 2) does not clear the `low` bound of 0.5, so a prior-only ranking put sonnet
first for scripts, against the memory the priors were seeded from. The session did not retune
the priors; `puppets.py assign` uses the ranking only once a verdict backs it and the default
table until then, so the priors decide nothing on their own. Codex also appended a section to
`IMPLEMENTATION.md`; the session wrote its own instead.
