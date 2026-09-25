# 81 - Jev, and a judge panel whose seats differ even on one model

Type: research
Status: claimed
Blocked by:
Opened: 2026-09-25, by the session, at the human's request, with codex out of its weekly quota
until 2026-09-27 and the judge panel unable to sit.

## Question

Two questions the human asked together.

1. **Jev.** Typesafe AI released Jev on 2026-09-15 in limited early access: a "System 1" model
   by Diego Almeida (ChatGPT and RLHF) that writes no text and returns typed values with a
   probability and a confidence score, trained with RLCD (Reinforcement Learning for Calibrated
   Decisions) on synthetic data. Sources the session found: MindStudio's explainer, TechCrunch
   2026-09-18, Wikipedia "Jev (AI model)". Could Hydra use it, where, and what would it cost?
   The candidates: a pre-filter in front of the judge panel that answers cheaply and escalates
   low confidence to the LLM seats; the router ticket 77 said needs no LLM, a classifier over
   record features; the closed-shape verdicts `parse_judge_answer` reads; and RLCD as a training
   recipe for the 4-bit local model of ticket 77.
2. **Distinct judges on one model.** With codex out, the panel's three seats cannot sit. Can a
   panel stay meaningfully independent when two or three seats run the same model, through
   different evidence, roles, prompts or pre-filters, and what does the literature say about
   correlated errors among LLM judges?

## Constraints any answer has to meet

- Anything that sends a run record or its text off this machine is an export, harness spec 14,
  the one path out. A Jev call over records goes through `hydra export`'s allowlist and
  redaction, or it does not happen.
- Map 76-I: the mode's review panel inherits rung two's rules against shrinking and substitution.
  A pre-filter that replaces a seat is substitution. One that only decides which diffs reach the
  full panel is a different thing, and the answer has to say which it is.
- D5: nothing crosses between the harness and Rungs except the run record and the skill store.

## Answer

(pending, research by `hydra run`)
