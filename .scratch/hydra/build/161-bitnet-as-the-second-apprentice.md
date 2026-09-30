# 161 - BitNet as the second apprentice, measured on this machine

Type: build
Status: resolved
Blocked by: 160

## What

Map ticket 87 gives the local model a bank of tasks about records. The model today is
`gemma3:1b-it-qat` on Ollama. The human downloaded `~/models/BitNet` on 2026-09-24:
Microsoft's `bitnet.cpp` with a `build/` directory and the open weights of BitNet b1.58 2B 4T
(`~/models/BitNet/models/BitNet-b1.58-2B-4T`, 1.8 GB in all). Map ticket 77 decided that
training a BitNet from scratch (about 67,000 USD) is not worth it; running the released one on
the CPU costs nothing, and the human asked on 2026-09-30 that BitNet be trained too.

This ticket measures before anyone trains: does the 2B ternary model run on the M2, how fast,
in how much memory, and how does it score on the apprentice bank against the 1B Gemma.

## Do

1. **Run it.** Find the built binary under `~/models/BitNet/build/` (or say exactly what
   `cmake` step is missing and stop). Measure on the M2: tokens per second at generation for a
   200-token answer, resident memory, and the load time. Three runs each, report the median.
   Nothing here needs the network.
2. **Serve it the way the apprentice expects.** `apprentice.py attempt` talks to an Ollama
   `generate` endpoint. Add to `integrations/apprentice/` a small stdlib HTTP shim,
   `bitnet_serve.py`, that answers `/api/generate` with `{"response": ...}` and `/v1/chat/completions` in the
   OpenAI shape, both by running the `bitnet.cpp` binary with the prompt, so `HYDRA_LOCAL_LLM_URL`
   can point at it and a later harness can too (map ticket 88). Test it
   against a stub binary.
3. **Score it on the bank.** With the real bank from build 160, run `attempt` once with Gemma
   and once with BitNet (separate `HYDRA_LOCAL_MODEL` names so the attempts stay apart) and
   print `score` for both. That is the number the human wants.
4. **What training would take.** One page in `integrations/apprentice/README.md`, from
   Microsoft's own repository and model card only: whether the released 2B4T can be fine-tuned
   (which weights, bf16 master or ternary), what the recipe needs, and a cost estimate at the
   rental prices of research 85 and the compute seat of 2026-09-29 (a 3090 at about 0.22 USD an
   hour). No training in this ticket.

## Done when

- The three measurements with their medians; the shim with its test; both scores on the real
  bank if Ollama and the binary run, or the reason one of them did not; the training page with
  sources.
- Nothing under `src/` changes. The report starts with what could not be measured.

## Outcome

Resolved 2026-09-30. Built by `claude` sonnet on run 01a0f246-232e under `standard`. The session
applied it as written. `integrations/apprentice/bitnet_serve.py` is a stdlib shim that answers
`/api/generate` and `/v1/chat/completions` with one binary call per request, one request at a
time; 5 tests against a stub binary. The README gained the measurements and the training page.

What the ticket assumed that turned out false: the binary under `~/models/BitNet/build/bin` is
mainline llama.cpp b9918, not the bitnet.cpp kernels. It reads the model as `Q1_0`, aborts on
Metal (`Asserting on type 36`) and prints `@@@@` for every prompt on the CPU. So the 3.1 tokens a
second, 1.52 GB peak and 9.4 s load are a floor for a broken run, and neither model has a score on
the bank: BitNet's would measure the bug, and Ollama was not running for Gemma's. The shim has
never talked to a working binary. Next step, before any score: rebuild through the checkout's
`setup_env.py`. Training costs in the README are the run's own estimates (LoRA on the bf16 master
weights, about 2.20 USD on a 3090); the 0.22 USD an hour is the ticket's figure, unchecked.
