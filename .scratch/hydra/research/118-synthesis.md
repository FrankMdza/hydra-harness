# Map 118, the session's synthesis for the human

Written 2026-10-13 by the orchestrating session from three seat memos. The panel sat degraded
and says so: Opus 5.5 (`118-seat-opus.md`, the code reader, 1.55M tokens), Gemini 3.8 Flash
(`118-seat-gemini.md`, research against the data, 0.88M; agy no longer offers Gemini Pro), and
GPT-6 astra (`118-seat-gpt.md`, the adjudicator, 0.65M). No Grok seat: Cursor is on the free
plan and refuses a named model. Before counting votes the session re-checked the claims the
seats disputed, in the repo: build 288's Outcome (70 verdicts, 7 with a diff, 62 branches
deleted), `crates/puppets/src/write.rs::tasks` (validates class and grade, nothing about ids,
deps or cycles), `integrations/bench/goals/l1-cli.md` (a binary, tests and a README section,
not a single-file edit), and `integrations/puppets/prompts/plan.md` (`paths` may be files or
directories). On each, the adjudicator was right. The decision is the human's; nothing below
is written into the ticket's Answer.

## Where the three agree

- The assignment table stays in code (`sheet.rs`, map 77). No model writes it.
- No retraining per Hydra release. Retrain on a contract change or on a new validated batch.
- Face delegations (26 child records) are the wrong training source; the planning act lives in
  the goal logs of `hydra-puppets`.
- The five bench goals alone cannot prove a planner is not worse.
- The `## Out of scope` line on fine-tuning does not close through this ticket.

## Where they split, and the recommendation

1. **The training example.** One row is a plan proposal: the goal text and the repository
   context the proposer had, pinned to `base_sha`, and the plan JSON it produced. Two labels on
   it, kept apart: whether the panel or human chose it (preference), and what happened to its
   tasks downstream (outcome). Opus said the join exists; GPT read `labels.rs` and found that
   proposers get a null result, so the plan-to-outcome join is a ticket to build, not a fact.
   Gemini wanted only verifier-accepted rows; GPT showed that would exclude research and review
   plans the contract allows. Recommendation: build the join first, train on nothing until the
   rows can be counted, and count them (the goal logs under `~/hydra/puppets/` were unreadable
   to every seat).
2. **What the model outputs.** The plan JSON of `prompts/plan.md`, as a draft the copilot and
   the human review. Unanimous that assignment stays in code. Gemini wanted a skeleton without
   paths; the contract already lets paths be directories or empty, so the existing shape is the
   right target and a skeleton would need a second completion step.
3. **Is 3B credible.** The real split. Gemini: no, budget 8B now. Opus and GPT: measure the
   untuned Ministral 3B on bounded planning with real repo context against a strong planner
   before paying for anything. Recommendation: measure first. The papers in the readings back an
   8B for open planning, but the repo holds no planning measurement of any size, and round one's
   2,048-token window was a T4 choice, not a property of the model.
4. **Who pays and what is the test.** The human, under a cap stated per round. The regression
   gate does not exist and has three parts to build: a static validator of the plan (unique ids,
   deps that resolve, no cycles; `tasks()` checks none of these today), a blinded draft
   comparison on held-out goals, and execution of the bench goals plus a few multi-task goals
   with a non-inferiority margin fixed before the run.
5. **The out-of-scope line.** Keep D12's boundary. A map note can say round one happened as an
   external experiment; the line changes when a corpus exists, and nobody can name its size
   from evidence today (Gemini's 100 is a proposal, not a measurement).

## What would become tickets if the human agrees

- Build: join each plan proposal to its tasks' outcomes in `crates/grader/src/labels.rs`, and
  print the count of rows per label.
- Build: validate a plan's graph in `write.rs::tasks` (ids, deps, cycles) and refuse the import
  that fails.
- Research: untuned Ministral 3B drafting plans for the bench goals with supplied repository
  context, scored blind beside one frontier proposal; the number that decides question 3.
- Map 112 stays the blocker for a live proposer seat; 118 should list it under `Blocked by:`.

Fable was not called as judge: two seats and the session's own checks agree on every question,
and the one split (question 3) resolves into a measurement rather than a vote.
