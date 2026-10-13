# Map 117, the adjudicator’s seat (GPT)

Written 2026-10-13. This is advice for the human’s grilling answer; the ticket remains unchanged. I read the ticket and both 118 panel documents first, then checked the two 117 memos against this checkout. The 118 decisions stand.

## 1. What the Jev tier reads and returns

**Recommendation:** evaluate it offline on the rendered delegation plus goal context, returning an explicitly unvalidated score. No launch refusal or automatic rewrite. Establish a rubric distinguishing required behavior, constraints and acceptance criteria from unnecessarily prescribed implementation steps.

**I side with Gemini on evaluation before presenting scores at approval, and with both seats on the input.** `crates/puppets/src/launch.rs::worker_task_text` combines task text, `done` and the request for a “Not confirmed” report; `launch` passes that rendering to `build_run_argv`. The builder’s first turn comes too late. Goal context can establish purpose, but context supplied only to the evaluator must not be mistaken for context supplied to the builder.

The decisive files are `.scratch/hydra/issues/109-a-system-one-of-our-own.md`, whose Answer says “rank, never a vote or a threshold,” and `.scratch/hydra/research/109b-kev.md`, which records option probabilities and a `noul` response. Opus correctly describes probability-shaped outputs; Gemini’s “arbitrary logit” description is inaccurate. Gemini correctly observes that neither file establishes calibration for delegation quality. Diff verdicts alone cannot label whether a prompt unnecessarily dictated implementation.

**Neither seat establishes that leaving implementation choices to a builder is harmful.** The supplied readings support strong planning and bounded jobs, but do not isolate this intervention. Gemini’s “proves” and Opus’s opposing-literature argument overreach. Opus also says none of the papers concerns coding, although the readings include COPE’s MBPP result alongside the advisor’s SWE-bench result.

`integrations/puppets/prompts/plan.md` settles the practical boundary: files, areas and acceptance checks belong in the contract. It permits a file that exists or something a reviewer can check, not only executable commands. Neither naming a file nor specifying expected behavior establishes that the planner dictated the how.

## 2. Where the check runs

**Recommendation:** an offline audit first; if useful, an optional advisory step in `hydra-puppets` before table approval. Bind scores to the rendered task, goal context and evaluator version, invalidating them when those inputs change.

**I side with Gemini on sequencing, Opus on the eventual integration layer, and against Gemini’s claim that memory rules out inline use.** `crates/puppets/src/fold.rs::launch_refusal` checks launch state, assignment and approvals, not delegation semantics. The hooks in `src/provider/claude.rs` and `src/provider/codex.rs` enforce tool-call policy. `src/face.rs::call` routes through the session launcher or `crate::run::run`, bypassing Puppets task import. A Puppets advisory would have limited coverage.

The deciding memory evidence is the measurement in `.scratch/hydra/research/109b-kev.md`: 3.0 GB steady footprint, 4.5 GB at load, new-state p50 258 ms, one live agent and zero OOM retries. It establishes constrained feasibility, not demonstrated instability or impossibility. It does not establish performance alongside two busy builders. Gemini’s proposed AUC 0.80 threshold is a suggested target, not a measured requirement.

Opus’s import-time location is feasible, but import is not permanently final: `fold.rs` replaces an existing task on a later `task` event. `worker_task_text` also adds a launch-time wrapper. Evaluation must use the same renderer and preserve input identity.

A current-checkout correction: `crates/puppets/src/write.rs::tasks` already calls `task_graph_offences` before appending tasks. It detects duplicate ids, missing dependencies and cycles. Earlier descriptions of this as future work are stale. This follows 118’s settled direction; it does not reopen it or establish a classifier’s usefulness.

## 3. Default builder and promotion

**Recommendation:** retain class-specific defaults and trial smaller builders on bounded tasks with explicit acceptance checks. Collect independent check results before accepting code where feasible, and retain substantive review. Escalate after diagnosing a builder capability failure exposed by either checks or review. Do not promote on Jev scores or ticket `Type:` alone.

**I side with Opus on today’s usable mechanism and Gemini on adding automated verification; neither exclusive trigger is adequate.** `crates/puppets/src/write.rs::DEFAULT_ASSIGNMENTS` chooses:

- Grok 4.6 high for scripting, docs, off-launch Rust and research.
- Sonnet for launch-path Rust.
- Codex without a named model for security review.
- Gemini 3.1 Pro high for probes.

`assign` leaves `agi` tasks for human assignment. There is no universal smallest default.

`crates/puppets/src/sheet.rs::rank` uses human verdicts, penalizes `fixed`, `rejected` and resumed acceptances, applies grade-specific Beta lower bounds, and orders qualifying candidates by median window share divided by that bound. This is a cost-and-acceptance heuristic, not model-size ordering or proof of passing ticket tests.

`ranked_assignment` requires a verdict behind its first row; `write.rs::assign` supplies only the current goal’s events. Both seats correctly identify missing cross-goal verdict input. Nevertheless, a builder can earn assignment within a goal.

`integrations/puppets/priors.json` supports Haiku’s [3,3] prior but contradicts Opus’s claim that only Sonnet has a prior clearing medium: Codex’s security-review override is also [5,1]. Neither seat measured how frequently goals provide enough suitable verdicts.

The `done` references in `crates/puppets/src/` store, fold and render it; there is no completion-check executor. `write.rs::verdict` records human results and requires diff approval for `applied` and `fixed`. Opus therefore describes an available signal; Gemini proposes infrastructure.

Tests do not catch every invented claim, and research or review tasks may need non-executable acceptance. Infrastructure failures and bad specifications also do not prove that a larger builder is needed. Preserve failure reasons separately, and do not automatically relaunch an already repaired, accepted task.

## 4. What would demonstrate that the tiers work

**Recommendation:** measure first-pass acceptance, eventual acceptance including `fixed`, repair/rejection rates, independently observed check results, and resources consumed across all attempts. Report tokens by provider/model, subscription-window share where available, and active human review/repair minutes separately. Include orchestration, review and classifier overhead.

**I side with Opus on what can be derived now and Gemini on what an economic claim additionally requires.** `crates/puppets/src/launch.rs::wait` emits `run_end` with outcome and tokens; launch events identify attempts and models; `write.rs::verdict` records result and run id. These support an event join, not an existing cost-per-resolved-ticket report.

A Puppets task is not automatically a map ticket. `fixed` is an accepted repair, and first-pass acceptance must account for earlier attempts and resumes. Count the resources spent on failures and retries in the numerator rather than considering only successful runs.

`src/cost.rs` and `integrations/bench/README.md` establish that `hydra cost` reports orchestrator/agent token ratios using repository and time association. It does not measure acceptance or human effort. `crates/puppets/src/log.rs` already timestamps events, but elapsed time includes waiting; another repair timestamp would not establish active labor.

**Gemini is closer than Opus on the benchmarks, but both overstate their inadequacy.** `integrations/bench/goals/l1-cli.md` specifies a binary, input/output behavior, tests and documentation while leaving implementation choices open. It is not a dictated patch. The other four are substantially narrower. All five can contribute smoke tests and paired delegation experiments, but cannot alone establish broad non-inferiority or classifier value.

Hold repository state and acceptance criteria constant. Compare delegation styles with the same builder, and builder sizes under the same delegation; otherwise the middle tier and cheaper workers are confounded. Add representative multi-task goals and labeled prompt pairs. Choose the quality margin and sample size beforehand. Repeating five goals does not create fifteen independent task types; neither seat supplies an empirical power calculation for this corpus.

## 5. Relationship to map 115

**Recommendation:** keep 117 separate while sharing outcome data and measurements with 115. Cross-goal builder evidence is a bounded routing change, not a prerequisite to settling the whole self-improvement loop.

**I side with Opus.** `.scratch/hydra/issues/115-the-harness-learns-from-its-own-use.md` concerns triggers for autonomous curate/evolve passes, their permitted changes and their safeguards. None is called by `crates/puppets/src/sheet.rs::rank`, which already adapts rankings using supplied verdicts. Gemini’s claim that dynamic tier 3 mathematically requires 115 is false.

The decisive second file is `crates/grader/src/labels.rs::labels_for_goal`: worker labels already contain class, grade and human verdict; plan proposers receive a role with a null verdict. The plan-to-outcome join settled by 118 matters for evaluating planners, but is not necessary to join builder attempts to builder verdicts.

Cross-goal routing needs goal-qualified task identities, correct attempt/run joins and deduplication. Blindly concatenating logs would collide on repeated task ids in `sheet.rs::rank`. These are concrete data requirements, not reasons to reopen 118 or block all of 117 on 115.

## Vote

Retain the large orchestrator and reviewer, keep current defaults, and measure bounded small-builder trials with independent acceptance checks and cross-goal verdict accounting. Evaluate the delegation classifier offline before considering advisory integration, because neither its accuracy nor the causal value of removing implementation guidance is established. Keep 117 separate from 115 and preserve 118’s decisions, leaving the human to settle the delegation rubric and acceptable quality/cost tradeoff.

## What I could not verify

- Actual goal-log distributions, the supplied counts of 70 verdicts and 26 child records, or realized costs and review time; external stores were not accessed.
- Kev’s delegation accuracy, calibration or concurrent-load behavior beyond the recorded measurement; build 294’s Outcome remains empty.
- External research beyond the supplied readings, or transfer of its results to Hydra’s Rust work.
- Runtime correctness, including the now-present graph validator; no cargo commands or tests were run.
- Which small builder meets the acceptance bar most cheaply; the inspected evidence establishes no winner.

## Pendiente de mí

Saving the memo remains blocked. The policy hook rejected the writing command as shell substitution involving a protected command name; I stopped without retrying through another route or lifting the restriction.

## Cambiado

No files changed by this seat. `.scratch/hydra/research/117-seat-gpt.md` was **not created**. Existing worktree changes were left untouched.

## Encontrado

The ranker lacks cross-goal verdict input but does not depend on map 115. The graph validator is already present, Kev’s recorded measurement shows constrained feasibility, and `l1-cli` leaves implementation choices open. Neither memo demonstrates that a delegation classifier improves small-builder outcomes.
