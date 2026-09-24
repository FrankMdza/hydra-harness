# Master of Puppets

A Python helper for the Master of Puppets mode: one append-only JSONL log per goal, state derived
by folding the log, and task launches through the Hydra binary with human approvals enforced.

Nothing under `src/` imports this directory. Hydra works with it stopped.

## Environment

| Variable | Default | Purpose |
|---|---|---|
| `HYDRA_BIN` | `hydra` on `PATH` | Hydra binary, same as the Herdr scripts |
| `HYDRA_PUPPETS_HOME` | `~/hydra/puppets` | Goal logs and per-goal plan copies |
| `HYDRA_ROOT` | `~/hydra` | Run files polled by `wait` |

## Run

```
python3 integrations/puppets/puppets.py new "goal text"
python3 integrations/puppets/puppets.py show <goal-id>
python3 integrations/puppets/puppets.py plan <goal-id> --seat codex --seat cursor-agent:composer-2.5
python3 integrations/puppets/puppets.py plans <goal-id>
python3 integrations/puppets/puppets.py choose <goal-id> <provider|path>
python3 integrations/puppets/puppets.py choose <goal-id> --judge
python3 integrations/puppets/puppets.py tasks <goal-id>
python3 integrations/puppets/puppets.py assign <goal-id>
python3 integrations/puppets/puppets.py set <goal-id> <task> --provider P --model M --policy POL
python3 integrations/puppets/puppets.py approve <goal-id> --table
python3 integrations/puppets/puppets.py approve <goal-id> --launch <task>
python3 integrations/puppets/puppets.py launch <goal-id> <task>
python3 integrations/puppets/puppets.py wait <goal-id>
python3 integrations/puppets/puppets.py review <goal-id> <task>
python3 integrations/puppets/puppets.py reconcile <goal-id> <task> <attempt> --run-id <id> | --abandon
python3 integrations/puppets/puppets.py verdict <goal-id> <task> applied|fixed|rejected
```

## Log events

One JSON object per line: `{"seq": n, "ts": "<iso utc>", "event": "<name>", ...}`.

| Event | Fields |
|---|---|
| `goal` | `text`, `repo`, `base_sha` |
| `plan_proposed` | `provider`, `model`, `run_id` |
| `plan_ready` | `provider`, `run_id`, `path` |
| `panel_verdict` | `judge_run_ids`, `chosen`, `path` |
| `human_plan` | `accepted`, `note` |
| `task` | `id`, `text`, `class`, `grade`, `deps`, `paths` |
| `assignment` | `task`, `provider`, `model`, `policy`, `source` |
| `approval` | `kind`, `ref`, `revision`, `yes` |
| `launch_intent` | `task`, `attempt`, `provider`, `model`, `policy`, `argv_sha256` |
| `run` | `task`, `attempt`, `run_id` |
| `run_end` | `task`, `run_id`, `outcome`, `total_tokens` |
| `review` | `task`, `judge_run_ids`, `verdict` |
| `human_verdict` | `task`, `run_id`, `result`, `note` |
| `intent_abandoned` | `task`, `attempt` |

`class` is one of `batch_scripting`, `rust_off_launch_path`, `rust_launch_path`,
`security_review`, `research`, `docs`, `probe`. `grade` is `low`, `medium`, `high` or `agi`.

Approvals bind to a plan revision (the count of `task` and `assignment` events). A launch needs
a table approval at the current revision, and one table approval covers every launch at that
revision (map ticket 76, decision C: the table once per batch). Tasks on `codex`, or `high`/`agi`
on `claude`, also need a per-task launch approval because those gates can fail open; that one is
spent by the launch. A verdict of `applied` or `fixed` needs a diff approval for the task at the
current revision; `rejected` needs none.

A `launch_intent` with no following `run` event is ambiguous after a crash. `launch` refuses until
the human settles it with `reconcile`, either naming the run id `hydra status` shows for that
attempt or abandoning it. Nothing is relaunched by the helper.

`plans` takes a proposer's plan from its run's `result_text` (the brief asks for the plan as the
answer), or from a file under the run's worktree `docs/proposals/` when one exists. `assign` asks
`sheet.py`'s ranking when sheets have been built under the puppets home, and falls back to a
default table (recorded as `source: default`) when none exist or nobody qualifies.

`choose --judge` launches the review panel (`claude` opus and `cursor-agent` composer-2.5 under
`judge`, `codex` under `standard` until map ticket 63) over the finished proposals, records a
`panel_verdict` with the judge run ids, and writes a `composed` plan when the panel picks one.

`review` launches the same panel over the task diff, records a `review` event, pushes judge
findings into the live Hunk session as comments when one exists, then opens the diff in Hunk:
`hunk diff` inside the run's worktree while it exists, `hunk show hydra/<run-id>` from the repo
once it is reaped. Inside Herdr it opens in a pane to the right. The mode has no diff viewer of
its own.

Inside Herdr, `launch` runs the binary directly and then opens a pane below the current one
tailing the run with the bridge's tokens on it. The launch never goes through `hydra-in-pane`,
which would not return until the run ended.

## Test

```
python3 -m unittest discover -s integrations/puppets/tests
```

The unit tests use a stub `HYDRA_BIN` script. The live test at the end runs `new` and `show`
against the real binary without launching a provider.

## The spec sheet and the rankings, `sheet.py`

`sheet.py` builds corpus measurements and merges `vendors.json`, using only Python's
standard library. It calls `hydra show <id> --events` once per new or changed record,
with the supplied corpus as Hydra's root. It never infers success from exit status.
Unchanged records reuse `sheet-cache.json`. Partial final lines wait for the next build.

```sh
python3 integrations/puppets/sheet.py build --runs /path/to/runs --home /path/to/puppets
python3 integrations/puppets/sheet.py show claude sonnet --home /path/to/puppets
python3 integrations/puppets/sheet.py rank --class rust_launch_path --grade medium --home /path/to/puppets
python3 integrations/puppets/sheet.py rank --class security_review --grade low --provider codex --home /path/to/puppets
python3 -m unittest discover -s integrations/puppets/tests -v
```

Defaults use `HYDRA_ROOT` (otherwise `~/hydra`) for the corpus and its `puppets`
subdirectory for output. `HYDRA_PUPPETS_HOME` overrides output; `HYDRA_BIN` selects
the executable. `agents/<provider>--<model>.json` contains each sheet. Unknown models
use `~null` in filenames and JSON null in the sheet, forming a provider-only bucket.
Model names are preserved as recorded; aliases have separate sheets. Filename parts
are URL encoded. The vendor inventory includes historical names, not a claim that
those models remain offered. Undocumented vendor facts remain null.

Each sheet contains individual runs, task lineage, policy, subscription, version,
launch mode, timestamps, outcomes, token and usage-source counts, duration, denials,
and judge parse counts. Every median carries `n`; count distributions sum to the
sheet's `sample_size`. Runs with no ending retain unknown outcomes and durations.
`resume` records are included under the original task; other commands besides `run`
are excluded. Judge parsing follows `src/rungs/review.rs`, including its acceptance
of extra JSON fields and its fixed checklist vocabulary.

Cost is the median window share with its measured sample count. Claude uses the first
to last seven-day utilization change, including the nested `rate_limit_info` shape.
A missing pair or a decreasing counter produces null. Other providers use
`(raw_input * cache_read_weight + output_tokens) / window_budget_tokens`.
The weight defaults to 1 because the ticket supplies no measured discount; set
`build --cache-read-weight` to change it. Missing usage or budget produces null.
These are per-run measurements, not isolated causal estimates of subscription spend.

Import `rank(sheets, task_class, grade, verdicts=None, policy=None)` to obtain ordered
rows with provider, model, bound, cost, score, verdict sample size and cost sample size.
A single sheet also works. Pass all sheets for a task-class ranking or filter by provider
for a provider ranking, as the CLI's `--provider` does. `--verdicts goal.jsonl` accepts
the goal log's `event: task` definitions and `event: human_verdict` events. A bare list
of `{task, run_id, result}` must already be scoped to the requested class; optional
`task_class` fields can supply that scope. Outcome completion never supplies acceptance.
The first verdict per task is counted once, and an applied resumed run counts as fixed.

Priors live in `priors.json`. The bound is the beta posterior's 10th percentile, a
one-sided 90% bound, computed without third-party libraries. Low, medium and high
require bounds of 0.5, 0.6 and 0.7; agi returns no candidates. Known costs sort by
cost divided by bound. Unknown costs sort last, then by descending bound, so missing
budgets still permit prior-only rankings without pretending their cost is zero.

Availability is an explicit sheet/vendor override (`available`, default true), not a
live authentication or quota probe. Callers should set it before ranking when a seat
is unavailable. `policies` can also narrow compatibility. Defaults reflect routed
policies, excluding Codex read-only. Rankings use judge for probes and security review,
standard otherwise; `--policy` overrides that choice. A judge brief must contain all
evidence, since judge grants no tools. Neither ranking launches nor assigns an agent.
