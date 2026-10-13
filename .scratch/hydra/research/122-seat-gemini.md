# 122, seat Gemini: whose plan was it when the panel composed one

Written 2026-10-13 by the Gemini seat of the four-seat panel on map 122. My role is to evaluate the three readings against the current codebase, what has been measured in this worktree, and the constraints of the training pipeline. Every claim about Hydra names the relative file and line I read it in.

## 1. When the panel composes, is any proposal credited?

**What the code does today:**
- In `crates/grader/src/labels.rs:475-481`, `picked` evaluates whether a verdict chose a specific plan:
  ```rust
  fn picked(plan: &Value, verdict: &Value, named: &Value) -> Result<bool> {
      let path = py::get(verdict, "path", Value::Null)?;
      if py::truthy(&path) {
          return Ok(py::eq(&py::get(plan, "path", Value::Null)?, &path));
      }
      Ok(py::eq(&py::get(plan, "provider", Value::Null)?, named))
  }
  ```
  When the panel chooses `composed`, `choose_judge` writes the task array to `plans/composed.md` and records that path in `panel_verdict` (`crates/puppets/src/panel.rs:989-998`). Because each proposer has its own plan path (e.g. `plans/claude.md` or `plans/composer-2.5.md`), `picked` compares `plan.path` against `verdict.path` and returns `false` for every proposer.
- In `crates/grader/src/labels.rs:554-570` (`labels_for_goal`), each proposal in `shown.state.plans` is evaluated against `picked`. Because `picked` returns `false`, `chosen` is set to `Value::Bool(false)`. The outcome is then set via:
  ```rust
  let ended = match chosen {
      Value::Bool(true) => outcome.clone(),
      _ => Value::Null,
  };
  ```
  Consequently, every proposer of a composed goal receives `chosen: false` and `outcome: null`.
- In `crates/grader/src/labels.rs:763-786`, unit test `chosen_is_null_without_a_verdict_and_false_for_every_proposer_of_a_composed_plan` explicitly asserts this behavior: `assert_eq!(chosen(r#"{"chosen":"composed","path":"/h/g/plans/composed.md"}"#), ["False", "False"])`.
- In `crates/puppets/src/write.rs:798-842`, `tasks` reads `plans/composed.md`, validates the task graph, and writes each task as an event to the goal log (`id`, `text`, `class`, `grade`, `done`, `deps`, `paths`). It records zero proposer provenance or task attribution.
- In `IMPLEMENTATION.md:4140-4143`, this behavior is documented: "A composed plan picks none, so every proposer of a composed goal is `false`. `outcome` is `null` unless `chosen` is `true`."

**What I measured or verified in this worktree against what I believe:**
- In `crates/puppets/tests/grader_labels.rs:175-179`, running the fixture comparison test (`cargo test -p hydra-puppets --test grader_labels -- --nocapture`) verifies 147 goal logs with 291 labels; across all 147 fixture logs, proposers are split: 4 chosen and 8 not chosen (12 proposer rows total).
- In `.scratch/hydra/build/303-a-plan-proposal-row-carries-what-became-of-its-tasks.md:88-100`, running `./target/release/hydra-grader labels count` on this machine's puppets home found 7 proposer rows: 1 chosen, 6 not chosen, 0 no verdict.
- Goal `a598914f` chose Claude's proposal: `outcome` carries applied 0, fixed 0, rejected 0, pending 5.
- Goal `ea69da1f` chose `composed`: its three proposers are all `chosen: false`, `outcome: null`. Its six completed task verdicts (3 applied, 3 fixed, verified in `docs/reviews/2026-09-24-master-of-puppets-first-goal.md:18`) land on no proposer row. Today there is exactly 1 row on this machine labelled `chosen: true`, and its outcome is all pending.
- In `docs/reviews/2026-09-24-master-of-puppets-first-goal.md:42-44`, the review shows that on `ea69da1f`, Codex (`01a0d57e-4cf7`) was rate-limited at launch, Composer 2.5 (`01a0d57e-52e4`) completed ("the plan the composed one came from"), and Gemini (`01a0d57e-8292`) completed a 20k token draft. The panel chose `composed`, not Composer directly.
- Comparing the three readings:
  1. "All, with a share": Tasks interact non-linearly. Attributing fractional downstream execution success to proposals whose full plans were rejected assumes linear task independence that does not hold in software engineering. Furthermore, `write.rs` records no task origin today.
  2. "The nearest": Highly subjective and fragile. If the panel chose `composed` instead of adopting Composer 2.5 directly, it did so because the draft required edits or synthesis. Crediting the nearest proposal as if it won erases the panel's critical contribution.
  3. "None": Accurate to authorship. A composed plan is synthesized by the panel. The individual proposers only drafted candidate material that was not accepted as a whole.
- What I believe: Assigning unearned outcome credit to rejected proposals corrupts training labels. Credit none.

**Recommendation:**
Credit none: a composed plan is synthesized by the panel rather than authored by any single proposer, and assigning downstream execution outcomes to rejected or altered proposals corrupts the training signal. Keep composed goals out of proposal outcome labels until an explicit, verified provenance mechanism is justified by data.

**What would change my mind:**
Evidence from goal logs showing that a clear majority (more than 50%) of multi-seat planning panels choose `composed`, which would starve the outcome training corpus if uncredited, combined with a deterministic task provenance log proving tasks survived verbatim from specific proposals without panel modification.

## 2. If the answer is a share or the nearest, what does the panel write at composition time so the join can read it later?

**What the code does today:**
- In `crates/puppets/src/panel.rs:714-721`, `build_choose_seat_prompt` formats instructions for the judge seats:
  ```text
  ## Instructions

  Pick exactly one proposal label (<provider_names>) or `composed` when no single proposal is best.

  Answer in exactly one JSON object and nothing else:

  ```json
  {"chosen": "<label>|composed", "findings": []}
  ```
  ```
- In `crates/puppets/src/panel.rs:555-568`, `composed_plan` extracts a fenced JSON task array from the seat's response via `write::fenced_array`.
- In `crates/puppets/src/panel.rs:637-640`, `plan_panel_verdict` takes the first non-empty `composed_plan` among the answers (`answers.iter().find_map(|a| a.composed_plan.clone())`).
- In `crates/puppets/src/panel.rs:989-998`, `choose_judge` writes that task array to `plans/composed.md` as `format!("Composed plan.\n\n```json\n{plan}\n```\n")`.
- In `integrations/puppets/prompts/choose.md:24-38`, the reference prompt schema for composed tasks specifies:
  `[{"id": "short-id", "text": "...", "class": "...", "grade": "...", "deps": [], "paths": [...]}]`.
- In `crates/puppets/src/write.rs:662`, `TASK_KEYS` requires `["id", "text", "class", "grade"]`.
- In `crates/puppets/src/write.rs:826-840`, `tasks` appends each task to the goal log with fields `id`, `text`, `class`, `grade`, `done`, `deps`, `paths`.
- In `crates/grader/src/labels.rs:507`, `labels_for_goal(shown: &Shown, events: &[Value])` reads only `Shown` (from `hydra-puppets show`) and the goal log `events`. It never opens or reads markdown plan files from disk.

**What I measured or verified in this worktree against what I believe:**
- In `crates/puppets/src/panel.rs:741-753` and `build 252` (`.scratch/hydra/build/252-the-plan-panel-judges-blind-in-turned-order-and-a-split-goes-to-two-vendors.md`), proposals are shown blind to judge seats (`Proposal A`, `Proposal B`) in turned order per seat. The mapping to providers lives only in `panel_verdict.label_map`. If a judge were asked to annotate tasks with source proposal IDs, it would have to emit blind labels like `"source": "Proposal A"`, which `choose_judge` would then have to translate back to real proposal IDs or provider names.
- In `.scratch/hydra/issues/121-an-untuned-3b-drafting-plans-for-the-bench-goals.md:82-87`, static validation of 15 drafts from an untuned 3B model found that 4 of 15 (26.7%) already failed basic JSON parsing due to unescaped line breaks and inline comments. Asking models during `choose --judge` to output nested provenance metadata increases prompt complexity and the likelihood of malformed JSON or hallucinated proposal IDs.
- In `crates/grader/src/labels.rs:507-520`, `labels_for_goal` has no filesystem access to `plans/composed.md`. If source IDs were stored only in `plans/composed.md`, `labels.rs` could not see them without breaking its contract and reading files outside `show`. Thus, source proposal IDs must be written to the goal log's `task` events by `write.rs:tasks` (or to `panel_verdict`), which alters the goal log contract and `fold.rs`.
- In `crates/puppets/src/panel.rs:637-640`, `plan_panel_verdict` currently does not synthesize or merge plans across judges: it simply grabs the first judge seat's composed plan. If multiple judges compose different plans, only one judge's composition is saved, and its internal attribution may be biased.

**Recommendation:**
If task-level attribution is ever required, do not alter LLM prompts to emit source proposal IDs; compute task attribution deterministically in code by matching task text and paths against candidate proposals, and write the resulting provenance mapping directly into the goal log's `panel_verdict` event.

**What would change my mind:**
Empirical measurements on a corpus of test goals proving that frontier models acting as judges reliably emit accurate, valid JSON source annotations for every composed task without hallucinating proposal IDs or dropping required fields.

## 3. Does a false proposer row teach anything?

**What the code does today:**
- In `crates/grader/src/labels.rs:546-569`:
  Build 303 decoupled preference from outcome:
  ```rust
  let ended = match chosen {
      Value::Bool(true) => outcome.clone(),
      _ => Value::Null,
  };
  ```
  When `chosen` is `false`, `outcome` is strictly `Value::Null`.
- In `crates/grader/src/collect.rs:360-398` (`dataset_row`), `dataset_row` currently selects `task_class`, `grade`, and `human_verdict` by name. Neither `chosen` nor `outcome` is exported to `dataset.jsonl` yet (`IMPLEMENTATION.md:4145`).
- In `.scratch/hydra/issues/118-a-specialist-orchestrator-not-a-generalist.md` and `118-synthesis.md:26-30`, the design keeps preference (`chosen`) and outcome (`outcome`) separate so that rejected proposals can serve as preference examples (`chosen: false`).

**What I measured or verified in this worktree against what I believe:**
- In preference optimization (DPO, Bradley-Terry, pairwise ranking), a training pair requires a preferred completion $y_w$ and a dispreferred completion $y_l$ for the same prompt context $x$: $(x, y_w, y_l)$.
- In goals where a single proposal was chosen (e.g. goal `a598914f`, which chose Claude over the other proposals), the winning proposal is `chosen: true` ($y_w$) and the remaining proposals are `chosen: false` ($y_l$). Here, `chosen: false` rows are essential negative examples that form valid preference pairs, even while downstream task outcomes are pending or null.
- In a composed goal (e.g. `ea69da1f`), every proposer is `chosen: false`. There is no `chosen: true` proposal among the candidates.
- Across the 7 proposer rows on this machine's puppets home (from `build 303 Outcome`):
  - Goal `a598914f`: 1 `chosen: true`, 3 `chosen: false`, generating 3 pairwise preference pairs (with pending outcome).
  - Goal `ea69da1f`: 0 `chosen: true`, 3 `chosen: false`, generating 0 intra-goal pairwise preference pairs.
- If all candidate proposals are `chosen: false`, they cannot form a standard pairwise preference pair unless the composed plan itself (authored by the panel) is injected as the winning completion $y_w$, which mixes model populations and authors.
- For outcome prediction (RL or value functions predicting downstream success), a `chosen: false` row with `outcome: null` provides zero outcome signal because its execution path was never taken. Counterfactual outcomes are unobserved.
- Therefore, question 1 matters decisively: if composed goals produce only `chosen: false` rows with null outcomes and no winning proposal, they yield zero outcome training rows and zero intra-goal preference pairs.

**Recommendation:**
A `chosen: false` row provides a valid negative contrastive example for preference learning only when paired with a winning `chosen: true` proposal for the same goal, but teaches nothing for downstream outcome prediction or on composed goals where all proposals are false.

**What would change my mind:**
Adopting an unreferenced, negative-only training objective (such as unlikelihood loss or non-paired KTO) where standalone rejected plan proposals measurably reduce planning errors without requiring a paired winning plan.

## Recommendation summary

1. Credit none; a composed plan is synthesized by the panel rather than authored by any single proposer, and assigning unearned outcome credit to rejected proposals corrupts the training distribution.
2. If task-level attribution is ever required, compute it deterministically in code via text and path matching rather than complicating the judge prompt, and record the mapping directly in the goal log.
3. A `chosen: false` row provides a valid negative preference example only when paired against a winning proposal in the same goal, and teaches nothing for outcome prediction or on composed goals where all proposals are false.
