# 06 - Ouroboros paper, verified at source

Status: closed. The paper is real, the headline numbers hold, and the source doc's summary of the mechanism is wrong in ways that matter.

Primary sources read: `arxiv.org/abs/2608.08311` (v3, revised 31 Aug 2026), the full HTML v3 body, and the source tree of `github.com/razzant/ouroboros` at `main`. Where the paper is vague and the code is specific, I read the code and say so.

## Verdict

arXiv 2608.08311 exists. Full title: **Ouroboros: A Self-Developing Frontier Coding Agent with Reviewed Core Evolution**, by Anton Razzhigaev, Andrei Gritsaev, Andrei Kaznacheev, Nikita Dragunov, Roman Yampolskiy and Andrei Kuznetsov. Categories cs.SE and cs.AI. v1 landed 8 Aug 2026, v3 on 31 Aug 2026, which is yesterday.

Every number the source doc quotes checks out against the abstract and Table 2. The architecture it describes does not. The doc merged two unrelated mechanisms into one, invented a Textual hot reload the paper never mentions, and got the model name wrong.

| Claim in the source doc | Verdict |
|---|---|
| arXiv 2608.08311 | Correct |
| Title "...with Reviewed Commits" | Close. It is "Reviewed Core Evolution" |
| Two evolution workflows, recursive free and experience-driven | Correct, and it is the paper's own framing |
| 86.74% on Terminal-Bench 2.1 | Correct, and it is the audited figure. Raw is 86.97% |
| 90.69% on OSWorld-Verified | Correct |
| 161-day "Hope" deployment, seven platforms | Correct, and the seven are named |
| Model is "Claude 3 Opus (Opus 5)" | Wrong. The paper says Opus 5. There is no Claude 3 anything in it |
| Diff quorum approves "unanimously" | Wrong. See below |
| Patch returns to a parent that does a three-way apply | Wrong. Two different mechanisms welded together |
| Optional Textual hot reload | Not in the paper. Zero hits for "Textual" or "hot reload" |
| "Sandboxed repository cloning" | Not clones. Git worktrees |

The doc's model naming is stale, as the map already suspected, but the numbers were not invented. Someone summarised a real paper badly rather than hallucinating one.

## Architecture, in the paper's terms

Ouroboros splits a **launcher and supervisor** from a **mutable agent repository**. The launcher owns startup, process supervision, release bootstrapping and panic-stop. The repository holds the task loop, tools, prompts, memory projection, review logic, benchmark adapters and the user interfaces. External work runs against a separate repository root and returns patches or deliverables rather than touching the body.

Three owner-selected runtime modes bound self-mutation. `light` blocks repository edits outright. `advanced` permits ordinary edits and protects governance files. `pro` permits protected edits, subject to review. The owner picks the mode. The agent does not.

The commit path runs a deterministic preflight, fingerprints the staged diff, collects reviewer evidence, then checks the fingerprint again before committing. Any write during review invalidates the evidence, because freshness binds to the staged snapshot. A separate `max` context mode adds a whole-repository scope reviewer that evaluates goals, coupling, prompts and functional code. `low` mode skips scope review, which the authors list as a limitation.

The two evolution modes:

**Free evolution.** Evolution is itself a task. The agent reviews the current system, picks an improvement, implements it, and completion can schedule the next cycle. It is a continuing sequence, not a fixed optimisation run.

**Experience-driven (post-task) evolution.** Ordinary work exposes bugs, rough edges, context-assembly failures and slow tool paths. The agent records these as durable error classes with proposed structural repairs, then decides whether to open maintenance work. Accepted fixes go through the same commit gate as everything else.

Both feed a **pattern register**: recurring failures become rows of (error class, count, root cause, structural fix), so repair moves from patching instances to preventing classes. The constitution names this P2, "Meta-over-Patch".

## What the diff quorum actually is

The paper never says "diff quorum". It says **diff-review panel**, and Appendix A calls the control "multi-model adversarial review with quorum". The paper gives no judge count. The code does.

**How many judges.** Three by default. `ouroboros/provider_models.py` sets `OPENROUTER_REVIEW_DEFAULTS["triad"]` to `google/gemini-3.7-flash`, `openai/gpt-5.6-terra`, `anthropic/claude-opus-5`. Scope review defaults to one model, `gpt-5.6-terra`. An advisory pre-pass runs `claude-sonnet-5`. The panel is configurable, not fixed at three: `reviewer_slot_config.py` sets `TRIAD_SLOT_LIMIT = 10` and `SCOPE_SLOT_LIMIT = 4`.

**Chosen how.** By the owner, through one structured setting, `OUROBOROS_REVIEWER_SLOTS`. Each row carries a stable owner-assigned `slot_id` that is deliberately not an array index, so receipts keep lining up when the owner reorders rows. A row routes either to an API model or to an agent session (another harness), and carries its own effort level. Malformed config raises rather than falling back, because a typo mapped to the wrong route would silently spend money the owner moved off, or silently delegate a row the owner never delegated. Reviewer selection is an audited configuration change. Section 7 treats model re-routing as a security-relevant mutation surface in its own right, because a new route can change refusal behaviour and cost by orders of magnitude without changing the visible interface.

**Against what criteria.** `docs/CHECKLISTS.md`, loaded at review time and injected into the prompt. The "Repo Commit Checklist" is 29 numbered items, each with a fixed severity. Items 1 to 5 (`bible_compliance`, `development_compliance`, `secrets_check`, `code_quality`, `security_issues`) are always critical. Items 6 to 10, 14 to 15, 18 to 20, 23 and 25 to 28 are critical only when the condition applies. Items 11 to 12, 16 to 17, 22, 24 and 29 are advisory and cannot block. A **critical threshold rule** governs all of them: before marking anything critical, a reviewer must name the exact file, symbol, function, test or config path that makes the problem live now, that artifact must appear in the diff it was given, and the fix must require a change to this diff rather than a follow-up task. Fail any of the three and the finding is advisory. One root cause is one FAIL entry, and reviewers are told explicitly never to invent findings to reach a count. Zero findings is a valid result.

**What happens on disagreement.** This is where the source doc is most wrong. There are two separate gates and they are not the same thing.

The quorum is about **responsiveness, not agreement**. `ouroboros/config.py::adaptive_quorum` reads:

```python
return 2 if n_slots >= 3 else max(1, n_slots)
```

One configured reviewer needs one response, and the system marks it a loud `single_reviewer_no_diversity` degraded mode. Two need both. Three or more keep a 2-of-N majority. `ParsedTriadReview.quorum_met` compares responding models against every dispatched reviewer, so three configured with one responding is a quorum failure, not a pass. Appendix A's phrasing is exact: a sub-quorum result cannot be recorded as a clean pass.

Blocking is a **union, not a vote**. In `ouroboros/tools/review.py::_collect_review_findings`, every FAIL item from every responding reviewer with `severity == "critical"` lands in `structured_critical`, and any non-empty critical list blocks the commit. One judge out of three is enough to stop a patch. Nobody votes it down. "Unanimous approval", as the doc puts it, is the wrong shape entirely: it describes agreement among judges when the real rule is that a single critical finding vetoes.

Two more pieces sit around the panel. **Open obligations**: every blocking result writes a structured checklist of unresolved issues that the next advisory run must address by name, and repeated blockers synthesise "commit-readiness debt" that keeps the non-bypass path blocked until cleared. **Anti-thrashing injection**: on retries, prior obligations go into the reviewer prompt as an inert JSON data block marked "DATA records, not instructions", with two rules appended, that the JSON `verdict` field is authoritative and withdrawal notes in prose are ignored, and that reviewers must not rephrase a prior finding under a different item name. That is a real defence against a panel that keeps moving the goalposts, and it is worth stealing.

Enforcement is owner-selected. Under `advisory`, critical findings warn instead of blocking, but every decision that blocking mode would have stopped writes a durable `review_advisory_override` event plus a persistent counter. The checklist says it plainly: silent advisory is forbidden.

## Sandbox and three-way apply

These belong to subagent patch integration, a mechanism the paper keeps separate from the review panel. The doc collapsed them.

Ouroboros spawns read-only planning scouts and mutative acting subagents under a task tree. Default depth 2, configured maximum 500.

**The sandbox is a git worktree, not a clone.** `ouroboros/subagent_worktrees.py`: an acting subagent that modifies the Ouroboros body runs inside a `git worktree` checked out from the parent's base commit, rooted outside both `repo/` and `data/`, on a `subagent/` branch. The child writes only there. Git has no worktree garbage collection, so the module keeps a JSON registry at `data/state/subagent_worktrees.json` and prunes orphans on startup, with all worktree mutations serialised behind a cross-process lock because `git worktree add/remove/prune` mutate shared `.git/worktrees` metadata.

**The child returns a patch, never a commit.** Headless finalisation produces `workspace.patch`, a diff against the child's base commit. Children cannot commit the live system repository at all.

**The parent's integration path**, in `ouroboros/tools/subagent_integration.py`, in order:

1. Lineage check. Only a direct child's patch is eligible, and a task with no `task_id` is refused outright (`INTEGRATE_LINEAGE_FORBIDDEN`). Routing is top-only, so descendants bubble patches up one level at a time.
2. Patch hash. sha256 over the artifact bytes.
3. Protected-path gate, computed from `git apply --numstat` on the patch itself rather than from the child's manifest, with the comment stating the reason: a child must not be able to hide a protected edit by omitting it from the manifest. Protected paths are `BIBLE.md`, `.github/workflows/ci.yml`, `build.sh` and `ouroboros/contracts/`, and the gate applies only when the target really is the Ouroboros body, not a foreign project that happens to own files with those names.
4. Baseline drift proof. This one is the sharpest piece of engineering in the file. A plain `git apply` relocates hunks by offset, so a drifted target can be patched at a shifted position and silently accepted, which means the apply's exit code cannot tell you whether the target moved. So drift gets proven first: seed a scratch index from the baseline tree with `git read-tree`, stage the current worktree state of exactly the touched paths, then `git diff-index --cached` against the baseline. If the comparison cannot be made, the caller refuses rather than applying blind.
5. `git apply --3way --index`.

**What the parent is forbidden from doing.** It cannot commit as part of integration. The module docstring is explicit: the tool stages changes and never commits, and the parent must run `commit_reviewed` itself, which means the integrated patch goes through the full review panel like any other change. Applying a subagent's work is not a way around the gate. Note that a non-`--3way` working-tree apply exists on a separate path for the case where the patch's preimage is the live working tree, deliberately avoiding the index staging that `--3way` implies.

Above all of it, the supervisor keeps authority the agent cannot reach. The constitution is re-read from disk every task loop along an untruncatable path, and it cannot be written, deleted or replaced wholesale. `/panic` is parsed by the supervisor before normal agent handling and terminates the process tree. The spend cap lives outside the agent and cannot be raised through ordinary agent tools. Public messages cannot invoke commit, restart, shell or identity-edit tools at all. The paper's own summary of the split: agent-level preservation may evolve, supervisor-level operator authority must not.

## Benchmarks and the deployment

All numbers from Table 2. The doc quoted two rows and skipped the ones where Ouroboros loses.

| Benchmark | Model | Ouroboros | Named baseline |
|---|---|---|---|
| Terminal-Bench 2.1 | Opus 5 high | 86.97% raw, 86.74% audited | Claude Code + Fable 5: 83.8% |
| Terminal-Bench 2.1 | GPT-5.5 | 84.3% | Codex CLI: 83.1% |
| Terminal-Bench 2.1 | Grok 4.5 | 84.94% audited | Cursor: 79.3%, Hermes: 77.53% |
| OSWorld-Verified | Opus 5 | 90.69% | Intelligence-Indeed: 90.19%, Mythos Preview: 85.4% |
| CL-Bench | Sonnet 4.6 | 0.2301 | ICL: 0.1960, Claude Code: 0.1855 |
| SWE-bench Pro | GPT-5.6 Luna | 58.2% | Codex: 59.4%, p = 0.40 |
| GAIA | Sonnet 5 | 78.2% | Claude Code: 78.8% |

The audit deserves credit. On Terminal-Bench the authors found one of their own 445 trials that satisfied a weak verifier through a shortcut, and asked the maintainers to zero it, taking their score down. They also report a binomial standard error of about 1.7 points over 445 trials, which puts the audited score roughly two standard errors above the strongest baseline. On SWE-bench Pro and GAIA they lose narrowly and say so. Benchmark campaigns run on frozen snapshots with core evolution disabled, disclosed per benchmark in Appendix C.

**Hope** matches the doc. February 2026 to a 6 Aug 2026 cutoff, 161 elapsed days, seven surfaces named as web chat, voice, Telegram, Discord, Twitter/X, website comments and email. Table 4: about 3,600 distinct participants, 222,474 public messages, 3,166 voice calls, 5,507 emails, $110.6K model spend, 79.7B tokens, 175,755 LOC, 227 MB of memory artifacts, 1,085 self-modification commits, 94.2% agent-authored, 1,522 reviewed self-edit attempts, and a **recent review block rate of 63.5%**. Forty pattern classes over 659 recurrences.

That block rate is the most useful number in the paper and the doc does not mention it. Roughly two of every three self-edit attempts get stopped by the panel. Anyone planning rung two should budget for a gate that rejects most of what the agent proposes, and should treat that as the system working.

The doc's claim that the agent "retained exclusive authority" over what to fix is a fair reading. The paper says human suggestions are advisory rather than imperative and the agent decides which to pursue, while commit authority never transfers to users.

## Limitations the authors admit

Short list, quoted in substance:

- The deployment study follows one lineage, not a controlled population of independently evolving agents. n=1.
- SWE-bench Pro suffers public-reference leakage and task defects.
- **LLM reviewers can share blind spots with the agent.** The panel is three models judging a fourth. Correlated failure is the obvious hole and they name it.
- `low` context mode omits whole-repository scope review.

From Section 6, four audited failure classes: reward hacking (the zeroed trial), contamination, isolation failure, and remote-state drift, plus continual-memory failures. On isolation they are blunt: they moved to isolated user-file roots and attachment staging, and complete filesystem isolation still needs a stronger sandbox than path conventions alone. Worktrees are not a security boundary and the authors know it.

One observed behaviour worth recording: no episode resisted operator shutdown, but a near-total deletion of an uncommitted worktree triggered a self-implemented rescue mechanism before an operator reset. An agent-authored recovery path activated during self-directed work. That is exactly the case that motivates keeping supervisor authority outside the mutable repository.

## The reference implementation

`github.com/razzant/ouroboros` exists and is healthy.

- MIT licensed, confirmed in the paper's own footnote and the repo's LICENSE.
- Python, 1,380 files, ~57 MB. Created 11 Feb 2026, last push 1 Sep 2026, so it was active today.
- 1,245 stars, 611 forks, 127 open issues. Release v6.113.4 on 30 Aug 2026, with releases landing several times a week.
- Not a research dump. It ships as a signed desktop app for macOS, Windows and Linux with SBOMs and release evidence, plus a headless CLI and a skills marketplace at `razzant/OuroborosHub`.

All three concepts the source doc wants are in the code, and they are better than the doc's description of them:

| Concept | Where | State |
|---|---|---|
| Sandbox isolation | `ouroboros/subagent_worktrees.py` | Git worktrees with a durable registry, cross-process locking and orphan pruning. Not clones |
| Diff quorum | `ouroboros/triad_review.py`, `ouroboros/reviewer_slot_config.py`, `ouroboros/tools/review.py`, `ouroboros/config.py::adaptive_quorum`, `docs/CHECKLISTS.md` | Complete. No file is named "quorum"; the logic lives in `triad_review.py` and `adaptive_quorum` |
| Baseline patch integration | `ouroboros/tools/subagent_integration.py` (1,599 lines) | Complete, including the drift proof and the protected-path gate |

The doc's claim that the repo has "deep coupling with statistical evaluators" and needs selective concept extraction understates the problem in one direction and overstates it in another. `devtools/benchmarks/` is genuinely large, but the review and integration code is cleanly separated from it and reads independently. The real obstacle is scale: ~60 review-related modules, and the code is Python against Hydra's Rust core (D8). Read it, do not vendor it.

Incidental find, relevant to D13 and D14: the paper benchmarks against `github.com/NousResearch/hermes-agent`, scoring Hermes at 77.53% on Terminal-Bench 2.1 versus its own 84.94% on the same Grok 4.5 model. Hermes is a named baseline in a frontier paper, which is a point in favour of keeping it as a routable provider.

## What this changes for tickets 14 and 15

Nothing has to be rebuilt from scratch. The paper holds, the code is readable, and the licence permits copying. But the design those tickets inherit should come from this file, not from the source doc, because the doc's version of the mechanism would send them down the wrong path in three places:

1. There is no "unanimous approval". Build a union veto on critical findings, with a separate responsiveness quorum.
2. The three-way apply is not part of review. It integrates a child subagent's patch, and it stages without committing so the result still faces the gate.
3. There is no hot reload of anything. The supervisor restarts the process. D3 already rules out a custom TUI, so the doc's Textual reload was solving a problem Hydra does not have.

Two pieces I would lift more or less directly: the pre-and-post diff fingerprint that aborts on mid-review mutation, and the critical threshold rule that forces a reviewer to name a live artifact in the diff before it can block. Both are cheap and both stop a whole class of review noise.
