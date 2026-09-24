# 77 - A small model of Hydra's own: research findings

Written 2026-09-24 by one Opus subagent. Every price was read on 2026-09-24 at the URL given. Prices are before tax. I did not check how Colombia taxes foreign digital services.

## Summary

1. Job 1, the classifier and router, needs no host. A gradient-boosted or logistic classifier on record features trains and runs in-process on the Mac. Cost: $0 a month.
2. If task text helps, OpenAI's text-embedding-3-small costs $0.02 per million tokens. A few hundred tasks a month cost about one cent.
3. Job 2, a generating model, should not be fine-tuned yet. 380 unlabeled records will not teach a 1.5B model more than a prompt does.
4. When the corpus justifies it, QLoRA a 1.5B to 4B open model on a rented RTX 3090 at $0.22 an hour, a few dollars a run.
5. Serve it from Modal with scale to zero. The Starter plan's $30 monthly credit covers the panel's volume, so expect $0, and about $5 in a bad month.
6. The no-GPU alternative is a gpt-4.1-nano fine-tune served per token, about $1 to $3 a month. The weights never leave OpenAI.
7. Together and Fireworks train cheaply but serve a fine-tune only on a dedicated H100 at $3.99 to $8.00 an hour, $2,900 to $5,800 a month.
8. Anthropic is out. Claude 3 Haiku was the only fine-tunable Claude, and Bedrock gives its end of life as 2026-09-10.
9. A from-scratch 1B pretrain at Pythia's token count costs about $5,700 of rented A100 time and would not write Rust.
10. A BitNet 2B4T-class pretrain is about $67,000 by my estimate. Microsoft published no GPU-hours.

## What Hydra can export today

D12 keeps training out of scope and lossless export as a constraint on the record. Harness spec 14 makes export default-deny. A run leaves only if every path in its subtree sits inside `config/export-allowlist.json`, protected config #3, which only a human writes.

`hydra export` writes one header line and one `{"events": [...]}` line per run, in the normalized vocabulary. It does not write TRL shapes; ticket 59 corrected that claim. Preference pairs leave only when `config/export-signoff.json` names the pass, the judge run and the confirmed finding. Neither file exists, per `IMPLEMENTATION.md`, so nothing has ever been exported. Either job starts with the human writing the allowlist.

Measured today, `~/hydra/runs/` holds 399 record files and 277 MB. The median record is 191 KB and the 90th percentile 2.0 MB. At a rough four bytes a token, that is about 72 million tokens of raw frames. The normalized export is smaller by an amount nobody has measured. A converter from events to prompt and completion pairs is the real work, and it is not priced here.

## Job 1, the classifier and router

The answer is a menu pick: one of seven task classes, one of four grades, a provider from a table. Nothing is generated, so an LLM is the expensive way to do a lookup.

The baseline is a non-LLM classifier over what the record already carries. That means tokens, duration, refusals, outcome, provider, and the corrections a human made in review. Scikit-learn fits 380 rows in seconds on the Mac. The model is kilobytes and runs inside Hydra's process, so there is nothing to host.

If the task's text adds signal, embed it. OpenAI's text-embedding-3-small and Cloudflare's bge-small both cost $0.02 per million input tokens, and Cloudflare adds 10,000 free Neurons a day. A local bge-small-class model on the Mac costs nothing. At 300 tasks of 2,000 tokens a month, the hosted ones cost about one cent.

Mistral's docs name a Classifier Factory for hosted classifiers. Its token price is not on the page I read. The $4 job minimum and $2 monthly storage already exceed the local path.

Labels limit this job, not compute. Seven classes, four grades and three or four providers make 84 to 112 cells, about four records each. The records do not carry the mode's class and grade, so a human labels them until the mode writes its own. Measure the first classifier against the mode's existing table before it replaces anything.

## Job 2, a generating model on the panel

This model writes a plan or a verdict. The baseline is BitNet b1.58 2B4T or a 1.5B to 4B open model at 4 bits, fine-tuned with QLoRA or QAT.

BitNet 2B4T's report gives it a 54.19 average against Qwen2.5 1.5B's 55.23. Its non-embedding memory is 0.4 GB against 2.6 GB. It decodes at 29 ms a token on eight threads of an i7-13800H with `bitnet.cpp`. The model card warns that plain `transformers` gives no efficiency gain. Fine-tuning starts from the bf16 master weights, `microsoft/bitnet-b1.58-2B-4T-bf16`. The license is MIT.

Training is cheap anywhere. One epoch over all 72 million raw tokens costs $27 at Together's $0.38 per million and $36 at Fireworks' $0.50. Vertex charges $97 for Qwen 3 4B at $1.35. OpenAI gpt-4.1-nano and Gemini 2.5 Flash-Lite both charge $108 at $1.50. Nobody trains on raw frames, though. Distilled to 380 pairs of about 4,000 tokens, the set is 1.5 million tokens and $0.57 to $2.25 an epoch. Together's $4 job minimum becomes the real price.

Rented QLoRA is cheaper still. A 1.5B to 4B model at 4 bits fits a 24 GB card, and RunPod's community RTX 3090 is $0.22 an hour. I have not timed a run, but a few epochs over 1.5 million tokens is hours. Call it under $2.

Serving is where the bill lives. Assume 100 goals a month, five panel calls each, 8,000 tokens in and 1,000 out. That is 4 million input and 0.5 million output tokens.

- An untuned model under 4B on Fireworks serverless, at $0.10 per million, costs $0.45 a month. Any fine-tune has to beat this.
- Fine-tuned gpt-4.1-nano at $0.20 in and $0.80 out costs $1.20.
- Tuned Gemini 2.5 Flash-Lite keeps the base price of $0.10 in and $0.40 out, so $0.60. From Gemini 3 on, Google charges tuned models 1.5 times the base.
- Your own weights on Modal: an L4 is $0.80 an hour. At 500 calls of 30 seconds with cold start, that is 4.2 hours, about $4 with CPU and memory, inside the $30 credit. The 30 seconds is my assumption.

Blunt verdict: every hosted option is under $5 a month at this volume. A fine-tune buys independence from vendors, not savings.

## Training from scratch at low bit width, costed

None of the three low-bit papers publishes GPU-hours for a useful model. So I cost them against Pythia's published table, 300 billion tokens on A100 40 GB cards. The cheapest A100 cited is RunPod community A100 PCIe 80 GB at $1.19 an hour. The cheapest H100 is RunPod community H100 PCIe at $1.99.

| Model | Tokens | GPU-hours | Source | Dollars |
|---|---|---|---|---|
| Pythia 70M | 300B | 510 A100 | Pythia table 5 | $607 |
| Pythia 160M | 300B | 1,030 A100 | Pythia table 5 | $1,226 |
| Pythia 410M | 300B | 2,540 A100 | Pythia table 5 | $3,023 |
| Pythia 1B | 300B | 4,830 A100 | Pythia table 5 | $5,748 |
| Sub-100M QAT study, every run | 10B shard | 2,020 H100 | Thomassen 2026 | $4,020 |
| One 100M QAT run | 9,000 iterations | about 1.7 H100, 25 min on 4 cards | Thomassen 2026 | $3.32 |
| CQAT, OLMo-1B at 1.58 bits | 40B | not published | Nielsen et al. 2025 | about $766, scaled from Pythia 1B |
| BitNet b1.58 2B4T | 4T | not published | Ma et al. 2025 | about $67,000, my 6ND estimate |

The BitNet row is 6 × 2 billion parameters × 4 trillion tokens, or 4.8e22 FLOPs. At 40 percent of an H100's 989 dense BF16 TFLOPS, that is about 33,700 H100-hours. Scaling Pythia 1B to 4 trillion tokens gives a similar $76,600.

The recipe makes it worse. The CQAT paper finds 1.58 bits from scratch loses to 16-bit training that switches after 2,000 steps. It also says QAT "initially requires more memory and compute than pre-training a regular language model", because of the full-precision shadow weights. So "4 bits from the first step" is not even the best recipe. The $3.32 run is a research probe, not a model that plans.

Colab does not change this. Pay As You Go and Pro sell 100 compute units for $9.99, and an A100 40 GB burns about 5.4 an hour. Both figures are from third-party reports, since Colab's signup page would not render. That is $0.54 an hour, the cheapest A100 found. Pythia 1B would still cost about $2,600, cut into roughly 400 sessions by the 12-hour cap.

Blunt verdict: $600 for a toy, $5,700 for a 1B model that cannot write Rust, five figures for the 2B class. It stays under D12.

## Hosting a 4-bit model as an endpoint, costed

A month is 730 hours. All rows read 2026-09-24.

| Service | Hardware | Price | Per month always on | Persistent | Limits | URL |
|---|---|---|---|---|---|---|
| Oracle Always Free | Ampere A1, 2 OCPU, 12 GB | free | $0 | yes, until reclaimed | halved from 4 OCPU and 24 GB on 2026-06-15; reclaimed if 95th-percentile CPU, network and memory stay under 20% for 7 days | docs.oracle.com, Always Free Resources |
| Hetzner CAX11 | 2 Arm vCPU, 4 GB | $6.99/mo | $6.99 | yes | excludes VAT | docs.hetzner.com, price adjustment |
| Hetzner CAX21 | 4 Arm vCPU, 8 GB | $12.49/mo | $12.49 | yes | excludes VAT | same |
| HF Spaces CPU Basic | 2 vCPU, 16 GB | free, but Docker Spaces need PRO at $9/mo | $9 | no, sleeps when unused | non-persistent disk | huggingface.co/docs/hub/spaces-overview |
| HF Inference Endpoints | 1 vCPU; T4; L4 | $0.03/h; $0.50/h; $0.80/h | $22; $365; $584 | yes | none noted | huggingface.co/pricing |
| Modal | T4; L4 | $0.59/h; $0.80/h, plus CPU and memory | $430; $584 | scales to zero | $30 free a month on Starter | modal.com/pricing |
| RunPod Serverless | 16 GB; 24 GB GPU | $0.58/h; $0.69/h flex | $423; $504 | scales to zero | per-second rates not shown | runpod.io/pricing |
| Replicate | CPU small; T4 | $0.09/h; $0.81/h | $66; $591 | private deployments bill idle | fast-booting fine-tunes bill active time only | replicate.com/pricing |
| Cloudflare Workers AI | serverless, your LoRA | $0.011 per 1,000 Neurons, 10,000 free a day | per token | yes | rank up to 32, under 300 MB; smallest LoRA base is Gemma 2B | developers.cloudflare.com/workers-ai |
| Together dedicated | H100 | $3.99/h promo, $5.49 regular | $2,913 | while running | serverless LoRA discontinued | together.ai/pricing |
| Fireworks on-demand | H100 | $8.00/h | $5,840 | while running | docs call dedicated the only way to serve a trained model | fireworks.ai/pricing |
| Google Colab | T4 to A100 | compute units | not allowed | no | 12 h cap, 24 h on Pro+ | research.google.com/colaboratory/faq.html |

One correction to ticket 75. The Colab FAQ bans SSH only on runtimes with no compute-unit balance. It bans web services and remote proxies on every managed runtime, paid ones included. The conclusion stands.

The Oracle rule matters. An endpoint idling between a few hundred calls a month is exactly what it reclaims.

Fireworks' pricing page says fine-tunes serve "for the same price as base models". Its fine-tuning docs say dedicated deployment is the only supported method. I trust the docs.

## Platforms that train and serve

All rows read 2026-09-24.

| Platform | Trains | Training per 1M tokens | Serves the result | Serving price | Verdict | URL |
|---|---|---|---|---|---|---|
| OpenAI | gpt-4.1-nano; 4o-mini; 4.1-mini | $1.50; $3.00; $5.00 | serverless | nano $0.20 in, $0.80 out | fits, $1 to $3/mo | developers.openai.com/api/docs/pricing |
| Google Vertex | Gemini 2.5 Flash-Lite; 3.1 Flash-Lite; Gemma 3 1B; Qwen 3 4B; Llama 3.2 1B | $1.50; $3.00; $0.47; $1.35; $0.28 | tuned Gemini serverless; open models not checked | 2.5 Flash-Lite $0.10 in, $0.40 out | fits for Gemini | cloud.google.com/vertex-ai/generative-ai/pricing |
| Together | 0.8B to 9B | SFT $0.34 to $0.38, DPO $0.84 to $0.94, minimum $4 | dedicated only | H100 $3.99/h | trains, does not serve cheaply | together.ai/pricing |
| Fireworks | up to 16B | LoRA SFT $0.50, DPO $1.00 | on-demand GPU | H100 $8.00/h | same | fireworks.ai/pricing |
| Mistral | not listed on the page read | not listed; $4 minimum, $2/mo storage | not stated | not stated | unpriced | docs.mistral.ai/capabilities/finetuning |
| Anthropic via Bedrock | Claude 3 Haiku only | not listed | Provisioned Throughput only, price from an account manager | unpublished | out, end of life 2026-09-10 | docs.aws.amazon.com/bedrock |
| Modal | your script | A100 80 GB $2.50/h | your container | per second, scales to zero | fits | modal.com/pricing |
| RunPod | your script | RTX 3090 $0.22/h | serverless | from $0.58/h | fits | runpod.io/pricing |
| Vast.ai | your script | RTX 3090 $0.10 to $0.14/h, search snippets only | no managed serving | n/a | training only | vast.ai/pricing |
| Lambda | your script | A10 $1.29/h, A100 $1.99/h | no scale to zero | per hour | too dear | lambda.ai/pricing |

Hugging Face, Replicate and Cloudflare host but were not priced for training. OpenAI discounts fine-tuned inference when the job enables data sharing. That sends Hydra's runs into OpenAI's training, which the export allowlist exists to prevent.

## What the human gives up on each path

The local classifier gives up labeling time and little else. It is tied to the record's features, so a new field means retraining. At 380 records it may lose to the hand-written table.

OpenAI or Gemini fine-tuning gives up the weights. They cannot run on the Mac, and the vendor can retire the model, as Bedrock just did with Claude 3 Haiku. Every exported run also goes to a third party. That is the failure mode protected config #3 names, so the allowlist decides, not convenience. Neither model is 4-bit or open.

RunPod plus Modal gives up simplicity. The human owns a training script, a GGUF conversion and a deployment. Cold starts cost seconds a call, unmeasured. Modal can change the $30 credit. In return the weights move unchanged to the Mac, Oracle or Hetzner.

Oracle Free gives up reliability. InfoQ reports the June cut came without announcement. Idle endpoints get reclaimed, and two Arm cores run a 1.5B model at a speed I have not measured.

Hetzner gives up $7 to $12.49 a month. It is the plainest always-on CPU host for a 0.4 GB BitNet or a 1 GB Q4 model.

Together or Fireworks serving gives up $3,000 to $6,000 a month. Nobody at this scale should take it.

A from-scratch pretrain gives up $600 to $67,000 and months, for a model weaker than any open 1.5B model available today.

## Sources

Read 2026-09-24.

- Repo: ticket 77; research 75, last section; `docs/harness-spec.md` section 14; map D12; `IMPLEMENTATION.md` on `hydra export`.
- BitNet b1.58 2B4T report, https://arxiv.org/html/2504.12285 ; model card, https://huggingface.co/microsoft/bitnet-b1.58-2B-4T
- Continual Quantization-Aware Pre-Training, https://arxiv.org/abs/2502.11895
- Sub-100M QAT, https://arxiv.org/html/2605.25966v1
- Pythia, table 5, https://arxiv.org/abs/2304.01373
- Together, https://www.together.ai/pricing and https://docs.together.ai/docs/lora-inference
- Fireworks, https://fireworks.ai/pricing , https://docs.fireworks.ai/serverless/pricing , https://docs.fireworks.ai/fine-tuning/fine-tuning-models
- OpenAI, https://developers.openai.com/api/docs/pricing . openai.com/api/pricing returned 403.
- Vertex AI, https://cloud.google.com/vertex-ai/generative-ai/pricing
- Mistral, https://docs.mistral.ai/capabilities/finetuning/ . mistral.ai/pricing showed no fine-tuning rates.
- Bedrock, https://docs.aws.amazon.com/bedrock/latest/userguide/prov-throughput.html and https://docs.aws.amazon.com/bedrock/latest/userguide/model-card-anthropic-claude-3-haiku.html
- Modal, https://modal.com/pricing
- RunPod, https://www.runpod.io/pricing
- Vast.ai, https://vast.ai/pricing/gpu/RTX-3090 . The live page showed no offers; the prices are from search snippets of vast.ai.
- Lambda, https://lambda.ai/pricing
- Replicate, https://replicate.com/pricing
- Hugging Face, https://huggingface.co/pricing and https://huggingface.co/docs/hub/spaces-overview
- Cloudflare, https://developers.cloudflare.com/workers-ai/platform/pricing/ and https://developers.cloudflare.com/workers-ai/features/fine-tunes/loras/
- Oracle, https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm ; InfoQ, https://www.infoq.com/news/2026/07/oracle-cloud-free-tier-limits/ . oracle.com/cloud/free returned 403.
- Hetzner, https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/
- Colab FAQ, https://research.google.com/colaboratory/faq.html . Compute-unit figures are secondary: https://techcrunch.com/2022/09/29/google-colaboratory-launches-a-pay-as-you-go-option-premium-gpu-access/ and http://mccormickml.com/2024/04/23/colab-gpus-features-and-pricing/
