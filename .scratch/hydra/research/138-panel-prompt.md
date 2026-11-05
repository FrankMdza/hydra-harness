# Decision panel for map ticket 138

You are one seat on a decision panel. Read only: do not edit, create or delete a file in the
repository, and do not commit. Scratch experiments under `/tmp` are fine. Your seat name is in your
task line. The human is present and decides on the panel's recommendation, so commit to one option
per question and say why. Where a question is the human's by an earlier answer, say what you
recommend and mark it as the human's.

## Read

1. `.scratch/hydra/issues/138-nano-llms-of-our-own-from-scratch-in-rust.md`, the human's words.
2. `.scratch/hydra/research/138-nano-from-scratch.md`, the research, read on 2026-11-04.
3. `.scratch/hydra/issues/137-the-lab-s-next-arc-cloud-experts-and-post-quantum.md` (its Answer)
   and builds `.scratch/hydra/build/381-*.md` and `383-*.md`, the plan this has to fit beside.
4. `.scratch/hydra/issues/133-tiny-experts-from-recorded-runs.md`, the answered question 3 on
   training signal.
5. `crates/pyjson/` and the grader's differential tests, the house pattern for two
   implementations checked against each other.

## Questions

1. **When.** (A) After round two (build 383): the adapter first, then a 10M to 100M model from zero
   on one narrow domain as a separate cheap experiment. (B) Now, and the from-scratch model
   replaces 383's base. (C) Not at all; only adapters or chosen layers on open bases.
2. **Framework.** (A) Burn for training and serving. (B) A small PyTorch reference trains the same
   model, and the Burn version must match its loss curve within a stated tolerance before any
   result counts, the `pyjson` pattern. (C) Candle.
3. **Retraining chosen parts, which technique first.** Chosen layers trained with the rest frozen,
   LoRA, model editing (ROME, MEMIT, AlphaEdit), or weight-space programming by a frontier model
   as in the short. Say what each would mean on a model the lab trained itself.
4. **The teacher.** The research reads three uses of "retraining from the big LLMs": evaluation,
   distillation of outputs, and a frontier model writing weight diffs. Map 133 settled the first
   two in part. Recommend, and mark what is the human's. Note Anthropic's Usage Policy of
   2026-11-12 as the research quotes it.
5. **The first domain and its verifiable reward.** Name one narrow domain the lab owns data for
   and a reward a program can compute, so no frontier text is needed. Candidates the session sees:
   predicting a run's outcome from its record, classifying a shell command as the gate would, or
   finding non-post-quantum crypto in a repository (map 137's option D). Pick one or name better.
6. **The first tickets**, at most three, smallest first, each with what it measures and the
   dollar ceiling.

Check at least two claims of the research against its sources or the repository and say which.

At most 900 words. End with `## Pending from me`, `## Changed` and `## Found`.
