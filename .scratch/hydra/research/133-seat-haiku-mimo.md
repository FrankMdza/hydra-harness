# 133 seat, Haiku: what Xiaomi's MiMo-V2.6 training offers Hydra

Tags used below. Verified means I read it in a primary source this session. Summary means it came from a model-written summary of the arXiv HTML past character 39k, so check it against the PDF. Inferred means my reasoning, not a source.

## 1. The model

The video is about MiMo-V2.6, released in September 2026. Pro is a sparse MoE with 1.02T total and 42B active parameters, 70 layers, 384 routed experts with 8 active, and a 1M context. Attention is GQA. Sixty layers use a 128-token sliding window and ten use global attention, a 6:1 ratio. Flash is 309B total and 15B active. Verified from https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Pro-RL and https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Flash-RL. Both cards say MIT. The Pro repo's LICENSE file returns 404, so the card tag is the only licence text I found. The video's architecture figures match.

Report: https://arxiv.org/html/2610.11959v1. Multi-token prediction is on, as a 5-layer drafter that predicts 7 tokens per pass. Verified from the card. The report's section 6.4 says RL uses block-6 drafting, which I did not reconcile.

## 2. The training process

Verified in the report's abstract and section 4.1: thousands of GPUs in one RL run. Each step samples 1,568 prompts with 16 rollouts, 25K trajectories of 110K to 150K tokens. The report gives $2.6M for Pro and $0.9M for Flash on RL post-training.

Summary only: 30T pretraining tokens for Pro and 48T for Flash. A dynamic sampler drops groups that are all-pass or all-fail. Partial rollouts with staleness 4. Binary test rewards for code. A length penalty on passing rollouts only. A hack agent ran until it found no exploit, and training audits kept confirmed hacks under 2%. The 9B model is SFT of Qwen3.5-9B on 77.4B tokens.

Not verified: the video's 123 and 82 hour durations, the $854,000 Flash figure, the compute split, the KL values and the trajectory lengths. I did not reach the live dashboard.

The video's claim that two-thirds of coding answers sit in git history comes from Vals AI. Its X post returned HTTP 402 to me, so I could not check it. The report says git history is kept only up to the base commit, section 4.2.6.

## What is open

- Weights: all three, MIT per the cards.
- Training code: https://github.com/XiaomiMiMo/verl, Apache-2.0, a verl fork whose README says it "adds reproduction code for five RL environments". The paper calls the release an end-to-end framework. I read the README, not the code.
- Environments: https://huggingface.co/datasets/XiaomiMiMo/MiMo-V2.6-RL-oss, Apache-2.0, 7,780 parquet rows in five subsets. The columns are prompt, reward_model and extra_info. Reward is rule style. extra_info names a Docker image per task, so the test most likely lives in the image. Inferred from one row.
- Images: https://hub.docker.com/r/xiaomimimo/mimo-v2.6-rl-oss, all linux/amd64 on the first page of tags, largest shown 6.09 GB.
- On an 8 GB Mac with no GPU: reading the parquet is fine. Running one task means pulling a multi-gigabyte amd64 image and running its tests under emulation. I expect that to be slow. I did not try it. An RL step is out of reach.
- Not released: pretraining data, GPU counts and hours, and the eval sets, which are third-party datasets.

## 4. The honest answer for Hydra

Reusable with no GPU:

- The task shape. A prompt, a rule reward and a per-task verifier is the split `bank.rs` and `score.rs` already make. The dataset's schema is a template for a bank record.
- Group selection. Drop tasks where every attempt passes or fails before training. `pairs.rs` already keeps only better or worse verdicts. This needs `attempt.rs` to draw several answers per task, and it draws one today.
- The leak check. Before a repo task counts as passed, confirm no commits sit after the base. Cheap to add to `score.rs` or the bank build.

Needs one rented GPU for hours, inferred and not measured: LoRA SFT of a 1B to 4B model on the export of about 350 lines and the pairs file.

Needs a cluster: the 30-step RL with 25K trajectories a step, and SFT at 9B scale on 77.4B tokens.

Map constraints: maps 88 and 111 say no local model yet drives a real tool loop, so group sampling waits on that harness. Map 109 rejected JEPA at this data size, and this report does not change that. The 9B recipe is plain SFT, not RLCD. Map 111's Ministral 3B stays the candidate. The 9B card says MIT, but it is a Qwen3.5-9B fine-tune, and I did not check Qwen's licence.

## Pending from me

- Whether to rent a GPU for the LoRA step.
- Whether group sampling in `attempt.rs` goes on the map.
- Someone with X access or the live dashboard should check the Vals claim and the timings.

## Changed

Wrote this memo at `.scratch/hydra/research/133-seat-haiku-mimo.md`. Nothing else changed. Nothing committed.

## Found

- The video's Flash cost, $854,000, does not match the report's $0.9M.
- The video's "3.59B" for the 9B model is a transcription error. The card says 9B.
- The report says block-6 in section 6.4 and 7 tokens in section 2.4.
- The fork README describes five environments, not the full framework.
- The dataset card gives 12.1 GB, but the root parquet files total about 18 MB.
