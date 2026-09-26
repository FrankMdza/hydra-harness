# 76 - The nine decisions under Master of Puppets

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-24, by the session, from ticket 75's panel. Needs the human.

## Question

Ticket 75's panel left nine decisions the code cannot take. Each is one sentence, and the reasoning
behind each is in [research/75](../research/75-master-of-puppets-panel.md) and in the opus and
codex proposals beside its brief. Build items 5 and later in that document wait on A to C; the
mascot's refused pose on an interactive run waits on G.

- A. Does the human confirming a proposed assignment table satisfy "a model inside the provider,
  never chosen on your behalf" (`src/cli.rs:49`, map ticket 20)?
- B. Where does the plan log live, and does `~/hydra/puppets/` join the deny list every gated
  policy compiles, which is protected config #1?
- C. Is the approval unit the plan, the batch assignment table and each diff, with per-launch
  approval only where a gate can fail open (`codex` on a dead hook, `claude` on a gate that fails
  to start), or every `hydra run`?
- D. Is a goal a harness run whose tasks are children, which widens contract 5.1's `command` enum
  and meets `Lineage::of` refusing a parent with a null policy?
- E. Does the human's verdict on a diff (applied, fixed, rejected) become a record event under
  contract 15, so rung one can learn which runs a human applied?
- F. May an interactive run go to a provider Hydra cannot harvest, or does harness spec 3's rule
  (gate it through a root Hydra owns, or do not route it) refuse it?
- G. May the Herdr bridge tail `claude`'s append-only vendor transcript live, so the mascot sees a
  refusal on an interactive run before the harvest at exit?
- H. Is cost the share of the binding window over hand-entered budgets, and who revises a budget
  when a window closes early?
- I. Does the mode's review panel inherit rung two's rules against shrinking and substitution, or
  may planning run with two seats when one is limited?

- J. Added by the judge (research/75, "The judge"): before any automatic execution, who owns crash
  recovery, bounded retries, cancellation, the concurrency limit on this machine, dependency
  integration and the immutable review artifact, and does approval authority live outside every
  file an agent can write?

## Answer, partial

Answered with the human on 2026-09-24 through AskUserQuestion, one recommended option per
question, the four that block the first build. D to H stay open and block nothing in the manual
pilot.

- **A. Yes.** A proposed assignment table (task, provider, model, policy) confirmed by the human
  once per batch is the human choosing the model. Changing any row after confirmation reopens the
  table.
- **B. `~/hydra/puppets/`**, one append-only JSONL per goal, outside every worktree. It joins the
  deny list. That list turned out to live in code, `denied_write_paths` in `src/policy/mod.rs`,
  not in the five protected configs, so joining it is build ticket 110 rather than a human edit.
- **C. The plan once, the assignment table once per batch, and every diff.** Per-launch approval
  only where a gate can fail open: `codex` on a dead hook, `claude` on a gate that fails to start.
  An approval binds to a plan revision and is consumed once.
- **I and J. The orchestrator is this session with a skill plus `puppets.py`**, and the first
  version is a manual pilot over the log: proposers launched by the helper, tasks and table edited
  by the human, launches fired by the session, diffs applied by the human. Automatic execution
  waits for bounded retries, crash recovery with a unique attempt per launch, and approval kept
  outside every file an agent can write. Rust after three goals, as a checkpoint.

Build tickets 108 to 111 carry the code.

## Answer, the rest

Answered with the human on 2026-09-24 in the evening, through AskUserQuestion, after the first
goal (ea69da1f) had run on the manual pilot. The session wrote the recommendations; research/75
left D to H without one.

- **D. No.** A goal is not a harness run and its tasks are not children. The contract stays as it
  is: no new `command`, and nothing for `Lineage::of` to decide. The goal's log in
  `~/hydra/puppets/<goal>.jsonl` already joins goal, tasks and run ids. Revisit at the Rust
  checkpoint after three goals, once the log format has survived.
- **E. Not now.** The human's verdict on a diff stays in the goal's log, append-only and outside
  every worktree, where `sheet.py` already reads it. It does not become a record event: rung one
  reads only records (D5), and a line after `run_end` would break the rule that a closed record
  is a run that stopped. It goes on the map as fog beside "who reads the policy signal", a signal
  kept before anything reads it. Decide when rung one has a reader for it.
- **F. Not refused, and `hy` asks for the provider and the model.** The human's answer rejected
  both offered options: `hy` should prompt for a provider and a model instead of assuming `claude`.
  A follow-up settled the gate. `hy` offers only providers whose gate is measured in their own
  interactive TUI, which today means `claude` alone. Each of `cursor-agent`, `agy` and `codex`
  joins after a build that launches it interactively under Hydra and proves in its TUI that a gate
  refusal arrives. The half of spec 3 about gating through a root Hydra owns stands. The half about
  harvest is relaxed: until Hydra can harvest a provider, that provider's interactive record is
  `run_start` and `run_end` and says so.
- **G. Yes, read-only, in the bridge only.** The Herdr bridge may tail `claude`'s append-only
  transcript under `~/.claude/projects/<slug>/` to pose the mascot on a refusal during an
  interactive run. Nothing under `src/` reads it, and the record still comes only from the harvest.
  A vendor format change breaks the mascot, not Hydra. The bridge runs as the human, outside every
  gate.
- **H. Cost stays window share, and the session revises the budget on its own and tells the
  human.** When a record ends `rate_limited`, the session sums what that provider spent in the
  window and rewrites `window_budget_tokens` in `integrations/puppets/vendors.json`. It reports
  the change to the human, who can sign it or undo it. The human asked how this could go wrong,
  and the session named two ways: the session sees only Hydra's spending, so usage outside Hydra
  in the same window makes the measured sum too low; and a budget that moves silently makes an
  old ranking and a new one incomparable. So every rewrite keeps the previous value, the date,
  the run ids behind it, and whether the window saw spending Hydra could not count, in the same
  file, and one line undoes it.

Build tickets 129 to 132. D and E carry none.

Map ticket 81-a, 2026-09-26, amended I for the mode's panel: a degraded panel, declared and approved
per goal, may fill a missing seat. Rung two keeps the rule.

## Not to answer here

The mascot's look, the sprite set and the frame count are build work on the opus proposal's
drawing. The spec sheet schema is build work on the same proposal. Neither needs a decision.
