# Map 118, seat gemini: weighing research against Hydra's actual data

Written 2026-10-13 by the gemini seat of the four-seat panel on map 118. My role is to weigh the research against the data Hydra actually has, and state what a round two needs before it can be judged. Every claim about Hydra names the relative file I read it in. Literature citations come exclusively from the quotes in `.scratch/hydra/research/118-panel-readings.md`. I argue directly with seat opus (`.scratch/hydra/research/118-seat-opus.md`): opus mistakes conversational approval for verified training data, proposes competing a 3B model head-to-head with frontier planners, relies on an untested five-goal toy bench, and prematurely formalizes fine-tuning.

---

## 1. What the training example is

**Recommendation: The training example cannot be a standalone proposal draft or prompt acceptance. It must be an end-to-end verifier-accepted execution tuple: `(goal + repo summary, plan JSON)` paired strictly with downstream task success and surviving diffs. Do not train on raw proposal preference pairs or unverified goal logs.**

**What I read in the repo:**
- Hydra has 866 run records, but only 26 carry a `parent_run_id` from the delegation face (`.scratch/hydra/research/118-panel-readings.md`).
- There are 70 verdicts referenced in goal logs, but build 288 (`.scratch/hydra/research/118-seat-opus.md`, build 288 outcome) found that only 7 of 70 verdicts preserved a branch with code diffs; 62 were deleted with their commits.
- Round 1 filtered 351 runs down to only 24 usable trajectories because tasks did not cross into exports (build 290), and build 290 item 4 remains open (`.scratch/hydra/training/round1/README.md`, `.scratch/hydra/research/118-seat-opus.md`).
- Goal logs in `hydra-puppets` record `plan_proposed`, `panel_verdict`, and `human_plan`, but these record conversational or judging approvals, not whether the resulting builder executions succeeded.

**What the readings quote from papers:**
- *Training the Orchestrator* (Mangannavar et al., 2026) trains a QLoRA policy *only on verifier-accepted trajectories*.
- *ToolOrchestra* (2025) trains an 8B orchestrator using RL grounded in *outcome rewards and efficiency*, not intermediate approval.
- *SearchSwarm* (2026) mines delegation traces from harness runs filtered by verifiers.
- *LIMI* (2025) achieves agency with 78 samples only because each sample is a *complete, curated trajectory*.

**Argument with seat opus & what Round Two needs:**
Opus argues that "one example is one plan proposal" labeled by whether the panel or human chose it, claiming that goal logs provide DPO pairs. This is flawed: training on plans solely because a panel or human picked them at the drafting stage—when 62 of 70 verdicts ended in discarded diffs—trains the model to emit persuasive hallucinations rather than executable code. Furthermore, a plan cannot output accurate `paths` from a bare goal text without repo context. Round Two cannot be judged until the training example is defined as a plan that led to a *verifier-accepted, tested diff*, and until Hydra accumulates at least 50 such complete trajectories.

---

## 2. What the model outputs at inference

**Recommendation: The model should output a structured task-decomposition draft with class/grade metadata, but NOT full filepaths or autonomous plans competing as an unassisted peer in `choose --judge`. The assignment table stays in deterministic code (`rank`).**

**What I read in the repo:**
- The plan format (`prompts/plan.md`) demands exact `paths`, `deps`, `class`, `grade`, and task `text`.
- The assignment table is computed in `crates/puppets/src/write.rs` via a Bayesian beta posterior (`crates/puppets/src/sheet.rs`), falling back to `source: default`. Map 77 explicitly established that assignment routing does not need a generative model.
- Local models currently have no harness driver to inspect the repository (map 88 is unbuilt, map 112 is open).

**What the readings quote from papers:**
- *Zywot et al.* (2026) demonstrate that multi-agent systems are "planner-limited rather than executor-limited," showing that orchestrator reasoning capacity added 18.2 points on GAIA while worker size barely mattered.
- *Anthropic advisor strategy* and *Fable 5 / Sonnet 5* architectures place large frontier models in the planning/advising seat and cheap models in the execution seat—the exact inverse of putting a small model as the master planner.
- *AOrchestra* (2026) dynamically constructs sub-agents as `(instruction, context, tools, model)` tuples.
- *Training the Orchestrator* uses a small policy only to select discrete repair agents with three hardcoded rules.

**Argument with seat opus & what Round Two needs:**
Opus recommends having the small model emit the full `prompts/plan.md` JSON block to sit as an anonymous proposal seat beside frontier models in `choose --judge`. This ignores the core finding of Zywot et al.: reasoning lives in the planner. A small model with no repo access and limited context cannot reliably infer exact filepaths and non-cyclic dependencies. Entering it into `choose --judge` against frontier models guarantees near-100% rejection. Round Two needs the small model to output *intermediate drafting artifacts*—such as subtask breakdown skeletons or task `class`/`grade` tags to assist the copilot and `rank`—rather than pretending it can act as the sole architect.

---
## 3. Whether a 3B planner is credible

**Recommendation: A 3B model is NOT credible as an architectural planner, and waiting for it to fail in Round Two is a waste of resources. If Round Two trains an orchestrator, it must budget for an 8B model. If Round Two must use Ministral 3B, its scope must be reduced to discrete routing or metadata tagging.**

**What I read in the repo:**
- Round 1 on Ministral 3B suffered an out-of-memory crash during the backward pass at 4,096 tokens on a spot T4 GPU because fp32 vocabulary projections consume 2 GB alone (`.scratch/hydra/training/round1/README.md`).
- Round 1 trained at a 2,048-token window, forcing prompts to be cut to 2,000 characters and tool output to 600 characters.
- Round 1's measured gain (held-out next call 1/9 to 5/9) was entirely an artifact: an 8/9 collapse into a `Bash` prior. The README explicitly concludes: "Nothing here says the model got better at the work."
- Map 111 only tested Ministral 3B on single-file tool calls (10/10 first calls), never on multi-step architectural planning.
- The host machine is an 8 GB M2 where Ministral 3B consumes 2.7 GB at 4k context.

**What the readings quote from papers:**
- *Zywot et al.* (2026) report orchestrator wins only with an 8B model; worker size did not matter, but planner reasoning was critical.
- *ToolOrchestra* (2025) utilizes an 8B model trained with RL.
- The only paper achieving success with a smaller policy (*Training the Orchestrator*) used it for a 3-agent discrete selection problem with hardcoded rules, not open-ended generative planning.

**Argument with seat opus & what Round Two needs:**
Opus recommends keeping Ministral 3B for Round Two as a drafter and only budgeting an 8B arm "if it misses." This ignores the physics of the problem: a planning prompt requires repository structure, ticket context, and formatting instructions that cannot fit into Round 1's 2,000-character truncation ceiling. Round 1 already proved that Ministral 3B degenerates into a prior when context is starved. Round Two cannot be judged on 3B planning; it requires testing an 8B model (e.g., Qwen 2.5/3 8B or Llama 3.1 8B) on a 24 GB GPU (L4/A10G) with at least an 8k context window, or strictly confining 3B to narrow routing decisions.

---

## 4. Who pays per version, and the test that says it is not worse

**Recommendation: Do not tie retraining to Hydra releases or prompt edits. Retrain only when a validated batch of at least 50–100 verifier-accepted execution traces has accumulated. The human's GCP project pays under a $10 cap on an L4/A10G GPU (~$3–$5 per run). The regression test must be a two-stage evaluation: static structural validation followed by execution verification on composite multi-task problems, not the toy five-goal bench.**

**What I read in the repo:**
- Round 1 cost an estimated 0.25 USD on a spot T4 in GCP project `<gcp-project>` across 65 minutes, but its bill was never read (`.scratch/hydra/training/round1/README.md`).
- The bench (`integrations/bench/goals/`: `l1-cli`, `m1-chars`, `m2-saturating-bump`, `s1-readme-typo`, `s2-one-test`) consists of five trivial single-file edits against the `tally` crate.
- Build 274 notes that running the bench "is the session's and has not started." Furthermore, `src/cost.rs` measures orchestrator invocation cost, not plan quality.

**What the readings quote from papers:**
- *Plan-and-Act* (2025) separates planner and executor evaluation, validating on multi-step environments (WebArena, WebVoyager).
- *ProgRouter* (2026) demonstrates that routing trade-offs vary under real execution constraints.

**Argument with seat opus & what Round Two needs:**
Opus suggests retraining whenever the plan contract (`prompts/plan.md`) changes and using the five bench goals as an end-to-end regression test. This is impractical on two fronts:
1. Retraining immediately upon contract changes guarantees zero training examples under the new format.
2. The five bench goals are trivial single-file edits that do not require decomposition. Evaluating an *architect and orchestrator* on tasks that require no orchestration cannot prove the model is not worse.
Round Two needs a regression suite of at least 15–20 multi-task goals requiring dependency graphs and multi-file modifications, scored first by static DAG/path validity and second by the pass rate of downstream worker execution.

---
## 5. Whether this ticket closes the "Fine-tuning" line under ## Out of scope

**Recommendation: No. Ticket 118 should NOT close the "Fine-tuning" line in `map.md`. It should preserve D12's boundary and keep fine-tuning designated as out-of-scope for the core harness engine until the data collection pipeline produces a viable corpus.**

**What I read in the repo:**
- `.scratch/hydra/map.md` line 214 (D12): "Training is out of scope. Lossless export to SFT and DPO formats stays a constraint on the record."
- Line 413 under `## Out of scope`: "Fine-tuning. Axolotl, Unsloth, preference-optimisation training runs, any weights. Its own effort once there is a corpus worth training on."
- Round 1 ran an experimental fine-tune on 2026-10-04, but only 24 trajectories survived out of 351 runs, and only 7 verdicts retain code diffs across the entire project history.

**What the readings quote from papers:**
- *LIMI* requires 78 *complete, curated* trajectories.
- All effective orchestrator papers rely on rigorous verification traces.

**Argument with seat opus & what Round Two needs:**
Opus argues for closing the line via a rewrite that formalizes QLoRA in `.scratch/hydra/training/`. This is premature. Line 413 explicitly stipulated: "Its own effort once there is a corpus worth training on." Hydra does not yet possess that corpus: face delegations number 26, surviving diffs number 7, and task export preconditions remain open (build 290). Closing the line now signals that fine-tuning is an active core development focus, diverting engineering away from solving harness execution (map 88, 112) and verifiable data collection. Fine-tuning should remain out of scope in `map.md` until the corpus threshold (>=100 verifier-accepted execution runs) is demonstrably achieved.

---

## Vote

Do not train a 3B model as an unassisted architect on unverified proposal logs, because Hydra currently has only 7 surviving diffs and the research demonstrates that orchestrator quality requires at least 8B and verifier-accepted traces. Round Two must test whether a small model can perform bounded subtask drafting or metadata routing within a realistic context budget, backed by an 8B comparison arm and evaluated on multi-step composite goals rather than the five single-file bench toys. Keep fine-tuning formally out of scope in the map until map 112 delivers local inference and Hydra's harness accumulates at least 100 verifier-accepted execution traces.

---

## What I could not verify

1. The exact count and content of `plan_proposed`, `panel_verdict`, and `human_plan` events inside `~/hydra/puppets/`, which is gated outside this worktree's policy.
2. The outcome and diff retention of the 26 parent-delegated runs identified in `src/face.rs`.
3. The raw benchmark tables and full ablation parameters in the external research papers (Zywot et al., ToolOrchestra, Mangannavar et al., LIMI), as I was restricted to the quotes in `.scratch/hydra/research/118-panel-readings.md`.
4. The exact inference latency and memory ceiling of running an 8B quantized model (e.g. Q4_K_M) alongside macOS system overhead on the target 8 GB Apple Silicon host.
5. The actual GCP billing invoice for Round 1 in `<gcp-project>`, which `round1/README.md` noted had not been reconciled.
