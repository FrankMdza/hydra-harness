# 118 - The harness's own model is trained as an architect and orchestrator, not a generalist

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-12, by the session, from the human's note of the same day. Grilling: the human
answers, the session only asks and brings what was read.

## Question

The human's proposal: round two of the training (map 88, 111, 112, `.scratch/hydra/training/`)
does not fine-tune the small model to build; it fine-tunes it to plan and delegate. One model per
version of the harness, or one that follows it; undecided.

What the reading of 2026-10-12 found (`~/cerebro/research/deep-research.md`, section 8):
ToolOrchestra trains an 8B model to orchestrate tools and larger models and reports 37.1 on HLE
against 35.1 for GPT-5 at 30 percent of the cost; Training the Orchestrator learns a QLoRA policy
at 0.004 USD per task against 0.18; SearchSwarm turns harness-guided delegation trajectories into
SFT data; LIMI reaches agentic behaviour with 78 samples. All of them train on traces of
delegation, not of building.

1. What is the training example: a goal log and its assignment table (what `hydra-puppets`
   already writes), a run record where a parent delegated to children (`face.rs`), or the pair of
   task text and panel verdict? Hydra has 826 runs and 70 verdicts; how many are delegations?
2. What does the model output at inference: the assignment table, the task text per builder, or
   the plan the human approves in the cockpit? That decides whether it replaces the copilot's
   planning turn or only drafts it.
3. Ministral 3B is the default candidate for the harness's own model (map 112, still a grilling).
   Is a 3B planner credible given that the same reading says the planner is where quality lives,
   or does this change the size and the budget of round two?
4. One model per Hydra version means a training run per release. Who pays, how much (round one
   was 0.25 USD on a spot T4), and what is the test that says the new one is not worse?
5. Does this ticket close the "Fine-tuning" line under `## Out of scope` in the map, which the
   training directory already contradicts?

## Panel

Sat on 2026-10-13, degraded (no Grok seat, Gemini as flash): memos in
`.scratch/hydra/research/118-seat-{opus,gemini,gpt}.md`, readings in `118-panel-readings.md`,
the session's synthesis and recommendation per question in `118-synthesis.md`. The human
decides; nothing is written under Answer by the session.

## Answer

Resolved 2026-10-15 by the human, who took the panel's recommendation and map 121's
measurement ("sigo con las decisiones del panel").

1. The training example is one plan proposal: the goal text, the repository context the proposer
   had at `base_sha`, and the plan JSON it produced, with two labels kept apart, preference
   (chosen or not) and outcome (what became of its tasks). Build 303 built the join. On this
   machine it counts 7 proposer rows, 1 chosen, and the one goal with outcomes was not a panel
   composition (map 122). Nothing is trained until the rows can be counted in the hundreds.
2. The model outputs the plan JSON of `prompts/plan.md`, a draft the copilot and the human
   review. Assignment stays in `sheet.rs`; no model writes the table.
3. Map 121 measured the untuned Ministral 3B against Opus 5.5 on the five bench goals, scored
   blind: median 3 of 6 under Flash and 4 of 6 under Astra against 6 for Opus, and 5 of 15 drafts
   refused by the plan validator before anyone read them. An untuned 3B is not a credible planner.
   Round two does not start on a 3B planner now; the size and the budget are decided when the
   corpus of question 1 exists and says how large the gap to close is.
4. The human pays, under a cap stated per round. The regression gate has three parts: the static
   validator (builds 299 and 307, done), a blinded draft comparison on held-out goals (map 121's
   procedure, reusable as it is), and the bench goals plus a few multi-task goals with a
   non-inferiority margin fixed before the run.
5. D12's boundary stays. The map notes under `## Out of scope` that round one happened as an
   external experiment; the line changes when a corpus exists. Map 112 stays the blocker for a
   live proposer seat.

No new build ticket from this answer: the join, the validator and the measurement already
closed as builds 303, 299 and 307 and map 121.

Map 121 (2026-10-13, the session) measured question 3: untuned Ministral 3B against Opus 5.5 on the five bench goals, blind judged. The 3B ties only on the one-line typo and fails the JSON format on 4 of 15; the table and the quotes are under `## Findings` of `121-an-untuned-3b-drafting-plans-for-the-bench-goals.md`.
