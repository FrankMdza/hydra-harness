# Map 122, the session's synthesis for the human

Two seats sat on 2026-10-13: Opus 5.5 on `claude` (`122-seat-opus.md`) and Gemini 3.8 Flash on
`agy` (`122-seat-gemini.md`). The GPT seat died at its first turn on codex's usage limit; no Grok
seat while Cursor is on the free plan. Neither seat could read the goal logs under
`~/hydra/puppets/`, so every claim about goal `ea69da1f` comes from build 303's Outcome, the review
of 2026-09-24 and `IMPLEMENTATION.md`, and both seats say so. The session checked the claims that
count against the repo.

## A finding before the questions: no panel composed anything, and the live panel cannot

Opus found it and the session confirmed it in `docs/reviews/2026-09-24-master-of-puppets-first-goal.md`
lines 68 and 69: "codex is out of credits until 2026-09-27 23:04, so neither judge panel sat today:
the plan was chosen by the session". The `panel_verdict` on `ea69da1f` came from `write.rs::choose`
with a file argument, which records `chosen: "composed"`, the file's path and
`judge_run_ids: []`. The review's run table, line 43, names the Composer 2.5 proposal as "the plan
the composed one came from". So the ticket asks how to credit a plan the session wrote by hand
from one proposal, not a panel's synthesis.

Opus's second finding: the live panel cannot compose on its own instructions. The inline choose
prompt in `panel.rs` asks each seat for one JSON object with `chosen` and `findings` and "nothing
else"; `plan_panel_verdict` then refuses a `composed` verdict that carries no fenced task array.
The prompt that does ask for the array, `integrations/puppets/prompts/choose.md`, is read by
nothing: `launch.rs` reads `plan.md` and `stances/` from the prompts directory and no other file
(the session confirmed lines 420 and 433). A panel composition today needs every seat to say
`composed` and one of them to disobey "nothing else". Gemini read the same code and described the
mechanism without noticing it cannot fire. That is a bug, not a decision, and it comes before any
answer to question 2.

## Where the two agree

- **Question 1.** Credit none of the proposers. Both seats reject the weighted share (tasks do not
  compose linearly, and `write.rs` records no task origin) and the nearest reading (subjective; the
  panel composed because no draft was good enough as it was). Opus adds what to do with the
  outcome so it is not lost: give the composed plan a row of its own, `class: "plan_composed"`,
  `chosen: true`, with the goal's outcome. Gemini stops at "keep composed goals out of the outcome
  label". Recommendation: Opus's row, because a goal's six verdicts landing on no row at all is the
  gap 303 found, and the composed plan is the one that actually ran.
- **Question 2.** Do not ask the judge prompt for provenance. Gemini: models already fail the plan
  JSON one time in four (map 121), and a seat attributing its own merge is self-report; compute
  attribution deterministically in code by matching task text and paths against the proposals,
  and write the mapping into the goal log, since `labels.rs` reads events and the folded state and
  never the plan file. Opus: per-task `from`, if ever, imported by `tasks` into the `task` event;
  cleaner than a goal-level share. The two agree on where it would live (the goal log, additive to
  the `task` event, which is not protected config: harness spec 15.1 names six protected configs
  and the puppets prompts and log format are not among them) and on waiting until a panel
  composition exists to credit.
- **Question 3.** A `false` proposer row teaches only as half of a pair with a `true` row for the
  same goal, and only when it holds a draft. On a composed goal with no composed row there is no
  pair; with Opus's row, every composed goal yields pairs with no attribution needed. Both seats
  note the mild mismatch: the composed plan's author read the proposals, so its input was richer
  than a proposer's, which matters for imitation and not for preference pairs.

## Where they split, and the recommendation

1. **A defect only Opus found.** `fold.rs` gives a plan entry its `path` on `plan_ready` (line
   196) and creates it on `plan_proposed` (line 180); `labels.rs` labels every plan entry `chosen`
   whenever a verdict exists, path or not. A proposer that never produced a plan, such as codex
   rate-limited on `ea69da1f` with 0 tokens, gets `chosen: false` as if the panel had read and
   rejected a draft. Build 303's "not chosen 6" includes such rows. Recommendation: a proposer with
   no `path` gets `chosen: null`, or is left out of the split; this is an implementation fix.
2. **How much of 303's one usable row is real.** Gemini counts 3 preference pairs on `a598914f`
   from 1 true and 3 false rows; after the fix above the count may drop, since any of the three
   may have been a proposer without a draft. Not a split, a correction to the number both seats
   quote.

Fable was not called as judge: the seats agree on every question and differ only in how far they
carry the answer.

## What would become tickets if the human agrees

- Build, no decision needed: the inline choose prompt in `panel.rs` asks for the task array when
  it offers `composed`, so the live panel can compose at all; `tests/` gets a case where a seat
  answers `composed` with a fenced array and the verdict is written.
- Build, no decision needed: a proposer with no `plan_ready` gets `chosen: null` in `labels.rs`,
  and `labels count` splits "no draft" from "not chosen".
- Build, after the human's answer to question 1: a `plan_composed` row carrying `chosen: true`
  and the goal's outcome, and the equivalence tests extended the way 303 extended them.
- Not yet: per-task `from` or text-match attribution, until a panel composition exists to credit.
