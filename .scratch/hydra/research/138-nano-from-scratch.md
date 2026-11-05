# 138 research: nano models from scratch, in Rust, with chosen weights retrained

Every source was read on 2026-11-04. "Derived" marks my own arithmetic on a source's numbers.

## Short answer

Possible, with three corrections to the plan as worded. A 10M to 300M model trained from zero on a
narrow domain is cheap: about $36 at 124M on GCP spot, less below 10M. Retraining chosen parts of
it is ordinary practice, and at this size full fine-tuning is often as cheap. The corrections:
the short shows a frontier model editing weights with training forbidden, and that method failed
on the one model under 300M it was tried on. "SpaceAI" is xAI inside SpaceX, whose published stack
is Rust around JAX, now moving to C, with no Burn in it. And the human's "retraining from the big
LLMs" covers three different uses of a frontier model, two of which need the human.

## 1. What the short shows

I could not open the short. Every YouTube URL, the oEmbed endpoint and noembed failed TLS
verification from this machine (`unable to get local issuer certificate`), in WebFetch and in curl.
The thumbnail did load (`i.ytimg.com/vi/jNuDdqEwI58/maxresdefault.jpg`). I have not seen the video
or its description, so the match below is an inference from the thumbnail.

The thumbnail reads "AI JUST FIXED ANOTHER AI" over a weight matrix titled
`layers.15.mlp.down_proj`, one column red and one green, with the OpenAI logo and a presenter in a
Better Stack shirt. A search on the video id found nothing. A search on the content found one study,
out two days before this ticket, whose edit sits in the MLP output of layer 15 and was made by an
OpenAI model:

Takehi, Teoh, Zhang and Khattab (MIT CSAIL), "Sparks of Weight-Space Programming: LLMs can natively
read & write neural network weights", blog post published 2026-11-02,
<https://rikiyat.github.io/blog/weight-space-programming/>. Not peer reviewed. Logs and exact
weight diffs: <https://github.com/RikiyaT/lm_understand-logs>, `exp8_math/README.md`.

From that README:

- The editor was GPT-6 Astra through the Codex CLI with its own tools off. It had a CPU-only Python
  kernel over the weights (at most 300 code runs), practice problems (7,469 GSM8K train, 7,275
  MATH train) and 10, 25 or 50 forward passes. A pass generated answers to up to 512 problems or
  read activations over up to 1,024 texts.
- Forbidden: "gradient descent or any training (no backward passes, autograd, optimizers or
  iterative fitting of weights to a loss)", and edits to embeddings, output layer and final norm.
- The 50-pass run subtracted a "loop" direction "from the 64 output columns per layer of the MLPs of
  layers 15 and 19 that write it most": 131,072 weights targeted, 130,188 different after bfloat16
  rounding, 0.016% of the model per the blog. The 10-pass run made a rank-one edit,
  W' = W - 1.5 u(u^T W), on 30 matrices in layers 8 to 22.
- Qwen3.5-0.8B, thinking on, before and after: MATH-500 7.9% to 48.1%, GSM8K 13.8% to 67.1%,
  IFEval 21.0% to 62.3%, MATH-500 answers cut at 32,768 tokens 93.0% to 33.1%. 158 turns, 80 Python
  runs, 20.3 hours including a 4.7-hour outage. No dollar cost published.
- The blog's own limit: "Fixing loops, however, is not the same as making a model better at math:
  it lets the model finish answers it could already work out." On Gemma 3 270M, which loops on 8.5%
  of answers, "Astra could not improve it"; VentureBeat (2026-11-02) gives MATH-500 6.8% to at most
  7.7%.
- R&D World (2026-11-03) reports critics calling the fix steering, an existing technique (Sean
  Goedecke), and the announcement an overreach across experiments (Matthew Honnibal). Not opened.

So the method repairs a failure the model already shows, and nothing in it trains. It sits in the
model-editing family of section 3, close to activation steering written into the weights. It does
not "retrain the parts".

## "SpaceAI"

Most likely xAI after SpaceX bought it; Tesla North (2026-05-07) reports the merged brand as
"SpaceXAI". I found no SpaceX page using that name. What the lab has published on training:

- Grok-1 announcement, <https://x.ai/news/grok> (2023): a custom training and inference stack "based
  on Kubernetes, Rust, and JAX", Rust chosen for reliability in distributed systems. The Rust runs
  the infrastructure. The gradients are computed in JAX.
- 2026-05-28, Musk on X: SpaceX is close to V1.0 of its own training stack, mostly C with some C++,
  more than 10x faster than JAX on large runs, aimed at about 220,000 GB300 GPUs and Grok 5. Known
  only through secondary reports (Tesla North 2026-05-28, Dealroom, gagadget). The 10x has no
  published benchmark, and there is no engineering post or code.

Nothing published ties the lab to Burn or Candle.

## 2. What 10M to 300M from zero can do, and the price

| Run | Params | Tokens | Hardware and time | Source |
|---|---|---|---|---|
| TinyStories | under 10M; 2.5M "can attain factual knowledge"; 1M, 8 layers, fails all factual prompts | not stated | "a single V100 GPU within at most 30 hours" | arXiv 2305.07759 |
| llm.c GPT-2 | 124M | 10B FineWeb | 8x A100 80GB SXM, ~90 min, "about $20" on Lambda | llm.c discussion #481 |
| llm.c GPT-2 | 350M | 30B | 8x A100 80GB SXM, 14 h | same |
| nanochat d20 | not stated | not stated (4e19 FLOPs) | 8x H100, ~1.5 to 2 h, "$48", "~$15" on spot | github.com/karpathy/nanochat |
| SmolLM2 | 135M / 360M | 2T / 4T | not stated at these sizes | arXiv 2502.02737 |

TinyStories is the human's case. A child's vocabulary and short stories were enough for models
under 10M to write fluent, consistent English, and the same models fail open-domain questions.
SmolLM2 is the opposite path: at 135M it spends 2T tokens, 200 times llm.c's budget, to be useful
in general. A domain model gets cheap by narrowing what it has to know.

GCP spot, <https://cloud.google.com/spot-vms/pricing>, tables headed "Current Spot pricing (USD)".
The static page does not label the region; its selector lists Iowa (us-central1) first. Prices
"can change up to once every day".

| Machine | Spot $/h |
|---|---|
| T4 GPU $0.1981 + n1-standard-4 VM $0.09742 | $0.2955 (derived) |
| V100 GPU alone | $1.488 |
| g2-standard-4, 1x L4 | $0.423016 |
| a2-highgpu-1g, 1x A100 40GB | $1.98913 |
| a2-ultragpu-8g, 8x A100 80GB | $23.814198356 |
| a3-highgpu-8g, 8x H100 | $52.700460712 |

Derived costs on the source's own hardware, preemption ignored:

- TinyStories: at most 30 V100-hours, $44.64 of GPU plus a VM. No source gives T4 or L4 hours.
- 124M (llm.c): 1.5 h of a2-ultragpu-8g, $35.72.
- 350M (llm.c): 14 h, $333.40.
- nanochat d20: 1.5 to 2 h of a3-highgpu-8g, $79 to $105.
- SmolLM2-135M at 2T tokens: 6ND gives 1.62e21 FLOPs, about 40 nanochat runs at its
  utilisation, $3,200 to $4,200. A general model's price.

Map 77 put 1B from scratch at $5,700 or more. Under 300M on a narrow domain, compute drops by one to
two orders of magnitude and the work moves to building the domain corpus.

## 3. Retraining chosen parts of the weights

From arXiv abstracts. Few report a model under 1B, so "at nano size" below is my reading.

- Adapters. LoRA (2106.09685) freezes the model and trains low-rank matrices per layer, cutting
  trainable parameters "by 10,000 times" and GPU memory "by 3 times" on GPT-3 175B, with no added
  inference latency; GPT-2 is among its models. QLoRA (2305.14314) puts a frozen 4-bit base under
  it, 65B on one 48 GB GPU. DoRA (2402.09353) splits weights into magnitude and direction. At nano
  size it works, but the memory saving stops mattering: full fine-tuning of 100M fits on one L4. It
  still pays for keeping many small swappable deltas over one base.
- Chosen layers trained, the rest frozen. LISA (2403.17919) randomly freezes most middle layers and
  beats LoRA by 10% to 35% on MT-Bench at similar memory, at 7B and 70B. Any framework does this
  by excluding parameters from the gradient. Cheap at nano size.
- Model editing. ROME (2202.05262) locates facts in middle-layer MLPs and writes them with a
  rank-one update. MEMIT (2210.07229) reaches "thousands of associations" on GPT-J 6B and GPT-NeoX
  20B. AlphaEdit (2410.02355, ICLR 2025) projects the update onto the null space of knowledge to
  keep, +36.7% on average, tested on GPT2-XL among others. Sequential edits bring "gradual but
  progressive forgetting" and then "abrupt or catastrophic forgetting" (2401.07453). An edit takes
  a few matrices and writes a fact. It does not teach a skill. Section 1's method is in this family.
- Sparse fine-tuning. SpIEL (2401.16405) trains a moving list of parameter indices and deltas,
  updating, pruning and regrowing it, and often beats LoRA at similar run time on LLaMA 2 7B and
  13B. LT-SFT (2110.07560) learns sparse masks and beats adapters on cross-lingual transfer. I
  found no result at nano size.
- Pruning and regrowth. RigL (1911.11134) trains at a fixed sparse parameter count and moves
  connections by magnitude and gradient; it was tested on ResNet-50, MobileNets and RNNs. Sheared
  LLaMA (2310.06694) prunes LLaMA2-7B to 1.3B and 2.7B and continues training at "3% of compute"
  of training from scratch, a route from a big open model to a small one.

For "retrain the parts the way we want", the fit is chosen layers or a LoRA on the domain, plus
editing for single facts. Weight-space programming finds and removes a failure the model shows,
and each edit costs a frontier model's session.

## 4. Rust

Burn trains today. Its docs say a model trained on CUDA loads on Metal, I found no tested example
of that path, and open bugs sit on it.

- Burn, <https://github.com/tracel-ai/burn>: stable v0.22.1 released 20 Oct (year not printed,
  2026 by the issue dates), v0.23.0-pre.1 on 29 Oct. GPU backends CUDA, ROCm, Metal, Vulkan,
  WebGPU through CubeCL, with autodiff and fusion on top, optimizers, a `Learner` and data-parallel
  training. README: "there will be breaking changes". 0.22.0 removed the Candle backend; LibTorch
  and NdArray are removed in 0.23.0-pre.1.
- Portability, Burn book: "The record is backend-independent: weights saved with one backend can be
  loaded on another." Records are `.bpk` since 0.22 and "retain the checkpoint's dtype by default",
  so a bf16 checkpoint needs a cast if the Mac path wants f32. Older records do not load directly.
  #6391 (2026-11-04): a bool tensor moved between CubeCL devices fails to compile on WGSL.
- Multi-GPU: DDP exists. The distributed-computing page says non-CUDA backends sync by transfers,
  and the 0.22 migration page says "CubeCL all-reduce currently requires CUDA". The two pages
  disagree, which does not matter for a run on one GPU.
- I did not check Burn's API for freezing parameters.

Open Burn issues on the path, titles only, bodies not read, so whether `main` fixes them is
unknown:

- #5962 wrong, saturated inference outputs on Metal, CPU correct (2026-10-03). This one hits
  running on the Mac. <https://github.com/tracel-ai/burn/issues/5962>
- #6212 softmax, log_softmax and CrossEntropyLoss zero the target gradient at high confidence
  (2026-10-24), in the loss a language model trains on.
  <https://github.com/tracel-ai/burn/issues/6212>
- #4325 flash attention fails on most backends, blocker (2026-01-14); #4530 naive attention exceeds
  Metal shared memory (2026-02-16). <https://github.com/tracel-ai/burn/issues/4325>,
  <https://github.com/tracel-ai/burn/issues/4530>
- #5696 automatic mixed precision is a feature request (2026-09-16), so no AMP training.
  <https://github.com/tracel-ai/burn/issues/5696>
- #5891 system memory grows over multi-day wgpu runs (2026-09-28); #5212 GPU memory grows during
  training (2026-07-22). <https://github.com/tracel-ai/burn/issues/5891>,
  <https://github.com/tracel-ai/burn/issues/5212>
- #4512 Metal performance against MLX on Qwen3-0.6B, 20 comments (2026-02-12), body not read.
  <https://github.com/tracel-ai/burn/issues/4512>

Candle (<https://github.com/huggingface/candle>) lists "Model training" and CUDA multi-GPU, names
no optimizer or Metal backend in its README, and has open #4047 (no flash-attention backward,
2026-10-19) and #4020 (Metal SDPA silently wrong, 2026-10-03). Training on Metal unconfirmed.
tch-rs links libtorch v2.13.0; dfdx is "Still in pre-alpha state", CUDA only.

So "all in Rust" holds with Burn, on a pre-1.0 framework with open bugs in training correctness.
10M to 100M is the cheapest size to find them at: a run costs dollars, and the same model trained
in PyTorch gives a loss curve to compare against.

## 5. "Retraining from the big LLMs", against map 133

Map 133, answered 2026-10-21: training signal comes from verifiable rewards and from open-weight
teachers whose licence permits training on their outputs; frontier models stay judges and
label-writers. The human's words carry three uses:

1. Evaluation. A frontier model scores the nano model's outputs to pick checkpoints or edits.
   Nothing it writes enters the weights. Inside map 133.
2. Distillation. "Few shots" to a big model so it writes examples, then training on them. Map 133
   allows this only from open-weight teachers with a permitting licence. Frontier-written labels
   that are then trained on fall between 1 and 2, and map 133 does not say where.
3. Editing, the short's method. The frontier model's output is a weight diff. No terms I read
   cover this case, and whether it counts as training on outputs is for the human.

Terms, read today:

- DeepSeek-R1, MIT, allows "derivative works, including, but not limited to, distillation for
  training other LLMs" (<https://huggingface.co/deepseek-ai/DeepSeek-R1>). The only licence read
  that names distillation as allowed.
- Qwen3-8B and gpt-oss-120b: Apache 2.0 per their Hugging Face cards, which say nothing on outputs.
  Qwen3.5-0.8B is Apache 2.0 per VentureBeat.
- Gemma (modified 2026-04-01): a model trained on Gemma's synthetic outputs is a "Model Derivative"
  and carries Gemma's use restrictions, though "Outputs are not deemed Model Derivatives". Gemma
  4's separate Apache 2.0 page not read.
- Llama 3.1: allowed, and a distributed model "shall also include 'Llama' at the beginning" of its
  name.
- Anthropic Commercial Terms D.4: no using the service "to train competing AI models". The Usage
  Policy effective 2026-11-12 forbids using outputs "to train an AI model (e.g., ... 'model
  distillation') without prior authorization".
- Gemini API terms (effective 2026-03-23): "may not use the Services to develop models that compete
  with the Services".
- OpenAI terms: both pages returned 403. Astra, the editor in the short, is OpenAI's.

## Gaps

The short was not seen. Not in the pages read: TinyStories token count, nanochat parameter count,
SmolLM2 135M/360M GPU-hours, any T4 or L4 training time. Section 3 rests on abstracts. The GCP
region is my reading of the selector. OpenAI terms and the Mistral licence were not read.

## Pending from me

Nothing. The ticket also asks for `research/138-panel-prompt.md`, which this run's brief kept out of
scope.

## Changed

Wrote `.scratch/hydra/research/138-nano-from-scratch.md`. No other file touched, nothing committed.

## Found

- The short most likely covers MIT CSAIL's weight-space programming, which forbids training and
  did nothing for Gemma 3 270M.
- Burn's CUDA-to-Mac path is documented, untested, and crossed by #5962 and #6212.
- Distillation from frontier outputs and frontier editing of weights both go back to the human.
- Anthropic's Usage Policy naming distillation takes effect 2026-11-12, eight days from now.
