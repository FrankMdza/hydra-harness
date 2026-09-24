# 77 - A small model of Hydra's own: what job, where it trains, where it serves, at what price

Type: research
Status: resolved
Blocked by:
Opened: 2026-09-24, by the session, at the human's request after ticket 75's local-model note.

## Question

The human wants a model of Hydra's own, small, at 4 bits, trained on what Hydra records, and the
machine that runs Hydra (an M2 with 8 GB, swap already at 3 GB with the agents alive) is the
bottleneck for both training and serving. Colab is out for serving (ticket 75's note: 12-hour
runtimes, and the FAQ forbids web services and proxies). So: which cheap online services would
train a 4-bit model from scratch or fine-tune one at low bit width on Hydra's exported runs, which
would host the result as an endpoint the mode can call, and which platform's own fine-tuning and
inference (Together, Fireworks, OpenAI, Google, Anthropic, Mistral, or others) would do both without
us owning a GPU? Priced per month at this human's scale: hundreds of runs, a corpus of 380 records
today, one user in Medellín paying in USD.

Two jobs are on the table and they need different answers: first, the mode's classifier and
router (task class and grade, provider from a table), which needs no generation; second, a
generating model that could sit on the mode's planning or judge panel, which does. D12 keeps
training out of Hydra's scope; this ticket asks where it would happen, not builds it.

## Method

One Opus subagent with web search, writing `research/77-a-local-model-for-the-mode.md`: prices
and limits from the vendors' own pages, dated; GPU-hours for a from-scratch low-bit pretrain at
100M, 300M and 1B parameters from published reports; one recommended path per job with a monthly
cost, and what the human gives up by taking it.

## Answer

Resolved 2026-09-24 by an Opus subagent (139k tokens, 76 tool calls, every price from the
vendor's page with the date, gaps named), in
[research/77](../research/77-a-local-model-for-the-mode.md).

**Job 1, the classifier and router, needs no model host.** A gradient-boosted or logistic
classifier on record features trains and runs in Hydra's own process on the Mac, $0 a month; text
embeddings, if they help, are one cent a month at OpenAI's or Cloudflare's rate.

**Job 2, a generating model, is not worth fine-tuning yet.** 380 unlabeled records teach a 1.5B
model less than a prompt does. When the corpus justifies it: QLoRA a 1.5B to 4B open model on a
rented RTX 3090 (RunPod, $0.22 an hour, a few dollars a run), serve it from Modal with scale to
zero inside the $30 monthly credit, $0 expected and about $5 in a bad month, and the weights come
back to the Mac. The no-GPU alternative is a gpt-4.1-nano fine-tune at $1 to $3 a month, weights
kept by OpenAI. Together and Fireworks train cheaply but serve a fine-tune only on a dedicated
H100, $2,900 to $5,800 a month. Anthropic is out: Claude 3 Haiku was the only fine-tunable Claude
and Bedrock retires it on 2026-09-10.

**From scratch at low bit width does not pay.** A 1B pretrain at Pythia's token count is about
$5,700 of rented A100 time and would not write Rust; a BitNet 2B4T-class pretrain is about
$67,000 by a 6ND estimate, since Microsoft published no GPU-hours.

**Two corrections to ticket 75's note.** Oracle's Always Free tier was halved to 2 OCPU and 12 GB
on 2026-06-15 and reclaims idle machines, so it is not a host for an endpoint that idles between
calls. And nothing has ever been exported from Hydra, because `config/export-allowlist.json` does
not exist; the trainer's input is a file Hydra has not yet written.

Nothing in the mode depends on any of this, as ticket 75's judge asked. The one build that
follows is the classifier of job 1, and it waits for the mode's own log to carry enough human
verdicts to label with.

## Addendum, the native low-bit path

Asked by the human after the answer: QLoRA quantizes a base after training and freezes it, which
is the "compress it the dumb way" they wanted to avoid, and the merged adapter is usually
re-quantized a second time for serving. The path that keeps their intuition is quantization-aware:
a base whose weights were trained under the quantizer, plus adapters that merge into the same
integer grid. Two bases exist to download today: BitNet b1.58 2B4T, pretrained from scratch at
1.58 bits, and Google's Gemma 3 QAT checkpoints (1B, 4B, 12B, 27B at int4), finished with
quantization in the loop. On top of either, QA-LoRA (quantization-aware LoRA, Xu et al. 2023)
trains adapters that fold into the int4 weights without a second quantization; on BitNet, plain
LoRA over the ternary base with bf16 shadow weights, on the same rented 3090. "Nothing lost" is
not literally true on either: QAT loses less than post-training quantization, not nothing. The
recommendation moves to Gemma 3 4B QAT plus QA-LoRA as the default, BitNet 2B4T as the pure
experiment, same budget as the QLoRA line.
