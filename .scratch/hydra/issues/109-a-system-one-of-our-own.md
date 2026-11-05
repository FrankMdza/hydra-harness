# 109 - A System 1 of our own: Jev, OpenJev, JEPA, and where they fit Hydra

Type: research
Status: resolved
Blocked by:
Opened: 2026-10-04, by the session, from the human's words of 2026-10-03: extend Hydra's flows
with Jev (Typesafe AI's "System 1" model, which returns typed values with a probability and a
confidence; the human has free credits and an API key), check whether an "OpenJev" exists, and
find how Hydra could have its own System 1, possibly through JEPA (joint-embedding predictive
architectures), even though Typesafe uses another technique (RLCD, map 81). A fourth question, about one of the
author's private repositories, is withheld.

## Question

1. **Jev today.** What the API offers as of October 2026 (endpoints, typed outputs, latency,
   price, limits), and where in Hydra a typed, calibrated, cheap answer replaces an LLM call:
   the panel's pre-filter, the router of map 77 job 1, the grader's labels, the closed-shape
   verdicts. Rank by value. Map 81's answer is the start.
2. **OpenJev and peers.** Is there an open Jev or an open model of the same kind (typed,
   calibrated, small)? Name each with link, licence, size and what it can run on an 8 GB M2 or
   a single rented T4.
3. **Our own System 1.** What it would take to train one on Hydra's own records (the export now
   yields 350 lines): which technique (RLCD as Typesafe describes it, JEPA-style predictive
   embeddings, a plain classifier head on a small model, distillation from Jev's answers), what
   data, what compute, and what it would answer. Be concrete about what JEPA is and is not good
   for here; it is a representation-learning recipe, not a decoder.
4. [withheld: a question about a private repository]

## Answer

Resolved 2026-10-04 by run `01a1056c-6f06` (`claude-fable-5-1`, `permissive`, web, 0.83M tokens);
the report is `.scratch/hydra/research/109-a-system-one-of-our-own.md`, every outside claim with its link.

- **Jev today**: one endpoint, `jev-1.13.0`, 0.042 USD per million input tokens, output free.
  Its best use in Hydra is the order in which diffs are read (build 134 measured AUC 0.964, Brier
  0.332): rank, never a vote or a threshold. Nothing in the repository can call it since build
  220 dropped `jev ask`; reviving it reverses map 94 and sends text off the machine, which the
  export allowlist does not cover. Both are the human's: map 110.
- **OpenJev exists** (MIT, a frozen Qwen3.5-4B, no training code), and so do mini-jev, Laya
  (ModernBERT 421M, Apache-2.0, runs on this Mac) and others; none reproduces RLCD, which is
  still unpublished.
- **Our own System 1**: Hydra already has a small one, `crates/grader/src/baseline.rs`. What it
  lacks is labels, not a technique. The path is a logistic head with temperature over embeddings
  of the diff and the task, trained on this Mac; JEPA is a representation recipe and would add
  nothing at 350 to 550 examples, so it waits for the phase-3 LoRA. Builds 288 and 289.
- [withheld: the answer about a private repository]

**2026-10-04, Kev.** The "Kev-9B" this answer could not verify is real: Kev 1.0, announced
2026-10-01, four open decision models from 0.8B to 27B under Apache-2.0 with Jev's interface,
the smallest stated to run on any Apple Silicon Mac. The note is
`.scratch/hydra/research/109b-kev.md`; build 294 measures Kev-0.8B on this machine beside
`system-one`.
