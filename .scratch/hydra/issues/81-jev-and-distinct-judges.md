# 81 - Jev, and a judge panel whose seats differ even on one model

Type: research
Status: resolved
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

Resolved 2026-09-26. Research by `composer-2.5` on run 01a0d756-7074, saved as
[research/81](../research/81-jev-and-distinct-judges.md) with the session's check of three
sources. Two decisions with the human through AskUserQuestion.

What the research found. Jev is real and cheap: $0.042 per million input tokens, output free,
typed `noul`, `choice` and `score` answers with a `confidence`, 64k context. Independent
measurements put it well behind good LLMs on hard labels (a median 11.6 macro-F1 points behind the
best per task, on 7,977 human labels), and its calibration varies by dataset. It cannot write a
judge's `note`. RLCD is unpublished, so there is nothing to copy; the usable part for ticket 77's
local model is a proper scoring rule and temperature scaling. On panels: nine frontier judges carry
about 2.18 independent votes (mean error correlation 0.391), and prompt or temperature variants do
not fix that. Different evidence per seat and adversarial roles help some.

- **81-a. A degraded panel, declared.** When a seat's provider is out, the human may approve, per
  goal, a panel whose missing seat is filled by an available provider with a different role (a
  devil's advocate or a security reader) and a different slice of evidence (the diff alone, or the
  diff with its tests). The verdict carries `degraded` in the goal's log and is never reported as a
  full panel. This amends map 76-I for the mode's review panel only; rung two's panel keeps its rule
  against substitution. Every seat gets a distinct role and evidence slice even on a full panel,
  since that costs nothing.
- **81-b. Probe Jev now, on committed diffs.** The human asked why diffs would be public and what
  earlier decision governs them. Harness spec 14 governs run records; a diff already committed to
  master is the author's code, which the four routed vendors already read on every run. Sending it
  to Typesafe adds a fifth vendor, and no record text leaves. The probe uses about 30 build
  tickets' diffs, labelled by their `## Outcome` (applied as written, or changed by the session),
  asks checklist questions as `noul`, and scores calibration against the labels. Jev never votes
  and never replaces a seat. A live pre-filter over uncommitted diffs or records waits for the
  export allowlist.

Build tickets 133 and 134.

Build 134's probe, 2026-09-26: on 25 agent diffs Jev ranked well (AUC 0.964) and calibrated badly
(Brier 0.33 against 0.13 for the base rate). Its Outcome holds the numbers.
