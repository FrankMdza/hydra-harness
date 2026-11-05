# Round one, 2026-10-04

The first fine-tune on Hydra's own records. It tests the pipeline, not the model: 21 training
trajectories and 3 held out, which cannot show a model getting better.

## What ran

- `build_sft.py` turns `hydra export` into chat trajectories under five tool names, cut at the
  first tool outside them. The export does not yet carry the run's task (map 113, build 290),
  so 24 of 351 runs survive, all of them ones whose first user message is an injected skill body
  or an interactive turn.
- `round1.py`, on one spot T4 in `<gcp-project>` (`n1-standard-4`, deep learning image
  `pytorch-2-9-cu129`, `--max-run-duration 2h`, deleted by hand after the run along with its
  disk, no bucket used). QLoRA on `mistralai/Ministral-3-3B-Instruct-2512-BF16`: 4-bit NF4,
  fp16 compute, r 16 on the language model's projections, 24.7M trainable parameters, 3 epochs,
  lr 2e-4, training window 2,048 tokens. transformers 5 with mistral-common; `torchaudio` had to
  be uninstalled from the image because it no longer loaded against its torch.

## Result (`report.json`)

| | before | after |
|---|---|---|
| next call, held-out trajectories | 1 / 9 | 5 / 9 |
| map 111's bake-off | 10 / 10 | 9 / 10 |

Loss per epoch 1.14, 0.60, 0.31, all 63 steps finite, 341 s of training, 5,761 label tokens.

**The next-call gain is mostly the Bash prior.** After training the model answered `Bash` on 8 of
9 held-out steps, and 5 of the 9 wanted `Bash`. The bake-off miss is "change a word in
README.md": the trained model reads the file first, which is what the trajectories do and what
a real loop would do, but the bake-off scores only the first call. Nothing here says the model
got better at the work.

## What it took to get there

Three attempts. The first ran out of memory in the backward pass at 4,096 tokens (the fp32
logits over a 131k vocabulary are 2 GB on their own). The second trained at 2,048 tokens but kept
1,519 label tokens, and rows whose first answer fell past the cut gave a 0/0 loss, NaN on 9 of
21 steps. The third cut prompts to 2,000 characters and tool output to 600, and dropped rows
without labels.

Cost: one spot T4 for about 65 minutes, an estimate of 0.25 USD against the 10 USD cap, not yet
read off the bill.

## Before round two

Build 290 left one precondition: read the eligible tasks with fenced code for content from
outside `~/hydra` before an export with tasks leaves the machine.


Build 290 (the task crosses) multiplies the usable runs. Round two wants a held-out set big enough
to tell a prior from a skill, a per-tool score instead of a total, and a multi-turn check where
reading before editing counts as right.

Round two's preflight is build 380; its exclusion list is `../round2/exclude-private-repo.txt`.
