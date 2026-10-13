# Map 117, the empirical research seat (Gemini 3.8 Flash)

Written 2026-10-13 by one seat of the panel on map 117. My mandate is to weigh the research
against what Hydra's goal logs can measure, and define what must exist before each tier can be
empirically judged. Repo citations come strictly from files opened in this worktree via relative
paths. Research citations come from the verbatim excerpts in
.scratch/hydra/research/118-panel-readings.md (section 5 of `deep-research.md`). I did not run
cargo, did not execute tests, and did not read `~/hydra/runs`.

The 118 synthesis settled that the assignment table stays in code (`sheet.rs`), retrainings do
not track releases, face delegations are not training sources, and a static plan validator
belongs in `write.rs::tasks`. I build directly on those decisions without reopening them.

## The Research vs. The Premise: The "Leave the How" Fallacy

Ticket 117 proposes a middle tier (Jev) to verify that the orchestrator "leaves the how to the
builder." The empirical literature contradicts this premise when paired with small builders:
- **Zywot et al. (Jan 2026)** demonstrate that agent systems are "planner-limited rather than
  executor-limited." Worker reasoning added little or hurt (+18.2 on GAIA from orchestrator
  reasoning; worker size 1.7B vs 32B made virtually no difference).
- **MinionS (ICML 2025)** shows that a large cloud model must decompose problems into narrow,
  parallel "chunk jobs"; naive unstructured delegation lost 13% quality.
- **Anthropic's Advisor Strategy (April 2026)** paired Haiku with an Opus advisor, where Opus
  actively directed and guided Haiku to jump from 19.7 to 41.2 on BrowseComp. Haiku succeeded
  because Opus supplied the guidance, not because it withheld the how.
- **Practitioner findings (Arize, 2026)** document that cheap workers double hallucination rates
  when unguided, and delegation succeeded only with the strongest planners.

Withholding the "how" (subtask decomposition, precise files, API constraints, and executable
acceptance criteria) from a small worker (Haiku, Flash, or a local 3B) forces reasoning into the
component least capable of it. If Jev penalizes prompts that prescribe the how, it penalizes the
exact structural scaffolding that makes cheap models viable.

---

## 1. What the Jev tier reads and what it returns

**Recommendation: In production, Jev should read nothing and return nothing today. If built as
an offline experiment, it must read the rendered `worker_task_text` paired with the goal context,
returning an uncalibrated ranking score logged as an event. It must never issue a launch refusal
and never attempt a rewrite.**

- **Evidence from the repo:**
  - `launch.rs:759` defines `worker_task_text`, combining the task's `text`, the `done`
    definition, and a standing request for a "Not confirmed" section. This string is the only
    instruction handed to the builder via `--task` (`launch.rs:50`).
  - `prompts/plan.md` specifically directs the planner to name files/areas and formulate `done`
    as an executable command with expected output. The prompt deliberately asks for concrete
    anchors.
  - Goal logs record verdicts (`applied`, `fixed`, `rejected`), not labels of delegation quality
    or prompt prescriptive depth.
  - Kev/Jev (`109b-kev.md`) operates as an option-scoring ranker over provided candidate choices,
    not a generative rewriter or an uncalibrated binary oracle.
- **Evidence from the readings:**
  - The readings emphasize that delegating to small models requires checkable, narrow boundaries.
  - OpenAI's work on LLM Critics (2024) proves that critic models frequently hallucinate defects;
    an uncalibrated automated gate without an empirical ground truth will reject valid delegations.
- **Argue with Opus:** Opus suggests having Jev output a probability event before table approval
  indicating whether the prompt prescribes steps. This assumes a zero-shot model can output a
  meaningful probability without calibration. Without a labeled dataset or paired alternatives,
  Kev/Jev emits an arbitrary logit. Putting an unvalidated pseudo-probability in front of a human
  creates the illusion of governance without empirical backing.
- **What must exist before this can be judged:** A calibrated evaluation dataset of task prompts
  (prescriptive vs outcome-only) with verified small-model execution outcomes to prove that
  withholding the "how" actually improves, rather than impairs, small-builder pass rates.

---

## 2. Where the check runs

**Recommendation: The check must not run inline in the execution path. It belongs offline as an
analytical audit pass on completed goal logs. It must not run as a gate hook, not as a panel seat,
and not inside `write.rs::tasks`.**

- **Evidence from the repo:**
  - Memory bounds on developer machines are critical. `109b-kev.md` establishes that Kev-0.8B
    requires 3.0 GB resident and 4.5 GB under load, leaving only 1.0 GB swap when a single agent
    runs on an 8 GB Mac. `AGENTS.md` explicitly warns to run at most two agents. Running an
    in-process or loopback model during orchestration risks memory pressure and swap churn.
  - Gate hooks (`src/provider/claude.rs:367`, `src/provider/codex.rs:6`) are `PreToolUse` policy
    enforcers compiled per run; they govern tool execution inside the sandbox, not initial prompt
    ingress.
  - Face delegations (`src/face.rs`, 26 records) bypass `hydra-puppets` entirely.
- **Evidence from the readings:**
  - Mangannavar et al. (2026) achieved high performance with three hardcoded heuristics and a
    targeted QLoRA model, avoiding bloated inline classification pipelines.
  - ToolOrchestra (2025) and SearchSwarm (2026) trained routing and delegation offline from
    execution traces, not by inserting heavy intermediate evaluators at prompt dispatch.
- **Argue with Opus:** Opus argues for placing Jev at `write.rs::tasks` alongside the static
  validator proposed in 118 synthesis. But the 118 static validator checks deterministic graph
  properties (unique ids, resolvable dependencies, acyclicity) that execute in microseconds with
  zero model overhead. Bundling an uncertain, memory-intensive model inference into `write.rs::tasks`
  conflates static compilation with speculative statistical evaluation. If prompt guidance is
  needed, update `plan.md` directly.
- **What must exist before this can be judged:** Benchmarked memory and latency profiles proving
  Kev can run alongside active builders without swap thrashing, and an offline classifier ROC-AUC
  exceeding 0.80 on human-labeled delegation prompts.

---

## 3. The default builder and what promotes a ticket

**Recommendation: Maintain `DEFAULT_ASSIGNMENTS` (`write.rs:36`) as the default. Promotion to a
larger model must be triggered by an automated test failure, never by a Jev score, and not by
relying solely on human review rejections.**

- **Evidence from the repo:**
  - `sheet.rs:333` computes Beta-binomial ranking, but `write.rs:762` passes only the current
    goal's log (`log::read_log`), completely discarding historical performance across goals.
  - Haiku's prior in `priors.json` is [3, 3] (10th percentile 0.245). To clear the `low` threshold
    (0.50), it requires four consecutive first-pass `applied` verdicts in a single goal—a condition
    that almost never occurs in 1-2 task goals.
  - `crates/puppets/` contains no automated test runner for task completion; `done` in `plan.md` is
    optional free text that is never automatically executed before human review.
- **Evidence from the readings:**
  - Arize (2026) reports that cheap models double hallucination rates.
  - Mangannavar et al. (2026) route tasks successfully because their policy was trained strictly
    on "verifier-accepted trajectories."
  - ProgRouter (2026) demonstrates that static routing models break under cost-time constraints.
- **Argue with Opus:** Opus recommends promoting builders upon a human `rejected` or `fixed`
  verdict. Relying on human rejections to drive promotion is economically backwards: human review
  is the most expensive resource in the system. If every small-model hallucination must be caught
  and rejected by a human before promotion, the saved token cost is consumed many times over in
  developer time. Automated verification must run before the human ever sees the diff.
- **What must exist before this can be judged:**
  1. An automated test harness in `crates/puppets/` that executes the task's `done` criterion in
     the worktree before presenting the diff for human verdict.
  2. Cross-goal persistence of Beta verdict distributions in `sheet.rs`.

---

## 4. What is measured to say the tier works

**Recommendation: The tier's viability must be judged on three empirical metrics evaluated
across a new, multi-task benchmark set: (1) Automated Verification Pass Rate, (2) Human Review
Burden (the proportion of tasks requiring human `fixed` or `rejected` verdicts), and (3) Net
Effective Cost per Accepted Task (token/subscription cost adjusted for human intervention). The
five bench goals in `integrations/bench/goals/` cannot measure this tier.**

- **Evidence from the repo:**
  - `integrations/bench/goals/` contains five micro-goals (`l1-cli`, `m1-chars`,
    `m2-saturating-bump`, `s1-readme-typo`, `s2-one-test`). Four of the five explicitly dictate
    the exact code edits, function names, and file paths. They test whether a model can apply a
    dictated patch, not whether a small model can execute an outcome-based delegation.
  - `hydra cost` measures orchestrator-to-agent token ratios per goal, but does not correlate
    tokens with resolution outcomes or human review effort.
  - Goal logs capture `run_end.total_tokens` and `human_verdict`, but track neither wall-clock
    review time nor financial dollar costs across subscription tiers.
- **Evidence from the readings:**
  - The Anthropic Advisor study showed an 85% cost reduction while measuring exact benchmark
    performance on BrowseComp.
  - MinionS achieved 97.9% quality at 5.7x lower cost, but showed that naive unguided execution
    suffered significant degradation.
  - The readings establish that cheap worker failure appears as missed or invented facts rather
    than broken structure. Measuring syntactic completion without measuring human repair rate
    misses the primary failure mode.
- **Argue with Opus:** Opus proposes measuring `total_tokens` per `applied` task from `run_end`
  events. But raw token counts are misleading: frontier tokens and cheap-model tokens have vastly
  different unit costs, and Hydra operates predominantly on flat subscriptions (Cursor, Claude)
  rather than per-token meters. More importantly, Opus omits human repair latency. A small model
  that saves 50,000 tokens but requires 15 minutes of human line-by-line debugging (`fixed`) is
  an operational failure.
- **What must exist before this can be judged:**
  1. A benchmark corpus of 15–20 multi-task goals specified strictly by outcome and acceptance
     criteria, with no implementation hints.
  2. Logging of automated test pass/fail events and human repair timestamps in goal logs.

---

## 5. Same as map 115, or a different ticket

**Recommendation: Different tickets, but bound by a strict consumer-producer dependency. Ticket
117 defines the runtime orchestration topology; ticket 115 provides the offline learning loop.
Tier 3 cannot function dynamically without 115.**

- **Evidence from the repo:**
  - Map 115 governs offline harness self-improvement (`hydra curate`, `hydra evolve`, updating
    skills and provider facts).
  - Map 117 governs the runtime execution pipeline inside `hydra-puppets`.
  - In `sheet.rs`, the assignment table cannot dynamically adapt to builder performance across
    goals because no cross-goal learning mechanism exists.
- **Evidence from the readings:**
  - SearchSwarm (June 2026) demonstrates that delegation policies are trained from historical
    harness traces.
  - ToolOrchestra (Nov 2025) demonstrates that routing to tool/model tiers requires training on
    outcome and efficiency reward signals.
- **Argue with Opus:** Opus characterizes the two tickets as distinct except for sharing a
  verdict metric, and aligns 117 primarily with 118's static plan validator. But mathematically,
  Tier 3 ("the smallest model that passes") cannot exist as a learning system without the
  cross-goal feedback infrastructure of 115. Without 115, Tier 3 is merely a static hardcoded
  table. 117 provides the runtime execution harness that generates the traces 115 learns from,
  and executes the routing policies 115 updates.
- **What must exist before this can be judged:** The plan-to-outcome join in
  `crates/grader/src/labels.rs` (highlighted in 118 synthesis) to allow historical goal logs to
  update builder priors in `sheet.rs`.

---

## Vote

Reject the middle tier (Jev delegation check) because empirical research demonstrates that small
builders require rigorous structural guidance rather than open-ended delegation, making an
uncalibrated classifier both counterproductive and memory-prohibitive. Retain the two-tier
structure (frontier orchestrator/reviewer over specialized builders) and prioritize automated
in-worktree test verification before routing tasks to small models. Defer dynamic model promotion
until a cross-goal verdict store exists and non-inferiority is validated on a multi-task
benchmark.

---

## What I could not verify

- The contents of `~/hydra/puppets/` goal logs, as access to that path is gated for launched runs;
  I could not inspect historical verdict distributions or task complexities directly.
- The empirical inference latency and classification accuracy of Kev-0.8B on task delegation
  prompts, as build 294 has no recorded outcome.
- Real-world failure and hallucination rates of small models (Haiku, local 3B) on Hydra's internal
  Rust tasks versus Sonnet or Grok.
- The external papers cited in `118-panel-readings.md` beyond the verbatim text provided in
  section 5.

---

## Pendiente de mí

- None. The empirical trade-offs, research contradictions, and verification prerequisites are
  documented for the human's decision.

## Cambiado

- Created `.scratch/hydra/research/117-seat-gemini.md`. No other files modified.

## Encontrado

- Empirical research (Zywot et al., MinionS, Anthropic Advisor) proves small workers fail when
  the "how" is withheld; delegating without implementation structure is a primary failure mode.
- Kev-0.8B consumes 3.0–4.5 GB RAM (`109b-kev.md`), leaving inadequate headroom on an 8 GB host
  with a live agent, making an inline classification tier a stability risk.
- Goal logs lack automated execution checks for `done`, forcing human review to act as the sole
  filter for small-builder hallucinations.
- `sheet.rs` cannot learn small-builder competence because `write.rs:762` isolates verdicts to
  single goals, resetting priors on every goal run.
- The five goals in `integrations/bench/goals/` dictate implementation details explicitly, making
  them incapable of testing delegation boundaries.
