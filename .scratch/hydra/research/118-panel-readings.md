# Readings for the panel on map 118 (and 117): the harness's own model as an orchestrator

Assembled by the session on 2026-10-13 for the four seats. Everything below is either copied
from a file outside this worktree (`~/cerebro/research/deep-research.md`, section 5, read on
2026-10-12) or counted on the machine, because a run cannot read `~/hydra/runs/` and cannot
read `~/cerebro`. What is inside the repo is named by relative path so you can read it yourself.

## Counts on this machine, 2026-10-13

- 866 run records. 26 of them carry a `parent_run_id`, that is, were launched by another run
  through the delegate face (`src/face.rs`). Everything else was launched by a human or by
  `hydra-puppets`.
- 0 files under the verdict store; the 70 verdicts map 118 cites live inside goal logs
  (`crates/puppets/`, `hydra-puppets verdict`), not as files of their own.
- `integrations/bench/goals/`: five bench goals (`l1-cli`, `m1-chars`, `m2-saturating-bump`,
  `s1-readme-typo`, `s2-one-test`).
- `.scratch/hydra/training/round1/`: the first fine-tune, 2026-10-04. Read its `README.md`:
  QLoRA on Ministral 3B, 21 trajectories, 0.25 USD on a spot T4, next-call 1/9 to 5/9 on held
  out, and the gain was mostly a `Bash` prior.

## In the repo, read these

- `.scratch/hydra/issues/118-a-specialist-orchestrator-not-a-generalist.md`, the five questions.
- `.scratch/hydra/issues/117-three-tiers-of-orchestration.md`, the sibling grilling.
- `.scratch/hydra/issues/109-a-system-one-of-our-own.md`, the `## Answer`: what a System 1 is
  here and why JEPA waits.
- `.scratch/hydra/issues/112-hydra-s-own-harness-and-its-local-driver.md`, open: where the local
  model's URL and name live, and Ministral vs Granite.
- `.scratch/hydra/issues/115-the-harness-learns-from-its-own-use.md`, open.
- `.scratch/hydra/issues/88-a-harness-of-our-own-for-the-minis.md` and `111-which-small-model-calls-tools.md`, resolved.
- `crates/puppets/src/` for what a goal log records: plans, the assignment table, approvals,
  verdicts. `COCKPIT.md` for how the human reads it.
- `IMPLEMENTATION.md`, the sections on the grader, the apprentice and the export.

## From `~/cerebro/research/deep-research.md`, section 5, verbatim

## 5. The second question: small models as orchestrators, small models as workers

### (a) Specializing a small model as the planner or orchestrator

- **"Can Small Agents Collaborate to Beat a Single Large Language Model?"** (Zywot, Chen, Yuan, Sogaard, de Rijke, January 2026). Qwen3 1.7B to 32B in orchestrator and worker roles on GAIA, GPQA, AIME, MuSiQue, HLE. An 8B orchestrator with 8B workers tied a 32B single agent on GAIA (23.0 vs 23.0) and beat it on AIME (55.0 vs 45.0), at 7.9 s per query against 32.9 s for the 32B. Reasoning in the orchestrator added 18.2 points on GAIA; reasoning in workers added little or hurt; worker size barely mattered once the orchestrator reasoned (23.0, 23.0, 23.6 for 1.7B, 8B, 32B workers). Conclusion in their words: "planner-limited rather than executor-limited". https://arxiv.org/abs/2601.11327.
- **ToolOrchestra** (NVIDIA authors, November 2025). An 8B orchestrator trained with RL on outcome, efficiency and user-preference rewards, routing to tools and to larger models. 37.1 percent on HLE against GPT-5's 35.1 at 2.5x the efficiency; about 30 percent of GPT-5's cost on tau2-Bench and FRAMES. https://arxiv.org/abs/2511.21689.
- **Training the Orchestrator** (Mangannavar et al., June 2026). A QLoRA-tuned small policy plus three hardcoded rules chooses which repair agent to call for PDDL planning, trained only on verifier-accepted trajectories. Matches GPT-4-mini, within 3 points of Gemini 3 Flash, 0.004 USD per task against 0.18, and 40 to 50 percent fewer LLM calls. https://arxiv.org/abs/2606.21740.
- **AOrchestra** (February 2026). An orchestrator that creates sub-agents on the fly as (instruction, context, tools, model) tuples; 16.28 percent relative gain over the strongest baseline with Gemini 3 Flash. https://arxiv.org/abs/2602.03786.
- **SearchSwarm** (June 2026), above: delegation as a trained skill, from harness traces. https://arxiv.org/abs/2606.09730.
- **Plan-and-Act** (Berkeley, ICML 2025). A fine-tuned planner and a separate executor, with synthetic plan annotations on ground-truth trajectories; 57.58 percent on WebArena-Lite, 81.36 percent text-only on WebVoyager. https://arxiv.org/abs/2503.09572.
- **AgentTuning** (Tsinghua, ACL Findings 2024). The older generalist route: AgentInstruct trajectories mixed with general instructions; AgentLM-70B matched GPT-3.5-turbo on unseen agent tasks. Useful as the baseline the specialized papers moved away from. https://arxiv.org/abs/2310.12823.
- **LIMI: Less is More for Agency** (GAIR, September 2025). 78 curated samples reached 73.5 percent on their agency benchmark, against 45.1 for GLM-4.5 and 53.7 percent better than a 10,000-sample baseline. The point for a solo developer: the training set for an orchestrator can be tiny if the samples are complete trajectories. https://arxiv.org/abs/2509.17567.
- **Magentic-One** (Microsoft, November 2024). The reference prompted orchestrator: a task ledger and a progress ledger, re-planning on stall, specialized agents for browser, files and code; competitive on GAIA, AssistantBench, WebArena. https://arxiv.org/abs/2411.04468. **AgentOrchestra** (June 2025) is the hierarchical version with a planning agent over specialists and an MCP manager, 83.39 percent on GAIA test: https://arxiv.org/abs/2506.12508.
- **Critic and judge models.** "LLM Critics Help Catch LLM Bugs" (OpenAI, June 2024): RLHF-trained critics whose critiques were preferred over human ones 63 percent of the time on natural LLM errors, with the warning that critics hallucinate bugs: https://arxiv.org/abs/2407.00215. "Agent-as-a-Judge" (Meta and KAUST, ICML 2025): an agent judging agents on the 55-task DevAI set agreed with human consensus 90 percent against 70 percent for a plain LLM judge: https://arxiv.org/abs/2410.10934.

### (b) Cheap worker under a large planner, measured

- **Anthropic advisor strategy** (April 2026): Haiku executor with Opus advisor, 41.2 vs 19.7 on BrowseComp, 85 percent cheaper than Sonnet alone while 29 percent behind it; Sonnet with Opus advisor +2.7 points on SWE-bench Multilingual at 11.9 percent lower cost per task. https://claude.com/blog/the-advisor-strategy. API docs: https://platform.claude.com/docs/en/agents-and-tools/tool-use/advisor-tool.
- **Fable 5 orchestrator over Sonnet 5 workers**: 96 percent of all-Fable accuracy at 46 percent of the cost on BrowseComp, secondary source attributing to Anthropic: https://the-decoder.com/anthropics-fix-for-fable-5s-high-cost-is-turning-it-into-a-manager-that-delegates-to-sonnet-5/.
- **COPE, "Efficient LLM Collaboration via Planning"** (KAIST, June 2025). Small (Qwen3-8B, Qwen2.5-1.5B, EXAONE 2.4B) and large (GPT-4o, GPT-4o-mini, Gemini 2.0 Flash) models alternate as planner and executor in a cascade. MATH-500 75.8 vs 75.2 for GPT-4o alone at about 45 percent less API cost; MBPP 66.4 vs 64.0 at about 74 percent less; ALFWorld 44.8 vs 42.5 at about 22 percent less. https://arxiv.org/abs/2506.11578.
- **MinionS** (Stanford, ICML 2025). Cloud model decomposes, local model executes chunk jobs in parallel: 97.9 percent of cloud-only quality at 5.7x less cloud cost; the naive chat protocol only recovered 87 percent at 30.4x less. https://arxiv.org/abs/2502.15964.
- **HybridFlow** (ICML 2026). A small edge model builds a dependency DAG of subtasks and routes each to local or cloud by a learned benefit-cost model; reports lower latency and cloud usage at competitive accuracy on GPQA, MMLU-Pro, AIME24 and LiveBench, without numbers in the abstract. https://arxiv.org/abs/2512.22137.
- **ProgRouter** (EMNLP Findings 2026). Per-step routing between models under time and cost budgets; the abstract's finding is that "neither always-small nor always-large" works under real constraints. https://arxiv.org/abs/2608.25992.
- A practitioner summary of the failure mode, with the Grok 4.5 worker datapoint (about 2.50 USD per task against 12 for Fable 5, but hallucination roughly doubled) and the observation that delegation only worked on the two strongest planner models: https://arize.com/blog/how-cheap-models-changed-multi-agent-economics/. Secondary.

The consistent shape across all of these: put the reasoning budget in the planner, make the worker's job narrow and checkable, and expect the quality gap to show up as missed or invented facts rather than as bad structure. That is why the citation pass is not optional in the cheap-worker configuration.

