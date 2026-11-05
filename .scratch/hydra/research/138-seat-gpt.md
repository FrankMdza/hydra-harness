Seat: GPT. I recommend **A on timing, B on framework, and chosen-layer training first**. These are panel recommendations for the human.

1. **When: A, after build 383.** Map 137 explicitly preserves base-plus-adapters as the first measurement. Complete 381’s Mac footprint test and 383’s adapter experiment, then try a separate 10M model from random initialization. Expand toward 100M only after a useful result. A few demonstrations can specify the task; they do not establish that a randomly initialized model can learn it with little data.

2. **Framework: B, Burn with a small PyTorch reference.** Rust remains the training and serving implementation; Python supplies an independent comparison.

   Share initial tensors, tokenization, batches, optimizer settings and precision. My proposed acceptance threshold: over 500 deterministic FP32 steps, validation cross-entropy differs by at most 0.02 nats at each scheduled checkpoint. Check forward outputs, gradients and one optimizer step separately; matching aggregate loss can conceal errors. Freeze tolerances before running.

   The repository supports this pattern: `crates/pyjson/tests/math_matches_python.rs` compares against recorded CPython hashes; `crates/grader/tests/differential.rs` compares Rust against frozen Python results, including errors and output bytes. GPU training needs numerical tolerances rather than byte identity.

   **Source check:** Burn #6212 confirms the reported high-confidence gradient failure, but also says PyTorch shares it. Add an independent FP64 analytical check for extreme logits and masked inputs. PyTorch agreement alone cannot establish correctness. [Burn issue #6212](https://github.com/tracel-ai/burn/issues/6212)

3. **Chosen parts: train selected layers first.** After initial training, freeze embeddings and all but the final transformer block and output head. Compare adaptation against full fine-tuning under equal data and compute budgets; verify frozen tensors remain unchanged.

   On our own model:
   - **Selected layers** update existing weights directly and make the experiment easy to inspect.
   - **LoRA** trains small additive matrices over our frozen base; useful later for multiple interchangeable specializations.
   - **ROME/MEMIT/AlphaEdit** attempt localized association changes. Owning the weights does not establish that their localization assumptions hold in our tiny architecture.
   - **Frontier weight diffs** let another model propose direct tensor edits. That is a separate experimental intervention, with locality and regression tests required. The research’s identification of the unseen short remains an inference.

4. **Teacher: preserve map 133.** Start with program-computed labels and rewards. Permit open-weight distillation only after checking the specific model’s licence and applicable service terms. Keep frontier evaluation separate from training examples.

   **Human’s decision:** whether frontier-written labels may enter optimization, and whether frontier-written weight diffs are allowed. I recommend neither for the first experiment. Evaluation that selects checkpoints still influences the result; “no teacher text in the dataset” is the narrower, defensible claim.

   **Source check:** Anthropic’s published policy does contain the prohibition on using outputs to train models without prior authorization. Its stated effective date is **November 12, 2026**, eight days after this panel, so it must not be described as already effective. Human approval cannot substitute for provider authorization. [Anthropic Usage Policy](https://www.anthropic.com/legal/aup)

5. **Domain: predict the gate’s shell-command decision.** Hydra owns the gate implementation and fixtures in `tests/gate.rs` and `tests/corpus/`. Feed command text plus policy and relevant execution context; command text alone is insufficient. Obtain labels by passing synthetic payloads to a pinned gate, without executing the commands.

   Reward: exact agreement on allow/deny. Report balanced accuracy and false-allow rate separately. Split by command family before generating variants, keeping adversarial compositions held out. Compare against majority and simple lexical classifiers. Use the model as an advisory experiment; the deterministic gate remains authoritative. This measures imitation of the gate, not independent security truth.

6. **First tickets, smallest first.** Proposed ceilings are stopping limits, not promised completion prices:

   | Ticket | Measurement | Ceiling |
   |---|---|---:|
   | Corpus and baseline | Deduplicated family splits, label reproducibility, baseline balanced accuracy and false allows | $0 cloud |
   | Reference parity | Forward/backward correctness, 500-step loss agreement, CUDA checkpoint loaded and checked on Mac | $2 |
   | 10M pilot and selective adaptation | Held-out improvement over lexical baseline, retention after adaptation, throughput and footprint beside Kev | $8 |

   Keep the combined ceiling at $10, including VM, disk and retries; reserve teardown time. Use ephemeral spot infrastructure in `<gcp-project>`, return weights and report, and delete VM and disk. Stop if measured throughput cannot fit the remaining budget.

   **Cost check:** llm.c confirms 124M, 10B tokens and roughly 90 minutes on eight A100s. That optimized C/CUDA result does not establish Burn’s cost or justify the research’s blanket “less below 10M” claim. [llm.c reproduction](https://github.com/karpathy/llm.c/discussions/481)

## Pending from me

None. Timing, budget and teacher-boundary decisions remain the human’s.

## Changed

Nothing; read-only review.

## Found

The existing adapter plan stands. PyTorch parity needs an independent numerical oracle. Nano-model cost must be measured on the actual implementation.
