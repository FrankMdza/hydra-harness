# 76 - The nine decisions under Master of Puppets

Type: grilling
Status: open
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

## Not to answer here

The mascot's look, the sprite set and the frame count are build work on the opus proposal's
drawing. The spec sheet schema is build work on the same proposal. Neither needs a decision.
