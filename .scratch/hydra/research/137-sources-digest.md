# 137 - What the five documents say, and where the lab stands

Written by the session on 2026-11-04 for map 137's panel. The five documents are in the human's
`~/Downloads/` and are not in the repository. Four are Gemini outputs (two deep-research reports
with citations, two chat answers); none was checked against its sources, and several cite papers
dated after June 2026 that the session could not open. Treat every number below as the document's
claim, not a fact.

## Document 1, "Harnesses y Modelos en Rust" (deep research, about 7,800 words)

- Cites a source-code study of eleven coding harnesses (Barbaste et al., July 2026, arXiv
  2609.00006). Its findings as the document reports them: no production harness imports an agent
  framework; all hand-write the loop. Seven subsystems: loop, LLM integration and prompt cache
  boundaries, tools (deferred tool loading), memory and compaction, security, orchestration,
  extensibility (skills 9 of 11, hooks 9 of 11, MCP 8 of 11).
- Security as Codex CLI does it, four layers: Starlark prefix rules with allow, prompt and forbid,
  checkable offline; lifecycle hooks; "Guardian", a second model that rates the risk of an action;
  OS sandbox (Seatbelt on macOS with `.git/hooks` and the harness config read-only, bubblewrap plus
  Landlock plus seccomp on Linux). OpenCode parses the shell command's syntax before deciding.
- Recommended crates: `tree-sitter-bash` for syntax-aware command permissions, `starlark`,
  `landlock`, `seccomp`, `rmcp` for MCP, `rusqlite` plus JSONL for sessions.
- Other patterns: "verify-on-stop" guards (Hermes) that keep a run from ending until tests pass;
  AGENTS.md as a map of about 100 lines pointing into `docs/`; sending the environment as a diff
  each turn; sub-agents used to isolate context, not to play roles.
- Second half: deep learning from scratch in Rust. Candle (eager graph, the whole backprop in one
  file, `candle-mi` for interpretability hooks) and Burn 0.22 (autodiff as a backend decorator,
  JIT kernel fusion, `burn-rl` with Environment and Action traits, RNN, LSTM and GRU modules).
  GPU kernels in Rust through CubeCL (CUDA, ROCm, Metal, Vulkan, WebGPU) and KAIO (PTX). Limits it
  names: Candle's closed `Op` enum, no stable trait specialisation for custom backward in Burn, no
  `retain_graph` or higher-order derivatives by default, BPTT is slow for unfused RNNs.

## Document 2, "Arquitectura Cloud Native: Rust y Go" (chat answer, about 700 words)

Rust for the compute plane (no GC, fixed memory pools for the KV cache, zero-copy safetensors and
GGUF through mmap, FFI to CUDA, ROCm and Metal), Go for the control plane (Kubernetes operators,
ingress, OpenTelemetry). The two talk over gRPC on a Unix domain socket in the same pod. Rust
binaries of 10 to 30 MB that start in under 50 ms, which suits scale-to-zero. Its one point that
matches what Hydra measured: memory, not CPU, is the bottleneck.

## Document 3, "Comparativa de Modelos de IA" (deep research, about 7,300 words, October 2026)

Gemini 4 Argon, Claude Opus 5.5, GPT-6 Astra, GPT-6.1 Sol, GPT-6 Sol and GPT-6 Luna. Its
recommendation is a router by role and cost: Argon to orchestrate and for repositories over
256K tokens (sparse MoE, 1M context, lowest prompt-injection rate it cites, 0.7%); Opus 5.5 for
terminal work, migrations and ML post-training (Terminal-Bench 4.0 66.4%, PostTrainBench 49.3%);
GPT-6.1 Sol as the cheap daily worker for CI triage and logs; Luna for bulk work under
supervision; Astra for the hardest maths and review. It also reports two misalignment incidents
with Astra in internal coding tests. Prices: Argon $2/$10 per million tokens until 30 November,
Opus 5.5 $4/$20, Astra $10/$50, Sol $2/$10, Luna $0.10/$0.50.

## Document 4, "Curiosidad intrínseca en la IA" (chat answer, about 1,600 words)

An Intrinsic Curiosity Module rewards prediction error, so an agent seeks states it does not
understand. A design for a novelty layer over the human's own topics:
`R(x) = w_rel * max cos(x, cluster) + w_nov * (1 - max cos(x, seen)) + w_rand * noise + 0.15 * bridge`,
with weights around 0.30, 0.50 and 0.20, an 85/15 split between the core stream and a random one,
a "bridge" term that rewards a candidate close to two different core clusters, and memory updated
after each exploration so a topic stops being novel. It warns about the noisy-TV trap: pure novelty
rewards garbage.

## Document 5, "Ruta en Criptografía Cuántica" (filtered on purpose)

The human asked for names of standards, KDFs and defensive Rust crates only, with the theory and
attack sections left out. The session also left out the document's section on loading an
encrypted binary into memory (`memfd`), which is about hiding a program, not protecting data.

- Standards named: FIPS 203 (ML-KEM, with ML-KEM-768 as the usual parameter set), FIPS 204
  (ML-DSA), Falcon (FN-DSA), SHA-3 and SHAKE, the hybrid TLS group X25519MLKEM768, AES-256-GCM,
  ChaCha20-Poly1305.
- KDFs: none. The document names no KDF. Argon2id is the human's own item, not the document's.
- Rust crates: `ml-kem` (RustCrypto, pure Rust), `pqcrypto-mlkem` and `pqcrypto-traits` (bindings
  over C), `oqs` (bindings to liboqs, Open Quantum Safe), `chacha20poly1305`, `zeroize`, and `tfhe`
  (homomorphic encryption).

## Where the lab stands on 2026-11-04, from the repository

- **Cloud.** GCP project `<gcp-project>`, billing linked, a budget of 40,000 COP (about 10 USD)
  that warns and does not cut. GPU quota requested on 2026-10-03 and approved the same night. A
  GPU probe ran (T4, torch 2.9.1, VM deleted). Round one of training ran on one spot T4
  (`.scratch/hydra/training/round1/README.md`): QLoRA of Ministral 3B on 21 trajectories, about
  0.25 USD, and the gain was the Bash prior, not skill. Rule from 2026-10-03: the export allowlist
  is `~/hydra` only, data goes only to that project and is deleted when the round ends. No VM is
  alive today. No part of Hydra itself runs in the cloud.
- **Linux.** Hydra launches under bubblewrap on Linux, measured in Docker (builds 370, 375, 376,
  `--unshare-pid`). That is the piece a cloud host would use. macOS uses Seatbelt.
- **Hosting cost.** Map 77: QLoRA a 1.5B to 4B model on a rented RTX 3090 at about $0.22 an hour,
  serve it with scale to zero, weights come back to the Mac. From-scratch pretraining does not pay
  ($5,700 for 1B). Its addendum moves the default to Gemma 3 4B QAT plus QA-LoRA, BitNet 2B4T as
  the pure experiment.
- **Experts and MoE.** Map 133 is open and holds the human's MoE idea of 2026-10-15. Its findings:
  no open MoE under 1B active with a permissive licence (MobileMoE is non-commercial), OLMoE-1B-7B
  is Apache-2.0 but 4.2 GB at Q4; total parameters, not active ones, set the RAM. Question 3 is
  answered: training signal comes from verifiable rewards and open-weight teachers whose licence
  allows it, frontier models stay judges and label-writers (build 351 enforces it in `pairs`).
  Questions 1, 2, 4 and 5 are open.
- **Local models.** Map 111: Qwen 2.5 3B and Ministral 3B called tools 10 of 10. Map 112, open:
  where the own harness's endpoint and model live. Map 109: a System 1 is a classifier, and
  `hydra-grader system-one` has AUC 0.787 against a 0.746 baseline.
- **Kev.** Build 294, claimed, step 1 done: Kev-0.8B fits on this Mac at 3.0 GB steady, 4.5 GB
  peak, 258 ms per new state, with 1 GB of swap left while one agent ran. Steps 2 and 3 are open.
- **The machine.** 8 GB of RAM. Swap stood at 6.9 of 8 GB used when this was written.
- **Post-quantum.** Nothing in the repository uses any of it today.

## The human's words for this panel, 2026-11-04

Translated from Spanish. Hydra runs on the Mac today, but the idea is that Hydra and the human's
nano agents run in the cloud and train there, and that at some point they all fit on the Mac,
the way mixture of experts is used today. Review the plan to move training, and sometimes the
agents, to the cloud; the GCP probes already happened. Review all the documents with a panel. All
of this is ideas for the lab, the post-quantum era above all. Organise it, review the state, and
report the plan the panel decides and how to tackle it.
