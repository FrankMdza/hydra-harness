# 117 - Three tiers: a large model orchestrates and reviews, Jev checks the delegation, a small model builds

Type: grilling
Status: open
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

## Answer
