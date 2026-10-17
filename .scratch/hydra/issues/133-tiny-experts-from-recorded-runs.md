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
3. **Where the training signal comes from.** The three vendors' terms forbid training a model
   that competes with theirs on their outputs; the apprentice's preference pairs come from a
   panel's grading of a local model's attempts, which is evaluation. The human's position of
   2026-10-17 is below. State which source feeds the first model: verifiable rewards with no
   teacher text, an open-weight teacher whose licence permits it, frontier outputs as labels and
   judge only, or frontier outputs as training text with the account risk accepted.
4. **Where it connects.** The fog item says an MCP server declared in `config/mcp.md`, admitted per
   policy, whose calls land as `mcp__` tool calls. Map 88 says a Rust child harness with four
   gated tools. Which of the two is the first body for a tiny model, and what does the record
   carry about the model's state that it does not carry today.
5. **The 2010 PC.** A serving constraint only: SSE4.2, no AVX2, 4 GB. Does the plan hold it as a
   hard requirement for the first model, or as the target for the model after the first.

Resolved by measurement where a seat can measure (builds 344, 345, 346) and by the human where it
is a choice (1, 3, 5).

## The human's input, 2026-10-17

The human read the plan's legal paragraph and answered that the goal is not a model that competes
with any vendor. It is access: micro-processes every person should have in the next years of AI,
a security-report process and anti-hacking checks among them, because automated attacks expose
people to systems they should not have to pay to be protected from. A licence of our own, in the
human's view, does not change that goal, and the terms should not stop it: "skip baby".

What the session put beside that, for the human to weigh when resolving question 3:

- The clause is a contract term, not a law. "Compete" is the vendor's word and the vendor reads
  it, by closing the account. A licence on Hydra's side changes nothing, because the restriction
  sits in the agreement the account accepted, and none of the three clauses has an exception for
  non-commercial or public-interest use. The consequence is not a lawsuit. It is the three accounts
  the router runs on, and a public README that says the models are distilled from Claude or GPT is
  the line a reader at those companies stops at.
- The goal does not need frontier text. Three sources are clean. Verifiable rewards, which is the
  MiMo recipe in the video: a fixture with a known CVE, open port or exposed key, and the reward
  is whether the report names it. Open-weight teachers whose licence allows training on their
  outputs: MiMo-V2.6 Pro and DeepSeek under MIT, Qwen3 and OLMoE under Apache-2.0. Public security
  data no vendor owns: CISA KEV, NVD, MITRE ATT&CK, which the other session's KEV work already
  reads. Frontier models stay as judges and label-writers, which is what the panel does today.
- One grey spot is already in the code. `pairs.rs` puts the panel's `better_answer`, text a
  frontier model wrote, into `chosen`. The clean version keeps `chosen` as the local model's best
  attempt among several, graded by the panel, and never the panel's rewrite. That is a small build
  once this question is resolved.
