# 108 - `puppets.py`: the goal log, its fold, and the launcher

Type: build
Status: resolved
Blocked by:

## What

Master of Puppets mode, first version, from map tickets 75 and 76 (decisions A, B, C, I and J).
A Python program, `integrations/puppets/puppets.py`, standard library only, that keeps one
append-only JSONL log per goal under `~/hydra/puppets/<goal-id>.jsonl`, derives the goal's state
by folding the log (never stored, like a run's outcome), launches proposers and task runs through
the Hydra binary, and refuses to launch anything the human has not approved at the current plan
revision. Nothing under `src/` changes. `HYDRA_BIN` names the binary, like the Herdr scripts;
`HYDRA_PUPPETS_HOME` overrides the log directory for tests.

Read first: `.scratch/hydra/research/75-master-of-puppets-panel.md` (the shape, "The mode" and
"The judge"), `.scratch/hydra/issues/76-*.md` (the answers), `integrations/herdr/bin/hydra-in-pane`
(how a run is launched in a pane and its id read from stderr), `docs/harness-spec.md` section 17
(what `hydra run` prints and returns) and the `hydra run --help` text in `src/cli.rs`.

**The log.** One JSON object per line: `{"seq": n, "ts": "<iso utc>", "event": "<name>", ...}`.
Events, all of them, and no others:

- `goal {text, repo, base_sha}`
- `plan_proposed {provider, model, run_id}`
- `plan_ready {provider, run_id, path}` when the proposal file was copied out of the run's worktree
- `panel_verdict {judge_run_ids, chosen, path}` where `chosen` is a provider or `composed`
- `human_plan {accepted, note}`
- `task {id, text, class, grade, deps, paths}`; `class` is one of `batch_scripting`,
  `rust_off_launch_path`, `rust_launch_path`, `security_review`, `research`, `docs`, `probe`;
  `grade` is `low`, `medium`, `high` or `agi`
- `assignment {task, provider, model, policy, source}` with `source` `ranking` or `human`
- `approval {kind, ref, revision, yes}` with `kind` one of `plan`, `table`, `launch`, `diff`
- `launch_intent {task, attempt, provider, model, policy, argv_sha256}`, written before the spawn
- `run {task, attempt, run_id}`, written the moment the id is known
- `run_end {task, run_id, outcome, total_tokens}`
- `review {task, judge_run_ids, verdict}`
- `human_verdict {task, run_id, result, note}` with `result` `applied`, `fixed` or `rejected`

**The fold.** `state(goal)` walks the log once and returns the goal, the plans, the chosen plan,
the tasks with their latest assignment and status, the approvals, and `revision`: the count of
`task` and `assignment` events so far. An approval carries the revision it was given at, and it is
spent once by the launch or verdict that consumes it; a later revision voids it. A launch on a
task with no table approval at the current revision is refused with a message naming the missing
approval. A task whose provider is `codex`, or whose grade is `high` or `agi` on `claude`, needs
its own `launch` approval as well, because those gates can fail open; say so in the refusal.

**The launcher.** `launch` writes `launch_intent` with a fresh attempt id, then spawns
`hydra run --provider P [--model M] --policy POL --timeout 0 --task "<text>"`, reads
`hydra: run <id>` from stderr and writes `run`. Inside Herdr (`HERDR_ENV` set) it launches through
`integrations/herdr/bin/hydra-in-pane` in a new pane instead, so the bridge labels it. On restart,
a `launch_intent` with no `run` after it is reported as ambiguous and never relaunched: the
command prints the attempt id and asks the human to reconcile with `hydra status`. `wait` polls
each pending run's file under `HYDRA_ROOT`-aware `~/hydra/runs/<id>.jsonl` for a `run_end` line
and appends `run_end`, reading `outcome` from `hydra show <id>`.

**The CLI**, argparse, subcommands: `new "<goal text>"` (prints the id), `show <goal>`,
`plan <goal> --seat codex --seat cursor-agent:composer-2.5 --seat agy:gemini-3.1-pro-high`
(launches one proposer per seat with the brief in `integrations/puppets/prompts/plan.md`, which
you write: it asks for a plan ending in one fenced JSON block of tasks in the `task` shape),
`plans <goal>` (copies finished proposals out of `~/hydra/worktrees/<run-id>/docs/proposals/` into
`~/hydra/puppets/<goal-id>/plans/`), `choose <goal> <provider|path>` (records `panel_verdict`
with an empty judge list for now; the judged version is ticket 111), `tasks <goal>` (imports the
JSON block of the chosen plan as `task` events), `assign <goal>` (one `assignment` per task from
`integrations/puppets/sheet.py` if it exists, else from a default table in the code: composer-2.5
for `batch_scripting`, `docs` and `rust_off_launch_path`; `claude` sonnet for `rust_launch_path`;
`codex` for `security_review`; `agy` gemini-3.1-pro-high for `probe`; nobody for `agi`, which
asks the human), `set <goal> <task> --provider --model --policy` (a human assignment),
`approve <goal> --plan | --table | --launch <task> | --diff <task>`, `launch <goal> <task>`,
`wait <goal>`, `verdict <goal> <task> applied|fixed|rejected [--note]`.

## Done when

`integrations/puppets/tests/test_puppets.py` runs green under `python3 -m unittest` with a stub
`hydra` (a shell script the test writes: `run` prints `hydra: run <uuid>` on stderr and writes a
two-line run file with `run_start` and `run_end`; `show` prints `{"outcome": "completed"}`) and a
temporary `HYDRA_PUPPETS_HOME`, and covers: the fold's revision and status; a launch refused
without a table approval; an approval voided by a later assignment; a `codex` task refused without
its own launch approval; `launch_intent` present before `run`; a restart with a dangling intent
reported and not relaunched; `plans` copying a file out of a worktree path; `tasks` importing the
JSON block. Then `puppets.py new` through `show` work against the real binary on this machine
without launching a provider. `integrations/puppets/README.md` says how to run it and lists the
events. Do not run cargo. Prose in the README follows the repo's rules: no em dashes.

## Outcome

Built 2026-09-24 by composer-2.5 on run 01a0d36f-1612 (1.35M tokens), `integrations/puppets/`:
`puppets.py`, `prompts/plan.md`, `tests/test_puppets.py`, `README.md`; eleven tests green. The
session's review changed five mechanisms before master, each with a test: a table approval is no
longer spent by the first launch, because decision C says the table once per batch; `plans` reads
the proposal from the run's `result_text`, since the brief asks for the plan as the answer and the
agent had looked only for a worktree file; the launch runs the binary directly and attaches a
Herdr pane afterwards, because routing through `hydra-in-pane` would have blocked until the run
ended; `reconcile` settles a dangling intent, which nothing could clear; and a verdict of
`applied` or `fixed` needs a diff approval. `assign` now asks `sheet.py`'s ranking and takes it
only when a verdict backs it. Twenty-three tests green on master. Assumed by the ticket and false:
that the agent should look for the plan in the worktree.
