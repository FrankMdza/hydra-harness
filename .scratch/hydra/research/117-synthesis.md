# Map 117, the session's synthesis for the human

Written 2026-10-13 by the orchestrating session from three seat memos, same degraded panel as
118: Opus 5.5 (`117-seat-opus.md`, the code reader, 1.78M tokens), Gemini 3.8 Flash
(`117-seat-gemini.md`, research against what the logs can measure, 0.74M), GPT-6 astra
(`117-seat-gpt.md`, the adjudicator, 0.33M; its memo file was refused by the floor and was
recovered from the record). No Grok seat, Cursor being on the free plan. The session checked
the claims the seats disputed: `DEFAULT_ASSIGNMENTS` is `write.rs:36`; `sheet.rs::rank` takes
the goal's own events as `verdicts`, so it adapts within one goal and not across goals, which
is what Opus said is missing; `fold.rs::launch_refusal` checks intents, the task and approvals
and nothing about what the task says, so no delegation check runs anywhere today. The decision
is the human's; nothing is written under the ticket's Answer.

## Where the three agree

- The middle tier does not exist and nothing should refuse a launch on a classifier score today.
  Opus: log a score at `tasks` import, never a refusal. Gemini: nothing in production. GPT:
  offline audit first, then at most an advisory line before table approval.
- The default builder stays `DEFAULT_ASSIGNMENTS` per task class. Nobody promotes on a Jev score
  or on a ticket's `Type:` alone.
- Whatever the tier is, the numbers that judge it come from goal-log events that exist today:
  first-pass acceptance, eventual acceptance counting `fixed`, rejections, tokens per attempt.
  No report prints them yet.
- 117 and 115 are different tickets. Gemini said a dynamic third tier needs 115 first; GPT
  read 115 and `rank` and found no such dependency.

## Where they split, and the recommendation

1. **What Jev reads and returns.** Unanimous on the input: the exact `--task` string
   `worker_task_text` renders, plus the goal text, before launch; the builder's first turn is
   too late. Split on the output. Recommendation, GPT's: an offline audit of past delegations
   with a written rubric (what is required behaviour and acceptance, what is implementation
   prescribed when it need not be), a score that is declared unvalidated, no rewrite. Only when
   that audit shows the score tracks rejections does it earn an advisory line in the cockpit.
2. **Where it runs.** `hydra-puppets`, at `tasks` import beside the graph validator of build
   299, as a logged event and never a refusal; not a gate hook (the hook sees tool calls, not
   task semantics) and not a panel seat.
3. **Default builder and promotion.** Keep the defaults. Trial a small builder only on bounded
   tasks with an acceptance check the harness can run itself (the ticket's tests), and promote
   after a diagnosed failure of that check or of review, never on a score alone. Opus's point
   stands that `rank` needs verdicts across goals before "the smallest model that passes"
   means anything: today each goal starts from the prior.
4. **What is measured.** Build the report before the tier: per task class and per
   provider-and-model, first-pass acceptance, eventual acceptance, rejections, tokens and the
   subscription-window share where the record carries it, with orchestration, review and any
   classifier counted as cost. The five bench goals plus the goals already on disk are the
   corpus; no goal needs re-running to print the first version.
5. **Same as 115?** No. 117 is how one goal hands work out; 115 is how the harness changes
   itself. They share the outcome data and nothing else.

One thing all three noticed and the human should weigh: the readings back a planner that hands
down more detail to a small builder, not less. "Leave the how to the builder" is the premise of
the middle tier, and the measured results in section 5(b) of the readings point the other way
for small workers. The audit in question 1 is where that gets settled with Hydra's own
rejections rather than with papers.

## What would become tickets if the human agrees

- Build, no decision needed: the per-class, per-model report of question 4 from the goal logs,
  printed by `hydra-puppets`. Opened as build 301 by the session.
- Build, after the human's answer: `sheet.rs::rank` fed verdicts across goals, with the prior
  stated.
- Research, after the human's rubric: the offline audit of question 1 over the delegations on
  disk, scored by Kev (build 294 measures Kev-0.8B on this machine) and checked against the
  recorded verdicts.
