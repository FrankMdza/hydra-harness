# Seat: Opus, map ticket 138

## 1. When: A

Run the adapter round (383) first, then train a 10M to 100M model from zero on one narrow domain as a separate experiment. The human already chose base plus adapters as the first measurement on map 137 question 2, and B would quietly reverse that. B also puts a pre-1.0 framework with open bugs in the loss (#6212) and on Metal (#5962) under the round that has to beat the always-Bash baseline. Either result would then be unreadable. C ignores what the human asked for. At nano size the research prices from-zero training at tens of dollars or less, so the experiment is cheap enough to run beside 383 without replacing it.

## 2. Framework: B

A small PyTorch reference trains the same model, and the Burn version must match its loss curve within a tolerance written down before the run. This is the house pattern. `crates/grader/tests/differential.rs` compares the Rust to fixtures the Python recorded, to the byte, and lists exactly what is normalized. Here "to the byte" becomes a per-step loss tolerance with the same initial weights, data order and seed, compared in float32 with no mixed precision, since Burn has no AMP (#5696). Burn with no reference (A) would leave #6212 undetected, because a loss with a zeroed gradient still goes down. Candle (C) has no named optimizer or Metal training, and Burn dropped its Candle backend in 0.22. Burn stays the target because one codebase trains on CUDA and loads on Metal, at least on paper. The first ticket tests that path.

## 3. Retraining chosen parts: train chosen layers with the rest frozen, first

Here is what each technique means on a model the lab trained itself:

- **Chosen layers, rest frozen.** Because the lab trained the model, it knows the layer count and has every checkpoint. It can retrain the last two blocks on a new class of input and measure forgetting on the old held-out set. It needs no new code beyond leaving parameters out of the gradient. The research did not check Burn's API for that, so the first ticket checks it.
- **LoRA.** The memory saving that justifies LoRA on 7B is gone at 10M, because full fine-tuning fits on one L4. It still pays for keeping several swappable deltas over one base, which is what 381 measures on the Mac. Do it second, on the same data, against frozen layers.
- **ROME, MEMIT, AlphaEdit.** These write single facts into a few MLP matrices. On a classifier of shell commands a "fact" is one verdict, so this is a patch, not training. Sequential edits bring forgetting (2401.07453). Do it later and only for a single correction.
- **Weight-space programming.** On the one model under 300M where it was tried, Gemma 3 270M, it did nothing (MATH-500 6.8% to at most 7.7%). It fixes a failure the model already shows, and every edit costs a frontier session. The blog's own limit says this. On a 10M model there may be no such failure to fix. Do not take it first.

## 4. The teacher

- **Evaluation:** yes. A frontier model scores outputs and picks checkpoints, and nothing it writes enters the weights. Map 133 already allows this.
- **Distillation:** only from open-weight teachers whose licence permits it, as map 133 settled on 2026-10-21. DeepSeek-R1 under MIT is the only licence the research read that names distillation as allowed. With a verifiable reward (question 5), no teacher text is needed at all.
- **Frontier labels that are then trained on:** this is the grey zone map 133 left open. It is the human's call. I recommend no.
- **A frontier model writing weight diffs:** this one is the human's. I recommend no. The Usage Policy effective 2026-11-12, as the research quotes it, forbids using outputs "to train an AI model (e.g., ... 'model distillation') without prior authorization". A weight diff written by Claude is an output that changes a model's weights, and I would not bet the router's accounts on a reading that says it is not training. This seat runs on one of those accounts.

## 5. First domain: classifying a shell command as the gate would

The lab owns the labeller. The compiled gate is a program, `tests/gate.rs` already feeds it synthetic payloads, and `tests/corpus/` holds the attacks map 68 built. The reward is agreement with the gate's verdict, so labels are free and exact, nothing has to leave the machine, and no frontier text enters. Map 137 put `tree-sitter-bash` in the gate, which will give a second labeller. That gives retraining in question 3 a natural new class to learn.

The honest limit: a model that copies a regex gate inherits its blind spots and is not useful as a gate. Its value is that it tests the pipeline against an exact reward.

I rejected the other two candidates. Outcome prediction has a few hundred runs, and round one shows how a small set hides a prior. A post-quantum crypto finder needs a labelled scanner the lab has not written.

## 6. First tickets

1. **Burn against a PyTorch reference, 10M, byte-level, on the Mac, CPU first.** Same initial weights, data order and seed. It measures per-step loss difference over 2,000 steps, with the tolerance written before the run, plus Metal against CPU logits on 100 held-out inputs (#5962) and whether Burn can freeze parameters. Ceiling $0, local. Open it after 381, because both use the Mac's memory.
2. **The gate model from zero on one spot L4.** It measures held-out agreement with the gate and recall on `tests/corpus/`, both thresholds stated before the run. Ceiling $5. Blocked by 1 and 383.
3. **Frozen layers against LoRA on the same model.** The new class is the `tree-sitter-bash` verdicts. It measures the gain on the new class and forgetting on the old held-out set. Ceiling $5. Blocked by 2.

## Claims checked

- **Cost arithmetic.** I recomputed every figure from the spot prices it quotes, and all match: T4 plus VM $0.2955, TinyStories $44.64, 124M $35.72, 350M $333.40, nanochat $79 to $105, and 6ND for SmolLM2-135M at 1.62e21 FLOPs.
- **Map 133's answer.** The research quotes it correctly. The human answered on 2026-10-21 with verifiable rewards and an open-weight teacher whose licence permits it, with frontier models as judges and label-writers.
- **The differential pattern.** I read `crates/grader/tests/differential.rs`. It matches the pattern this panel's prompt describes for `pyjson`.

I did not open any source URL, because the shell has no network.

## Pending from me

Nothing.

## Changed

Nothing. I edited no file and made no commit.

## Found

- Two questions are the human's: frontier labels that are then trained on, and frontier-written weight diffs. I recommend no on both, and the Usage Policy of 2026-11-12 is the reason for the second.
- Training a model on the gate's verdicts tests the pipeline. It does not make a better gate.
- The research left open whether Burn can freeze parameters. First ticket checks it.
- The gate refused a shell command that named `~/hydra/runs`, so I did not count the records. The design is working as intended there.
