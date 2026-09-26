# 81 - Jev, and a judge panel whose seats differ even on one model

Written by `composer-2.5` on run 01a0d756-7074 under `standard`, 2026-09-25, about 0.74M tokens,
and saved by the session as the run returned it. The session checked three claims against their
sources the same day: Jev's price and context on docs.typesafe.ai/models, the three question
types on docs.typesafe.ai/api, and the nine-judge paper's n_eff of 2.18 and mean phi of 0.391 on
arXiv 2605.29800. All three hold. One internal error: section 2 says export redaction covers
"four secret classes". It redacts four ways, and since build ticket 128 across twelve classes.
The placeholder link in the summary is the agent's; the file it means is `docs/rungs-spec.md`.

## Summary

Jev is TypeSafe AI's hosted System One model: it takes `state` plus typed `noul`, `choice`, or `score` questions and returns probabilities and (for choice/score) a `confidence` field, with no generated text ([TypeSafe API reference](https://docs.typesafe.ai/api), [TypeSafe models page](https://docs.typesafe.ai/models)). Vendor claims put it at 70–500 ms and $0.042 per million input tokens with free output ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev)); independent tests find it competitive on short binary tasks but dataset-dependent on calibration, with no published RLCD paper ([Aman Kumar](https://amankumar.ai/blogs/jev-measured), [DEV Community review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln)). For Hydra, any Jev call over record text is harness spec 14 export; a triage gate that only routes diffs to the full panel is not map 76-I substitution, but replacing a judge seat or shrinking the panel is ([`docs/rungs-spec.md`](https://github.com/...)). LLM judge panels carry far fewer independent votes than their seat count ([arXiv:2605.29800](https://arxiv.org/html/2605.29800v1)); role, evidence, and prompt diversity help but do not restore provider diversity. RLCD could train a local classifier only after labelled outcomes exist; the grader schema lacks per-item judge labels and calibrated targets ([`integrations/grader/SCHEMA.md`](integrations/grader/SCHEMA.md)).

---

## 1. Jev (Typesafe AI)

### What it is

Jev is the first public "System One" model from TypeSafe AI, founded by Diogo Almeida (formerly OpenAI; credited with InstructGPT/RLHF work, not the original RLHF paper) ([Wikipedia](https://en.wikipedia.org/wiki/Jev_(AI_model)), [TechCrunch](https://techcrunch.com/2026/09/18/a-new-kind-of-ai-model-from-a-chatgpt-inventor-is-thrilling-developers/)). It launched in limited early access on 2026-09-15 with a $40M seed round led by DCVC ([Forbes via Wikipedia](https://en.wikipedia.org/wiki/Jev_(AI_model))). It does not generate natural-language text; it returns typed decisions for software consumption ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev)).

TypeSafe describes it as transformer-based, trained on synthetic data with RLCD (Reinforcement Learning for Calibrated Decisions), optimizing for "epistemically honest probabilities" rather than human preference (RLHF) or verifiable answers (RLVR) ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev)). No RLCD paper, reward formulation, or weights have been published ([learnjev.com RLCD page](https://learnjev.com/concepts/rlcd), [System One Models architecture guide](https://systemonemodels.org/guides/jev-architecture/)).

### API and typed output

**Endpoint:** `POST https://api.typesafe.ai/v1/systemone` with `Authorization: Bearer <API_KEY>` ([API reference](https://docs.typesafe.ai/api)).

**Request shape:** `state` (string, JSON object, or array of text), `model` (e.g. `jev-latest` → `jev-1.13.0`), and a `questions` map ([API reference](https://docs.typesafe.ai/api)).

**Question types** ([API reference](https://docs.typesafe.ai/api), [Wikipedia](https://en.wikipedia.org/wiki/Jev_(AI_model))):

| Type | Purpose | Answer shape |
|------|---------|--------------|
| `noul` | Yes/no | `noul`: float 0–1 (probability of yes) |
| `choice` | Pick one of up to 255 options | `choice`, `probabilities` map, `confidence` 0–1 |
| `score` | Ordered 2–10 levels | `score`, `legend`, `probabilities`, `confidence` |

**Example response** (from independent testing; vendor docs match the shape):

```json
{
  "model": "jev-1.13.0",
  "answers": {
    "sentiment": {
      "type": "choice",
      "choice": "negative",
      "probabilities": {"negative": 0.97, "positive": 0.03},
      "confidence": 0.93
    }
  },
  "usage": {"input_tokens": 336}
}
```

([Aman Kumar measured blog](https://amankumar.ai/blogs/jev-measured))

TypeSafe states that `confidence` for choice/score is "derived from the answer's probability distribution" but does not publish the formula ([API reference](https://docs.typesafe.ai/api)). Independent work notes it is not always equal to the top probability ([Aman Kumar](https://amankumar.ai/blogs/jev-measured)).

### Access

- **2026-09-15:** Limited early access and waitlist ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev), [Wikipedia](https://en.wikipedia.org/wiki/Jev_(AI_model))).
- **2026-09-16:** Available on Vercel AI Gateway as `typesafe-ai/jev` ([The Next Web](https://thenextweb.com/news/typesafe-jev-decision-model-vercel-fastest-adopted)).
- **2026-09-21:** Waitlist removed; open signup with $5 credit reported ([Julian Goldie blog](https://juliangoldieaiautomation.com/blog/jev-waiting-list/)).
- Also routed via OpenRouter (`typesafe/jev-1.13`) and Cloudflare Workers AI per third-party summaries ([Julian Goldie blog](https://juliangoldieaiautomation.com/blog/jev-waiting-list/)).

TypeSafe states Jev is not fine-tuned per customer; customization is via `state`, `instructions`, and `criteria` ([models page](https://docs.typesafe.ai/models)).

### Pricing (vendor)

| Item | Value | Source |
|------|-------|--------|
| Input | $0.042 / million tokens ($42 / billion) | [Models page](https://docs.typesafe.ai/models) |
| Output | Free | [Models page](https://docs.typesafe.ai/models) |
| Rate limits | 250,000 tokens/s, 1,200 req/min (may change) | [Models page](https://docs.typesafe.ai/models) |
| Context | 64k total; 32k for state + longest question | [Models page](https://docs.typesafe.ai/models) |

Launch post notes pricing sustainability is unproven long-term ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev)).

### Latency

**Vendor:** 70–500 ms end-to-end from US West Coast ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev), [Wikipedia](https://en.wikipedia.org/wiki/Jev_(AI_model))).

**Independent:**

| Measurement | Result | Source |
|-------------|--------|--------|
| Median per call (4 public datasets, 20 concurrent) | 0.8–0.9 s | [Aman Kumar](https://amankumar.ai/blogs/jev-measured) |
| Single call from laptop | 850 ms | [Aman Kumar](https://amankumar.ai/blogs/jev-measured) |
| Server-side (minus network) | ~76–105 ms cited in review | [DEV Community review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) |
| vs frontier LLMs | 1.04×–12.1× faster depending on comparator | [DEV Community review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) |

### RLCD

**Vendor claim:** RLCD rewards calibrated decisions whose stated probabilities match outcome frequencies ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev)).

**What is not published:** Training objective, calibration metric used in training, ablations, or a paper ([learnjev.com](https://learnjev.com/concepts/rlcd), [DEV Community review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln)). CEO reportedly said on Hacker News the architecture is "close to the chest" and a paper may come later ([DEV Community review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln)).

### Independent evaluations (vendor vs measured)

**Vendor workflow evals** ([evals.typesafe.ai](https://evals.typesafe.ai/)):

- Reference labels = average of GPT-6 Astra and Claude Fable 5.1 at high thinking, not ground truth ([evals site](https://evals.typesafe.ai/)).
- Jev reported at 67.8% agreement vs frontier leader 74.1% ([DEV Community review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) citing eval board).
- Peak speed/cost claims (193.6× faster, 444.6× cheaper) come from these four in-house workflows; TypeSafe calls them "on the higher end of real world gains" ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev)).

**Independent accuracy:**

| Study | Finding | Source |
|-------|---------|--------|
| Aman Kumar, 16k calls, 4 public + private sets | 91–99% on short tasks when confident; 76% on Banking77 (77 options); beats small LLMs on 3/4 sets; loses on long/fuzzy tasks | [amankumar.ai](https://amankumar.ai/blogs/jev-measured) |
| TrueStandard, 108 hand-labelled grounding claims | 96.3% accuracy vs 94.4% Flash Lite; ECE 0.066 vs 0.061 (tie at n=108) | [truestandard.ai](https://truestandard.ai/blog/jev-accuracy-tested) |
| Janardhan, 200 items, 4 datasets | 72.5% accuracy; ECE 0.161 (highest of 6 models) | [DEV Community review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) citing [jev-frontier-bench](https://github.com/manjunathshiva/jev-frontier-bench) |
| Ibrahim & Zaki, 7,977 human labels | Median 11.6 macro-F1 points behind best LLM per task | [arXiv:2609.24574](https://arxiv.org/abs/2609.24574) via [DEV review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) |
| Phishing bench | 62.6% vs 81.3% Haiku 4.5; 5 narrow nouls + logistic regression → 95% | [amankumar.ai](https://amankumar.ai/blogs/jev-measured) citing jev-phishing-bench |
| Spam, 18,514 emails | 98.33%, level with TF-IDF logistic regression | [DEV review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) citing [bitnovus/jev-spam-eval](https://github.com/bitnovus/jev-spam-eval) |
| Vercel adoption | ~13% of paid teams in 24h (gateway telemetry, not accuracy) | [The Next Web](https://thenextweb.com/news/typesafe-jev-decision-model-vercel-fastest-adopted) |

**Independent calibration:**

| Study | Finding | Source |
|-------|---------|--------|
| TypeSafe | No published ECE, Brier, or reliability curves | [learnjev.com/calibration](https://learnjev.com/concepts/calibration) |
| Pre-registered 8,576 responses | ECE 0.0204 on CLINC150; ECE 0.0936 on Banking77 | [learnjev.com/calibration](https://learnjev.com/concepts/calibration) |
| Bespoke 13-subset suite | Median ECE 0.071 (Jev) vs 0.114–0.180 (Gemma 4 variants) | [DEV review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) |
| Temperature scaling on 50–300 labels | ECE drops sharply (e.g. civil_comments 0.16–0.21 → <0.025) | [DEV review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln) |

**Vendor-known weaknesses** (literal reading, no reliable math/counting, large irrelevant state, adversarial content, no generation): [jev-1.13 jaggedness doc](https://docs.typesafe.ai/model-jaggedness/jev-1.13).

---

## 2. Where Jev could sit in Hydra

Harness spec 14: export is the only path off-machine; default deny; allowlist scopes by repo paths; redaction covers four secret classes, not codebase shape ([`docs/harness-spec.md` §14](docs/harness-spec.md)). `config/export-allowlist.json` does not exist yet; nothing has been exported ([ticket 77 research](.scratch/hydra/research/77-a-local-model-for-the-mode.md)).

Map 76-I / rungs spec: the panel never substitutes a judge; all configured judges must return parseable verdicts; `panel_size_unshrunk` is a critical checklist item ([`docs/rungs-spec.md` §4.6](docs/rungs-spec.md), [`src/rungs/review.rs`](src/rungs/review.rs)). D5: only the run record and skill store cross harness and Rungs ([map D5](.scratch/hydra/map.md)).

### Candidate A: Pre-filter before the judge panel

| Dimension | Assessment |
|-----------|------------|
| **Input** | Redacted diff + task text (or summary), exported via `hydra export` or a bounded slice written for the probe |
| **Typed output** | `noul` per checklist item ("does this diff touch tests?"), or one `noul` for "safe to skip full panel" |
| **Replaces or adds** | Adds a routing step; does not replace a seat if low-confidence cases still go to all seats |
| **Data leaving machine** | Full diff + task text to `api.typesafe.ai` unless export allowlist/redaction is wired first |
| **Spec 14** | Fails closed without human-authored allowlist and export path |
| **Map 76-I** | **Not substitution** if it only decides which diffs reach the full panel and every diff that passes still gets all seats ([ticket 81 constraint](.scratch/hydra/issues/81-jev-and-distinct-judges.md)). **Substitution** if a confident "pass" skips any seat or replaces `parse_judge_answer` output |
| **D5** | OK if the gate reads exported data and writes only to the goal log or orchestrator state outside the record; a verdict in the run record would need a contract decision |

Jev's jaggedness doc recommends filtering large state before the call ([jaggedness #5](https://docs.typesafe.ai/model-jaggedness/jev-1.13)), which fits a diff-review gate if the diff is trimmed to checklist-relevant hunks in Hydra's process first.

### Candidate B: Router / classifier (ticket 77 job 1)

| Dimension | Assessment |
|-----------|------------|
| **Input** | Record features: tokens, duration, refusals, outcome, provider, optional embeddings of `task_text` ([ticket 77](.scratch/hydra/issues/77-a-local-model-for-the-mode.md)) |
| **Typed output** | `choice` over task_class (7), grade (4), provider table |
| **Replaces or adds** | Replaces nothing in rung two; adds mode routing |
| **Data leaving machine** | Jev path: `task_text` (+ features as JSON in `state`) to TypeSafe. Local sklearn path: $0, no export ([ticket 77 research](.scratch/hydra/research/77-a-local-model-for-the-mode.md)) |
| **Spec 14** | Jev route needs export; local classifier does not |
| **Map 76-I / D5** | No panel interaction; no D5 crossing if labels stay in goal log |

Ticket 77 already recommends gradient-boosted or logistic regression on the Mac at $0/month before any hosted model ([ticket 77 answer](.scratch/hydra/issues/77-a-local-model-for-the-mode.md)).

### Candidate C: Closed-shape verdicts for `parse_judge_answer`

`parse_judge_answer` expects a single JSON object: `verdict` ∈ {`pass`,`block`}, `findings[]` with `item`, `severity`, `note` against a fixed checklist ([`src/rungs/review.rs`](src/rungs/review.rs), [`integrations/puppets/puppets.py`](integrations/puppets/puppets.py)).

| Dimension | Assessment |
|-----------|------------|
| **Input** | Diff + checklist definitions in `state`; many atomic `noul` questions per checklist item |
| **Typed output** | Jev nouls per item; Hydra assembles `{verdict, findings}` in Rust/Python |
| **Replaces or adds** | Would replace LLM judge **generation** of the JSON, not the seat itself, if a full provider run still occurs for audit |
| **Data leaving machine** | Entire diff + checklist to TypeSafe |
| **Spec 14** | Export required |
| **Map 76-I** | **Substitution** if a Jev-assembled answer fills a provider slot without that provider's run ([rungs spec "never substitutes"](docs/rungs-spec.md)). **Not substitution** if Jev is a pre-step and each seat still runs and may disagree |
| **D5** | Judge answers today live in provider runs; moving verdict authority to an external API without a record event breaks the audit trail unless E is revisited |

Jev cannot emit the `note` string field; Hydra would template notes from item id + severity, losing judge prose ([jaggedness #9: no generation](https://docs.typesafe.ai/model-jaggedness/jev-1.13)).

### Candidate D: RLCD as training recipe for ticket 77 local model

See section 4.

---

## 3. Distinct judges on one model

### Published work on LLM-as-judge panels

| Paper | What it measured | Finding |
|-------|------------------|---------|
| Zheng et al., "Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena" ([arXiv:2306.05685](https://arxiv.org/abs/2306.05685)) | GPT-4 as judge vs humans on MT-Bench | >80% agreement with humans; documents position, verbosity, self-enhancement biases and mitigations |
| Kim et al., "Correlated Errors in Large Language Models" ([arXiv:2506.07962](https://arxiv.org/abs/2506.07962), [MLR v267](https://proceedings.mlr.press/v267/kim25e.html)) | 350+ LLMs on leaderboards + hiring task | Models agree ~60% of the time when both err; shared provider/architecture increases correlation; LLM-as-judge inflates accuracy of correlated models |
| "Nine Judges, Two Effective Votes" ([arXiv:2605.29800](https://arxiv.org/html/2605.29800v1)) | 9 frontier judges, 7 families, 3 NLI datasets | Effective sample size ≈2; mean error correlation φ≈0.39; panel 8–22 pp below independent-vote accuracy; best single judge matches or beats full panel; prompt/temperature/CoT variants do not fix it |
| "Agreement Overstates Evidence" ([arXiv:2609.22512](https://arxiv.org/abs/2609.22512)) | 10 judges, error dependence | Avg pairwise error correlation 0.21; 10 judges ≈ 3.5 independent; up to 28% of significance claims reverse when dependence is modeled |
| CARE ([arXiv confounder-aware aggregation](https://arxiv.org/html/2603.00039v1)) | 12 benchmarks, multi-judge aggregation | Shared confounders (verbosity, etc.) bias majority vote; confounder-aware aggregation cuts error up to 26.8% |
| "Judging with Many Minds" ([arXiv:2505.19477](https://arxiv.org/html/2505.19477v2)) | Multi-agent debate vs meta-judge | Debate **amplifies** position, verbosity, CoT, bandwagon biases after round 1 |
| DEBATE ([ACL 2024 Findings](https://aclanthology.org/2024.findings-acl.112.pdf)) | Devil's Advocate + Scorer + Commander | Critic role reduces groupthink; adversarial persona improves evaluation consistency |
| CalibraEval ([ACL 2025](https://aclanthology.org/2025.acl-long.808.pdf)) | Pairwise LLM judges | Position/ID-token swap bias; label-free calibration (NOA) mitigates |
| BayesPE ([ACL 2024 Findings](https://aclanthology.org/2024.findings-acl.728.pdf)) | Prompt ensembles | Weighted ensemble over semantically equivalent prompts improves calibration on small validation sets |
| EL-DGR ([arXiv:2608.07813](https://export.arxiv.org/pdf/2608.07813)) | Evidence-locked judge routing | Judge override allowed only with extractive evidence certificate; reduces judge-caused regressions |
| Stopping and Routing Judge Panels ([arXiv:2608.19802](https://arxiv.org/pdf/2608.19802.pdf)) | Panel composition | Second prompt from same family can be a "conditional copy" with zero marginal gain after the first |

**Takeaway for Hydra:** Different providers are weakly less correlated than same-family pairs in the nine-judge study (φ 0.437 same-OpenAI vs 0.389 cross-family mean), but the highest-correlation pairs are cross-family (Claude×Gemini φ≈0.603) ([arXiv:2605.29800](https://arxiv.org/html/2605.29800v1)). Running codex three times with different prompts is unlikely to yield three independent seats.

### What makes seats more independent (literature)

1. **Different evidence subsets:** EL-DGR and evidence-locked routing tie overrides to extractive certificates ([arXiv:2608.07813](https://export.arxiv.org/pdf/2608.07813)). PROClaim uses progressive RAG so judges see expanding, non-identical evidence ([arXiv:2603.28488](https://arxiv.org/html/2603.28488)).
2. **Adversarial roles:** DEBATE's Devil's Advocate opposes the Scorer ([ACL 2024](https://aclanthology.org/2024.findings-acl.112.pdf)); "Confident Liar" uses Constructor vs skeptical Auditor with different log-prob trajectories ([ACL SRW 2026](https://doi.org/10.18653/v1/2026.acl-srw.121)).
3. **Prompt ensembles:** BayesPE weights multiple instruction paraphrases ([ACL 2024](https://aclanthology.org/2024.findings-acl.728.pdf)); Zheng et al. mitigate position bias by swapping order ([arXiv:2306.05685](https://arxiv.org/abs/2306.05685)).
4. **Calibration / debiasing:** CalibraEval for selection bias ([ACL 2025](https://aclanthology.org/2025.acl-long.808.pdf)); CARE for shared confounders ([arXiv:2603.00039v1](https://arxiv.org/html/2603.00039v1)).

### Application to Hydra's puppets panel

Current seats: `claude` (opus, judge), `codex` (standard), `cursor-agent` (composer-2.5, judge) ([`integrations/puppets/puppets.py`](integrations/puppets/puppets.py)). With codex rate-limited, two seats remain on different binaries but the panel cannot reach its configured quorum.

**Same model on multiple seats** (e.g. three codex runs with different prompts):

- Can remain **meaningful** if seats see **different evidence** (diff-only vs diff+tests vs diff+checklist slice), **different roles** (proposer-defender vs devil's-advocate vs security), and **non-overlapping prompts** calibrated on a held-out set ([DEBATE](https://aclanthology.org/2024.findings-acl.112.pdf), [BayesPE](https://aclanthology.org/2024.findings-acl.728.pdf)).
- Will **not** restore rungs-spec provider diversity, which is explicit design for rung two ([`docs/rungs-spec.md`](docs/rungs-spec.md)).
- Must still produce four (rung two) or three (puppets) **parseable** answers; `panel_size_unshrunk` forbids verdicts with fewer seats ([`src/rungs/review.rs`](src/rungs/review.rs)).
- Correlated errors mean majority vote gains are capped near ~2 effective votes regardless of seat count ([arXiv:2605.29800](https://arxiv.org/html/2605.29800v1)).

Map 76-I answer: the mode panel inherits rung two anti-substitution rules; shrinking from three to two when one provider is limited is **not** allowed without a map change ([ticket 76](.scratch/hydra/issues/76-master-of-puppets-decisions.md)).

---

## 4. RLCD against ticket 77's plan

### Ticket 77 plan (job 1)

A gradient-boosted or logistic classifier on record features (tokens, duration, refusals, outcome, review corrections), $0/month on the Mac; optional embeddings at ~$0.01/month ([ticket 77](.scratch/hydra/issues/77-a-local-model-for-the-mode.md), [research/77](.scratch/hydra/research/77-a-local-model-for-the-mode.md)). Labels come from human verdicts in the mode log; grader `dataset.jsonl` carries `task_class`, `grade`, `human_verdict` when a goal log names the run ([`integrations/grader/SCHEMA.md`](integrations/grader/SCHEMA.md)).

### Could a calibrated-decision objective train Hydra's local model?

**Conceptually yes, with labels.** RLCD's stated goal is probabilities that match empirical correctness ([launch post](https://typesafe.ai/blog/introducing-system-one-models-and-jev)). That maps to:

- **Binary targets:** pass/block, applied/rejected, outcome buckets → train with proper scoring rules (log loss / Brier) plus post-hoc temperature scaling ([Guo et al. 2017](https://arxiv.org/abs/1706.04599), cited in [DEV review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln)).
- **QLoRA path (job 2):** A generative model could be fine-tuned to emit verbalized probabilities, but ticket 77 defers that until the corpus is larger ([ticket 77](.scratch/hydra/issues/77-a-local-model-for-the-mode.md)).

**RLCD specifically:** Without a published objective, "train with RLCD" means either (a) replicate the **API pattern** (typed questions + softmax over label tokens) on an open model ([Schick & Schütze 2021 verbalizers](https://arxiv.org/abs/2001.07676), [GPT-3 multiple-choice scoring](https://arxiv.org/abs/2005.14165)), or (b) wait for TypeSafe's method. The DEV review notes open replicas do not replicate RLCD because no method exists publicly ([DEV review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln)).

### What the grader dataset lacks

| Needed for calibrated classifier / RLCD-style training | Present in `dataset.jsonl`? |
|--------------------------------------------------------|----------------------------|
| Ground-truth label per decision | Partial: `human_verdict`, `outcome`, `grade` when `labels_source=puppets` |
| Enough labelled rows per class | No: ~380 records, 84–112 routing cells → ~4 records each ([ticket 77](.scratch/hydra/issues/77-a-local-model-for-the-mode.md)) |
| Per-checklist-item judge findings | No |
| Diff or code features for review calibration | No (only redacted `task_text`) |
| Probability targets / historical confidence | No |
| Export-eligible training corpus | No: allowlist never written ([ticket 77 research](.scratch/hydra/research/77-a-local-model-for-the-mode.md)) |
| Provider-quota / rate-limit features for routing | Partial: `tokens`, `duration_s`; no `rate_limit_event` projection in schema |
| Holdout by time / goal for calibration fitting | No defined splits in schema |

**Practical path:** Collect human verdicts in the goal log → export allowlist → extend grader or a separate calibration set → sklearn + Platt/temperature scaling on the Mac ([ticket 77](.scratch/hydra/issues/77-a-local-model-for-the-mode.md), [Aman Kumar threshold advice](https://amankumar.ai/blogs/jev-measured)). QLoRA with a proper scoring loss waits on labels and GPU budget ([ticket 77 addendum](.scratch/hydra/issues/77-a-local-model-for-the-mode.md)).

---

## 5. Recommendations

Costs are order-of-magnitude at hundreds of runs/month and ~380 records; Jev per-call cost is sub-cent at typical diff sizes if exported ([Aman Kumar](https://amankumar.ai/blogs/jev-measured), [models page](https://docs.typesafe.ai/models)).

| Candidate | Recommendation | Cost of choice |
|-----------|------------------|----------------|
| **A. Pre-filter before panel** | **Try in a bounded probe** after export allowlist exists; gate only "defer to full panel," never skip a seat | **Probe:** human time to write allowlist + ~$5 credit covers millions of tokens ([Julian Goldie](https://juliangoldieaiautomation.com/blog/jev-waiting-list/)); miscalibrated gate risks false passes ([learnjev.com/calibration](https://learnjev.com/concepts/calibration)). **Skip** if export is not ready |
| **B. Jev as router/classifier** | **Skip for now**; ticket 77's local sklearn path is $0 and needs no export | **Adopt Jev later:** API spend + ongoing export compliance; **opportunity cost** of skipping free local baseline |
| **C. Jev replaces `parse_judge_answer` generation** | **Wait** (or **skip**) | Replacing a seat violates map 76-I unless every provider still runs; loses `note` prose; full diff export; correlated errors if same API called thrice |
| **D. RLCD trains local model** | **Wait** on labels and published RLCD; **try** label-probability training via sklearn + temperature scaling now | **Wait cost:** months without router; **try cost:** labeling labor, no GPU spend |
| **Distinct judges, same model** | **Try in bounded manual pilot**: split evidence + adversarial roles per seat; log disagreements in goal log | **Cost:** 3× quota on one provider; effective independence stays ~2 votes ([arXiv:2605.29800](https://arxiv.org/html/2605.29800v1)); false confidence if roles are cosmetic ([arXiv:2505.19477](https://arxiv.org/html/2505.19477v2)) |
| **Distinct judges, provider diversity** | **Adopt when quotas allow** (status quo design) | **Cost:** four provider bills; highest independence Hydra's design buys ([`docs/rungs-spec.md`](docs/rungs-spec.md)) |

**Immediate path while codex is out:** Keep three seat slots in the log but accept the panel is blocked until quorum returns, **or** run the same binary with different evidence/role prompts on the two available providers and treat the third seat as `parse_error` → block (fail closed per `parse_judge_answer`). Do **not** silently shrink to two seats ([`docs/rungs-spec.md`](docs/rungs-spec.md), [`src/rungs/review.rs`](src/rungs/review.rs)). Document that correlated-error literature predicts limited gain from prompt-only diversity ([arXiv:2605.29800](https://arxiv.org/html/2605.29800v1)).

---

## Source list

**TypeSafe (vendor)**

- [Introducing System One Models & Jev](https://typesafe.ai/blog/introducing-system-one-models-and-jev)
- [Models and pricing](https://docs.typesafe.ai/models)
- [HTTP API reference](https://docs.typesafe.ai/api)
- [Workflow evals](https://evals.typesafe.ai/)
- [Jev 1.13 jaggedness](https://docs.typesafe.ai/model-jaggedness/jev-1.13)
- [Documentation index](https://docs.typesafe.ai/llms.txt)

**Press and reference**

- [TechCrunch, 2026-09-18](https://techcrunch.com/2026/09/18/a-new-kind-of-ai-model-from-a-chatgpt-inventor-is-thrilling-developers/)
- [Wikipedia: Jev (AI model)](https://en.wikipedia.org/wiki/Jev_(AI_model))
- [The Next Web on Vercel adoption](https://thenextweb.com/news/typesafe-jev-decision-model-vercel-fastest-adopted)
- [Vercel AI Gateway model page](https://vercel.com/ai-gateway/models/jev)
- [InfoWorld, 2026-09-17](https://www.infoworld.com/article/4223468/typesafe-ais-new-models-work-with-machines-not-humans.html)

**Independent Jev evaluations**

- [Aman Kumar, jev-measured](https://amankumar.ai/blogs/jev-measured)
- [TrueStandard, 108 claims](https://truestandard.ai/blog/jev-accuracy-tested)
- [DEV Community, eight-day review](https://dev.to/gde/jev-after-eight-days-of-independent-tests-level-with-mid-price-llms-behind-the-frontier-1kln)
- [learnjev.com: calibration](https://learnjev.com/concepts/calibration)
- [learnjev.com: RLCD](https://learnjev.com/concepts/rlcd)
- [systemonemodels.org: architecture](https://systemonemodels.org/guides/jev-architecture/)
- [Janardhan jev-frontier-bench](https://github.com/manjunathshiva/jev-frontier-bench)
- [Ibrahim & Zaki, arXiv:2609.24574](https://arxiv.org/abs/2609.24574)
- [onlyoneaman/jev-eval](https://github.com/onlyoneaman/jev-eval)

**LLM-as-judge and panel independence**

- [Zheng et al., arXiv:2306.05685](https://arxiv.org/abs/2306.05685)
- [Kim et al., arXiv:2506.07962](https://arxiv.org/abs/2506.07962)
- [Nine Judges, arXiv:2605.29800](https://arxiv.org/html/2605.29800v1)
- [Agreement Overstates Evidence, arXiv:2609.22512](https://arxiv.org/abs/2609.22512)
- [CARE, arXiv:2603.00039v1](https://arxiv.org/html/2603.00039v1)
- [DEBATE, ACL 2024](https://aclanthology.org/2024.findings-acl.112.pdf)
- [Bias in multi-agent LLM-as-judge, arXiv:2505.19477](https://arxiv.org/html/2505.19477v2)
- [CalibraEval, ACL 2025](https://aclanthology.org/2025.acl-long.808.pdf)
- [BayesPE, ACL 2024](https://aclanthology.org/2024.findings-acl.728.pdf)
- [EL-DGR, arXiv:2608.07813](https://export.arxiv.org/pdf/2608.07813)
- [Stopping and Routing Judge Panels, arXiv:2608.19802](https://arxiv.org/pdf/2608.19802.pdf)
- [Confident Liar, ACL SRW 2026](https://doi.org/10.18653/v1/2026.acl-srw.121)

**Calibration methods (cited for local training)**

- [Guo et al., On Calibration of Modern Neural Networks](https://arxiv.org/abs/1706.04599)
- [Schick & Schütze, verbalizers](https://arxiv.org/abs/2001.07676)

**Hydra internal (context only; not web sources)**

- [Ticket 81](.scratch/hydra/issues/81-jev-and-distinct-judges.md)
- [Ticket 77](.scratch/hydra/issues/77-a-local-model-for-the-mode.md)
- [Ticket 76](.scratch/hydra/issues/76-master-of-puppets-decisions.md)
- [Research 77](.scratch/hydra/research/77-a-local-model-for-the-mode.md)
- [`docs/harness-spec.md` §14](docs/harness-spec.md)
- [`docs/rungs-spec.md` §4.6](docs/rungs-spec.md)
- [`integrations/grader/SCHEMA.md`](integrations/grader/SCHEMA.md)
- [`src/rungs/review.rs`](src/rungs/review.rs)
- [`integrations/puppets/puppets.py`](integrations/puppets/puppets.py)