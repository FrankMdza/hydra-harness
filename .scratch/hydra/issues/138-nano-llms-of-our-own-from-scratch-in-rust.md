# 138 - Nano LLMs of our own, from scratch in Rust, with chosen parts of the weights retrained

Type: research
Status: open
Blocked by:
Opened: 2026-11-04, by the session, from the human's answer to map 137 question 2.

## The human's words

Translated from Spanish: "the idea I had was to create our own nano LLMs from zero, under a
training system of few shots and retraining from the big LLMs, and use something like this
<https://youtube.com/shorts/jNuDdqEwI58> to retrain the parts, the weights of each model at the
nano level, the way we want. Would it be possible? All in Rust, using Burn or whatever framework
they use at SpaceAI."

The session could not open the short (no network from its shell). The research run opens it and
says what it shows. "SpaceAI" is the human's word; the research says which lab it most likely
means and what that lab has published about its training stack, and does not guess past that.

## What this must not redo

- Map 77: from-scratch pretraining at 1B is about $5,700 and does not pay; QLoRA of a 1.5B to 4B
  model on a rented GPU does. This ticket asks about models far smaller than 1B, which map 77 did
  not price.
- Map 133 question 3, answered by the human on 2026-10-21: training signal comes from verifiable
  rewards and from open-weight teachers whose licence permits training on their outputs; frontier
  models stay judges and label-writers. "Retraining from the big LLMs" has to be read against
  that answer. If the human's new words change it, the research says so and the panel puts it
  back to the human, it does not decide it.
- Map 137: training happens on spot GPUs in `<gcp-project>`, nothing stays after the round, and
  the Mac has 8 GB with Kev at 3 GB.

## Research questions

1. What the short shows, named as a technique with its paper or repository.
2. What a model from 10M to 300M parameters, trained from zero on a narrow domain, can do. Cite
   published small models (TinyStories-class, SmolLM-class, nanoGPT-class runs) with their
   token counts and the GPU-hours they report, and turn that into dollars on a spot T4, L4 or
   A100 in GCP at today's price.
3. The techniques for retraining chosen parts of a model's weights: adapters, training chosen
   layers or experts with the rest frozen, model editing (ROME, MEMIT and their successors),
   sparse fine-tuning, pruning and regrowth. For each: what it changes, what it costs, and
   whether it works at nano size.
4. Rust: what Burn, Candle and any other Rust framework can train today, on CUDA in the cloud and
   on Metal on the Mac. Whether one Burn codebase trains on a cloud GPU and runs the result on the
   Mac. What is missing, with issue links.
5. The lab's frontier-model question in plain terms: which of the human's "retraining from the
   big LLMs" is evaluation and labelling, which is distillation of outputs into weights, and which
   teachers are open-weight with a licence that allows it.

## Output

`research/138-nano-from-scratch.md`, at most 2,500 words, every number with its source and the
date read, gaps named. Then a panel under `research/138-panel-prompt.md`.

## Panel

Sat 2026-11-04, three seats (Grok not seated, Cursor login). Synthesis in
`research/138-synthesis.md`. Waiting on the human.

## Correction from map 140, 2026-11-04

Research 140 opened the short. It is Better Stack's retelling of MIT CSAIL's "Sparks of
Weight-Space Programming", confirmed by the video's own description. Research 138's "failed on the
one model under 300M" holds only for improving Gemma 3 270M at math. On networks under 10M
parameters the editor read, fixed and wrote working weights with no training. See
`research/140-two-videos.md` section 1.
