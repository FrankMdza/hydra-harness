# Map 118, seat GPT: adjudicating the two memos

Written 2026-10-13. This is a panel recommendation for the human's decision, not a resolution of the grilling. I checked the Opus and Gemini memos against repository files; I did not run cargo, tests, training, or a benchmark. Counts reported by other sessions remain attributed measurements, not counts I independently reproduced.

## 1. What is the training example, and how many delegations exist?

**Recommendation:** pair the goal and repository context available at planning time, pinned to `base_sha`, with the proposed plan JSON. Attach proposal selection, human approval, downstream task results, verification evidence, and retained artifacts as separate labels. Use outcome-supported plans for the initial success-training set; distinguish proposal preference from execution success. Split evaluation by goal, keeping sibling proposals and retries together.

**I side with Opus on the example's shape and verdict provenance, and Gemini on requiring downstream evidence before calling a plan successful. Neither establishes a minimum corpus size.** The deciding files are `crates/puppets/src/fold.rs`, `crates/grader/src/labels.rs`, and `.scratch/hydra/build/288-the-set-of-original-diffs.md`.

The fold records the goal's text/repo/base SHA, proposal run IDs, `plan_ready` artifact paths, panel selection, human plan acceptance, tasks, and human task results. These are useful join keys, not a complete training row. In `labels_for_goal`, builder runs receive `human_result`, but `plan_proposer` and `plan_judge` labels receive a null result. Opus's claim that the needed join already exists is too strong: role tagging exists; attributing downstream outcomes to a particular plan still needs work. `src/export_preference.rs` exports interactive corrections and rung-two review pairs, not Puppets proposal comparisons.

Gemini is right that selection alone does not verify execution. However, `crates/puppets/src/write.rs::verdict` requires a diff approval before recording `applied` or `fixed`; the logs contain more than conversational approval. That function records a human verdict, not a successful test execution. Keep repaired work distinct from first-pass acceptance, and do not automatically attribute builder failures to the plan.

Build 288 reports **70 verdicts in the grader's `ticket-verdicts.json`, seven recoverable original diffs, 62 missing branches, and one missing base tree**. `crates/grader/src/queue.rs::ticket_verdicts` and `crates/grader/src/originals.rs` confirm that provenance. Opus is right against both Gemini and the readings here. Missing branches mean missing evidence, not rejected work; Gemini's inference from deleted branches to failed plans is unsupported. Seven is not a count of all successful work or all retained diffs throughout the project.

The supplied readings report **866 records and 26 records carrying `parent_run_id`** on October 13; the ticket reports 826. The 26 count child records, not necessarily unique parent plans or successful delegation trajectories. Puppets records goal-level orchestration separately, so this is not the denominator for the proposed training set. I could not independently recount either store.

An additional correction to Opus: `src/face.rs::call` asks an interactive session's external launcher before checking the seatbelt; without that launcher it refuses delegation inside the seatbelt. The refusal is not unconditional. This does not authorize delegation by this run.

Retained diffs support verification of code-building goals, but Gemini's universal diff requirement would exclude research and review plans, both allowed by `integrations/puppets/prompts/plan.md`. Require checkable outcomes appropriate to each task. Build 290 reports 249 export-eligible runs after its fix, not 249 verified planning examples; its eligible-task content review remains explicitly unfinished.

## 2. What does the model output at inference?

**Recommendation:** draft the existing task-array JSON, including task text, class, grade, dependencies, affected paths or areas, and a checkable `done` criterion. Initially have the copilot and human review it; retain human planning approval and the assignment algorithm. A live proposal seat is a later integration and evaluation step.

**I side with Opus on the existing plan contract and with both seats on keeping assignment in code.** `integrations/puppets/prompts/plan.md` and `crates/puppets/src/write.rs::tasks` decide the output question. The prompt asks for a short narrative followed by one JSON array. Task text is already part of that array; `crates/puppets/src/launch.rs::worker_task_text` passes both text and `done` to the builder. A separate skeleton-only format would require another completion step before import.

Both memos overstate the exact-path requirement. The prompt allows files **or directories**, and asks for files or areas when knowable. The importer requires only `id`, `text`, `class`, and `grade`; missing `deps` and `paths` default to empty arrays and `done` to null. Empty metadata need not be good planning, but Gemini's blanket prohibition on paths has no basis in the contract. Supply repository context and distinguish existing paths from intended new files.

`write.rs::assign` uses `sheet.rs::ranked_assignment`, falls back to defaults, and leaves `agi` tasks for human assignment. Ranking incorporates cost and a lower bound on first-pass acceptance; it is not a generative assignment table.

Both seats identify a real integration gap: `src/provider/mod.rs` routes only four CLIs, and map 112 remains open on the local harness endpoint and model choice. This blocks a native tool-using proposer, not offline experiments with supplied repository context. The local question-answering facility documented in `IMPLEMENTATION.md` is not the missing agent loop. Opus's claim that nothing is trainable before map 112 lands is too broad; Gemini's guaranteed near-100% rejection is an unmeasured prediction. Anonymous proposal comparison measures draft quality, not autonomous authority.

## 3. Is a 3B planner credible, or must round two grow?

**Recommendation:** treat Ministral 3B as an unproven drafting candidate. First measure the untuned model on bounded planning tasks with adequate, identical repository context and a strong planner baseline. Fine-tune if that experiment identifies a useful target and the corpus supports it; compare an 8B candidate if resources permit. Existing results do not justify a sole architect.

**I side with Opus's conditional experiment, not Gemini's asserted 8B minimum. I also reject Opus's claim that a bigger card is already proved necessary.** The deciding evidence is `.scratch/hydra/training/round1/README.md`, its `build_sft.py`, `round1.py`, and `report.json`, plus `.scratch/hydra/issues/111-which-small-model-calls-tools.md`.

Round one used 21 training trajectories and three held-out trajectories. Held-out next-call accuracy rose from 1/9 to 5/9, mostly through a Bash prior; the first-call bake-off moved from 10/10 to 9/10. These measure tool-call behavior, not architectural planning. The README disclaims evidence of better work. Gemini's “entirely an artifact” and claim that context starvation caused degeneration exceed the evidence: no controlled experiment isolates the cause.

The code sets `PROMPT_CHARS = 2000`, `OUT_CHARS = 600`, and `TRAIN_TOKENS = 2048`. These are round-one choices after a T4 out-of-memory failure at 4096 tokens, not an intrinsic 3B context ceiling. A larger model does not itself fix truncation. Measure retained input and target lengths, memory, and throughput before choosing a GPU; neither memo demonstrates a particular larger card is necessary or sufficient.

Map 111 reports Ministral at 8/10 on a six-turn tool loop and 2.7 GB loaded at 4096-token context on the 8 GB M2. It supplies no planning score or 8B deployment measurement. The panel readings' 8B successes cannot establish that all smaller planners fail. The repository supports neither a proven 3B architect nor a minimum size of 8B.

## 4. Who pays per version, and what proves no regression?

**Recommendation:** the human funds any approved round under a separately stated cap; do not automatically train on each Hydra release. Version the model with its data, prompt/schema, context construction, and evaluation set. Contract changes trigger compatibility evaluation and data migration; new validated examples or measured regressions may justify retraining. Neither automatically mandates it.

**I side with both seats against per-release retraining, Opus on retaining the bench, and Gemini on broader architectural evaluation.** `integrations/bench/goals/l1-cli.md` and `integrations/bench/goals/m1-chars.md` settle their factual disagreement: the CLI goal requests several planned and assigned tasks, tests, and README changes; `m1-chars` changes code and documentation. Gemini's description of all five goals as single-file edits is false. Five tiny goals still cannot establish general architectural competence.

`.scratch/hydra/build/274-a-fixed-goal-set-measures-the-orchestrator.md` says execution had not started at its recorded outcome. `integrations/bench/README.md` defines a cost bench, not a measured plan-quality regression gate. That historical statement does not prove no later session ran it.

A gap neither memo pinned down: `crates/puppets/src/write.rs::tasks` parses JSON and checks required fields plus class/grade membership, but **does not validate unique task IDs, dependency references, cycles, or path existence**. `fold.rs` replaces a previous task with the same ID. The proposed static validator is work to build, not a capability already demonstrated. Blanket path-existence checks would also reject valid plans to create files.

My proposed gate combines structural checks, blinded draft comparison on held-out goals, and execution of the five smoke goals plus representative multi-task goals under matched worker and resource settings. Record verified completion, human repairs, policy failures, latency, and total cost. Group repetitions by goal and report uncertainty. Set an acceptable non-inferiority margin before evaluation; a 50% panel win rate alone does not establish unchanged execution quality. Gemini's 15–20 goals and 50–100 examples are proposed quantities, not thresholds derived from repository evidence.

Round one's README reports an **estimated USD 0.25**, about 65 minutes on a spot T4, against a USD 10 cap, with the invoice unread. This is historical evidence, not round-two pricing or authorization. Gemini's USD 3–5 estimate and proposed GPU lack a supporting measurement in the cited files. Profile the actual workload before budgeting; include preparation, evaluation, and workers rather than training alone.

## 5. Does ticket 118 close the Fine-tuning out-of-scope line?

**Recommendation:** no automatic closure. Preserve the core/export boundary. The human's eventual decision should distinguish external training experiments from core-supported training; a future map clarification can acknowledge round one without claiming a viable specialist corpus.

**I side with Gemini on not closing the line through this panel, and Opus on acknowledging the experiment; neither establishes a numerical condition for changing scope.** `.scratch/hydra/map.md` D12 and its out-of-scope bullet, alongside `docs/run-record-contract.md` section 16, are decisive: training is outside scope while export remains a record constraint. `.scratch/hydra/training/round1/README.md` proves an external experiment occurred, not that the core now trains models.

Opus's clarification is useful, but calling the experiment a contradiction assumes every research script is a core feature. Gemini's threshold of 100 verifier-accepted runs is absent from D12 and the ticket. Corpus readiness and project scope are separate decisions. This panel does not resolve the human grilling; I have not edited the map or its Answer.

## Vote

Pursue a specialist planning drafter that emits the existing plan contract, while assignment and approvals remain in their present code and human workflow. Build the missing plan-to-outcome dataset and evaluate a context-grounded 3B baseline before buying an 8B training run or claiming either a minimum model size or a viable corpus. Keep external fine-tuning distinct from core scope, and make spending depend on measured planning and execution results rather than release numbers or unsupported sample thresholds.

## What I could not verify

- Current run, child-run, goal, proposal, and verdict counts; overlap between the grader verdict map and Puppets events; outcomes of the 26 reported child records.
- Whether retained plans, original input context, and downstream artifacts suffice to reconstruct a complete planning dataset without leakage or ambiguity.
- External papers' tables or claims: I audited what the supplied excerpts support, not the papers themselves, and treat no paper score as independently verified.
- Current GPU prices, round one's invoice, longer-context training fit, or 8B inference performance on the target Mac.
- Any live regression result, later bench execution, or completion of build 290's outstanding content review. Code inspection does not replace those measurements.

## Pendiente de mí

Ninguna acción pendiente para este encargo. Las verificaciones no realizadas quedan enumeradas arriba; las decisiones del grilling corresponden al humano.

## Cambiado

Creado únicamente `.scratch/hydra/research/118-seat-gpt.md`. No ejecuté cargo ni tests, ni modifiqué los otros archivos que ya aparecían cambiados en el worktree.

## Encontrado

Opus acierta sobre la procedencia de los veredictos y el formato de salida; Gemini acierta en exigir evidencia posterior de ejecución. Ambos omiten límites importantes: falta unir planes con resultados, el importador no valida el grafo, el bench sí contiene trabajo de varios archivos y los datos no establecen un mínimo de 8B.
