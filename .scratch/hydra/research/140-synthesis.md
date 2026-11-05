# 140 synthesis: weight editing joins 386, Infinity lends ideas only

Panel of 2026-11-04 on `research/140-two-videos.md`. Seats under `standard`, read only, from
`research/140-panel-prompt.md`:

- GPT, codex `gpt-6-astra`, run `01a1a913-be90`, 210k tokens, memo `140-seat-gpt.md`.
- Gemini, agy `gemini-3.8-flash-high`, run `01a1a915-61a9`, 1.17M tokens, memo `140-seat-gemini.md`.
- Opus, `claude-opus-5-5`, run `01a1a917-15ea`, 763k tokens, memo `140-seat-opus.md`.
- Grok not seated, Cursor's login.

Question 3 split three ways. The session's reading is below, and the human decides.

## What the session checked

- **A resumed run starts at depth 0.** `src/resume.rs:121` passes `parent: None`, and
  `src/run/lineage.rs:42-47` gives a run with no parent depth 0. A depth-2 child that is resumed
  can delegate two more levels. Opus found it. It is harmless while a human types every resume and
  a hole once anything resumes on its own.
- **Spec 10's "fails loudly" for a provider with no resume has no case.** `supports_resume`
  returns `true` for all four (`src/provider/mod.rs:271-273`). Opus found it.
- **An interactive run cannot be resumed** (`src/resume.rs:50-55`). So wake on message cannot put
  the orchestrating session to sleep, only recorded runs that delegate.
- **No tokio in Hydra.** `grep -c '^name = "tokio"' Cargo.lock` returns 0. Infinity's core crate
  brings tokio, reqwest and hyper.
- The two spec sentences Opus names exist: harness spec line 1131 ("Hydra needs no scheduler") and
  line 1272 ("nothing needed to be running").
- **Gemini's reason for writing the loop is contradicted.** It says `ModelProvider` is built for
  remote HTTP endpoints. GPT and Opus both read the trait and found that it needs async streamed
  completions and `Send + Sync`, not HTTP. Opus cites `SingleModelProvider` at
  `infinity-provider-protocol/src/lib.rs:85-104`. The session did not open the file. Two seats
  against one.

## Question 1, the short and map 138: no vote changes, three of three

All three keep Burn with the PyTorch reference, frozen layers before LoRA, the gate classifier
first, and frontier weight diffs with the human. On adding an edit experiment:

- GPT: yes, a human-written edit after 385 and before 386, on one diagnosed failure, against a
  sham edit, $0.
- Opus: yes, as an extra arm of 386 rather than a new ticket, measured the way 386 measures frozen
  layers and LoRA, $0 on the Mac's CPU.
- Gemini: no, the fog after 386, because editing cannot teach a missing ability.

The session's reading: two of three add it, and Opus's form is the smaller one. 386 gains a third
arm, a direct weight edit on the gate model for one class of false allows, written by a human or a
script from activations until the human answers the frontier-diff question. Gemini's point holds
inside it: the arm fixes, it does not teach, so it cannot come before 385.

## Question 2, Infinity: A, three of three

Borrow ideas only. C is out under map 137's answer, three of three: histories in Aurora DSQL, a
per-completion bill, no pty on Lambda. B waits for a nano model that calls tools, three of three,
which map 138 does not produce (the gate classifier calls no tools).

Opus and GPT both say A's RAM gain is unmeasured and smaller than the ticket's 200 MB per run
suggests: only a waiting parent is freed, a working child keeps its process.

## Question 3, which idea first: split

- GPT: stable input ids, with a map decision first because duplicate state touches D10.
- Gemini: reserved-suffix synthetic events in `src/face.rs`, no map decision.
- Opus: measure first. Count the recorded runs that delegate, the peak waiting at once and the RAM
  they held, with the threshold written before the run (under 400 MB, wake is dropped). Then a map
  ticket on who wakes a parent.

Against the code: Gemini's suffix needs input delivered to a live run, and section 10 says "No
attach, no input", so it is not a formatting change. Opus found no duplicate for stable ids to fix,
and GPT names none. Opus's measurement costs no code and decides whether the other two are worth
it. The session's reading is Opus's order. A judge is not needed unless the human wants one.

On liveness, GPT and Opus agree: wake on message keeps the pid rule if each wake is a fresh linked
run, as `hydra resume` already makes. Gemini reaches the same place by another road (exit, then
resume).

## Question 4, B later: write the loop, two of three

When: after a model that calls tools, three of three. Opus and Gemini write the loop. GPT reuses
Infinity's core behind an adapter. Opus's reason holds: the core crate adds tokio, hyper, reqwest
and rhai to a binary with none, so borrow the trait's shape and keep history as linked runs, so D5's
single record stays. GPT adds that B still needs an explicit exception to "Hydra never reconstructs
conversation state itself".

## Question 5, security

All three: no RAP endpoint and no network listener. The credential deny (387) is needed but does
not authenticate an event. Opus's design is the simplest: events are derived from records, not
received. A waker reads the child's record, which the corpus deny already keeps from runs
(`src/sandbox.rs:465`), and launches through `run::run` so the allowlist and 387 apply. GPT's
envelope (issuer, destination, sequence, digest, replay rejection) and 382's dual signature apply
only if an event ever crosses a process or a machine.

## The tickets that follow

1. **Build 389, a resumed run keeps its depth.** Depth through the `resumed_from` chain as well as
   `parent_run_id`. In `tests/delegation.rs`, a resumed depth-2 child is refused at the cap. Then
   revert the change and watch the refusal disappear. It is a bug fix and needs no vote.
2. **Build 390, measure the waiting parents.** No change to `src/`. A tally over the records, tested
   on a fixture corpus, with Opus's threshold written first.
3. **Map 141, wake on message**, blocked by 390. Who wakes a parent, what spends the quota, and how
   lines 1131 and 1272 change.
4. **Build 386, when it is opened**, carries the weight-edit arm. Map 138's tickets 384 to 386 are
   proposed and not yet open.

## What the human decides

- Questions 1, 2, 4 and 5 as above.
- Question 3's order, measure first, or GPT's or Gemini's.
- Whether 389 waits for these answers or goes now.
