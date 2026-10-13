# Map 122, the Opus seat

Written 2026-10-13 by the Opus 5.5 seat of the four-seat panel. I read the ticket, build 303,
the 118 synthesis, `crates/grader/src/labels.rs`, `crates/puppets/src/{write,panel,fold,launch}.rs`,
`integrations/puppets/README.md`, `integrations/puppets/prompts/`, harness spec 15.1, and
`docs/reviews/2026-09-24-master-of-puppets-first-goal.md`. I could not read the goal logs under
`~/hydra/puppets/`, which are outside this worktree. Every claim about `ea69da1f` below comes from
build 303's Outcome, `IMPLEMENTATION.md` line 3460 and the 2026-09-24 review, and I say which.

## Before the questions: the panel did not compose ea69da1f

The ticket's premise is that "the panel chose `composed`" on `ea69da1f`. Two documents in this
repo say no panel sat that day. The 2026-09-24 review, line 68: "codex is out of credits until
2026-09-27 23:04, so neither judge panel sat today: the plan was chosen by the session". And
`IMPLEMENTATION.md` line 3464 says the same. So the `panel_verdict` on that goal came from
`write.rs::choose` with a file, lines 592 to 611: a file argument records `chosen` as the file's
stem, `composed`, its absolute path as `path`, and `judge_run_ids: []`. Build 303's own count fits
this. It found 3 `plan_judge` rows across both goals, which is one panel of three, and the other
goal, `a598914f`, is the one that chose a proposer.

The same review names where the composition came from. Its run table, line 43, marks the
composer-2.5 proposer `01a0d57e-52e4` as "the plan the composed one came from". So the "nearest"
reading of question 1 was declared by the session on the day, in prose, and never reached the log.

The second fact the ticket did not have: **the live panel cannot compose on its own instructions.**
`panel.rs::build_choose_seat_prompt`, lines 714 to 721, tells each seat to pick a label "or
`composed`" and to "Answer in exactly one JSON object and nothing else". It never asks for a task
array. `plan_panel_verdict`, lines 634 to 640, then refuses a `composed` verdict with no fenced
task block: "composed chosen but no task block found". The prompt that does ask for the block,
`integrations/puppets/prompts/choose.md`, is read by nothing. `launch.rs` reads `plan.md` and
`stances/` from `prompts/` (lines 420 and 433) and no other file there; `choose.md` and
`diff.md` have been dead since the Rust port, build 195, whose `panel.rs` already carried the
inline "nothing else" prompt. A panel composition today needs all three seats to say `composed`
and at least one to disobey "nothing else". On top of that the verdict needs unanimity, or two
tie-break seats that agree.

So "composed" in practice means "the human or the session wrote the plan and pointed `choose` at
it". The ticket is asking how to credit a human-authored plan, not a panel's. That changes the
answers below.

## 1. When the panel composes, is any proposal credited?

**What the code does.** `labels.rs::picked`, lines 475 to 482: a verdict with a path picks the
proposal whose own `path` equals it. `plans/composed.md` is no proposer's path, so every proposer
gets `chosen: false` and `outcome: null` (the loop at lines 554 to 569). The outcome, `tally` at
line 485, is computed once per goal and attached only to a `true` row, so a composed goal's
outcome lands on no row at all.

**What I verified.** `cargo test --release -p hydra-grader --lib labels`: 9 passed, including
`chosen_is_null_without_a_verdict_and_false_for_every_proposer_of_a_composed_plan`, which pins
exactly this. On the count the session asked for, I can measure only what the repo holds. The 164
fixtures under `crates/puppets/tests/fixtures/read/` are harvested differential cases, not real
goals: `composed` appears in 2 of them, `h-009` and `h-015`, both hand-built logs with a goal text
of `"t"`. They say nothing about how often goals compose. The real number is build 303's: two goal
logs on this machine, one `composed`, and that one written by the session. Panel compositions
recorded: as far as the repo's documents show, zero, and the prompt mismatch above says why.

**Recommendation.** Credit none of the proposers, and give the composed plan a row of its own,
say `class: "plan_composed"`, with `chosen: true` and the goal's `outcome`. The outcome label is
not lost; it goes to the plan that actually ran. Don't pay for a share before the panel can
compose at all, and before two or three panel compositions exist to look at.

**What would change my mind.** If the model 118 trains is only ever a proposer and the reward has
to land on its own drafts, a proposer-level credit matters and the nearest reading becomes worth
recording. Also, if after the prompt is fixed most goals compose, the per-task attribution of
question 2 is cheap enough to build.

## 2. What would the panel write at composition time?

**What the code does.** A composed plan is written by `panel.rs` line 994 as
`Composed plan.` plus the fenced array, nothing else. The blind label map is already recorded on
the verdict event as `label_map`, line 922, so a seat that writes `"Proposal B"` is resolvable to a
provider later, since labels are fixed per goal and only the order turns per seat. But
`write.rs::tasks`, lines 826 to 840, copies seven named fields from each task into the `task`
event and drops the rest. A `from` field written into `composed.md` today would vanish on import,
and `labels.rs` reads events and the folded state, never the plan file.

**Is it a protected prompt?** Not by the spec. Harness spec 15.1 lists six protected configs and
neither the puppets prompts nor the goal-log format is one of them, and no file under `policies/`
or `config/` names `puppets`. The choose prompt is inline Rust in `panel.rs`, so changing it is a
code change under review like any other. The goal log's contract is the event table in
`integrations/puppets/README.md` lines 122 to 137. Adding an optional field to `task` is additive.

**The smallest change, if the human wants a share.** Per task, not a weighted goal share:
1. The inline prompt in `panel.rs` asks for the task array when it offers `composed`, which it has
   to do anyway or panel composition stays dead, with `"from": ["Proposal A", ...]` on each task.
2. `validated_tasks` accepts an optional `from` list and `tasks` copies it into the `task` event.
3. `labels.rs` gives each proposer the tally over the tasks whose `from` names it.
4. `choose <file>` has no seat to ask. The human writing a composed plan would add `from` by hand,
   or that goal stays proposer-uncredited.

Per-task credit is cleaner than the ticket's weighted share. A task's verdict belongs to the
proposals that task came from, and nothing has to be weighted.

**Recommendation.** Fix the prompt so the panel can compose at all, since that is a bug and not a
decision, and leave `from` unbuilt until there are panel compositions to credit.

**What would change my mind.** A cheap offline check that the self-reported `from` is wrong often.
A seat attributing its own merge is self-report. Text similarity between composed tasks and each
proposal's tasks could measure it from the plan files without any prompt change. I could not run it
here because the plan files are outside this worktree.

## 3. Does a `false` proposer row teach anything?

**What the code does.** A preference label is useful as a pair: same goal, one chosen completion,
one rejected. On a composed goal every proposer row is `false`, so there is no chosen side among
the proposer rows and no pair can be formed from them. The rejected drafts teach something only
if the composed plan becomes the chosen side, which is the row question 1 recommends.

**A defect I found.** `fold.rs` lines 180 to 203 add a plan entry on `plan_proposed` and give it a
`path` only on `plan_ready`. `labels.rs` lines 554 to 559 label every plan entry with `chosen`
whenever a verdict exists, path or not. A proposer that never produced a plan, such as one that
hit its rate limit, gets `chosen: false` as if the panel had read and rejected it. On `ea69da1f`
the review's table, line 42, lists codex as a proposer, "rate_limited until 2026-09-27 23:04",
0 tokens. So at most two of that goal's three `false` rows are drafts, and one is a run that wrote
nothing. Build 303's machine count of "not chosen 6" includes rows like this, how many I cannot
check from here. This is an implementation fix, not a decision: a proposer with no `path` should
get `chosen: null`, or be left out of the split.

**Recommendation.** Yes, a `false` row teaches something, but only beside a `true` row for the
same goal, and only when it holds a draft. Fix the no-path case; then the composed-plan row of
question 1 turns every composed goal into pairs with no attribution needed.

**What would change my mind.** If the training recipe is plain SFT on chosen plans only, `false`
rows teach nothing and only the composed row matters. One caveat on the composed row as the chosen
side: whoever composed it read the proposals, so its input is richer than a proposer's goal and
repository context. For DPO the prompt is still the same goal, which is what the pair needs. For a
model meant to imitate proposers, it is a mild mismatch worth naming in the training ticket.

## Recommendation summary

1. Credit no proposer; record the composed plan as its own row carrying `chosen: true` and the outcome, and note that on `ea69da1f` the composer was the session, not a panel.
2. First fix the inline choose prompt in `panel.rs`, which never asks for the task block a composed verdict requires; a per-task `from` imported by `tasks` waits until panel compositions exist, and it touches no protected config.
3. A `false` row teaches only beside a `true` row and only if it holds a draft; give a proposer without a `path` `chosen: null`, and the composed-plan row makes every composed goal a source of pairs.
