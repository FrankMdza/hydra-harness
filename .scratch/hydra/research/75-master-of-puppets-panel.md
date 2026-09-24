# 75 - Master of Puppets: what a four-proposer panel said, and what the session recommends

Written 2026-09-24 by the session. Four proposers read the same brief
(`~/.claude/handoffs/2026-09-24-puppets-panel/task-base.txt`) and each wrote one proposal. The
raw proposals sit beside the brief as `proposal-<provider>.md`, outside the repo like the panel
reviews before them. Every claim below that names a file and line was checked by the session
against commit 18a9d72 before it was kept.

| Seat | How it ran | Tokens | Words | Read the code? |
|---|---|---|---|---|
| `codex`, default model | `hydra run`, `standard`, 01a0d21e-01a1 | 2.57M | 2,887 | yes, and found five stale claims |
| `cursor-agent`, composer-2.5 | `hydra run`, `standard`, 01a0d21e-2ffe | 0.91M | 2,170 | yes |
| `agy`, gemini-3.1-pro-high | `hydra run`, `standard`, 01a0d223-510f | 0.50M | 1,050 | eight citations, two of them wrong |
| `claude`, opus | Agent-tool subagent in the main checkout, unrecorded | 0.18M | 3,665 | yes, and counted the corpus |

The Claude seat ran as a subagent rather than a `hydra run` because an opus run that reads the
repo costs about 0.57 of the five-hour window (measured 2026-09-22) and that window was at 0.60
when the panel launched. The three recorded runs cost nothing on it.

## Where all four agree

- **The mascot and the pane launcher are Herdr plugin work, and Hydra draws nothing.** D3 and
  ticket 44 decide it, and nobody argued. A `hydra` subcommand that paints above the vendor's UI
  would be the TUI the map ruled out, and the vendor repaints the whole screen anyway.
- **The orchestrator lives outside every Hydra run.** The delegate face refuses calls from inside
  a seatbelt (`src/face.rs:236-238`) and takes no `model`, and assigning agents is choosing
  models. The spec already says a parent that wants two children in parallel is an orchestrator
  launching from outside.
- **A rate-limited task blocks and is never reassigned by the mode.** Three of four cite rung two's
  rule that the panel never substitutes a judge. `agy` proposed continuing with `hydra resume` on
  another provider, which `resume` refuses: it keeps provider, policy and model.
- **State is one file per goal under `~/hydra/puppets/`, and the directory must join the deny
  list**, which is protected config #1, so the human edits it. Codex wants a versioned JSON plus a
  journal; Opus wants one append-only JSONL folded into state the way an outcome is derived. The
  fold is the better fit for a repo whose contract already says never store what you can derive.
- **Cost is window share, not dollars.** Every provider here is a subscription. Vendor prices go in
  a hand-kept file with a date and a URL. What Hydra measures is tokens, duration, outcome,
  denials and whether a judge answer parsed.
- **The corpus has no task class and no "a human applied this" bit.** Those are the two signals a
  per-class ranking needs, and both come from the mode's own log, plus a one-off backfill from
  build tickets, which name their run ids under `## Outcome`.
- **No LLM is needed to classify.** Seven closed classes, the composing judge proposes, one rule
  raises the floor (a protected path is at least high and classed security), the human confirms.

## Where they split, and the session's call

**Where the orchestrator's code lives.** Composer: a Herdr plugin plus a skill. Codex: a separate
small program the human runs. Opus: a Claude Code skill plus a Python helper, Rust after three
goals. `agy`: `hydra puppets` in Rust now. The session takes Opus's order. The session already does
this loop by hand (CLAUDE.md, the third population), so the first version is that loop written
down, with a helper that launches, waits on `run_end`, projects and appends. Rust comes when the log
format has survived three goals; Rust first would freeze a format nobody has used. A Herdr plugin
as owner fails D10, since nothing may need to be running for Hydra to work.

**What the human approves.** Composer and codex: every launch. Opus: the plan once, the assignment
table once per batch, every diff, and a launch only where the gate can fail open (`codex` on a
dead hook, `claude` on a gate that fails to start). The session sides with Opus, with one
addition from codex: an approval binds to a plan revision and is consumed once, so an edit
after approval reopens it. This is decision C for the human, because it is the mode's whole
safety story.

**One head per live run.** Opus and `agy` accept, codex and composer reject (flicker, width,
brand). The session accepts Opus's version: up to four heads, `+n` beyond, three sleeping heads at
idle. `LIVE_RUN_CAP` is 8 and this machine lost a run to the OOM killer at six, so the head count
is a RAM gauge, which is the one thing that makes the mascot information rather than decoration.

**Seats.** Planning: `codex`, composer-2.5 and `agy` with a closed brief, no `claude`, because
the session's window pays for the orchestrator (Opus). Review: `claude` opus, `codex` and
composer-2.5 under `judge` with everything in the prompt, where an opus seat costs tens of
thousands of tokens and not 9.3M (Opus). `agy` stays off review: it scored 10/10 in the hiring
panel after eight files. Codex wanted `agy` on review and Sonnet on planning; the session's
memory of `agy` as a judge decides against the first, and the window against the second.

## What the panel found that the request did not ask

1. **The mascot cannot animate beside the session it was asked for.** An interactive run's record
   is `run_start` alone until the harvest at exit (`src/interactive.rs:588-600`), so a mascot
   reading the record sees idle or working and never a refusal or a burst. Only `claude` writes an
   append-only transcript during the run (`docs/harness-spec.md:1176`). Reading it live from the
   bridge is decision G. Codex and Opus both found this; composer and `agy` missed it.
2. **`hydra-in-pane` cannot carry an interactive run.** It sends stdout to `/dev/null`, waits for
   the command to return and then tails the log (`integrations/herdr/bin/hydra-in-pane:10-33`).
   The launcher the human wants is a sibling that inherits the terminal and tees stderr for the
   run id. Codex and Opus.
3. **The delegate face's provider enum omits `codex`** (`src/face.rs:175-184`) while the parser
   accepts it. Codex. A one-line build ticket.
4. **Rung two's rate-limit abort reads the last status and never its expiry**
   (`src/rungs/evolve.rs:1783-1794`). Codex. Small build ticket.
5. **Four documents still say three.** `docs/rungs-spec.md:534` says "all three configured
   judges", `docs/harness-spec.md:92` says "the other two providers", and the map's D11 line says a
   panel of three; the code requires four (`src/rungs/review.rs:73-78`). Codex and Opus.
6. **92 provider runs carry a null `model`, every `codex` run among them**, against 44 of 203 in
   the contract's own count. Opus, from line 0 of all 378 records. The ranking rolls those up to
   the provider until the stream names the model.

## What to build, in order

1. Small, today. `hy`, a shell function, and `hydra-tui-in-pane`, a Herdr action that runs
   `hydra run --interactive --provider claude --policy standard` with inherited IO, tees stderr
   for the `hydra: run <id>` line and makes the same two `herdr pane report-*` calls as
   `hydra-in-pane`. It also forces the one unmeasured thing: no human has yet confirmed the vendor
   TUI under the seatbelt (`IMPLEMENTATION.md:2741`).
2. Small. Two bridge tokens, `denials` and `tool_rate`, from the events JSON the bridge already
   fetches per live pane.
3. Medium. `hydra-mascot`, a Herdr pane: Opus's half-block sprite (11 rows, 36 columns, ASCII
   set as a second sprite, no Kitty graphics in v1 because ticket 45 never measured pass-through),
   14 frames per head, poses from the pane tokens, head count from `herdr pane list`.
4. Small. `puppets.py sheet`: one JSON per provider and model pair from one `hydra show` per
   record, plus the hand-kept vendor file. Backfill class and applied bit from build tickets.
5. Medium, after decisions A to C. The plan log, the launcher and the skill; a plan-picking
   prompt and a general diff checklist in `parse_judge_answer`'s closed shape.
6. Small. Findings 3, 4 and 5 above.
7. Large, later. `hydra puppets` in Rust, after three goals on the log.
8. Research, one per provider, before any interactive launch on `codex`, `cursor-agent` or `agy`.

## Decisions the map has to take, for the human

Graduated to grilling ticket 76. In one sentence each:

- A. Does the human confirming a proposed assignment table satisfy "never chosen on your behalf"?
- B. Where does the plan log live, and does it join the deny list every gated policy compiles?
- C. Is the approval unit the plan, the batch table and each diff, with per-launch approval only
  where a gate can fail open?
- D. Is a goal a harness run whose tasks are children, which widens contract 5.1's `command` enum?
- E. Does the human's verdict on a diff become a record event, so rung one can learn from it?
- F. May an interactive run go to a provider Hydra cannot harvest?
- G. May the bridge tail `claude`'s vendor transcript live, so the mascot sees a refusal on an
  interactive run?
- H. Is cost the window share over hand-entered budgets, and who revises a budget?
- I. Does the mode's review panel inherit rung two's rules against shrinking and substitution?

## The judge

The consolidation above went to `codex` as a judge, the mode's own last step run on itself. The
first attempt, under the `judge` policy (run 01a0d229-ffd9), was gated on its first read: `codex`
has no file tool but the shell, and `judge` denies the shell, which is map ticket 63 and a fact the
mode has to carry: a `codex` judge runs under `standard` until 63 is built. The second attempt
(run 01a0d22b-6516, 0.64M tokens, `standard`) answered `accept_with_changes` and named the codex
proposal the best of the four, "because it most clearly distinguishes a convenient manual workflow
from a controller with enforceable authority". The session accepts that verdict. What it corrects
in the text above:

- A Herdr plugin owning the orchestrator is a portability and recovery choice, not a D10
  violation; an optional plugin calling the CLI adds no dependency. The section above overstated it.
- The `hy` launcher's tee puts a pipe on the provider's fd 2, so "inherited terminal IO" is true
  of stdin and stdout only. Build ticket 106 measures it with a human at the keyboard.
- The head count is an activity gauge, not a memory gauge. `LIVE_RUN_CAP` limits children and
  measures nothing, and pane enumeration can double-count a run or miss one without a pane.
- The state location was not unanimous: `agy` put `plan.json` inside the goal's worktree, and the
  external protected location is a correction of that, not a consensus.
- The spec says only `claude` is confirmed to write an append-only transcript; the other three are
  unmeasured, not incapable.

What the consolidation dropped from the codex proposal and should not have: a deterministic
controller owns execution authority and the skill only assists; approval must be isolated from
files an agent can write and from Herdr metadata, because a `standard` run has already reached the
human's live Herdr pane (`IMPLEMENTATION.md:2802`); launch intent is persisted before spawning and
reconciled after a crash by a unique attempt id, never relaunched blind; reviewed trees are frozen
and an apply approval binds to the integration base; a synthesized plan is a new candidate that
gets its own review; rankings carry uncertainty and repair effort, so a repaired diff does not count
as a first-pass success. The judge's build-order changes: the small fixes (face enum, stale
documents, rate-limit expiry) move before anything that reuses the quota check; the manual plan
pilot moves ahead of the mascot animation; item 5 splits into plan schema and manual pilot, durable
launch and recovery, and artifact-bound review; and three goals are a checkpoint for a Rust
rewrite, not a mandate. Ticket 76 gains decision J for the prerequisites the judge listed.

## A local model, Colab, and training at 4 bits from the start

Session material, not a panel conclusion, and no dependency of the mode.

Answered by the session on 2026-09-24, not by the panel.

**What the machine can hold.** The MacBook is an M2 with 8 GB and, while two panel runs and one
subagent were alive, swap sat at 3.2 GB of 4 GB. A 3B to 4B model at 4 bits is 2 to 2.5 GB
resident and would push the agents out. What fits beside them is a model under about 1.3 GB: a
1.5B to 2B model at 4 bits, or BitNet b1.58 2B4T at 0.4 GB (the technical report,
https://arxiv.org/abs/2504.12285, puts it within one to two points of same-size fp16 models on
MMLU, GSM8K and HumanEval+, and `bitnet.cpp` has ARM kernels). No inference runtime is installed
today: no `ollama`, no `llama-server`, no `mlx_lm`.

**Colab on the Google AI plan is for jobs, not for hosting.** The announcement of 2026-09-22
(https://developers.googleblog.com/colab-is-now-part-of-your-google-ai-plan/) says AI Plus, Pro and
Ultra get priority on faster accelerators; only Ultra gets premium GPUs and background execution.
It names no compute-unit figure; secondary sources say about 200 units a month on Pro. The Colab
FAQ (https://research.google.com/colaboratory/faq.html) caps a runtime at 12 hours, times it out
when idle, and lists among disallowed uses "web service offerings not related to interactive
compute", "connecting to remote proxies" and "remote control such as SSH shells". A model served
from a notebook through a tunnel is exactly that, and it dies every 12 hours anyway. So Colab is
where a training or evaluation job runs and where its weights are downloaded from, and the model
that Hydra talks to runs on the Mac or on a real endpoint.

**"An empty shell trained at 4 bits from the start" is a real line of work, and it does not
change the arithmetic.** Training with quantized weights from initialization, with a full-precision
shadow copy for the gradient, is quantization-aware pretraining; BitNet is the 1.58-bit case and
there is sub-100M-parameter work on when to switch bit width during training
(https://arxiv.org/abs/2605.25966). The result is that the model finds its own way to place
information in coarse weights, which is the intuition behind the idea, and the reports show it
costs one to two points at 2B. What it does not remove is the token count: BitNet 2B4T saw four
trillion tokens. On one Colab GPU a from-scratch pretrain of even 500M parameters to a useful
coder is months of compute units. A "model that maps rather than compresses" with a lookup or
hash structure instead of dense layers is cheaper to train and much weaker at generation; nobody
has shown one that writes Rust.

**What is worth doing, in order.**
1. Nothing that needs an LLM. The task grade and task class in the mode, and the ranking per
   class, are a small classifier over record features (tokens, duration, refusals, outcome, the
   review corrections a human made) trained on the 380 records here. That is minutes on the Mac
   and it is the map's "mini agent as a peripheral controller" item made concrete.
2. BitNet b1.58 2B4T or a 1.5B Qwen-class model at 4 bits on the Mac, behind an MCP server,
   admitted per policy as `config/mcp.md` already allows, for the jobs where a menu answer is
   enough: classify a task, summarize a record, choose a provider from a table. Its calls land in
   the record as `mcp__` tool calls today.
3. Colab on the AI Pro plan for a QAT fine-tune of that small model on Hydra's own corpus once the
   corpus is large enough to teach anything, which it is not at 380 records. Weights come back to
   the Mac; nothing is served from the notebook.
4. A from-scratch low-bit pretrain is a research project with its own budget, and it stays under
   D12 ("training is out of scope") until the corpus exists that would justify it.
