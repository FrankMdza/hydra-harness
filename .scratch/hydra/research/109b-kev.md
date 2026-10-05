# 109b - Kev, read on 2026-10-04

A research run on `agy:gemini-3.1-pro-high` (`01a109c6`), asked by the human. It opened one page,
the announcement at https://jaredpalmer.com/blog/introducing-kev (dated 2026-10-01), and the
earlier note 109. The session checked its figures against the post's text: Apache-2.0, four sizes
from 0.8B to 27B, the pointer head, the shipped temperature, MLX, "any Apple Silicon Mac" for
0.8B and a 32 GB Mac for 4B and 9B all stand as the post states them. It did not open the GitHub
repository or the Hugging Face pages, so the code's shape and the weights' formats are unread.

**One claim the session corrects.** The report says Hydra's `jev.rs` needs only a new endpoint URL.
`crates/grader/src/jev.rs` builds and scores Jev files and has no caller: map 94 did not port
`ask`, and build 220 deleted the Python probe. Calling Kev needs that client written, over
`hydra_records::loopback_http`, which is small but is not a URL change.

**What follows for Hydra, the session's reading.** Kev-0.8B on this Mac through its MLX server,
on loopback, is the cheap first step: no record leaves the machine, the licence allows
fine-tuning on Hydra's records, and its API is Jev's shape (a document, questions, options,
a probability each). Measure it on the 559 test outcomes beside `system-one` (AUC 0.787) before
anything else. Not measured: its memory on 8 GB, and how it does on Hydra's records at all.

**Why this is not JEPA, and why JEPA is not the next step** (the session, 2026-10-05, answering
the human). Typesafe has not published Jev's method (RLCD). The open replicas that come close
(OpenJev at 0.845 modal agreement against Jev's 0.883, mini-jev, Kev) are all a language model
read as a classifier: one pass over the document and the question, the scores of the supplied
options read instead of generated text, a temperature fitted on held-out data; Kev adds a pointer
head and a fine-tune on about 146,000 examples. That Jev is of the same family is an inference
from how close a frozen Qwen gets, not a confirmed fact. JEPA trains an encoder to predict one
view's representation from another's; it yields representations, not typed answers or
probabilities, so the head and the calibration would be needed anyway, and LLM-JEPA's results are
on thousands of pairs where Hydra has a few hundred runs, 70 verdicts and 7 original diffs. The
bottleneck is labels. JEPA stays a possible auxiliary loss for a later fine-tune.

The run's report, as returned:

## Not confirmed

- I could not verify if Kev-9B actually fits on a 32 GB Mac, as the author states they have not measured it yet (https://jaredpalmer.com/blog/introducing-kev).
- I could not verify any direct accuracy or calibration comparison against Jev, because the blog post does not report Kev's performance against Jev (https://jaredpalmer.com/blog/introducing-kev).
- I could not verify exactly how a rented T4 performs with Kev, as the blog only provides benchmarks for Modal's H100, H200, L40S, L4, and Apple Silicon M5 (https://jaredpalmer.com/blog/introducing-kev).

Pages opened today: I opened .scratch/hydra/research/109-a-system-one-of-our-own.md and https://jaredpalmer.com/blog/introducing-kev.

## 1. What Kev is
Kev is an open source family of decision models made by Jared Palmer, VP of Engineering at Cognition (https://jaredpalmer.com/blog/introducing-kev). It adds a pointer head to a language model to score supplied answer options directly instead of generating tokens (https://jaredpalmer.com/blog/introducing-kev). Its inputs are a text document, questions, and possible answers, and its output is a probability for each answer (https://jaredpalmer.com/blog/introducing-kev). It is compatible with TypeSafe's Jev API (https://jaredpalmer.com/blog/introducing-kev). The family has four model sizes: 0.8B, 4B, 9B, and 27B, based on Qwen architectures, with the largest using Qwen3.8-27B (https://jaredpalmer.com/blog/introducing-kev). The models are released under the Apache-2.0 licence (https://jaredpalmer.com/blog/introducing-kev). The weights are hosted on Hugging Face and the code on GitHub (https://jaredpalmer.com/blog/introducing-kev). The 27B model was trained on a corpus of about 146,000 examples and 337,000 questions, combining existing data, public tasks, synthetic decisions, and agent logs (https://jaredpalmer.com/blog/introducing-kev). No Jev outputs were used in training (https://jaredpalmer.com/blog/introducing-kev). It is calibrated by fitting a single temperature scaling factor on held-out datasets (https://jaredpalmer.com/blog/introducing-kev). Kev-9B is indeed the model mentioned in the earlier research note, matching the size, Apache-2.0 licence, and Qwen base (.scratch/hydra/research/109-a-system-one-of-our-own.md).

## 2. How it runs
Kev can run via a server on GPUs or on Apple Silicon through MLX (https://jaredpalmer.com/blog/introducing-kev). It can also be deployed to Modal behind an HTTPS endpoint using a provided skill (https://jaredpalmer.com/blog/introducing-kev). Memory requirements vary: Kev-27B needs an 80 GB GPU for short inputs; Kev-4B and Kev-9B can fit on a 32 GB Mac or an L40S; Kev-0.8B runs on an L4 or any Apple Silicon Mac (https://jaredpalmer.com/blog/introducing-kev). Median server latency ranges from 23 ms for 0.8B to 67 ms for 27B (https://jaredpalmer.com/blog/introducing-kev). Kev-0.8B fits on an 8 GB Mac, as the author states it runs on any Apple Silicon Mac (https://jaredpalmer.com/blog/introducing-kev). A rented T4 GPU has 16 GB of VRAM, which would comfortably fit and run the Kev-0.8B and Kev-4B models, though specific latency for the T4 is not provided in the post.

## 3. Integration options for Hydra
Most practical first:
1. Call it over a loopback server: You can run Kev-0.8B locally via MLX on the Mac, or Kev-4B on the rented T4. Since Kev uses the exact same API as Jev (https://jaredpalmer.com/blog/introducing-kev), Hydra's existing jev.rs code only needs a change in the endpoint URL. This takes very little Rust code changes. It costs local RAM, which might strain the 8 GB Mac, or T4 rental time. What breaks is reliability if the local Python server crashes.
2. Distil it into Hydra's own small head: You can run Kev-4B on the T4 to score Hydra's historical diffs and run records, saving the probabilities as soft labels. Then train Hydra's baseline.rs logistics on these labels. This takes Python scripting to query the T4 and process the output, but keeps the Rust binary lightweight. It costs a few dollars in T4 time. What breaks is if Kev's decisions on Hydra's specific domain are inaccurate.
3. Port its method without its weights: You can replicate Kev's pointer head architecture and prefix caching in Rust. This takes significant machine learning engineering to implement and train a small model from scratch. It costs extensive compute time. What breaks is the model's accuracy, as it would lack Kev's large training corpus.
4. Load it from Rust: You can try to load Kev's weights directly in Rust using candle or ort. However, Kev uses a custom pointer head layer that scores options directly (https://jaredpalmer.com/blog/introducing-kev). Standard Rust inference libraries might not support this architecture out of the box. This takes writing custom tensor operations in Rust. It costs high development time. What breaks is compatibility with future Kev updates.

## 4. Licence and fine-tuning
The Apache-2.0 licence permits commercial use, modification, and distribution (https://jaredpalmer.com/blog/introducing-kev). This allows you to fine-tune Kev on Hydra's records and ship the resulting model without legal restrictions.

## 5. What Kev does differently
Kev processes the document once and caches that computation, so every subsequent question branches off that single cached read without seeing the other questions (https://jaredpalmer.com/blog/introducing-kev). This is highly worth copying for Hydra's system-one. Hydra often needs to answer multiple questions, like task class and outcome, about the same lengthy run record. Caching the state representation would save significant compute compared to processing the whole record from scratch for every question.

## Pendiente de mí
- The ticket 109 needs its answer section written by the session, as I did not edit any files directly.
- Decide whether to use Kev via a loopback server, distill it, or port its prefix caching method to system-one.

## Cambiado
- Nothing was edited or committed. I created a temporary python script to parse the blog post, which is a scratch file inside the worktree.

## Encontrado
- Kev matches the Jev API exactly, meaning the existing jev.rs code could be used to call it.
- Kev uses a pointer head and caches the document prefix, returning probabilities without generating tokens.
- The 0.8B model runs on any Apple Silicon Mac, making it a viable local option for an 8 GB machine.
