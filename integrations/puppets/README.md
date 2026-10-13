# Master of Puppets

The helper for the Master of Puppets mode: one append-only JSONL log per goal, state derived
by folding the log, and task launches through the Hydra binary. Since build ticket 196 the helper
is `hydra-puppets`, the Rust binary of `crates/puppets`, built by `cargo build --release
--workspace` beside `hydra`. This directory holds what it reads, `vendors.json`, `priors.json`,
`aliases.json` and `prompts/`. `hydra-grader` reads `aliases.json` from here too, by the same
rule for finding this directory (`HYDRA_PUPPETS_DATA` below). Approvals are lines
appended to the goal log that the CLI checks before acting; anything that can run the CLI can write one, and the
helper does not verify that a human typed it.

Build ticket 196 deleted `puppets.py`, the Python the helper was ported from, with the seven
test files that tested it. Before it went, `crates/puppets/tests/record.py` ran it on every
input of the differential tests and wrote its output under `crates/puppets/tests/fixtures/`,
3.3 MB, which `crates/puppets/tests/differential.rs` now compares the Rust with.

Build ticket 219 ports the sheet builder to `hydra-grader sheet build|budget|show`.
`rank` and `beta_lower` remain in `crates/puppets/src/sheet.rs`, where `assign` uses them.
Build ticket 220 moved `models.py` to `integrations/apprentice/`, beside `records.py`, the one
file that loaded it (build ticket 234 deleted both), and deleted `jev_probe.py` with the rest of the grader's Python. No Python
is left in this directory.

Nothing under `src/` imports this directory. Hydra works with it stopped.

## Environment

| Variable | Default | Purpose |
|---|---|---|
| `HYDRA_BIN` | `hydra` on `PATH` | Hydra binary, same as the Herdr scripts |
| `HYDRA_PUPPETS_HOME` | `~/hydra/puppets` | Goal logs and per-goal plan copies |
| `HYDRA_ROOT` | `~/hydra` | Run files polled by `wait` |
| `HYDRA_PUPPETS_DATA` | `integrations/puppets` under `HYDRA_HOME`, else under the first directory above the binary that holds `integrations/puppets/prompts` | Where `hydra-puppets` reads `vendors.json`, `priors.json`, `aliases.json` and `prompts/`, and `hydra-grader` reads `aliases.json` |
| `HYDRA_PUPPETS_VENDORS` | `vendors.json` in the data directory | The vendors file alone |

The data directory has to exist. A `hydra-puppets` copied out of its checkout with neither
`HYDRA_PUPPETS_DATA` nor `HYDRA_HOME` set has none, and every command that reads a data file
then prints `no data directory at <path>; set HYDRA_PUPPETS_DATA to the directory that holds
vendors.json, or HYDRA_HOME to the checkout` and exits 1 (build ticket 221). A `vendors.json`
missing from a data directory that exists still means every provider is up.

Nothing finds the helper on `PATH`. The cockpit runs the `hydra-puppets` beside the real `hydra`
binary, symlinks resolved. `hydra-board` and the grader take `HYDRA_PUPPETS_BIN`, which must be
an absolute path, else the one beside `HYDRA_BIN`. With no helper there the cockpit and the
board run nothing and say `no hydra-puppets beside <path>; run cargo build --release
--workspace`.

## Run

```
hydra-puppets new "goal text"
hydra-puppets show <goal-id>     # the state as JSON
hydra-puppets board <goal-id>    # the state at a glance, and what the human still owes
hydra-puppets plan <goal-id> --seat codex --seat cursor-agent:composer-2.5
hydra-puppets plans <goal-id>
hydra-puppets choose <goal-id> <provider|path>
hydra-puppets choose <goal-id> --judge
hydra-puppets tasks <goal-id>
hydra-puppets assign <goal-id>
hydra-puppets set <goal-id> <task> --provider P --model M --policy POL
hydra-puppets approve <goal-id> --table
hydra-puppets approve <goal-id> --table --expect-revision <n>
hydra-puppets approve <goal-id> --plan --expect-plan <path> --expect-revision <n>
hydra-puppets approve <goal-id> --diff <task> --expect-run <run-id> --expect-revision <n>
hydra-puppets approve <goal-id> --degraded codex=cursor-agent:composer-2.5
hydra-puppets approve <goal-id> --launch <task>
hydra-puppets pending <goal-id> --json
hydra-puppets launch <goal-id> <task>
hydra-puppets wait <goal-id>
hydra-puppets review <goal-id> <task>
hydra-puppets review <goal-id> <task> --no-open
hydra-puppets queue <goal-id>
hydra-puppets reconcile <goal-id> <task> <attempt> --run-id <id> | --abandon
hydra-puppets verdict <goal-id> <task> applied|fixed|rejected
```

## Builder report

`hydra-puppets report builders` reads every goal log directly under the puppets home.
It prints tab-separated rows per task class and provider/model, followed by totals per
provider/model. `--class docs` and `--model sonnet` select exact values before totals;
`--json` prints the same rows as one JSON object per line (`total: true` and `class: null`
mark totals). No harness records or agent sheets are needed.

`tasks` counts distinct goal/task pairs for each builder that launched them; assigned
but unlaunched tasks count too, with zero attempts. Reassignments attribute each launch,
its tokens and its verdict to the assignment in force at launch. A task tried by two
builders appears in both builders. `first_pass_accepted` uses the ranking rule: only
the first known verdict per task counts, and an applied resumed attempt counts as fixed.
An attempt numbered above one is a continuation for this purpose. `eventually_accepted`
counts tasks with any applied or fixed verdict. `rejections` counts rejected runs once.
Acceptance ratios use all counted tasks, including `no_verdict` tasks. Attempts count
unique launched runs; total tokens sum recorded run_end counts once per run, including
failed and rejected work. Missing token counts contribute zero. Tokens per accepted task
use eventual acceptances; an empty denominator prints `-` in text and `null` in JSON.

A torn tail is ignored after the last complete newline and its goal id is reported on
stderr. The log stays unchanged. A damaged complete line refuses the report until repaired.

`hydra-puppets report tickets --dir .scratch/hydra/build --runs <records directory>`
reads numbered `Type: build` tickets and the text under `## Outcome`. Run prefixes
of at least 13 characters resolve against record filenames; missing and ambiguous
prefixes remain visible. It prints TSV totals per provider/model, then tickets in
numeric order, with policy, normalized outcome, tokens, prose class and matched phrase.
`HYDRA_BIN` selects the harness reader (`hydra show --events`), as for `wait`.
A temporary layout in the current directory links the supplied records directory and
is removed after reading; source records are unchanged. Unfinished records are marked
`unfinished` without asking the harness to harvest them.

Tickets and prose counts are distinct per builder; runs and their token costs count
once per builder even if several tickets name the same run. Token median is per run;
missing token counts make totals and median unavailable. `rest` includes every outcome
except `completed` and `failed`. The fixed phrase list in `report/tickets.rs` is a
heuristic: rejection wins over manual completion, correction and acceptance; unmatched
prose is `unsaid`, counted plainly after the tables. Tickets tried by several builders
appear in each builder's ticket count. Unresolved prefixes do not enter builder totals.
Omit `--runs` for a tickets-only report with record columns marked `unavailable`.

## Log events

One JSON object per line: `{"seq": n, "ts": "<iso utc>", "event": "<name>", ...}`.

| Event | Fields |
|---|---|
| `goal` | `text`, `repo`, `base_sha` |
| `plan_proposed` | `provider`, `model`, `run_id` |
| `plan_ready` | `provider`, `run_id`, `path` |
| `panel_verdict` | `judge_run_ids`, `chosen`, `path`; optional `degraded`, `substitutions` |
| `human_plan` | `accepted`, `note` |
| `task` | `id`, `text`, `class`, `grade`, `deps`, `paths` |
| `assignment` | `task`, `provider`, `model`, `policy`, `source` |
| `approval` | `kind`, `ref`, `run_id` (diff only), `revision`, `yes`; degraded adds `missing`, `substitute`, `model` |
| `launch_intent` | `task`, `attempt`, `provider`, `model`, `policy`, `argv_sha256` |
| `run` | `task`, `attempt`, `run_id` |
| `run_end` | `task`, `run_id`, `outcome`, `total_tokens` |
| `review` | `task`, `judge_run_ids`, `verdict`; optional `degraded`, `substitutions` |
| `human_verdict` | `task`, `run_id`, `result`, `note` |
| `intent_abandoned` | `task`, `attempt` |

`class` is one of `batch_scripting`, `rust_off_launch_path`, `rust_launch_path`,
`security_review`, `research`, `docs`, `probe`. `grade` is `low`, `medium`, `high` or `agi`.

`hydra-puppets check <path>` runs the same import checks as `tasks` without a goal or home, prints offences to stderr and exits 1, or prints the task count and count with empty `done` and exits 0.

`tasks` validates the chosen plan as a dependency graph before appending any `task` event.
Repeated ids, dependencies naming no task in that plan, and dependency cycles refuse the
import with exit 1; stderr names every duplicate, unknown dependency and task in a cycle,
and the goal log stays unchanged. Ids use the fold's key equality (so `1` and `1.0` repeat).
`paths` are not checked against disk, since tasks may create the files they name.

Approvals bind to a plan revision (the count of `task` and `assignment` events). The CLI checks
for an unconsumed `yes` line at the current revision before each gated step:

| Kind | Checked before | Binds to |
|---|---|---|
| `plan` | `tasks` | current revision |
| `table` | `launch` | current revision; one covers every launch at that revision (map ticket 76, decision C) |
| `launch` | `launch` for `codex`, or `high`/`agi` on `claude` | task id and revision; spent by the launch |
| `diff` | `verdict applied` or `verdict fixed` | task id, the task's current `run_id`, and revision; spent by the verdict |

`rejected` needs no diff approval. A later `assignment` voids approvals at an older revision.

`--expect-revision N` on `approve` refuses when the goal is no longer at revision `N`. It prints
one line to stderr naming both revisions, writes nothing, and exits 2. Without the flag, `approve`
records the current revision as before. `--expect-run <run-id>` with `--diff` refuses the same
way, exit 2 and nothing written, when that task's current run is different. `--expect-plan <path>`
with `--plan` refuses the same way when the chosen plan's path is different. Each `--expect-*`
check runs again while the goal log's lock is held, on a fold of the lines read under that lock,
and a mismatch still writes nothing. When the log's last line has no newline, both readers ignore
that tail, and every writer under the lock leaves the file unchanged and exits 1. A
newline-terminated line that is blank or is not JSON makes the log damaged: every reader and
writer refuses with one line on stderr and exit 1, and nothing is written. Only `\n` ends a
line. A lone `\r` inside the file is part of the line it sits in, so `BAD\r` followed by good
lines is one bad line that runs to the next newline. A CRLF line still parses. An integer of
more than 4,300 digits is not JSON here, because Python refuses to read one.

`repair <goal-id>` lists each bad newline-terminated line, then any unterminated tail, cut to
300 characters, and the move `--yes` would make, and writes nothing. `repair <goal-id> --yes`
holds the log's lock and writes the bad lines and any unterminated tail to
`<goal-id>.torn-<UTC timestamp>.txt` beside the log. It then builds the new log, the good lines
and, when it removed one or more bad lines, a `repair` event, writes that to
`<goal-id>.jsonl.repair-tmp`, created exclusively without following links, fsyncs it and
renames it over the log. An existing regular temporary file is removed and creation is
retried once. A symlink or any other file type is refused with `UnsafeRepairTemporary`,
exit 1, before writing anything. A repair killed before the
rename leaves the old log and a torn file that copies part of it. One killed after leaves the
new log. There is no state in between. The `repair` event bumps the revision by the number of
lines removed plus one, so earlier table, launch, diff and degraded approvals void. A `removed`
that is not an int from 1 to 1,000,000,000 counts as zero. Because the rename puts a new file at
the path, every command that locks the log checks, once it holds the lock, that the path still
names the file it opened, and opens the log again when it does not. A log with neither damage
nor a torn tail says so and exits 0 either way.

A second approval of the same kind, ref and run already in the log at the current revision prints
`already approved <kind> at revision <n>`, writes nothing, and exits 3. For a degraded approval
the ref is the missing provider. A diff's run is the run id on the approval; other kinds have
no run.

`pending <goal-id> --json` prints a JSON array of the approvals the helper is waiting on. It only
reads the log. Each object has `kind` (`plan`, `table`, `launch`, `diff`, `degraded`), `ref`,
`revision`, and `argv`. `ref` is the task id for `launch` and `diff`, the missing provider for
`degraded`, and null for `plan` and `table`. `argv` is the exact command that records that
approval. It ends with `--expect-revision` set to that revision. A `diff` argv also carries
`--expect-run` set to the task's current run, and a `plan` argv carries `--expect-plan` set to
the chosen plan's path. The list is taken from the same
fold the refusals read: `plan` when `tasks` would refuse for a missing plan approval (the chosen
plan has a path), `table` when `launch` would refuse for the table, `launch` when `launch` would
refuse for that task and the task is still `assigned`, `diff` when the task has a run, no human
verdict yet, and `verdict applied` would refuse, and `degraded` when `resolve_panel_seats` asks
for one and a panel command would reach it (two finished proposals, or any task that already has
a run). `review` can still refuse first when the run record has no branch; pending cannot see
that from the fold. A launch approval a run has already spent is not listed again, and neither
is a diff after a human verdict. The `--degraded` value is
`provider=<substitute-provider[:model]>`. Running that argv unchanged writes nothing; replace the
placeholder with a provider before it will record. A later seat is not listed until the helper
asks for it.

A `launch_intent` with no following `run` event is ambiguous after a crash. `launch` refuses until
the human settles it with `reconcile`, either naming the run id `hydra status` shows for that
attempt or abandoning it. Nothing is relaunched by the helper.

`plans` takes a proposer's plan from its run's `result_text` (the brief asks for the plan as the
answer), or copies one regular file from the run's worktree `docs/proposals/` when one exists.
The copy does not follow links, refuses non-regular files, and is size-capped. `assign` asks
the sheet ranking in `hydra-puppets` when sheets have been built under the puppets home, and falls back to a
default table (recorded as `source: default`) when none exist or nobody qualifies.

`choose --judge` launches the review panel (`claude` opus and `cursor-agent` composer-2.5 under
`judge`, `codex` under `standard` until map ticket 63) over the finished proposals. Each seat
has a distinct role and evidence slice (reviewer with task context, adversary with proposals
alone, security with test coverage). When a seat's provider is marked unavailable in
`vendors.json`, the panel refuses until the human records
`approve <goal-id> --degraded missing=substitute[:model]`. A degraded run fills the missing
seat's role with the substitute; the log carries `degraded: true` and the substitution, and
`board` labels the verdict as a degraded panel rather than a full one. Rung two's panel in
`src/rungs/review.rs` never substitutes. Records a `panel_verdict` with the judge run ids, and
writes a `composed` plan when the panel picks one.

`review` launches the same panel over the task diff with the same role and slice rules and the
same degraded path. It records a `review` event, pushes judge findings into the live Hunk session
as comments when one exists, then opens the diff in Hunk with
`hunk show <branch>` from the goal's repository. Git diff for the panel uses the same branch from
the run record, never a run-writable checkout. If the run record has no branch, review refuses:
an unreaped worktree is the only place an uncommitted diff might live, and the helper will not
run git or Hunk there. Inside Herdr it opens in a pane to the right. With `--no-open` the panel,
the verdict and the findings are unchanged, including the push of findings into a live Hunk
session, and the helper then prints the `hunk show <branch>` command and exits 0. It does not
exec Hunk and it does not open a pane. Without the flag it still opens Hunk. The mode has no
diff viewer of its own.

`queue <goal-id>` lists tasks whose current run has ended and that still have no human verdict
for that run. They are ordered by the `verdict` model's `p` in `$HYDRA_GRADER_HOME/scores.jsonl`,
highest first. The top line names the newest verdict `model_version` in the file (latest
`scored_at`, then the later line) and how many of these tasks have no score from it. A run scored
only by an older version, or not scored at all, sorts last and is marked `no score`. `board`
prints that same `p` in a short column beside a task that is waiting for a review, so the cockpit
and `hydra-board` show it without another command. The order is a suggestion and the human still
reviews every task. Nothing here writes `scores.jsonl`, and the sheet ranking does not read it.

Inside Herdr, `launch` runs the binary directly and then opens a pane below the current one
tailing the run with the bridge's tokens on it. The launch never goes through `hydra-in-pane`,
which would not return until the run ended.

## Test

```
cargo test -p hydra-puppets
cargo test -p hydra-grader
```

The first runs the Rust's unit tests and the differential tests. Those run `hydra-puppets` on
each recorded input, with stubs for `hydra`, `hunk` and `herdr`, and compare its output, its
exit code and the files it leaves with what the Python gave. No test starts `python3`. The
second checks the grader port, including recorded sheet commands with a shell `HYDRA_BIN` stub.
`sheet_build.rs` also checks that assignment uses freshly built Rust sheets and matches the
recorded Python ranking.

The fixtures cannot be recorded again from this checkout, because the Python they record is
gone. `record.py`'s first lines say how it was run and how to run it against a checkout from
before build ticket 196. A case added to `differential.rs` after that has no recording and
fails until one exists, so new behaviour of the helper is tested in the crate's unit tests.

## The spec sheet and the rankings

`hydra-grader sheet build` builds corpus measurements and merges `vendors.json`. It calls
`hydra show <id> --events` once per new or changed record,
with the supplied corpus as Hydra's root. It never infers success from exit status.
Unchanged records reuse `sheet-cache.json`. Partial final lines wait for the next build.

```sh
hydra-grader sheet build --runs /path/to/runs --home /path/to/puppets
hydra-grader sheet budget <run-id> --runs /path/to/runs
hydra-grader sheet show claude sonnet --home /path/to/puppets
```

Defaults use `HYDRA_ROOT` (otherwise `~/hydra`) for the corpus and its `puppets`
subdirectory for output. `HYDRA_PUPPETS_HOME` overrides output; `HYDRA_BIN` selects
the executable. `agents/<provider>--<model>.json` contains each sheet. Unknown models
use `~null` in filenames and JSON null in the sheet, forming a provider-only bucket.
Recorded model spellings are retained in `recorded_models`; aliases share a canonical sheet.
Filename parts are URL encoded. The vendor inventory includes historical names, not a claim that
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
When a run ends `rate_limited`, the session calls `hydra-grader sheet budget <run-id>` to set
`window_budget_tokens` in `vendors.json` to the summed token spend of every Hydra run
for that provider in the window that just closed. The window's length is the row's
`windows` (`"5h"`, `"7d"`), or `--since <ISO time>` for the start; with neither the command
refuses rather than guess. Claude is refused, because its cost comes from utilization.
Each write appends a `budget_history` entry with the previous value, the new one, the
date, the run ids summed, and `outside_spend_possible` (true when spending outside Hydra
could have shared the window). The command prints one line for the human; relay it.
To undo one revision, remove the last `budget_history` entry on that vendor row and
set `window_budget_tokens` back to that entry's `previous` value.

`hydra-puppets` uses its Rust `sheet::rank` function when assigning tasks. It reads
these sheets and the goal log's `event: task` definitions and `event: human_verdict`
events. There is no separate ranking CLI. Outcome completion never supplies acceptance.
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
