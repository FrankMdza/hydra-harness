# 121 - An untuned 3B drafting plans for the five bench goals, scored blind beside a frontier draft

Type: research
Status: open
Blocked by:
Opened: 2026-10-13, by the session, from the map 118 panel (`118-synthesis.md`, question 3),
where the split between the seats (Gemini: budget an 8B now; Opus and GPT: measure the 3B
first) resolved into a measurement rather than a vote. This ticket is that measurement. It
decides nothing: the human reads the numbers and answers 118.

## Question

Can `ministral-3:3b`, untuned, with the repository context a proposer would be handed, draft a
plan for a bounded goal that a blind reader cannot tell from a frontier model's draft, or that
scores within a stated margin of it on the validator and on a rubric?

## Method

1. Goals: the five under `integrations/bench/goals/`, against a fresh copy of
   `integrations/bench/repo/` each. Context handed to both proposers is the same string: the
   goal text, the repository tree, and every file under twelve lines, which is the whole
   `tally` crate.
2. Prompt: `integrations/puppets/prompts/plan.md` as it is, with `{{GOAL}}` replaced. No
   prompt tuning for the small model; if it cannot follow the JSON shape as written, that is a
   finding, not something to patch around.
3. Proposers: `ministral-3:3b` through local Ollama (`ollama serve` by hand, stopped after),
   and one frontier draft per goal from `claude` on Opus 5.5 through a recorded `hydra run`.
   Three drafts per goal from each, temperature as each default ships.
4. Scoring, two parts kept apart:
   - Static: `hydra-puppets`'s plan validator from build 299 (`task_graph_offences`): parse
     failure, duplicate ids, unresolved deps, cycles, and the count of tasks with an empty
     `done`. A number per draft.
   - Blind rubric: thirty drafts shuffled and renamed, scored by one judge seat (Gemini
     `gemini-3.8-flash-high` on `agy`, read-only) on three questions from 0 to 2: would each
     task run in isolation, does each `done` name a check a reviewer could run, does the plan
     cover the goal with nothing missing and nothing extra. The judge never sees which model
     drafted.
5. Report: a table per goal, both proposers, median static offences and median rubric score,
   with the spread; the drafts themselves under `research/121-drafts/`; and the three things
   that most often went wrong in the 3B's drafts, quoted.

## What this is not

Not a decision on 118's question 3, not a fine-tune, not a change to any code under `src/` or
`crates/`. Map 112 (where the local model's URL and name live) is still the human's. RAM: the
3B needs about 3 GB and the machine has 8; run this alone, with no Hydra build in flight.

## Findings
