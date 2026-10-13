# 121 - An untuned 3B drafting plans for the five bench goals, scored blind beside a frontier draft

Type: research
Status: resolved
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

Measured 2026-10-13 by the session. Everything under `research/121-drafts/`: `prompts/` is the
exact text both proposers answered (the goal, the three-file tree and the files, 2.9 to 3.2 KB),
`ministral/` and `opus/` the 15 drafts each with Ministral's timing beside each draft, `blind/`
the shuffled copies the judge saw, `blind-scores.md` the judge's table, `blind-map.json` the
key. The static check is the session's copy of the rules `write.rs::tasks` applies at import
(fenced array, four keys, known class and grade, unique ids, deps that resolve, no cycles).

| Goal | Proposer | Static offences, median (range) | Rubric of 6, Flash seat, median (range) | Rubric of 6, GPT seat, median (range) |
|---|---|---|---|---|
| s1-readme-typo | Ministral 3B | 0 (0 to 0) | 6 (6 to 6) | 6 (6 to 6) |
| s1-readme-typo | Opus 5.5 | 0 (0 to 0) | 6 (6 to 6) | 6 (6 to 6) |
| s2-one-test | Ministral 3B | 0 (0 to 1) | 4 (1 to 4) | 6 (2 to 6) |
| s2-one-test | Opus 5.5 | 0 (0 to 0) | 6 (6 to 6) | 6 (6 to 6) |
| m1-chars | Ministral 3B | 1 (0 to 1) | 2 (2 to 3) | 2 (2 to 4) |
| m1-chars | Opus 5.5 | 0 (0 to 0) | 6 (6 to 6) | 6 (5 to 6) |
| m2-saturating-bump | Ministral 3B | 0 (0 to 0) | 3 (3 to 3) | 4 (4 to 5) |
| m2-saturating-bump | Opus 5.5 | 0 (0 to 0) | 6 (6 to 6) | 6 (6 to 6) |
| l1-cli | Ministral 3B | 1 (0 to 3) | 3 (1 to 3) | 4 (1 to 4) |
| l1-cli | Opus 5.5 | 0 (0 to 0) | 6 (6 to 6) | 6 (5 to 6) |

Over all 15: Ministral passes the static check on 10 and scores a median 3 of 6 with the Flash
seat and 4 with the GPT seat; Opus passes 15 and scores 6 on every draft with Flash and a mean
of 5.9 with GPT. Both judges matched every blind draft to its goal. The second seat was added
after the first table was written, `blind-scores-gpt.md`, `gpt-6-astra` on `codex`, same
rubric, same blind files, told not to read the first table; 3 of 30 rows differ by more than
one point between the seats, all three Ministral drafts the GPT seat read more kindly.
The only goal where the 3B ties is the one-line typo. On the two mediums and the large it
loses on every draft, and the spread inside one goal (1 to 4 on `s2`, 1 to 3 on `l1`) says
three samples of the 3B are three different plans.

The three things that went wrong most often in the 3B's drafts, quoted:

1. JSON that is not JSON, 4 of 15. Three drafts put a raw line break inside a string value
   (`"text": "Add tests for `chars` in `src/lib.rs` for the following cases:` and the list
   on the next lines, `m1-chars-1`), and one put a comment after a value
   (`"paths": ["tests/"]  // Assume tests are added here.`, `l1-cli-3`). `write.rs::tasks`
   refuses all four at import, so a planner seat on this model would fail one goal in four
   before anyone read the plan.
2. Tasks that are not work, 5 of 15. A `research` task to read the function the goal
   already describes (`"text": "Analyze the current `bump` function to confirm it uses
   `wrapping_add` and understand the wrapping behavior."`, `"done": "Confirmed that `bump`
   uses `wrapping_add` and wraps at 255."`, `m2-saturating-bump-1`), and a last task that
   only runs the tests the previous task's `done` already ran (`"text": "Run `cargo test` to
   ensure the new test passes and all existing tests continue to pass."`, `s2-one-test-1`).
   The judge marked these as over-split with untestable `done`.
3. Dependencies that do not resolve or point backwards, 2 of 15. `l1-cli-2` names its tasks
   `add-cli-entry-point` and `add_chars_to_library` and then depends on `add-chars-to-library`
   and `json-output-in-cli`, three unknown deps from mixing hyphens and underscores; the
   judge read the other large draft's deps as inverted (the CLI before the library function it
   calls), which builds nothing in isolation.

Cost and time. Ministral on this machine through Ollama: 21 to 85 s per draft, prompt 1,350 to
1,472 tokens, output 216 to 1,020 tokens, 3 GB of unified memory while loaded, run with no
other model up. Opus: fifteen recorded runs of about 30 s each. The judge, one Gemini
3.8 Flash seat on `agy`, read-only: 394k tokens.

What this does not say. The rubric has a ceiling and Opus sits on it, so the measurement
cannot rank frontier planners, only say the untuned 3B is not one. Two judge seats, not three, and they moved
three rows and no conclusion. The bench goals are small by design, and
the 3B already fails the format on them, so a harder set would not help it. Untuned means
untuned: a fine-tune on plan rows is exactly what map 118 is deciding, and the first reason
there are no plan rows is map 122.
