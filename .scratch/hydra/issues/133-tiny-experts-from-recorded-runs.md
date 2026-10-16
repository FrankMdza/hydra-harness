# 133 - Tiny experts from recorded runs: which small model, what its router does, and what one GPU-hour buys

Type: research
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from the human's words of that night: mixture-of-experts models
that activate the fewest parameters, a way to see which experts handle which domain, reinforcing
specific experts instead of adding noise, tiny local models for very specific cases connected
through Hydra, minimal open weights on CPU, a PC from 2010 onward. The plan that frames it is
`.scratch/hydra/research/133-the-plan-of-2026-10-15.md`; four recorded seats fed it
(`research/133-seat-*.md`). Maps 88, 109, 111 and 112 hold the local-model decisions this one must
not redo, and the map's fog item "a mini agent as a peripheral controller" is the shape this gives
a body.

## Question

1. **Which model.** The seats found no open MoE under 1B active parameters with a licence that
   lets everyone run it: MobileMoE is non-commercial, OLMoE is Apache-2.0 but 6.9B total and 4.2 GB
   at Q4. Is the first version of the idea a dense model under 1B plus a classifier head, which is
   map 109's answer, with MoE deferred until an Apache-2.0 model under 2B total exists? Or is
   OLMoE on this Mac, 8 GB, the right first measurement anyway, because it is the one MoE whose
   experts are documented to cluster by domain?
2. **What the router does on our data.** Build 345's contract `expert_profile`: per layer, which
   experts fire for the apprentice bank's prompts by domain, on CPU, through Transformers'
   `output_router_logits`. The question for the map is what result would make the idea worth a
   GPU-hour and what result kills it: a stated threshold before the run, not after.
3. **What is trainable without breaking anyone's terms.** The three vendors forbid training a
   competing model on their outputs; the apprentice's preference pairs come from a panel's grading
   of a local model's attempts, which is evaluation. State which Hydra data may enter a training
   set at all: records of local-model runs, human corrections, panel verdicts as labels, and
   nothing a frontier model wrote.
4. **Where it connects.** The fog item says an MCP server declared in `config/mcp.md`, admitted per
   policy, whose calls land as `mcp__` tool calls. Map 88 says a Rust child harness with four
   gated tools. Which of the two is the first body for a tiny model, and what does the record
   carry about the model's state that it does not carry today.
5. **The 2010 PC.** A serving constraint only: SSE4.2, no AVX2, 4 GB. Does the plan hold it as a
   hard requirement for the first model, or as the target for the model after the first.

Resolved by measurement where a seat can measure (builds 344, 345, 346) and by the human where it
is a choice (1, 3, 5).
