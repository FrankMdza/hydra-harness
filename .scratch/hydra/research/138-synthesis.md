# 138 synthesis: nano models of our own, from zero, in Rust

Panel of 2026-11-04 on `research/138-nano-from-scratch.md`. Seats under `standard`, read only,
from `research/138-panel-prompt.md`:

- GPT, codex `gpt-6-astra`, run `01a1a865-9948`, memo `138-seat-gpt.md`.
- Gemini, agy `gemini-3.8-flash-high`, run `01a1a866-4715`, memo `138-seat-gemini.md`.
- Opus, `claude-opus-5-5`, run `01a1a868-1fc0`, memo `138-seat-opus.md`.
- Grok not seated: Cursor's login still fails (`auth_failed` on maps 134 and 137).

Every question went three of three, so no judge sat. The human decides.

## What the session checked

- Gemini's reward names `policies/standard.yaml` and a "`tree-sitter-bash` evaluator". The policy
  is `policies/standard.json`, and no tree-sitter evaluator exists yet; map 137 only put it on the
  map. The vote does not depend on either, because the other two seats name the compiled gate
  and `tests/gate.rs`, which do exist.
- GPT read Burn #6212 and found that PyTorch shares the high-confidence gradient failure. So
  matching PyTorch does not prove the loss is right, and GPT adds an independent float64 check
  of the loss at extreme logits. The session takes that into ticket 1.
- Opus recomputed every cost figure in the research from the quoted spot prices, and they match.
- Nobody opened the YouTube short. The research's identification of it, MIT CSAIL's
  weight-space programming, is an inference from the thumbnail.

## The answers

1. **When: A, three of three.** After round two (build 383), a model from zero of 10M to 100M on
   one narrow domain, as a separate experiment. B would reverse the human's map 137 answer and
   put a pre-1.0 framework under the round that has to beat the always-Bash baseline.
2. **Framework: B, three of three.** Burn trains and serves. A small PyTorch reference trains the
   same model with the same initial weights, data order and seed in float32, and Burn must match
   its loss within a tolerance written before the run. This is the `crates/grader/tests/differential.rs`
   pattern with a tolerance in place of byte identity. Plus GPT's float64 check of the loss,
   because both frameworks may share #6212.
3. **Retraining chosen parts: chosen layers with the rest frozen, three of three.** Then LoRA on
   the same data, compared against it (Opus). Model editing (ROME, MEMIT, AlphaEdit) only for a
   single correction. Weight-space programming is not taken: on Gemma 3 270M it did nothing, and
   it fixes a failure the model already has rather than teaching one.
4. **The teacher.** Evaluation by frontier models, yes, as map 133 allows. Distillation only from
   open-weight teachers whose licence permits it (DeepSeek-R1 under MIT is the one licence read that
   names it). With a program-computed reward, no teacher text is needed. **The human's:** whether
   frontier-written labels may be trained on, and whether a frontier model may write weight diffs.
   All three recommend no on both. Opus's reason for the second: Anthropic's Usage Policy,
   effective 2026-11-12, forbids using outputs "to train an AI model (e.g., ... 'model
   distillation') without prior authorization", and the router runs on those accounts.
5. **The first domain: classifying a shell command as the gate would, three of three.** Hydra owns
   the labeller: the compiled gate fed synthetic payloads, as `tests/gate.rs` does, with
   `tests/corpus/` for held-out attacks. Labels are free and exact, and nothing leaves the
   machine. All three state the limit. A model that copies the gate inherits its blind spots, so
   it tests the pipeline and does not make a better gate. The deterministic gate stays
   authoritative. Outcome prediction was rejected for having too few runs, and the post-quantum
   finder for needing a scanner nobody has written.
6. **Tickets.** The seats agree on three, in this order:
   1. Burn against a PyTorch reference, 10M, on the Mac, CPU first. Per-step loss agreement over
      2,000 steps, the float64 loss check, Metal against CPU logits on 100 inputs (#5962), and
      whether Burn can freeze parameters. Corpus of gate verdicts split by command family with
      adversarial compositions held out. $0, local, after build 381 because both use the Mac's
      memory.
   2. The gate model from zero on one spot L4: held-out agreement, false-allow rate and recall on
      `tests/corpus/`, thresholds stated before. $5. Blocked by 1 and 383.
   3. Frozen layers against LoRA, the new class being a second labeller's verdicts: gain on the
      new class, forgetting on the old held-out set. $5. Blocked by 2.

   GPT keeps the three under 10 USD together, cleanup included.

## What the human decides

- 1 to 3 and 5 as voted.
- Question 4's two items: frontier labels trained on, and frontier weight diffs.
- Whether build tickets 384 to 386 are opened as above.
