# Map 118, seat opus: the code reader

Written 2026-10-13 by the `claude` opus seat of the four-seat panel on map 118. Every claim about
Hydra below names the file I read it in. Claims from papers come from
`.scratch/hydra/research/118-panel-readings.md`, which copies them from a file outside this
worktree. I did not open any paper, so each of those is the readings' word, marked "readings".

Two numbers in the inputs disagree before anything starts. The ticket says 826 runs and the
readings count 866 on the same machine a day later, which is plausibly growth. The second matters
more. The readings say the 70 verdicts "live inside goal logs". Build 288's outcome,
`.scratch/hydra/build/288-the-set-of-original-diffs.md`, measured them in the grader's
`ticket-verdicts.json`, a `{run_id: verdict}` map that `hydra-grader baseline label-import` fills
from a CSV queue (`crates/grader/src/queue.rs` line 297, `crates/grader/src/collect.rs` lines
748 to 779). Goal logs carry `human_verdict` events too (`integrations/puppets/README.md`, the
event table), and `crates/grader/src/labels.rs` reads them as `human_result`. So there are two
verdict stores, and nobody has counted how far they overlap.

## 1. What the training example is

**Recommendation: one example is one plan proposal. The input is the goal text at a `base_sha`,
the output is the plan JSON, and the label is whether the panel or the human chose it. Do not
train on face delegations or on task-and-verdict pairs.**

What I read:

- A goal log already holds the whole planning act. `integrations/puppets/README.md` lists the
  events: `goal` with `text`, `repo`, `base_sha`; `plan_proposed` with `provider`, `model`,
  `run_id`; `panel_verdict` with `chosen`; `human_plan` with `accepted`; then `task` with `id`,
  `text`, `class`, `grade`, `deps`, `paths`. The proposals themselves are the `result_text` of
  each plan run, imported from the chosen plan's fenced JSON block (`IMPLEMENTATION.md`, "Master
  of Puppets"). The output shape is fixed in `integrations/puppets/prompts/plan.md`.
- The grader already tags these runs. `labels_for_goal` in `crates/grader/src/labels.rs` labels
  every plan run `plan_proposer`, every panel seat `plan_judge`, and every build run with its
  task's class, grade, text and human result. The join the training set needs exists and runs.
- Since build 290 the export puts `run_start.task` first as a `user` message
  (`.scratch/hydra/build/290-the-task-crosses-into-the-export.md`). That ticket's item 4 is still
  open: nothing with tasks leaves the machine until the session has read the eligible tasks.
- Face delegations are the wrong source. The readings count 26 records with a `parent_run_id`.
  `IMPLEMENTATION.md`, "Delegation, and the face", says the face is admitted to `permissive` only,
  refuses from inside a seatbelt, and passes no `model`. The delegations Hydra actually runs go
  through `hydra-puppets`, a program outside every run, so the parent of most real delegation is
  a human-approved goal log, not a run record.
- Task-and-verdict pairs grade the builder, not the plan. They are also nearly diffless: build
  288 found 7 of 70 verdicts with a branch left, 62 deleted with their commits.

What the readings quote: SearchSwarm and Training the Orchestrator learn from delegation traces
filtered by an outcome or a verifier. LIMI reaches its result with 78 complete trajectories.

My caveat: I could not count goals or `plan_proposed` events, since `~/hydra/puppets/` is outside
what this run may read. Each goal yields one chosen plan and up to three rejected ones, so the
set is preference pairs as much as SFT rows. Credit is the hard part. A task rejected at review
may be a bad plan or a bad builder, and nothing in the log separates the two.

## 2. What the model outputs at inference

**Recommendation: the plan JSON block of `prompts/plan.md`, and nothing else. It sits as one more
proposal seat beside the large ones and drafts. It replaces no turn, and the assignment table
stays out of its reach.**

What I read:

- The plan's `text` field is the task text each builder receives, so "the task text per builder"
  is already part of the plan output (`integrations/puppets/prompts/plan.md`).
- The assignment table is not a language model's output today and should not become one.
  `assign` in `crates/puppets/src/write.rs` line 762 takes `ranked_assignment` from
  `crates/puppets/src/sheet.rs`, the 10th percentile of a beta posterior on first-pass acceptance,
  and falls back to a default table recorded as `source: default`. Map 77's answer
  (`.scratch/hydra/issues/77-a-local-model-for-the-mode.md`) already decided that routing needs
  no generating model. A 3B that writes the table would replace code with a sampler.
- Plans already arrive from several seats and get judged. `hydra-puppets plan --seat` launches
  one proposer per seat with a stance file, and `choose --judge` puts three seats on the
  proposals under neutral labels (`IMPLEMENTATION.md`, "The judged steps"). A small model's plan
  fits that loop as one more anonymous proposal, and the panel's choice is its live score.
- Seats are `provider[:model]` over the four routed CLIs. A local model is none of them. A plan
  proposer has to read the repository to name `paths`, and a local model gets hands only through
  the harness of map 88, which is decided and not built, and whose endpoint question is map 112,
  still open (`.scratch/hydra/issues/112-hydra-s-own-harness-and-its-local-driver.md`). So 118
  depends on 112 in fact, even though its `Blocked by:` is empty.

What the readings quote: Plan-and-Act fine-tunes a planner separate from its executor. The
Anthropic advisor result puts the large model in the advising seat, not the small one.

## 3. Whether a 3B planner is credible

**Recommendation: keep Ministral 3B for round two as a drafter that must beat a stated bar.
Budget an 8B arm only if it misses. A 3B is not credible as the plan of record, and nothing
measured in the repo says otherwise.**

What I read:

- The only measurements of Ministral 3B are tool calls, not plans. Map 111's answer
  (`.scratch/hydra/issues/111-which-small-model-calls-tools.md`) has it at 10 of 10 first calls
  and 8 of 10 in a six-turn loop, on single-file tasks.
- Round one's gain was a prior, by its own README (`.scratch/hydra/training/round1/README.md`).
  Next call went from 1 of 9 to 5 of 9, and the trained model answered `Bash` on 8 of 9 held-out
  steps where 5 wanted it. "Nothing here says the model got better at the work."
- The training window is the binding limit. Round one ran out of memory at 4,096 tokens on a T4
  and trained at 2,048, with prompts cut to 2,000 characters and tool output to 600
  (`round1/README.md`, `round1/build_sft.py`). A planner's input is a goal plus enough of the
  repository to name files. That does not fit in 2,000 characters, so round two needs a bigger
  card whatever the model size.
- The machine is an M2 with 8 GB (map 111's answer). Ministral 3B loads at 2.7 GB at 4,096
  tokens of context. I found no measurement of anything larger on it.

What the readings quote: Zywot et al. call these systems "planner-limited rather than
executor-limited". Their smallest orchestrator with a reported win is 8B, and the readings give
no number for a 1.7B orchestrator. ToolOrchestra is 8B with RL. Training the Orchestrator is the
one small-policy result, and its policy picks among a few repair agents plus three hardcoded
rules. That job is closer to Hydra's `rank` than to writing a plan.

So the papers that back a small orchestrator back an 8B, or a small model making a narrow choice.
Hydra's narrow choice is already made by code. What is left for a model is the plan, which is the
part the readings say needs the reasoning budget.

## 4. Who pays per version, and the test that says it is not worse

**Recommendation: not one model per Hydra version. Retrain when the plan contract changes, meaning
`prompts/plan.md`'s shape or the class and grade lists, or when the held-out score drops. The
human's cloud project pays, under a cap written down per round. The regression test is a replay
of held-out goals plus the five bench goals run end to end.**

What I read:

- Cost so far: one spot T4 for about 65 minutes in `<gcp-project>`, an estimate of 0.25 USD
  against a 10 USD cap, "not yet read off the bill" (`round1/README.md`). Map 77's answer prices
  a rented 3090 at 0.22 USD an hour. Neither covers a window long enough for a planner.
- Most Hydra versions do not touch what a planner emits. The contract a plan must meet is
  `prompts/plan.md` and the class list repeated in `integrations/puppets/README.md`. That is the
  trigger to version on.
- A cheap test already exists in pieces. The plan output is checkable by code: the JSON parses,
  `class` and `grade` are in the lists, `deps` names ids that exist and has no cycle, and `paths`
  exist at `base_sha`. The bench (`integrations/bench/README.md`, five goals against the `tally`
  crate) gives each goal a done check, for example `cargo run --quiet -- --json` printing
  `{"words":0,"lines":0,"chars":0}` in `integrations/bench/goals/l1-cli.md`.
- The bench has never run. Build 274's outcome: "Running them is the session's and has not
  started." It also measures orchestrator cost, `hydra cost` in `src/cost.rs`, not plan quality.
  So the test round two needs is a second use of the bench that does not exist yet.

The proposed bar is mine, not measured. A new model is not worse if, on held-out goals, its plans
pass the code checks at least as often as the last model's, and the judge panel picks it over the
last model's plan at least half the time under neutral labels. On the bench, the human's diffs
must be applied as often as with the large planner. Five goals times three repetitions is a
small sample, and the memo should not pretend otherwise.

## 5. Whether this closes the "Fine-tuning" line under `## Out of scope`

**Recommendation: yes, rewrite the line rather than delete it, and keep D12's constraint. The
rewrite names what is in scope: QLoRA on a small open model, trained off the machine from
`hydra export`, its code under `.scratch/hydra/training/`, its weights never loaded by `hydra`
itself.**

What I read:

- `.scratch/hydra/map.md` line 214, D12: "Training is out of scope. Lossless export to SFT and
  DPO formats stays a constraint on the record." Line 413, under `## Out of scope`:
  "Fine-tuning. Axolotl, Unsloth, preference-optimisation training runs, any weights. Its own
  effort once there is a corpus worth training on."
- Round one fine-tuned weights on 2026-10-04 (`round1/README.md`). Map 77's answer already
  planned QLoRA on a rented GPU. The map has contradicted itself for nine days.
- The line's own condition, "once there is a corpus worth training on", is not met by the
  numbers in the repo: 24 of 351 runs survived round one's filter, and 7 of 70 verdicts keep a
  diff (build 288). So the honest rewrite says the effort has started, and also that its corpus
  is still being built.

Whether 118 is the ticket that closes it is a question of form. The gap is wider than the
orchestrator: round one trained a builder. I would close the line in a map entry of its own that
118 links to, so a later ticket about a builder model does not have to cite an orchestrator
grilling for permission.

## Vote

Train the small model on plans, since Hydra's logs already record plans, choices and verdicts
for them, while face delegations number 26 and diffs survive for 7 of 70 verdicts. Keep it a
drafting seat whose output is `prompts/plan.md`'s JSON, keep the assignment table in `rank`, and
treat 3B as a test to pass on held-out goals, with an 8B arm budgeted only if it fails. Before
any of that is trainable, map 112's harness and endpoint decision has to land and the bench has
to run once, because today a local model cannot read the repository it is asked to plan.

## What I could not verify

- How many goals, `plan_proposed` events, `panel_verdict` choices and `human_plan` acceptances
  exist. They live under `~/hydra/puppets/`, which this run may not read. That count decides
  whether question 1's set is 20 rows or 200.
- Whether the 70 verdicts of `ticket-verdicts.json` and the goal logs' `human_verdict` events
  are the same 70. The readings and build 288 place them differently.
- The 26 delegations: which providers, which parents, and whether any parent was a plan run.
- Any paper's numbers. I read only the readings' copy. In particular, whether Zywot et al.
  report a 1.7B or 4B orchestrator result, which is the number question 3 most needs.
- Whether a planner prompt with repository context fits any card at round two's budget. Round
  one's 2,048-token window is measured. The window a plan needs is not.
- The real bill for round one, which its README says was never read.
