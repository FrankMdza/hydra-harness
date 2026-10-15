# 117 - Three tiers: a large model orchestrates and reviews, Jev checks the delegation, a small model builds

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-12, by the session, from the human's note of the same day. Grilling: the human
answers, the session only asks and brings numbers.

## Question

The human's proposal, in his words made plain: the largest model is the board, it orchestrates
and reviews; in the middle a Jev checks that the orchestrator's prompt delegates the build
without confirming the task for the builder, that is, the prompt says what to build and why and
leaves the how to the builder; the builder is the smallest model that passes the ticket's tests
(Haiku, Flash, Composer, Grok fast, the local 3B).

What the reading of 2026-10-12 says (`~/cerebro/research/deep-research.md`, section 5, every
claim with its URL): three independent results find these systems planner-limited, not
worker-limited. Reasoning in the orchestrator alone moved GAIA by 18.2 points and reasoning in
the workers moved nothing; Haiku went from 19.7 to 41.2 on BrowseComp with an Opus advisor; cheap
workers keep 92 to 98 percent of quality at 46 to 85 percent less cost, and the loss shows up as
hallucination, which is why the large model's review at the end is not optional.

Today Hydra has the board (`masterofpuppets`, Opus or Fable by memory rule) and the builders
(`vendors.json`, Opus or Grok 4.6 for tasks). The middle tier does not exist. Jev is a language
model read as a classifier (map 109).

1. What exactly does the Jev tier read: the task text the orchestrator hands the builder, the
   whole goal log, or the builder's first turn? And what does it return, a refusal before launch,
   a score in the record, or a rewrite?
2. Where does the check run: in `hydra-puppets` before `hydra run`, as a gate hook on the first
   prompt, or as a judge seat in the panel?
3. Which builder is the default, and what promotes a ticket to a larger builder: a failed test
   run, a Jev score, the ticket's own `Type:`?
4. What is measured to say the tier works: cost per resolved ticket and review rejections per
   ticket, before and after, on `integrations/bench/goals/`?
5. Is this the same thing as map 115 (the harness learns from its use) seen from the launch side,
   or a different ticket?

## Panel

Sat on 2026-10-13, degraded (no Grok seat, Gemini as flash): memos in
`.scratch/hydra/research/117-seat-{opus,gemini,gpt}.md`, readings shared with 118 in
`118-panel-readings.md`, the session's synthesis and recommendation per question in
`117-synthesis.md`. The human decides; nothing is written under Answer by the session.

## Answer

Resolved 2026-10-15 by the human, who took the panel's recommendation as it stands ("sigo
con las decisiones del panel").

1. Jev reads the exact `--task` string `worker_task_text` renders plus the goal text, before
   launch. It returns a score logged on the goal log, never a refusal and never a rewrite. The
   score is declared unvalidated until an offline audit of past delegations, against a written
   rubric (what is required behaviour and acceptance, what is implementation prescribed when it
   need not be), shows it tracks review rejections. Only then does it earn an advisory line in
   the cockpit, before table approval.
2. In `hydra-puppets`, at `tasks` import beside the graph validator of build 299, as a logged
   event. Not a gate hook, which sees tool calls and not task semantics, and not a panel seat.
3. The default builder stays `DEFAULT_ASSIGNMENTS` per task class. A small builder is trialled
   only on bounded tasks whose acceptance check the harness runs itself, the ticket's tests, and a
   ticket moves to a larger builder after a diagnosed failure of that check or of review, never on
   a score alone. `sheet.rs::rank` has to read verdicts across goals before "the smallest model
   that passes" means anything; today each goal starts from the prior.
4. The report came before the tier: build 301 prints first-pass acceptance, eventual acceptance,
   rejections and tokens per task class and per model from the goal logs. The tier is judged on
   those numbers with orchestration, review and the classifier counted as cost, on the five bench
   goals plus the goals already on disk.
5. Not the same as map 115. They share the outcome data and nothing else.

The readings' warning stands: small builders did better with more detail handed down, not less,
so the audit of question 1 settles the premise of the middle tier with Hydra's own rejections.

Tickets: build 327 (`rank` fed verdicts across goals, with the prior stated); map 128 (the
offline audit, scored by Kev once build 294 is in master).
