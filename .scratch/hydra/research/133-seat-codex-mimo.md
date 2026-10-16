# Xiaomi MiMo and Hydra

## Verification boundary

I read `CLAUDE.md`, relevant implementation notes in `IMPLEMENTATION.md`, tickets 88, 109, 111 and 112, and `integrations/apprentice/README.md`. Those establish the local findings below.

External verification was blocked. Hydra refused the primary-source lookup with `hydra/unknown-tool: webrun is not in Hydra's capability table for this provider`. My batch handling hid the first refusal, and I repeated the lookup before seeing it. Both calls were refused. I stopped external research and used no alternative route.

Consequently, this is an incomplete research memo. I cannot supply verified primary-source URLs or treat the transcript as proof of a release. Numbers below are attributed only to the supplied transcript, not independently established outside claims.

## What the video claims

The transcript identifies MiMo V2.6 Pro and Flash. It describes Pro as a mixture-of-experts model with one trillion total parameters and 42 billion active parameters, grouped-query attention, a 128-token local window, six local layers per global layer, and a million-token context. I could not verify architecture, Flash parameter counts, licence, weights, technical report, Hugging Face card or GitHub repository. I cannot establish pretraining token counts, data composition or multi-token prediction.

The claimed process adds agent-focused midtraining and large-scale RL. Each update samples 1,568 prompts with 16 trajectories each, or 25,088 attempts. Dynamic sampling excludes groups with uniformly successful or failed outcomes. Verifiers establish success, and group-wise grading redistributes advantage among successful trajectories. Environment sanitization, network isolation and adversarial agents seek reward exploits. Partial rollouts limit waiting, with trajectories discarded beyond four steps of staleness. These are transcript claims, not verified descriptions of Xiaomi's implementation.

The transcript quotes 30 RL steps, about 123 hours for Pro and 82 for Flash, and costs of $2.62 million and $854,000. It separately gives longer elapsed durations. It attributes 43.5 percent of Pro compute to updates, 43.8 percent to rollouts and 12.7 percent to grading. These are presented as RL costs, not demonstrated total pretraining costs. I could not verify their accounting basis, actual rewards, code test construction or curricula.

It also claims distillation into a 9B model using 77.4 billion tokens, and release of roughly 3,000 coding, 1,000 cybersecurity, 1,000 general and 2,000 visual tasks, plus a framework, mini harnesses and evaluation machinery. I could not verify any downloadable artifact, format, dependencies, licence or release status. A task count does not establish that executable environments or hidden tests were published. I therefore cannot say their environments run on this Mac.

## What Hydra can reuse

The following are engineering inferences from the transcript and verified local interfaces, not recommendations to reproduce an unverified Xiaomi stack.

Use the apprentice's existing `bank build` and `bank.jsonl` as the task source. Its exact, numeric and alternative-answer checks already provide objective rewards. Keep independent tasks from separate source runs for evaluation so related excerpts cannot leak across training and testing.

Add repeated attempts experimentally. Current `attempt` uses temperature zero and skips tasks already answered for a model and prompt hash, so it does not produce the diverse groups the video describes. A future sampler would need explicit attempt identity, sampling settings and per-task success distributions. Keep uniformly easy and hard tasks in evaluation even if an RL sampler excludes them.

Use `score` to retain per-template accuracy. For open tasks, panel grades already feed `pairs`, which writes TRL-style JSONL containing message lists for `prompt`, `chosen` and `rejected`. Prefer verified correctness before judging clarity or efficiency. The local Gemma judge is not evidence that a grading signal is reliable. Nothing currently trains on these pairs, and `hydra export` does not ingest them.

Ticket 109 already selects a logistic head over embeddings before phase-three LoRA. Do not replace that with frontier RL. Ticket 111 favours Ministral 3B after tool-loop measurements. Ticket 88 fixes a separate Rust child harness with four gated tools. Ticket 112 still reserves endpoint configuration and the driver choice for the human.

## Pending from me

Primary-source verification remains undone: model identity and licence, pretraining and prediction objectives, rewards and curricula, cost accounting, and actual released test artifacts. No Xiaomi environment was downloaded or run. Estimated compute tiers below were not benchmarked.

## Changed

Created only this memo. No source, configuration, model or ticket was changed. Nothing was committed.

## Found

Local record checks, dataset preparation, scoring and a small classifier fit the existing Mac workflow. A bounded LoRA or preference-training pilot on a small model is a candidate for one rented GPU over hours, with duration dependent on tokens and context. Frontier pretraining and the transcript's billions of rollout tokens per update require a cluster. Reuse verification, trajectory comparison and distillation principles; no released Xiaomi test suite or Mac-compatible environment was verified.
