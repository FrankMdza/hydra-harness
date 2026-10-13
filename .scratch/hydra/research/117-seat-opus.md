# Map 117, the code reader's seat (Opus 5.5)

Written 2026-10-13 by one seat of the panel on map 117. Every claim about Hydra today comes from
a file I opened in this worktree, cited by relative path. Claims from the deep research note
come through `.scratch/hydra/research/118-panel-readings.md` and are marked "the readings say". I
did not run cargo or the suite. The one number I computed myself is the Beta percentiles in
question 3, with a 200,000-draw Monte Carlo in `python3`, and it reproduces the
`beta_lower(5, 3) = 0.4038` the repo's own test pins in `crates/puppets/src/sheet.rs`.

The 118 panel settled that the assignment table stays in code, that nobody retrains per release,
and that a static plan validator belongs in `write.rs::tasks`. I build on those and do not
reopen them.

## What the code does today, the facts the answers rest on

- **What a builder receives.** `launch` in `crates/puppets/src/launch.rs:776` builds one argv
  through `build_run_argv` (`launch.rs:50`): `hydra run --provider P [--model M] --policy
  standard --timeout 0 --task T`. `T` is `worker_task_text` (`launch.rs:759`), which is the
  task's `text`, then "Done means:" and the plan's `done` or an admission that it has none, then
  a standing ask to open the report with a "Not confirmed" section. That string is the whole
  delegation. It is fixed in the goal log when `tasks` imports the chosen plan
  (`crates/puppets/src/write.rs:699`), well before launch.
- **What the planner is told to write.** `integrations/puppets/prompts/plan.md` asks for tasks
  that "name the files or areas it touches when that is knowable", a `done` that is "a command
  and its expected result", and `paths`. The prompt says nothing about leaving the how to the
  builder. It asks for the where on purpose.
- **What stands between a task and its launch.** `launch_refusal` (`crates/puppets/src/fold.rs:490`)
  refuses on a dangling intent, a task with no assignment, no table approval at the current
  revision, and, for claude and codex, no per-task launch approval because their gates can fail
  open. Nothing reads the task text.
- **How a builder is chosen.** `assign` (`write.rs:762`) asks `sheet::ranked_assignment`
  (`crates/puppets/src/sheet.rs:466`) first and falls back to `DEFAULT_ASSIGNMENTS`
  (`write.rs:36`): Grok 4.6 high on cursor-agent for `batch_scripting`, `docs`,
  `rust_off_launch_path` and `research`; `claude sonnet` for `rust_launch_path`; codex for
  `security_review`; agy Gemini 3.1 Pro high for `probe`. An `agi` grade always waits for the
  human's `set` (`write.rs:187`). `rank` (`sheet.rs:333`) keeps an agent only if the 10th
  percentile of Beta(prior plus verdicts) clears 0.5, 0.6 or 0.7 for `low`, `medium`, `high`
  (`sheet.rs:345`), then sorts by median window share over that bound (`sheet.rs:450`). That is
  already "the cheapest builder that clears the bar", the human's third tier, measured on human
  verdicts rather than on tests.
- **Where the ranker's evidence comes from.** `assign` passes `log::read_log(home, goal_id)`, the
  events of this one goal, as the verdicts. `ranked_assignment` returns `None` when the first row
  has no verdict behind it (`sheet.rs:507`), and the default table wins. The sheets
  `crates/grader/src/sheet.rs` builds carry `runs` and cost, no human verdicts. So a builder's
  record does not carry from one goal to the next. Every goal starts on the default table.
- **What review exists.** `verdict` (`write.rs:459`) records the human's `applied`, `fixed` or
  `rejected`, and the first two need a diff approval. The diff panel prompt is
  `integrations/puppets/prompts/diff.md`, three seats from `PANEL_PROVIDERS` (`fold.rs:527`).
  `wait` (`launch.rs:890`) writes `run_end` with the record's outcome and `total_tokens`. Nothing
  in `crates/puppets/` runs a task's `done` check.
- **Jev and Kev.** `grep` finds no mention of either in `crates/puppets/src/`. Map 109's Answer
  says nothing in the repo can call Jev since build 220 dropped `jev ask`, and that Jev's best
  use is ordering diffs: "rank, never a vote or a threshold". Map 110, which would decide whether
  text may go to Typesafe, is open. `.scratch/hydra/research/109b-kev.md` measured Kev-0.8B on
  this Mac: 3.0 GB steady, 4.5 GB at load, 1.0 GB of swap left with one agent running, about
  260 ms per new document. Kev and Jev score options a caller supplies and return a probability
  each. They do not write text. Build 294, Kev beside `system-one`, is claimed with no Outcome.

## 1. What the Jev tier reads and what it returns

**Recommendation: it reads the exact `--task` string `worker_task_text` produces, plus the goal
text for the why. It returns a probability written into the goal log as an event, shown to the
human before `approve --table`. No refusal and no rewrite.**

- Read the launched string, not the plan narrative and not the goal log. `worker_task_text` is
  the only thing the builder sees, so judging anything else judges the wrong text. Without the
  goal text a classifier cannot tell whether the "why" is there, so the goal goes in beside it.
- The builder's first turn is out. It exists only after launch, which is too late for a check on
  the delegation. It also lives in the run record, and a run cannot read `~/hydra/runs/`.
- A rewrite is impossible by construction. Kev and Jev pick among supplied options
  (`109b-kev.md`, the request and the `noul` answer). Rewriting the task needs a generator,
  which is the planner again.
- A refusal needs a threshold, and map 109 says this family of model is for ranking. There is
  also no label for "this prompt confirms the how". The 70 verdicts are about diffs. With zero
  labels, any cut-off is a guess. A logged score can become a gate once it has been checked
  against verdicts.

One thing has to be decided first, and it is the human's call: what "the how" means here. The
plan prompt asks the planner for paths and a runnable `done`. If naming `src/lib.rs` counts as
dictating the how, the check fights `plan.md` on every task. My reading: what, why, where and
done are the planner's, and the steps are the builder's. The check then asks whether the task
prescribes the steps.

## 2. Where the check runs

**Recommendation: in `hydra-puppets`, at `tasks` import, beside the static validator the 118
synthesis proposed for `write.rs::tasks`. Not a gate hook, not a panel seat.**

- The task text is final once `tasks` imports it. `launch` only wraps it. Scoring at import
  puts the number in front of the human at the table approval, while changing the task costs
  nothing.
- A gate hook is the wrong layer. Hydra's gates are `PreToolUse` hooks on tool calls
  (`src/provider/claude.rs:367`, `src/provider/codex.rs:6`), compiled per policy and running
  inside the run. None fires on a prompt. A model call from inside the gate would add a network
  or loopback dependency to the one piece of the harness that has to stay simple. Hydra
  already holds the text in the argv before the child exists, so the gate would be checking
  something Hydra decided itself.
- A panel seat is the wrong cost. Panel seats are full agent runs over diffs (`diff.md`). A
  scored-option model answers in a quarter of a second. Spending a seat on a three-line string
  is a waste.
- One gap stays open whatever is chosen. The 26 face delegations (`src/face.rs`, counted in the
  readings) never go through `hydra-puppets`, so this check does not see them.
- Which model? Remote Jev waits on map 110 and on a client nobody has written. Kev-0.8B on
  loopback fits, barely, per `109b-kev.md`. Build 294 measures it on run outcomes, not on
  delegation text. The first step is also the cheapest: add one sentence to
  `integrations/puppets/prompts/plan.md` telling the planner to leave the steps to the builder,
  and see whether the score is ever needed.

## 3. The default builder and what promotes a ticket

**Recommendation: keep `DEFAULT_ASSIGNMENTS` as the default. Add a small builder only as a
human-set trial on `low` tasks whose `done` is a command. Promote on a `rejected` or `fixed`
verdict, and fix the ranker so a builder's verdicts survive the goal they were earned in.
Jev's score promotes nothing. `Type:` is not the right input either.**

- The ranker is already the tier-3 rule. What it lacks is memory. Its evidence is one goal's
  log, so Haiku can never become the default by earning it. With Haiku's prior of [3, 3] in
  `integrations/puppets/priors.json`, the 10th percentile is 0.245. It needs four straight
  first-pass `applied` verdicts in one goal to reach 0.509 and clear `low`. Most goals do not
  have four `low` tasks. Grok 4.6 at [4, 2] sits at 0.416 and Sonnet at [5, 1] at 0.632. No
  prior alone clears `medium`'s 0.6 except Sonnet's, and none counts anyway, because a row with
  no verdict behind it falls back to the table. The build ticket is to have `rank` read verdicts
  across goals, or have the grader's sheets carry them. Without it, "the smallest model that
  passes" cannot be learned and has to be typed by hand.
- Promotion on a failed test needs Hydra to run the test. Nothing in `crates/puppets/` runs
  `done`. Today the failed test is the human's `rejected` or `fixed`, and the human reassigns
  with `set`. A new `assignment` event bumps the revision (`fold.rs:246`), so the table needs
  approval again. That is the right stop. An automatic "relaunch one size up after a rejected
  verdict" is a small build. "Run `done` and promote on failure" is bigger, because `done` is
  free prose today (`plan.md` calls it optional).
- Ticket `Type:` does not exist inside a goal. Puppets tasks carry `class` and `grade`, and
  those already pick the row. Jev's score must not promote, for the threshold reason in
  question 1.
- The memory says tasks go to Opus or Grok 4.6. The code's table sends `rust_launch_path` to
  Sonnet. The ticket's own line, "vendors.json, Opus or Grok 4.6 for tasks", also misplaces it:
  the defaults live in `write.rs:36`, and `vendors.json` only lists models.

## 4. What is measured to say the tier works

**Recommendation: three numbers per task class, all from goal-log events that exist today:
first-pass `applied` rate, `fixed` plus `rejected` per task, and agent `total_tokens` per
`applied` task. Fix a non-inferiority margin before the run. The five bench goals are a smoke
test for this question, not the evidence.**

- The events exist. `run_end` carries `total_tokens` (`launch.rs:890`), `human_verdict` carries
  the result (`write.rs:459`), and `launch_intent` carries the provider and model. "Cost per
  resolved ticket" is a sum over those, but in tokens, not dollars. `rank` scores by window
  share, because the providers here are subscriptions.
- `hydra cost` (`integrations/bench/README.md`, `IMPLEMENTATION.md` on build 274) measures a
  different thing: orchestrator tokens over agent tokens per goal. It neither counts verdicts nor
  divides by resolved tasks. It needs a column, or a sibling command.
- The bench goals are the wrong instrument for the middle tier. All five spell out the how
  themselves. `m1-chars.md` says where to put the function and which tests to write.
  `m2-saturating-bump.md` names the fix, saturating add. `s1-readme-typo.md` names the word. A
  delegation check has nothing to find in them. The repo is twelve lines
  (`integrations/bench/README.md`), so every builder likely passes, and a small one will look
  free. Only `l1-cli.md` is several tasks.
- The sample is small. Five goals times three repetitions is about 15 goal runs per arm. At a
  first-pass rate near one half, the 95 percent interval on the difference between arms is about
  plus or minus 35 points. Before and after on this set catches a collapse and nothing finer.
  The goal set needs a few goals stated as outcomes, with no how in the text, and the
  multi-task goals the 118 synthesis already asked for.
- The readings say the cheap worker's loss shows up as invented facts, not bad structure. The
  measure that catches that is `fixed`, a diff the human had to repair. `worker_task_text`'s
  "Not confirmed" section is the guard Hydra has today. Count how often the human's fix touched a
  claim the builder did not list there. That last count needs reading, not a field, and it is
  the human's.

## 5. Same as map 115, or a different ticket

**Recommendation: different. 117 is how one goal hands work out, 115 is how the harness changes
itself. They share one measurement, and 117's middle tier sits on the same seam as 118's plan
validator.**

- 115's reach is skills, provider facts and Hydra's source, through `hydra curate` and
  `hydra evolve`. 117 changes none of those. It changes what `assign` picks and what the human
  sees before `approve --table`, inside one goal log.
- The overlap is real in two places. The ranker learning across goals, question 3's build, is
  learning from use, but it learns which builder to pick and leaves the harness alone. And 115's
  question 4, what "better" is measured by, should use the same per-class verdict rates as
  question 4 here, so the two do not grow two scoreboards.
- The closer sibling is 118. 118 is about the planner's output, and 117's middle tier checks the
  planner's output. Both land at `write.rs::tasks`. I would have the two build tickets share that
  spot: the graph check from 118, then the delegation score from 117.

## What the readings say against the premise

The human's middle tier strips the how out of the delegation. The readings point the other way.
Zywot et al. find these systems "planner-limited rather than executor-limited", with reasoning
in workers adding "little or hurt". MinionS has the cloud model decompose into chunk jobs. The
advisor strategy has Opus advising the Haiku executor while it works. The readings' own summary
is "put the reasoning budget in the planner, make the worker's job narrow and checkable".
Leaving the how to a 3B builder moves reasoning from the planner to the worker, which is the
configuration these results found weakest. None of the papers is about coding, though. The one
software result is Sonnet with an Opus advisor on SWE-bench Multilingual, +2.7 points, and there
the advisor adds guidance. The human may mean something narrower that I would back: don't
pre-write the code, but do say what, why, where and how to check. That matches `plan.md` as it
stands.

## Vote

Build the third tier first. It is mostly there in `sheet.rs`, and it needs only verdicts that
cross goals plus a promote-on-rejection step. That beats a new middle tier. Hold the Jev tier at
a logged Kev score at `tasks` import, with one prompt line in `plan.md` before it, and never
make it a refusal until enough verdicts exist to check the score against. Settle what "the how"
means before building any of it, because the readings back a planner that hands down more
detail, not less.

## What I could not verify

- The goal logs under `~/hydra/puppets/`, so I counted no real tasks, verdicts or `done` fields.
  The 70 verdicts are the 118 synthesis's figure, not mine.
- Whether any goal so far has had four `low` tasks of one class, which question 3's arithmetic
  depends on.
- How Kev-0.8B scores a delegation question. `109b-kev.md` measured memory and latency, not
  accuracy on any Hydra text, and build 294's Outcome is empty.
- The deep research note itself. I saw section 5 only as the readings copy it, and opened none of
  the URLs.
- Whether the advisor-strategy result applies when the executor writes code, as opposed to
  browsing.
- `src/face.rs`, beyond the readings' count of 26 child records.

## Pendiente de mí

- Nothing for the seat. The human decides what "the how" means and answers the ticket.

## Cambiado

- Created `.scratch/hydra/research/117-seat-opus.md`. Nothing else edited.

## Encontrado

- `rank` learns only from the current goal's verdicts (`write.rs:762` passes one goal's log), so
  a small builder cannot earn the default across goals.
- `DEFAULT_ASSIGNMENTS` sends `rust_launch_path` to Sonnet, while the memory and the ticket say
  Opus or Grok 4.6.
- All five bench goals dictate the how, so they cannot test a delegation check.
- `plan.md` asks the planner for files and paths, which a strict "leave the how" check would
  flag on every task.
- The worktree had uncommitted changes from another session when this seat started, in
  `IMPLEMENTATION.md`, `tests/` and build 294's ticket. I left them alone.
