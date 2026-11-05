# 82 - A judge that needs subagents inside the seatbelt

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-27, by the session, at the human's request, after a panel judge in a private repository hit
map ticket 64's refusal and worked around it without a record.

## Question

Map ticket 64 closed delegation from inside a macOS run on 2026-09-22: `delegate` answers
`hydra/seatbelt-delegation` (`src/face.rs:57`, build ticket 76), because the child is Hydra itself
and the inherited seatbelt denies the corpus. The reason it gave for not building a way around it
was that the face "was used on two runs in its whole history", a feature nobody uses.

On 2026-09-27 a use appeared. A panel in one of the author's private repositories (four seats on
four model families) called a Fable 5.1 judge that orchestrates its own subagents to validate
hypotheses before it recommends. Run `01a0e135`
(`claude-fable-5-1`, policy `permissive`) called `mcp__hydra__delegate` twice and got the refusal
both times. The brief allowed a fallback, so it used Claude's native Agent tool four times instead.
The work was good, but:

- **Nothing of the four subagents is in the corpus.** The parent's record shows four `Agent` tool
  calls and their returned text. There is no child record, no usage per child, no gate per child,
  nothing `hydra show` or rung one can read. That is the opposite of what the face exists for.
- **The fallback needed `permissive`.** It is the only policy with `subagent_spawn`
  (`policies/permissive.json`), and it also grants `file_write` anywhere, `mcp_call` and
  `provider_native`. A judge that only needed to spawn readers was handed the whole disk. The
  session backed up that repository's state by hand before the launch and diffed it after; nothing
  was touched, but the gate did not ensure that.

The question is whether 64's answer still holds now that the feature has a user, and if not, which
shape. The options the session sees, none decided:

A. **Keep 64.** The judge runs under `standard`, writes the hypotheses it wants validated, the
   orchestrator outside the run launches each child with `hydra run --parent`, and a second judge
   run reads the results. Recorded and safe, two or three round trips, and the judge cannot follow
   up on a surprising number within its own run.
B. **Move the launch to the watcher**, the detached half outside the seatbelt, with the face asking
   it over a channel. 64 declined this as a second channel between a run and Hydra that D5 would
   have to rule on. The use case is new evidence for or against that.
C. **Record what native subagents do** without granting delegation: read the subagent transcripts
   Claude Code writes and fold them into the parent's record, or into child records marked as
   in-process. Ticket 78 found Claude Code 2.1.281 saves large outputs under
   `~/.claude/projects/<slug>/<session>/tool-results/`, which `standard` cannot read; where the
   subagent transcripts live has to be measured first.
D. **A narrower policy with `subagent_spawn` and without `file_write` anywhere**, so a judge that
   spawns readers is not handed the disk. That changes the five policies of harness spec 7.2, which
   is protected config #1 and the human's to edit.

C and D are not exclusive, and either one without the other leaves half the problem.

## Evidence

- Run `01a0e135-5a8b-7602-9a4f-74ca5db46961`: the two refusals and the four `Agent` calls.
- The judge's brief and verdict, in the private repository. The verdict's section 2 names each
  native subagent by its agent id, the only trace of them.
- Map ticket 64's answer and `docs/harness-spec.md` section 9 on the face.

## Grilling, 2026-09-28

New evidence the same night: the human's two interactive `hy` sessions under `standard`
(`01a0e68b`, training; `01a0e715`, web) both needed a panel or a browser and were refused
(`mcp_call`, `subagent_spawn`). The human's intent for Master of Puppets is stated: a real
interactive session launched from Hydra, gated and recorded, that orchestrates, reviews and
delegates to the multi-vendor panel. Option B (the watcher launches children for the run) is the
direction; this grilling fixes its shape.

- Q1, who approves a launch the session asks for. Answer: nobody, when the request fits a shape
  the human approved beforehand (for example the fixed three-seat panel, read only, at most N
  launches per hour); the human, every time, for anything outside that shape.
- Q2, what each seat may do. Answer: the policy the approved shape names for that seat, capped at
  the requesting session's policy (a `standard` session never gets a `permissive` child). Measured
  the same night: `judge` is `"allows": "nothing"` by design (`policies/judge.json`), so a judge
  seat gets its evidence pasted into the prompt. A seat that must read files it was not handed
  needs a read-only policy, which does not exist today; that is option D, now required by B.
  Run `01a0e72a-4d32-7283` (composer as `judge`, told to read files) is the refusal on record.
- Q3, the read-only policy (`reader`, name not fixed). Answer: file reads inside the seat's
  worktree, plus a shell limited to an allowlist of read-only commands (`grep`, `rg`, `find`,
  `sed`, `cat`, `head`, `tail`, `wc`, `ls`, `git log`/`show`/`diff`, list to be fixed by the human),
  plus a seatbelt profile that denies every write and every read outside the worktree. The
  allowlist is the first filter, the seatbelt the guarantee. The build has to settle: flags that
  write or execute (`sed -i`, `find -delete`, `find -exec`), and pipes, `$(...)` and `xargs`
  (split them or refuse what the gate cannot read). No tests under this policy: `cargo test`
  writes `target/`. The policy JSON is protected config #1, the human's to write.
- Q4, the channel. Answer: the existing MCP face (`mcp__hydra__delegate`, `tests/face.rs`,
  `tests/delegation.rs`), which stops launching the child itself and hands the request to the
  watcher, outside the seatbelt, which launches it. The session's policy gains `mcp_call` scoped
  to `mcp__hydra__*` only, never every MCP server. The face keeps marking each child's text as
  foreign on the way back. The face-to-watcher hop is the second channel D5 must rule on.
- Q5, where approved shapes live. Answer: a new protected config, `config/panels.json` (#6 in
  harness spec 15.1), written only by the human. Each shape names its seats (provider, model,
  policy, role), a launch cap per hour, and which sessions may ask for it. The watcher refuses
  any request that matches no shape unless the human approves it (Q1).
- Correction to Q2 and Q3, found by the validation run below: `policies/read-only.json` exists
  (file reads in the worktree and skill snapshot, skill invoke, no shell, no writes). Q3's shape
  is that policy plus the allowlisted read-only shell and a seatbelt that denies every write,
  whether as a change to `read-only` or a sixth policy is for the human when writing the JSON.
- Q4 amended: interactive mode has no detached half (`src/interactive.rs:7`). The process that
  launches children for an interactive session is the waiting `hydra run --interactive` itself,
  which sits outside the seatbelt.
- Q6, how the human approves a request outside every shape. The human proposed (b) a prompt in
  the same TUI plus (c) approval from the orchestrating session. A validation run
  (`01a0e75a`, claude-fable-5-1, `standard`) found against both as proposed: in interactive mode
  the gated child shares the human's controlling terminal, so (b) has two readers of one
  keyboard, a spoofable prompt, and a measured forgery path (see build ticket 137); and (c) lets a
  model approve, which contradicts Q1, and harness spec 10 (`docs/harness-spec.md:1199-1202`)
  already rules out answering a live run's request from outside without a map decision.
  Pending the human's answer.
- Q6 answered: only a human-typed `hydra approve <id>` in a terminal outside `hy`. The command
  refuses when `inside_seatbelt()` is true, as the face does (`src/face.rs:236`), and the approval
  store lives under `~/hydra`, where the kernel denies the run's writes. No model approves,
  orchestrator included. A prompt inside the same TUI waits for a map ticket on a Hydra-owned pty
  in interactive mode, and build 137 closes the injection path first.

## Answer

Resolved 2026-09-28 with the human, Q1 to Q6 above, Q6 after a validation run. Option B, with D
inside it; C is not built. It amends map 64 (delegation from inside a run returns, through the
launcher, for an interactive session) and map 63 (the bounded read-only command list is built,
for one policy, under a seatbelt that denies writes).

1. **Who launches.** For a gated interactive session, the waiting `hydra run --interactive`
   process, outside the seatbelt. The face (`mcp__hydra__delegate`) stops launching and hands the
   request to it. That hop is the second channel D5 rules on, and this ticket admits it: one
   channel, per run, from the face of that run to its own launcher, carrying a launch request
   and returning the marked result. Nothing else crosses it. A non-interactive run keeps 64's
   refusal until a ticket gives it a launcher.
2. **What may be asked.** A request that matches a shape in `config/panels.json` launches with no
   prompt. Each shape names its seats (provider, model, policy, role), a cap per hour, and which
   sessions may ask for it. The file is protected config #6, written by the human only.
3. **What a seat may do.** The policy the shape names, never more than the requesting session's.
   A judge gets its evidence in the prompt. A seat that reads files gets the read-only policy:
   file reads in its worktree, a shell limited to a list of read-only commands the human fixes,
   and a seatbelt that denies every write and every read outside the worktree.
4. **Anything else.** Pending until the human types `hydra approve <id>` in a terminal outside
   `hy`. The command refuses inside a seatbelt; the store lives under `~/hydra`, where the kernel
   denies the run's writes. No model approves, the orchestrating session included. A prompt
   inside the same TUI needs a Hydra-owned pty in interactive mode, which is a separate map
   question; build 137 closed the terminal injection path first.
5. **The record.** Every seat is a child run with the session as `parent_run_id`, so the panel
   is in the corpus, which is what 82 found missing.

Build tickets 138 to 141. Spec amendments ride in each: 7.2 and 15.1 (138, 139), 9 and 10 (140,
141).
