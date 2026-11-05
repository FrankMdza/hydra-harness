# Seat: Gemini — Decision Panel for Map 138

## 1. When
**Recommendation: (A) After round two (build 383).**

Build 381 measures base and adapter serving footprints on the Mac beside Kev, and build 383 establishes cloud training on spot GPUs under a $1 ceiling with a signed adapter artifact. Attempting from-scratch pretraining now (B) would stall round two on unvalidated Rust training infrastructure. Dropping it entirely (C) discards the human's goal. Testing a 10M to 100M from-scratch model on a single narrow domain after 383 isolates architectural and framework risk into a cheap experiment (~$5 to $35 on spot) without blocking the adapter pipeline.

## 2. Framework
**Recommendation: (B) PyTorch reference with Burn matching its loss curve (the `pyjson` pattern).**

Burn is pre-1.0 (v0.22.1 / v0.23.0-pre.1) and carries open training correctness bugs, notably #6212 (softmax/CrossEntropyLoss zeroing target gradients at high confidence) and #5962 (saturated Metal inference). Candle is worse for training: it lacks optimizers, has open issues on backward passes (#4047), and has silently incorrect Metal SDPA (#4020). Blind training in Burn risks debugging numerical divergence instead of model behavior. Following the `pyjson`/`differential.rs` house pattern—requiring Burn to reproduce a small PyTorch reference model's loss curve within a stated tolerance across 1,000 steps—guarantees mathematical parity before any cloud spend. Serving on the Mac can still target Burn Metal/CPU once validated.

## 3. Retraining Chosen Parts
**Recommendation: Chosen layers trained with the rest frozen.**

- *Chosen layers (rest frozen):* Requires only setting `requires_grad = false` on non-target modules. On a 10M–100M model, memory savings are secondary, but gradient updates to isolated middle-layer MLPs or attention blocks allow fast domain specialization with zero architectural changes or inference overhead.
- *LoRA:* Adds low-rank decomposition matrices ($A \times B$). Useful for multi-tenant swapping over large bases, but redundant for small models where the entire network fits easily in RAM, and Burn lacks mature built-in adapter tooling.
- *Model editing (ROME/MEMIT/AlphaEdit):* Closed-form rank-one algebraic updates into MLP keys/values. Suitable only for patching single facts, not teaching procedural reasoning or syntax, and suffers from catastrophic forgetting under sequential edits.
- *Weight-space programming:* Frontier LLM directly writes weight diffs without backpropagation (Takehi et al., 2026). Ineffective at sub-300M scale (failed on Gemma 3 270M), expensive in frontier inference turns, and fixes existing latent loops rather than learning new domain behavior.

## 4. The Teacher
**Recommendation:** Verifiable programmatic rewards and distillation from permissible open-weight teachers (e.g. DeepSeek-R1 under MIT, Qwen/OLMoE under Apache 2.0). Frontier models (Claude, Gemini, OpenAI) must remain strictly evaluators and judges. Weight-space programming edits from frontier models should be rejected.

**Marked as the human's:** Map 133 (answered 2026-10-21) established that training signals come from verifiable rewards and open-weight models, with frontier models restricted to judges and labelers. Anthropic's Usage Policy (effective 2026-11-12) explicitly prohibits using outputs to train or distill models without authorization, and Gemini terms forbid training competing models. Whether to permit frontier models to write weight diffs directly or to accept vendor account termination risk for frontier distillation is **the human's choice** and cannot be decided by this panel.

## 5. The First Domain and Verifiable Reward
**Recommendation: Classifying shell commands as the gate would.**

- *Domain:* Parsing and classifying shell commands (`allow`, `refuse`, `escalate`) against Hydra's gate policies. Sequences are short, syntactically rich, and directly relevant to Hydra's security perimeter. The lab already possesses policy fixtures and command traces.
- *Verifiable Reward:* Binary 1.0/0.0 match computed by running Hydra's reference policy gate (`policies/standard.yaml` / `tree-sitter-bash` evaluator) against the predicted classification. Deterministic, program-computed, and requires no frontier LLM in the training loop.

## 6. The First Tickets
1. **Ticket 1: Differential loss harness: PyTorch vs Burn on 10M model.**
   - *Measures:* Divergence between PyTorch and Burn loss curves over 1,000 steps on synthetic command tokens (tolerance < 1.0%).
   - *Ceiling:* $2 (local CPU / short spot verification).
2. **Ticket 2: Pretraining 10M gate classifier in Burn on spot GPU.**
   - *Measures:* Training throughput (tokens/sec), disk footprint (<45 MB), and held-out classification accuracy on 10,000 commands.
   - *Ceiling:* $5 (~12 hours on GCP L4 spot @ $0.42/h).
3. **Ticket 3: Frozen middle-layer fine-tuning on policy updates.**
   - *Measures:* Steps to reach >95% accuracy on new policy rules and retention on existing rules when training only layers 2–3 MLPs vs full fine-tune.
   - *Ceiling:* $5.

---

## Research Claims Checked
1. **Takehi et al. (MIT CSAIL, Nov 2026), "Sparks of Weight-Space Programming":** Verified. GPT-6 Astra edited 0.016% (131,072 weights) of `Qwen3.5-0.8B` without gradients, jumping MATH-500 from 7.9% to 48.1%, but failed to improve `Gemma 3 270M`.
2. **TinyStories (Eldan & Li, arXiv:2305.07759):** Verified. Models under 10M parameters (down to 2.5M) trained on synthetically constrained vocabulary generate fluent, coherent English, validating domain restriction for nano models.
3. **Map 133 Resolution in Repo:** Verified against `.scratch/hydra/issues/133-tiny-experts-from-recorded-runs.md`. Question 3 was answered on 2026-10-21: verifiable rewards and open-weight teachers only; frontier models stay judges.

## Pending from me
Nothing.

## Changed
No files modified, created, or deleted; nothing committed.

## Found
- Burn has unverified Metal/CUDA cross-loading and known gradient-zeroing bugs (#6212), making PyTorch differential verification mandatory.
- Weight-space programming is unsuitable for nano models (<300M).
- Gate command classification provides a zero-frontier, fully deterministic verifiable reward loop.

