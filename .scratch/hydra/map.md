# Hydra: unified agent harness and Ouroboros

Label: wayfinder:map

## Destination

Three documents, implementable and hand-offable without further discovery:

1. **Harness spec.** How Hydra launches, feeds and records the three agent CLIs.
2. **Ouroboros spec.** Both rungs of self-improvement, with rung one shippable first.
3. **The run-record contract.** The one format both write against.

No production code while this map is open. Throwaway prototypes are fine.

**Reached, 2026-09-05.** `docs/harness-spec.md` (1,356 lines), `docs/run-record-contract.md` (1,323)
and `docs/ouroboros-spec.md` (904). All three written under ticket 16, one document per session with
the seam check alone in a fourth, then tickets 37 and 38 closing what that check found.

**All four seam checks pass.** D5 holds, nothing crosses but the run record and the skill store. The
D12 export test holds. Every field Ouroboros reads is one the harness writes, on both launch modes
now that the producer column says which population it describes. And the harness claims no guarantee
Ouroboros does not rely on, which is the check that broke and took two tickets to fix.

**What the seam check was worth.** Two of the four broke. The gate turned out to be carrying four
guarantees nobody had written down, so a `permissive` run could write into `~/hydra/runs/` and edit
four of the five protected configs while both documents read as if it could not. And the producer
column, built as the seam check's own instrument, described recorded runs only, which left the one
launch mode that produces preference pairs with no specification at all. Neither was findable from
inside a single document, which is the argument for having run the check as its own session.

Nine contradictions between documents were fixed along the way, each carrying a decision another
document had already made. Two of them were counts stated wrong in a document that opens by
complaining about counts stated wrong.

**No production code was written while this map was open** (D1). The three documents are the
destination and someone who was in none of these sessions can build from them.

**The build started after this map closed**, and it is charted separately in
`.scratch/hydra/build/plan.md`, because wayfinder tickets resolve decisions and build tickets execute
slices. Nothing in that file reopens a decision here.

**Reopened 2026-09-06 at 38 of 43, by the build rather than by a session of its own.** Build tickets
07 and 12 hit four things that need a decision rather than an implementation, which is the one
condition for reopening this map, and ticket 08 then hit a fifth. All three wrote them into
`IMPLEMENTATION.md` beside the code that hit them and none decided one. They are tickets 39 to 43.
Ticket 39 closed on 2026-09-06 and the other four are open:

- [39](issues/39-linked-skill-nobody-can-reach.md) A linked skill is unreachable under four of the
  five policies. Injection is built and measured; under everything but `permissive` a skill reaches
  the model as a listing entry it can neither invoke nor read.
- [40](issues/40-where-the-skill-store-scope-points.md) Where the `skill_store` scope points.
  Harness spec 7.2 and `docs/ouroboros-spec.md` 3.6 disagree, the code answers, and the confirming
  never happened. Graduated from the fog.
- [41](issues/41-curate-checkout-against-the-store-pin.md) The curate checkout runs before the
  launch commits the store, so a pass on a dirty store reads a checkout its own launch moves past.
  Was blocked by 40, and closed the same day 40 did.
- [42](issues/42-a-record-carries-no-scopes.md) A record pins `policy_sha` and not the paths its
  scopes resolved to, so a build that moves a scope rewrites how every earlier record reads. Closed
  2026-09-06.
- [43](issues/43-restricted-keeps-mcp-on-judge.md) `--restricted` no longer drops MCP on `claude`
  2.1.263, so a `judge` run keeps the human's own servers. Provider drift under 6.5 and 6.7, and the
  fix is one flag on an argv the design writes. Closed 2026-09-06, and the premise was wrong.
- [47](issues/47-judge-gets-linked-skills-it-cannot-see.md) A `judge` run is linked skills it cannot
  list, read or invoke, and build ticket 16 papered over it with a null. Whether 6.4 should inject
  on `judge` at all. Opened 2026-09-07 from the build and closed the same day: no, and nothing is
  lost, because `judge` allows nothing and its diff rides the prompt. Build ticket 25.
- [48](issues/48-the-face-needs-the-run-id-and-the-document-has-no-field.md) The delegate face
  needs `--parent <run-id>` per run and the MCP document has no field that could say so, so every
  tree a document-spawned face makes is flat. Opened 2026-09-07 from the build and closed the same
  day: Hydra recognizes its own binary by basename and appends the flag, and the face is admitted
  under `standard` and `permissive`. Build ticket 26.
- [49](issues/49-what-the-herdr-bridge-reports.md) Which of Herdr's four states `timed out` sits
  on, and whether `base_tree` goes on the record. Two small decisions ticket 45 measured and left
  to the human, opened 2026-09-07 before the bridge gets built around a guess. Closed the same
  day: `unknown`, with the pane carrying the record's own facts as tokens, and yes, in
  `run_start.worktree`. Build tickets 23 and 24 carry the code.
- [50](issues/50-gate-on-an-interactive-claude-session.md) Whether Hydra's hooks, handed to an
  interactive `claude` session through `--settings`, gate it and leave their decisions in the
  transcript the harvest reads. Opened 2026-09-07 by the human's question on fase 2: interactive
  runs are the only producer of human corrections and today the only mode without the gate. Probed
  the same day: the gate applies, two refusals with Hydra's reasons on a steered session, and the
  denials reach the transcript as error tool results while allows leave nothing. Whether the
  interactive launch takes a policy was the decision it left, answered the same day: optional,
  none by default, `claude` only. Build ticket 28.
- [51](issues/51-cursor-runs-readlints-past-a-deny.md) `cursor-agent` ran `ReadLints` with a
  list-valued `paths` argument the table does not know, the replay derives a deny, and the stream
  shows the tool completing anyway. Opened 2026-09-08 from build ticket 29's run. Whether the hook
  fired, whether Cursor obeys a deny on that tool, and what a leaked tool means for the policy set.
  Measured three times the same day, print mode and interactive, down to a file that cannot
  parse: no hook, no analyzer, no content. Closed the same day: an `unhooked` row for three
  tools. Build ticket 35.
- [52](issues/52-rung-two-build-details.md) Rung two's three open details from Ouroboros spec 9:
  the proposer's policy, fitting a review prompt, tolerant parsing. Opened and closed 2026-09-08
  with the human: `standard`, a size cap on the proposer's diff so nothing is reviewed blind, and
  one fixed JSON shape where any deviation is a block. Build ticket 36 carries `hydra evolve`.

Three of the first four are one question seen from different sides: what a scope is, and who gets
to know. The fifth is the first flag claim to expire under the build plan's drift note. None of
them blocks build ticket 09 or 10, which is what the frontier will hand the next session.

**Tickets 39, 40 and 42 were prepared on 2026-09-06 without being resolved.** Each carries a
`## Prepared for the human` section: the claims measured against a compiled gate, the options
costed in files and lines and tests, and numbered questions only a human can answer. A session
resolving one starts from that section.

Ticket 39 closed on 2026-09-06 from its brief, in one session with one measurement. That session
also added tickets 44 to 46 at the human's request and closed 44.

**Ticket 45 closed the same day and took two of ticket 44's claims with it, so the map stands at 41
of 46.** Herdr and Hunk are installed and driven, and the probe found one thing that is a decision
rather than a correction: `diff_stat.base_tree`, which 44 has Hunk opening a run's branch against,
is on no record and reachable from no git ref. Whether Hydra writes it into the record is the
question, and it is noted in 44 rather than ticketed here. Tickets 40, 42, 43 and 46 stay open, all
four grillings that need the human, and 41 stays blocked by 40.

**Ticket 40 closed on 2026-09-06 from its brief, and the map stands at 42 of 46.** Four rounds with
the human, three from the brief and a fourth the human asked for: which policies can make Hydra
lose data. That round measured a gate per policy against 32 destructive routes and found the flat
list of seven denied paths protected the record and nothing else under `~/hydra` from a `standard`
run's shell. The list is two kinds of entry now, and 7.4 and 7.5 carry the measurements. Tickets
42, 43 and 46 stay open, all grillings that need the human, and 41 is unblocked and takeable.

**Ticket 41 closed on 2026-09-06 without the human, and the map stands at 43 of 46.** It was the
one task ticket, and 40 had decided it in passing: the store has two writers left, so the curate
pass pins before it checks out and the launch pins the same sha. Tickets 42, 43 and 46 are what is
open, all grillings.

**Ticket 42 closed on 2026-09-06 from its brief, and the map stands at 44 of 46.** Two rounds with
the human. The record gains `scopes` and `floor_lifted`, the reader prefers them and reports a limit
where they are absent, and no version bump. Build ticket 15 carries the code. Tickets 43 and 46 are
what is open, both grillings.

**Ticket 43 closed on 2026-09-06 in one round, and the map stands at 45 of 46.** The measurement
the ticket said needed the human's config was made under a scratch `HOME` instead, and it showed
the hole was never live: `--restricted` drops settings-defined servers and keeps only `--mcp-config`.
The flag goes on `judge` anyway, the probe follows, `ToolSearch` moves beside `skill_invoke`, and a
drift test guards the claim. Build ticket 16 carries the code. Ticket 46 is what is open.

**Ticket 46 closed on 2026-09-06 in one round, and the map is closed again at 46 of 46.** A user
hook's `additionalContext` is data, Hydra keeps loading the user's settings, the text stays in
`raw`, and the Cursor case is stated as unmeasured. Nothing on the map is open. Four of the five
reopening tickets left a build ticket each, 14 through 16 in `build/plan.md`, and the map closes
the way it closed the first time: a session that finds a decision rather than an implementation
reopens it.

**Build tickets 11, 15 and 16 closed on 2026-09-07 and two of them found a decision, so the map is
open again at 46 of 48, then 50 of 50 after tickets 47 to 50 closed, all on 2026-09-07, with 50 leaving one question for the human.** Ticket 16 measured `--restricted` dropping the skills Hydra links into a
`judge` run, which is ticket 47. Ticket 11 built the delegate face and found that the MCP document
cannot hand it the run id, which is ticket 48. Both are grillings, and neither blocks anything on
the build, which stands at 16 of 16.

**Tickets 51 and 52 opened and closed on 2026-09-08, and the map is closed again at 52 of 52.**
Build ticket 29's Composer run called `ReadLints` with a list of paths, the table knew no such
field, and the tool ran past the deny the replay derives; three measurements the same day showed
Cursor never asks the hook for that tool, nor for its MCP listing or its background-task wait,
and that nothing sits behind `ReadLints` in either mode. The human chose an `unhooked` row kind.
Ticket 52 took the three details Ouroboros spec 9 left open, once every other command was built
and `evolve` was the last, and the human closed all three in one round. Build tickets 35 and 36.

**The map reopened on 2026-09-09 at 54 of 56, on training.** Rung two produced its first verdict
worth reading: pass `01a084d3-bdd3` gave a proposer a task for the first time, got a real patch, and
the panel blocked it on a correct critical finding. That met the exact condition the fog entry on a
second corpus had been waiting for since ticket 19. Ticket 54 measured where a judge finding lives
and found the training channel built and empty, with a reader missing rather than a format. Ticket
56 put the first skill through that channel and watched it arrive at the model's listing and stop
there. Pass `01a08684-67f4` then found the first thing the new flag broke, which is ticket 57: the
proposer obeyed both contracts its task named, and `agy` blocked it for one of them, because
`judge_task` hands a judge the checklist and the diff and never the task. The map stands at 54
of 58 with tickets 53, 55 and 57 open, all grillings, all needing the human, and 58, a
research ticket on Codex CLI as the fourth provider, opened 2026-09-12 when the human asked
for more providers that sign in rather than take a key. Ticket 59, the same day, is the
three documents audited against the code before publishing: five claims marked wrong, among
them that admission refuses code rows (it refuses `policies/` and `config/` only), that
`hydra export` writes TRL shapes (it writes events), and that a curate run carries
`command: curate` (it carries `run`). Build tickets 40 to 43 carry the code half. Builds 40 and 43 landed the same day, and 40 found ticket 60: the hook-config
writer of 15.2's row 4 lives inside `src/run.rs`, which is the whole launch sequence, so
protecting it is a decision about the launch's shape rather than one more row. Ticket 55 was
rewritten on 2026-09-10 after the human said an SME is a model that gets trained and skills are
a separate thing: the session had framed it as a skill because that is what Hydra has. Spec 18's
training non-goal is about weights and says so, so a trained specialist is what the record's
lossless export exists to feed, coming back as an MCP server rather than a fourth provider. Three
things block it and the first is that `hydra export` refuses without `config/export-allowlist.json`,
protected config #3, which only a human writes.

## Notes

**Domain.** Hydra is a router, not an agent. One command picks a provider, injects the shared skills, launches that provider's own CLI as a child process on a pty it owns, and tees its structured output to a trajectory log. Ticket 02 settled that Hydra is the parent process rather than a client of a terminal; ticket 01 settled that it should speak ACP to the providers that have it. Three routable providers in v1: `claude` (2.1.261), `cursor-agent` (2026.09.02-c22c1a3), `agy` (1.1.26). `hermes` (0.14.0) was the fourth until ticket 22 unrouted it. Versions observed 2026-09-04 11:40; `agy` had already moved again since the earlier reading that day. **Two of the three self-update without asking. `agy` alone has moved through four versions in four days, and the drift has invalidated a finding three times**: `cursor-agent` went 2025.10.02 to 2026.08.31 on 09-01 and killed ticket 03's Cursor finding, then 2026.08.31 to 2026.09.02 on 09-02, superseding the build ticket 25 measured; `agy` went 1.0.10 to 1.1.23 mid-ticket-18, to 1.1.24 on 09-02, to 1.1.25 on 09-03 during ticket 15, then to 1.1.26 on 09-04 during ticket 16. `claude` broke its run of holding still and went 2.1.257 to 2.1.261 on 09-04 between tickets 34 and 35, which is why ticket 34 read one binary and ticket 35 exercised another. Ticket 26 probed the hook gate on 1.1.24 and ticket 30 re-measured it on 1.1.25; `--add-dir` and `--output-format stream-json` were re-checked on 1.1.26 and both survive. All three have now moved under this map, so none of them holds still. Re-read every version at the top of a session, and date every provider-specific claim you write. **Version is not the only thing that drifts.** Ticket 33 found `cursor-agent` reporting a different `model` on the build ticket 25 measured, because the account's model selection changed, so a provider claim can expire without a version moving. Record what the config said, not only what the binary was. Ticket 18 replaced `gemini` (0.42.0) with `agy` as the Google provider; `gemini` stays specced as a cold fallback on an API key. A session can delegate to another provider mid-flight, so a Claude Code session acting as orchestrator can hand a small task to cursor-agent and get a structured result back.

**Skills each session should call.** `mattpocock-skills:grilling` and `mattpocock-skills:domain-modeling` by default. `mattpocock-skills:research` for research tickets, `mattpocock-skills:prototype` for prototype tickets. All prose follows `~/.claude/skills/unslop/SKILL.md`.

**Source material.** `~/Downloads/Harness CLI Unificado para Cursor, Gemini y Claude.md` is the originating research doc. Treat it as a starting point, not as settled: its model naming is stale in places and several of its claims are unverified here.

**Vocabulary.** A **run** is a unit of recorded work Hydra owns, in two kinds: a **provider run**, which is one provider process Hydra launched or refused to launch, and a **harness run**, which is work Hydra did itself. Ticket 36 widened the word; `provider: null` tells them apart and today the only harness run is a `hydra evolve` pass. A run's **instructions** are the three things Hydra wrote at spawn: the task string, the policy and hook config, and the skills linked at the pinned sha. Everything a run reads after its process starts is **data**. Ticket 28 settled this and the test is authorship rather than timing. Do not call these channels, since D5 uses that word for the two crossings between the harness and Ouroboros.

**Reading code is not measuring.** Ticket 34 verified a traversal 35 times against real records and verified one field by reading a ternary that gated it. The traversal held and the field turned out to be empty every time a human used it, which ticket 35 found only by putting a person at a keyboard. A field's presence in a conditional says the branch exists, not that anything writes it. Where a finding rests on reading a binary, say so in the ticket, and treat it as a hypothesis until a record shows it.

**Per-run human flags.** Some limits lift only when a human types a flag on that run. The flag is never inheritable and the delegation policy cannot set it. D16's `--allow-billed` and ticket 31's floor escape are the two instances, so the spec names the shape once rather than restating it.

### Settled while charting

Constraints for every ticket below. Reopening one of these is a scope change, not a ticket.

- **D1** The destination is three documents, not running code.
- **D2** One repo at `~/hydra` holds the harness and Ouroboros, because they share a contract.
- **D3** Thin router first. No custom TUI in v1; the vendor CLIs keep their own. Ticket 13 found the price: a CLI keeping its own TUI cannot also speak ACP, so interactive runs record thin and are harvested from the vendor's own transcript at exit. A TUI is what would let ACP become the interactive path. Amended by ticket 44 on 2026-09-06: no TUI of Hydra's own. Herdr is the interactive layer and Hunk the diff viewer, and nothing crosses from Hydra to either; a bridge on their side tails `~/hydra/runs/`. Ticket 07's framework choice lapsed with the two requirements it rested on.
- **D4** Two auth planes, named separately. Agent-CLI plane: subprocess, per-provider OAuth, subscription pricing. Raw-model plane: LiteLLM, API keys, one OpenAI-shaped endpoint. Interactive work defaults to the agent-CLI plane. Corrected by ticket 18: plane names the auth mechanism and nothing else. Pricing and interactivity are a second axis, the recorded path against the interactive path, and it cuts across both planes. Google proved they come apart, since `agy` is an agent CLI on a subscription and `gemini` is an agent CLI billing per token on a key. Amended by ticket 12: v1 ships the agent-CLI plane only. The raw-model plane is one gated path, `GEMINI_API_KEY` to `gemini` behind `--allow-billed`, and the spec names LiteLLM as an extension point it does not build. Nothing on that plane has ever made a successful call. Ticket 22 emptied it further: the Copilot token only ever reached Hydra through hermes, so with hermes unrouted the raw plane is that one gated fallback and nothing else. Ticket 20 adds a consequence inside the agent-CLI plane: the paying subscription and the model's vendor can be different parties, since `agy` serves Claude Opus 4.6 on the Google subscription, so the record names which subscription paid rather than only which plane it was.
- **D5** The two systems converge on exactly two things: the run record and the skill store. Nothing else crosses.
- **D6** Agents never hold provider secrets. They ask the harness. Inherited from the HephaestusLab V3 rule. Made true in code by ticket 12: Hydra builds each child's environment from an allowlist, because `GEMINI_API_KEY` is exported in `~/.zshrc` and every child inherited it by default. Measured by ticket 21: the list is `HOME`, `USER`, `PATH`, `TERM`, plus `SHELL` and `TMPDIR` for `claude`. The allowlist is also an injection boundary, since `NODE_OPTIONS` and `PYTHONPATH` execute attacker-named code inside `cursor-agent` and hermes.
- **D7** The CLI is the real interface, MCP is a face over it. `hydra run --provider <p> --task "..." --parent <session-id>` returns JSON on stdout; a thin MCP wrapper gives Claude Code a typed `delegate` tool over the same path.
- **D8** The router core is Rust. A self-edit that does not compile never reaches review. Extended by ticket 17: Ouroboros is Rust too, in the same binary, so reviewer and reviewed sit on one compile gate. Confirmed by evidence from the reference implementation rather than by argument, since its `preflight_runner.py` spends 1,488 lines standing in for what `cargo build && cargo test` does.
- **D9** One canonical `~/hydra/skills/`. ~~Symlinked into the Claude and Cursor locations, plugin-wrapped for Gemini.~~ Corrected by ticket 03: `~/.cursor/skills-cursor` is Cursor's own managed cache, not a target. Replaced by ticket 10: injection is per run into the run's worktree, never into a location in the user's home. One first-party root per provider, `.claude/skills`, `.cursor/skills` or `.agents/skills`, each hidden from git by a `.gitignore` holding `*`, each linking into an immutable snapshot of the store at the sha the run pinned. `agy` also needs `--add-dir <worktree>` or none of it loads. No plugin wrapper for Google at any version Hydra routes. The store keeps its own git history, and Hydra never writes outside a path it created. Ticket 29 kept that last clause true where ticket 23 thought the vendor had broken it, and bought `claude` out of the shape for MCP alone: its MCP config goes on the command line, and the per-run artifacts Hydra generates at spawn live in `~/hydra/spawn/<run-id>/` rather than in the worktree.
- **D10** Run records are plain JSONL on disk under `~/hydra/runs/`, and they are the source of truth. Nothing needs to be running for the harness to work. Clarified by ticket 09: one file per **run**, named by a UUIDv7, with a session being a tree of runs linked by `parent_run_id`.
- **D11** Spec both Ouroboros rungs, ship rung one. ~~Rung one edits skills, prompts and config.~~ Amended by ticket 14: **rung one's write surface is the skill store and nothing else**, and in v1 it proposes rather than lands, even on skills it wrote itself. Config is struck because Hydra's config is the delegation policy and the environment allowlist, and a loop that can widen its own permissions walks around every gate on this map. Prompts are struck because Hydra injects skills and has no separate prompt. Rung two edits Hydra's own source behind a sandbox, a test gate and a diff quorum. Sized by ticket 17: rung two blocks on one review with no memory and no obligation ledger, because the reference implementation's ledger is 5,009 lines defending against a problem you only get after many rounds. The ledger is the first thing to build when thrashing appears in the run records. Amended by ticket 15: **rung two proposes rather than lands too**. A panel of three provider CLIs decides whether a patch may be offered, a human merges it, and auto-landing is a graduation condition judged from the run records rather than a v1 default.
- **D12** Training is out of scope. Lossless export to SFT and DPO formats stays a constraint on the record.
- **D13** Hydra is standalone. Hermes is worth copying, not depending on.
- **D14** ~~Hermes is a fourth routable provider.~~ Amended by ticket 22: hermes is not routable in v1. Ticket 08 found neither the xAI nor the Nous account exists, ticket 12 found the Copilot token exchange has never succeeded, and ticket 22 found every remaining credential is junk or empty. It is also the only provider Hydra cannot gate through the environment, since `~/.hermes/.env` loads with `override=True`. What Hydra keeps from Hermes is ticket 05's guardrail rules, which D15 already covers and which need no binary launched.
- **D15** Hydra gets its own curator, built on Hermes' guardrail rules rather than Hermes' code.
- **D16** Hydra never spends on a billed API key without asking. The AI Studio project behind `GEMINI_API_KEY` has billing enabled, so the raw-model plane is opt-in per run rather than a silent fallback. Settled by ticket 18. Ticket 12 gives it a mechanism: an `--allow-billed` flag typed per run, never inheritable, and the delegation policy cannot set it. Ticket 21 found that mechanism does not reach hermes, whose `~/.hermes/.env` loads with `override=True` and beats anything Hydra passes. The hole is latent rather than open: hermes holds no usable Google credential today, so nothing can be spent through it. Ticket 22 decides the gate, and `HERMES_HOME` is the lever, since it redirects both `auth.json` and `.env`.

## Decisions so far

<!-- one line per closed ticket, then zoom the link for detail -->

- [02 - Ghostty pane control](issues/02-ghostty-pane-control.md): Ghostty launches a command but exposes no PID and no exit status, by design. Hydra owns the pty per provider instead, since it must be the parent to tee anyway. Rendering splits off as a separate deferred question.
- [03 - Skill discovery rules](issues/03-skill-discovery-rules.md): one unchanged `SKILL.md` serves every provider, so no wrapper is forced. D9's Cursor path was wrong and is corrected above. The two symlink gaps it left open are closed by ticket 10: all three routable providers follow a symlinked skill directory.
- [04 - LiteLLM auth modes](issues/04-litellm-auth-modes.md): virtual keys satisfy D6; Vertex service accounts and Bedrock IAM both work upstream; nothing carries a Claude subscription, and that is a terms limit Anthropic enforces.
- [06 - Verify the Ouroboros paper](issues/06-ouroboros-paper-verification.md): paper and benchmarks are real, mechanism is not what we assumed. The quorum is a union veto over responsive reviewers, the sandbox is worktrees, and 63.5% of self-edits get blocked.
- [05 - Hermes curator rules](issues/05-hermes-curator-rules.md): fifteen rules with their reasons, plus two Hermes bugs not to copy. Its model is told archiving is recoverable while the tool it calls runs `shutil.rmtree`, so the lesson is that a guarantee written in a prompt is not a guarantee.
- [07 - Rust TUI against Textual](issues/07-tui-framework-cost.md): Textual wins on streaming markdown and the diff viewer, where ratatui costs 2 to 3 weeks of hand-built work. Live pane attach is the one thing that could put a third channel between the two systems, which is now ticket 09's problem.
- [01 - Provider headless modes](issues/01-provider-headless-survey.md): ~~drive ACP rather than print mode, because print mode leaves only five fields common to all four.~~ Superseded by ticket 32: both ACP speakers left the routable set, so every recorded run is print mode. Gemini's OAuth path is dead on this machine, which opens ticket 18. Ten named gaps become the record's optional fields, and Hydra must own run ids, timestamps and delegation lineage itself.
- [09 - The run record schema](issues/09-run-record-schema.md): one file per run, envelope plus verbatim raw frame, eleven normalized event types derived at read time. Usage carries a `usage_source`, so Cursor is `estimated` from a model-aware tokenizer rather than a fake zero. Redaction happens at export, which graduated ticket 19.
- [11 - The delegation contract](issues/11-delegation-contract.md): children spawn detached with blocking as a view over them, one git worktree each, permissions from a policy rather than a human, depth capped at 2. Results stay small on purpose, because delegation exists to save the orchestrator's context. Cursor cannot be delegated tool work under any policy narrower than fully permissive.
- [08 - Account and tier inventory](issues/08-account-inventory.md): one account per provider, so ticket 12 builds no rotation. Anthropic has no API key and no credit, and ticket 04 already ruled out carrying the subscription upstream, so it is agent-CLI only. The raw-model plane is one Google AI Studio key and one borrowed Copilot token. D14 corrected above.
- [18 - Which binary is the Google provider?](issues/18-google-provider-target.md): `agy` 1.1.23 grew `--output-format stream-json` during this session, so Google is Antigravity on the subscription with per-step token counts, and `gemini` on the key drops to a cold fallback nobody runs without approval. `agy` cannot be asked for permission, which puts it in Cursor's category.
- [21 - What environment each provider needs, and what it leaks](issues/21-provider-env-requirements.md): the allowlist is `HOME`, `USER`, `PATH`, `TERM`, plus `SHELL` and `TMPDIR` for `claude`. A wrong `HOME` silently logs out three of four providers, `CURSOR_AUTH_TOKEN` overwrites the stored login rather than overriding it, and the scrub turns out to be an injection boundary as much as a secret one. It also found the one thing the allowlist cannot do, which opened ticket 22.
- [22 - Where D16's gate sits for hermes](issues/22-hermes-spend-gate.md): nowhere, because hermes stops being routable in v1. It holds nothing that works and it is the one provider the environment allowlist cannot gate. The rule it sets outlives it: a provider Hydra cannot gate is either given a config root Hydra owns or is not routed, never launched with the gap recorded. The record gains `auth.gated`.
- [13 - Session launch and pane lifecycle](issues/13-session-launch-lifecycle.md): interactive and recorded are two launch modes, because a vendor TUI and ACP cannot share a process. Interactive runs are harvested from the vendor's own transcript at exit, and harvest is idempotent so any later `hydra status` can finish one. Liveness is derived from a pid and a process start time in the run file's opening envelope, and nothing appends to a run file except the process that owns it. Cleanup is always a command you type.
- [12 - The two auth planes, and where secrets live](issues/12-auth-planes.md): Hydra holds no provider credential, so D6 costs no code beyond an environment allowlist that stops children inheriting `GEMINI_API_KEY`. The raw-model plane does not ship in v1 and the one billed path needs a human to type `--allow-billed`. Auth failure is its own run outcome, and auth never happens inside a run.
- [10 - Skill store layout and injection](issues/10-skill-store-design.md): a flat git-versioned store injected per run into the worktree, not into your home. Links point at an immutable snapshot at a pinned sha, so a mid-session rewrite reaches nobody and the ticket's own scenario dissolves. Your existing skills stay put and are indexed read-only, because Hydra cannot give a run an isolated skill set at all. Collisions are prevented at write and fail the run at launch.
- [25 - Cursor's usage and thinking fields](issues/25-cursor-usage-and-thinking.md): ticket 09's Cursor facts have expired, so Cursor becomes `usage_source: provider` and `thinking` populates from it after all. The estimator survives on a narrower job, because Cursor writes no result frame when a run fails. Its `inputTokens` is net of cache and clamps to zero, and reasoning text and reasoning tokens turn out to be two capabilities that no provider gives both of.
- [17 - Reuse razzant/ouroboros, or reimplement it in Rust](issues/17-reuse-or-reimplement-ouroboros.md): reuse nothing, because there is no acyclic seam anywhere in 228,207 lines and the review panel is welded to another project's constitution. Take the critical-threshold rule with attribution and write Hydra's own checklist. Ouroboros is Rust, on the evidence of the 1,488 lines Python spent replacing a compiler. Three rules survive the ticket: severity has a Hydra-set ceiling, silence is not consent, and the apply path takes a type no code outside review can construct.
- [24 - Verify ticket 10's injection shape, and map skill precedence](issues/24-verify-injection-and-precedence.md): the shape works on all three current builds, and the precedence answer is that there is no shared notion of a skill's identity, so the collision check has to ask each provider what it resolved rather than compare strings. Two silent injection hazards fell out: a description containing a colon and a space breaks the skill on `agy` and Cursor, and injecting into a Claude Code run strips descriptions off skills the user already had.
- [26 - The hook-level action gate](issues/26-hook-level-action-gate.md): all three providers can hard-block a tool call from a config Hydra writes per run, and the hook beats the permission mode, so ticket 11's "Cursor cannot be delegated tool work" and ticket 18's `agy` equivalent are both too strong and are amended. The config is tamper-proof within a run because every provider snapshots it at start, Cursor reads Claude Code's hook format so one file gates two providers, and the three disagree on what a broken hook means.
- [14 - Ouroboros rung one: watching failures, evolving skills](issues/14-ouroboros-rung-one.md): invoked rather than scheduled, writing only to the skill store, proposing rather than landing. Two failure classes thought underivable came back alive on ticket 13's envelope and ticket 11's policy, and a fifth was added because three of four providers file a failed run as clean. No thresholds, because Hydra has run zero times. A change is measured against the one signal it was made for, which is also how a skill getting worse three weeks running shows up.
- [15 - Ouroboros rung two: sandbox, test gate, diff quorum](issues/15-ouroboros-rung-two.md): rung two proposes and a human merges, so the panel decides whether a patch is offerable rather than whether it commits. Judges are three provider CLIs on subscriptions, all three must answer or there is no review, and a patch touching a protected path is refused before review rather than blocked by it. Carries Hydra's own 29 checklist items.
- [19 - The export pipeline and redaction](issues/19-export-pipeline-and-redaction.md): scoping beats scrubbing, because the largest class in a trajectory is file contents and nothing redacts those. A run exports only if every path in its whole subtree sits inside a declared allowlist, which is Hydra's third protected config. Redaction then covers four classes that have a fixed shape, no model pass, and a hit on the verification scan aborts the entire export.
- [23 - MCP config across providers, against the ticket 10 shape](issues/23-mcp-config-injection.md): MCP rides ticket 10's shape on all three, each wanting a different launch flag, and one canonical document can generate the three files without being any of them. Cursor follows a symlinked MCP config while refusing a symlinked hook config, so "real files, never links" is a hook rule and not a general one. A delegated child inherits the human's user-scope servers on all three and only `claude` can scope them off per run.
- [27 - The shape of the delegation policy, now that every provider can be gated](issues/27-delegation-policy-shape.md): five named capability allowlists, deny by default, compiled per run into one script that lives outside the worktree it guards so a child cannot rewrite its own gate. The user's hooks compose with Hydra's because isolation is available on one provider of three, which ticket 30 is measuring. A gated run gets its own outcome, since all three exit 0 reporting success when the gate refused everything.

- [28 - Instruction provenance: whose instructions a run obeys](issues/28-instruction-provenance.md): a run's **instructions** are the three things Hydra wrote at spawn, the task string, the policy and hook config, and the skills at the pinned sha; everything read after the process starts is **data**, and the test is authorship rather than timing. Authority is per run, so a delegated child's task is authoritative for that child and a child's policy may never be wider than its parent's, measured as capability set containment. The rule text rides a preamble on the task string rather than an injected skill, it is Hydra's fourth protected config, and it is advice, because the gate is the only thing on this map that refuses anything.
- [30 - When two hooks answer, does deny win?](issues/30-hook-precedence-deny-vs-allow.md): deny wins on all three and a hook cannot grant at all, so the user's hooks can only tighten Hydra's floor and ticket 27 stands unchanged. It also found a cheaper isolation flag on `claude` than `--restricted`, which opened ticket 31, and that the reason text a provider reports does not identify which gate fired, which ticket 09 now handles with a field Hydra sets.
- [31 - Does Hydra load the user's settings on `claude`?](issues/31-claude-hook-isolation-flag.md): yes, it composes, and ticket 27's stated reason was not the real one. Hydra's policies bound where a run acts and the user's 418 lines of guards bound what a shell command means, so dropping them removes protection Hydra has never specced. A pattern floor on the `shell` capability ships on all three providers and under every policy, because the guards reach `claude` and `cursor-agent` and nothing reaches `agy`. `--restricted` narrows to `judge` and `--setting-sources project` is measured and unused.
- [29 - Where MCP config goes, and what to do about Cursor's approval state](issues/29-mcp-config-placement.md): Claude's MCP config moves to the command line, `--mcp-config` plus `--strict-mcp-config` on recorded runs and without the strict flag interactively, so the ancestor merge stops existing rather than being defended against. Cursor ships after all, because `CURSOR_DATA_DIR` moves its approval state into a path Hydra owns while the login and ticket 13's harvest source stay put, which also hands Hydra a per-run kill switch for inherited servers on a second provider. A shared tree gets no injected MCP, governed by a launch check rather than recorded as a gap. Per-run artifacts get a home at `~/hydra/spawn/<run-id>/`, reaped with the worktree, which turns D10 into a test the record has to pass.
- [20 - Claude models on the Google subscription](issues/20-claude-on-google-subscription.md): `agy --model claude-opus-4-6-thinking` is a model choice inside the `agy` provider, reachable by name and never taken on your behalf. The one case where the harness gap could not bite was `judge`, and ticket 15's panel killed it there too, because substituting puts two judges on one binary and one quota while its independence rests on one judge per provider. A limited `claude` aborts the review the way an expired login does. The rate-limit reading is recorded and changes no routing, the record gains one field naming which subscription paid, and `model` stays off ticket 11's delegate face for the reason three other things are already off it.

- [32 - Which transport a recorded run uses](issues/32-recorded-transport.md): print mode with `--output-format stream-json` everywhere, because no routable provider speaks ACP and the two that did left the set. ACP is struck rather than kept as an extension point, one capability-named watch replaces two, `--verbose` turns out to be mandatory, `permission_request` drops the event list to ten, and `message_delta` keeps its slot only because the contract gains a producer column, which is then the seam check's main instrument.

- [34 - Does any vendor transcript record a correction as a pair?](issues/34-correction-producer.md): one producer of three, `claude`, and the discriminator is a conjunction rather than a field, since `user-rejected` means the ask path answered and under `sdk-cli` the answerer is code. Cursor pairs but cannot tell a human from a hook, and `agy` drops the denial record and leaves a gap in `step_index`. The traversal is verified 35 times on exactly the population ticket 19 forbids exporting, and the human case had zero instances until ticket 35, which then falsified this ticket's `userFeedback` claim.

- [35 - Confirm the human rejection shape on `claude`, at a keyboard](issues/35-confirm-human-rejection-shape.md): the producer is real and two human preference pairs now exist, the first on this machine. Three of four checks pass, the rejected-`Edit` shape holds, and `userFeedback` is empty on all three, so ticket 34's richest field was an inference off a ternary rather than an observation. The words survive as an ordinary user turn, so step 4 becomes a chain walk that cannot tell a redirection from a change of subject. A denial also carries an interrupt marker, and .261 has dropped the reject-with-feedback option from the prompt entirely.

- [36 - What shape is a run record with no provider?](issues/36-pass-record-shape.md): a run is now a unit of recorded work Hydra owns, in two kinds, because a rung-two pass needs a record and its children need a parent that exists. Seven fields of twenty-six are filled, the record gains `command`, a pass's outcome describes the pass rather than the patch, and rung one and export both skip the kind.

- [33 - What the Cursor usage estimator keys on, when `model` says "Auto"](issues/33-cursor-estimator-model-family.md): the resolved id is recoverable, from the chat store at `providerOptions.cursor.modelName` rather than from any frame, and the Auto probe resolved to a model `--list-models` does not offer. `model` is the selection's display name and ticket 25's `Auto` reading expired on config rather than on a version. The estimator's premise is what fails: Cursor counts about twice what the transcript keeps, so `estimated` looks unearned and `unavailable` says the true thing.

- [16 - Assemble the three documents and check the seam](issues/16-assemble-the-specs.md): the three documents exist and the seam is checked. D5 and the D12 export test hold; the other two checks broke and are tickets 37 and 38. Six contradictions between documents fixed here, each carrying a decision another document had already made, the sharpest being a contract section that lost its heading and a harness spec naming a field the contract renamed. The producer column did its job and turned a thirty-ticket re-read into a lookup.

- [37 - Which gate denials hold under `permissive`?](issues/37-permissive-gate-denials.md): both documents move and the five claims split by mechanism. The gate's universal denial set becomes seven paths, adding `~/hydra/runs/` and the four configs that were protected against a rung-two patch and nothing else. The three `hydra` subcommands stop being a gate claim, because a binary reached through `shell` is an argv match with the pattern floor's failure mode, so `evidence_is_read_only` is what holds them and is now load-bearing. Protected paths go to twelve, and Cursor's gate gains `beforeReadFile`, since reads are a capability and `preToolUse` does not cover them.

- [38 - What does a record look like on an interactive run?](issues/38-interactive-run-record-shape.md): the contract specifies three shapes now, and the producer column says it describes recorded runs. `correction` and `permission_decision` can never appear in one run, which nothing had noticed. A probe found `usage` surviving harvest on `message.usage`, a producer the table would have denied. The finding the ticket did not start with: a transcript carries eight record types no stream has, and `attachment` is run content that entered through no tool call, so export's path extraction never sees it and a run holding one is not exportable.
- [39 - A linked skill is unreachable under four of the five policies](issues/39-linked-skill-nobody-can-reach.md): a linked skill is meant to be invocable and readable, and one measured run settled how. `Skill` becomes its own capability, `skill_invoke`, granted by every policy that reads at all, because an allowed `Skill` call delivers the body with no read the gate sees. The run's snapshot becomes a read scope of its own, `skill_snapshot`, resolved from the sha on the record, because the model opens a sibling file through the link with `Read` and the gate resolves that link out of `worktree`. Copies were rejected as writable instructions. Build ticket 13 carries the code, after 09 and before 40's.
- [44 - Herdr is the interactive layer, not a TUI of Hydra's own](issues/44-herdr-is-the-interactive-layer.md): D3 amended. Herdr owns the panes and Hunk shows the diffs, nothing crosses from Hydra to either, and a bridge on their side tails `~/hydra/runs/`. Ticket 45 installs both and probes the claims before any build.
- [45 - Install Herdr and Hunk and probe what ticket 44 assumed](issues/45-install-herdr-and-hunk-and-probe.md): herdr 0.8.2 and hunk 0.21.1 installed and driven on 2026-09-06, four of 44's claims measured true and two false. The bridge can assert a pane's state from outside it, but Hydra's three words are display labels rather than states, and Herdr has no reading of a Hydra run to correct. The `base_tree` Hunk was to open against is in no record and reachable from no ref, which is 44's one claim that needs a decision rather than a rewrite.
- [40 - Where the `skill_store` scope points, now that two documents disagree](issues/40-where-the-skill-store-scope-points.md): the run's own checkout wins and harness spec 7.2 moved; `skill_store` and `skill_checkout` are two names; `hydra reap` sweeps a leftover checkout commit-first. The human then asked which policies can lose data, and a measured matrix found a `standard` run's shell allowed to delete a sibling's worktree, the pruned records, the parent repo's `.git` and the store's history. The deny list became two kinds of entry, denied parents and own-path carve-outs, and the floor gained `gc`, `prune` and `worktree`. Build ticket 14 carries the code and two lines under `policies/` are the human's.
- [41 - The curate checkout happens before the launch commits the store](issues/41-curate-checkout-against-the-store-pin.md): the store is committed before the checkout. `hydra curate` calls `Store::pin` with its own run id and checks out at the sha it gets back, and the launch's pin then finds a clean store and returns the same sha. Chosen because ticket 40 left the store two writers, a human and `Store::pin`, which makes this Hydra's own write one launch earlier rather than a curate pass in the store's history. The trailer naming a run that has not started is accepted. Built and tested in the same change, no launch.
- [42 - A record pins the policy and not the paths it resolved to](issues/42-a-record-carries-no-scopes.md): harness spec 5's test applies to the compiled gate, so `run_start` gains `scopes`, the resolved paths under five keys, and `floor_lifted`. The reader prefers the stored paths, rebuilds them on older records and reports a limit rather than a disagreement, because a disagreement reaches the curator's brief as a clean-looking failure. Not a version bump, and contract 15 now says what is. Contract 5.1's false sentence on the gate sha is rewritten. Build ticket 15 carries the code.
- [46 - The user's own hooks add to a run's context, and section 8 says they only subtract](issues/46-user-hooks-add-to-a-run.md): section 8 was wrong about the mechanism. The user's `UserPromptSubmit` hook adds `additionalContext` to every recorded `claude` run, measured, and it is data under the authorship test as written. Hydra keeps loading the user's settings because the four guards live in the same file, even though `--setting-sources project,local` was measured to drop the hook with the gate intact. The text stays in `raw` as a `hook_response` frame, and the Cursor case is stated as unmeasured until the next paid run.
- [49 - What the Herdr bridge reports, and whether the record carries `base_tree`](issues/49-what-the-herdr-bridge-reports.md): `timed out` sits on Herdr's `unknown`, and the pane carries the record's own facts as tokens so a human reads the run without Herdr classifying it. `base_tree` goes on the record in `run_start.worktree`, as a name rather than a guarantee the object survives a `git gc`, so a reaped run's seed and its own edits can be told apart. Build tickets 23 and 24.
- [47 - A `judge` run is linked skills it cannot list, read or invoke](issues/47-judge-gets-linked-skills-it-cannot-see.md): stop injecting on `judge`. The policy allows nothing and the diff rides the prompt, so the links were never reachable and the record stops naming a store for a run that could touch none of it. Build ticket 25.
- [48 - The delegate face needs `--parent <run-id>` and the MCP document cannot say so](issues/48-the-face-needs-the-run-id-and-the-document-has-no-field.md): Hydra recognizes its own binary by basename and appends `--parent <run-id>` at launch, with no new field or token in the protected document. The face is admitted under `standard` and `permissive`. Build ticket 26.
- [50 - Can the gate ride an interactive `claude` session?](issues/50-gate-on-an-interactive-claude-session.md): opened 2026-09-07. Harness spec 3 gives an interactive run no gate. `claude` takes `--settings` interactively, so the hooks may apply and the question is whether their decisions reach the transcript the harvest reads. A prototype, not a decision. Probed the same day: the gate applies on a steered session and its denials reach the transcript as error tool results while allows leave nothing, so an interactive `claude` run can be gated and its refusals recorded. Whether the launch takes a policy is the question it leaves.
- [43 - `--restricted` no longer drops MCP, so a `judge` run keeps the human's servers](issues/43-restricted-keeps-mcp-on-judge.md): the premise was wrong. Measured on 2.1.263 under a scratch `HOME`, `--restricted` drops user-scope and project-scope servers and keeps only what `--mcp-config` passes, so `judge` already gets an empty set. The flag goes on the `judge` argv anyway because the vendor's help text names it as the guarantee, the probe runs under `judge`'s flags so `mcp.resolved` is a measured `[]`, `ToolSearch` moves to `skill_invoke` because MCP tools arrive deferred in print mode, and one skip-if-absent test guards the claim against the next version. Build ticket 16 carries the code.
- [51 - Cursor ran `ReadLints` past the gate, and its argument is a list](issues/51-cursor-runs-readlints-past-a-deny.md): the table gains an `unhooked` row kind for tools the provider runs without asking the hook, three on Cursor today, recorded as the provider's decision rather than a leak, with the per-tool risk written in `IMPLEMENTATION.md` and zero for all three. Build ticket 35.
- [52 - Rung two's three open details](issues/52-rung-two-build-details.md): the proposer runs as `standard`; a patch over a line cap is rejected for size before any reviewer sees it, so a judge sees a whole diff or none; the reviewer answers one closed JSON shape and any deviation is a block. Build ticket 36.
- [54 - Where a judge finding lives, and what would have to read it](issues/54-where-a-judge-finding-lives.md): the training channel is built and empty. The proposer already carries `skills.linked` from the store, the store holds no skills, and a judge finding is already in ticket 52's closed JSON shape on the judge's own record. Rung one cannot see it because its five classes all derive from one run's own shape while a verdict is a second run's opinion. Nothing bars it: a judge's parent is the pass, not the curator, so judge runs are already in the corpus. What is missing is a reader, not a format, which makes ticket 55 cheaper than the fog entry assumed. Caveat: two of three judges on pass 7 came out `gated` and wrote the better reviews, so a reader that filters on outcome throws away the best evidence rung two has.
- [56 - The first SME, a skill in the store, launched and measured](issues/56-the-first-sme.md): every wire from the store to the model's available-skills list works first try. `hydra-self-edit` linked, the snapshot resolved, `claude` discovered it and offered it. The model still did not open it: asked a question that matched no trigger, it hunted for the human's own skill root, took the refusal `standard` owes that path, and reported the skill unavailable rather than trying the Skill tool or the snapshot, both of which `standard` grants. A skill that reaches the listing has not yet reached the model, and on a weak model that gap is where a training signal disappears.
- [59 - The three documents against the code](issues/59-the-three-documents-against-the-code.md): audited 2026-09-12 by a read-only agent and checked at the cited lines. Five claims wrong, a dozen stale, the rest vague; the five fixes a first reader needs are listed, and four are build tickets 40 to 43. Two claims are more conservative than the code: rung two "shipped later", and "Hydra has run zero times".

- [58 - Codex CLI as the fourth provider](issues/58-codex-as-the-fourth-provider.md): measured on 0.154.0 without logging in. It has a `PreToolUse` hook in Claude's dialect, honours the exact deny shape `gate.py` writes, streams JSONL under `exec --json`, and signs in with a ChatGPT account. Routable with a compiled gate; three costs named, hook trust, no read tool, fail-open unmeasured. Build ticket 48.

- [61 - Should a checkout and a data root be the same directory?](issues/61-a-checkout-and-a-data-root.md): opened 2026-09-14 after a cold-clone review ran the quick start in `/tmp` and was shown 197 runs it had not made. `HYDRA_ROOT` and a root line on `status` patch the symptom; whether the corpus belongs to the machine or to the checkout is the question, and `hydra doctor` is the shape of the answer the reviewer asked for.

- [62 - Containment by argv, or by the operating system?](issues/62-containment-by-argv-or-by-uid.md): opened 2026-09-14 after an adversarial review found five holes in the gate, three of them the same bug in three spellings. The alternative is a second uid and file modes, which is the first thing in this design that needs something from the machine. What Hydra claims, containment or evidence or both, wants deciding before the repository is public.
- [53 - The panel margin](issues/53-the-panel-margin.md): resolved 2026-09-21. Four judges and unanimity stay; a judge that sees an edited test also sees its baseline and is told to compare. Build ticket 63.
- [55 - What an SME is and how training is measured](issues/55-what-an-sme-is-and-how-training-is-measured.md): resolved 2026-09-21. Judge verdicts become preference pairs through a reader of their own, withheld until a chosen half exists and signed off by a human; the specialist is the fog's MCP controller. Build ticket 65.
- [57 - The judge cannot see the task](issues/57-the-judge-cannot-see-the-task.md): resolved 2026-09-21. The task reaches the judge as evidence that waives nothing; a block is final for its pass and a human asks for a fresh review; findings train only when confirmed. Build tickets 63 and 64.
- [60 - The hook-config writer lives in the launch sequence](issues/60-the-hook-config-writer-lives-in-the-launch-sequence.md): resolved 2026-09-21. The writer moves to its own file and becomes a protected row; `run.rs` stays editable. Build ticket 66.
- [61 - A checkout and a data root](issues/61-a-checkout-and-a-data-root.md): resolved 2026-09-21. One root per machine; a launch from a foreign checkout refuses and names `HYDRA_ROOT`; `hydra doctor` is the twelfth command. Build ticket 67.
- [62 - Containment by argv, or by the operating system?](issues/62-containment-by-argv-or-by-uid.md): resolved 2026-09-21. No second uid; the macOS seatbelt around the vendor binary denies writes to Hydra's directories under every spelling, measured with all four logins intact. Build ticket 68.
- [64 - Delegation under the seatbelt](issues/64-delegation-under-the-seatbelt.md): resolved 2026-09-22. The face is not available under the seatbelt and refuses in Hydra's words; delegation stays with the orchestrator outside the run. Build ticket 76.
- [63 - A bounded shell read for a provider without a read tool?](issues/63-a-bounded-shell-read-for-codex.md): resolved 2026-09-21. `judge` grants nothing on any provider, so a judge never reads and `codex` sits on the panel; `read-only` stays refused, and the bounded command list is not built. Build ticket 61.
- [65 - Does the checkout the binary was built from join the deny list?](issues/65-does-hydras-own-checkout-join-the-deny-list.md): resolved 2026-09-22. The whole checkout is denied to writes under every policy, own worktree and curate checkout carved out; 15.2 gains the row. Build ticket 74.
- [66 - The admission gate runs proposer code outside the seatbelt](issues/66-the-admission-gate-runs-proposer-code-outside-the-seatbelt.md): resolved 2026-09-22. Admission runs under the proposer's own profile with an empty `HYDRA_ROOT`; spec 4.5's order stays; the home directory stays readable and the document says so. Build ticket 75.
- [67 - What the seatbelt is for](issues/67-what-the-seatbelt-is-for.md): resolved 2026-09-22. Defence behind the gate, not a boundary: the profile stays, `SECURITY.md` scopes it to the process tree, spec 7.5 says which it is. No build ticket.
- [68 - An attack corpus, and whether Hydra gets an immune loop](issues/68-an-attack-corpus-and-the-immune-loop.md): resolved 2026-09-22. No scheduled loop; a corpus under `tests/corpus` that rung two cannot edit, the raw pass's claim narrowed, the payload's shape refused, a non-ASCII root refused. Build tickets 77 to 80; `network_read`'s scope went to 69.
- [69 - A scope for `network_read`](issues/69-a-scope-for-network-read.md): resolved 2026-09-22. State the trifecta, build no half fence. No build ticket.
- [70 - A concurrent delegate face](issues/70-a-concurrent-delegate-face.md): resolved 2026-09-22. Serial on purpose, one sentence in spec 9. No build ticket.
- [71 - `codex` cannot read its own skills](issues/71-codex-cannot-read-its-own-skills.md): resolved 2026-09-22 after a three-reader panel. Copy on `codex`, seatbelt-denied, skill roots off the reap, a hash at exit. Build ticket 87.
- [72 - Which branch reap may commit on, and what the git carve-out lets a run write](issues/72-reap-and-the-git-carve-out.md): resolved 2026-09-23 after a four-reader panel. Reap commits only on its own branch; every unsandboxed git call takes its git directory from the record; the private carve-out narrows to the index. Build ticket 95.
- [73 - What rung two may edit](issues/73-what-rung-two-may-edit.md): resolved 2026-09-23 after a four-reader panel. The human names the paths per pass over a floor no name lifts; existing tests are refused, new ones allowed. Build tickets 80 and 96.
- [74 - What a run may write in the parent's object store](issues/74-the-object-store-under-the-seatbelt.md): resolved 2026-09-23. A private object store per run, the parent's read as an alternate, reap moves checked objects in from outside. Build ticket 103.
- [75 - Master of Puppets: an orchestration mode, a mascot, and a ranking of agents](issues/75-master-of-puppets-mode.md): a four-proposer panel agreed the launcher and the mascot are Herdr work, the orchestrator lives outside every run, and cost is window share; it found the interactive record too thin for the mascot and `hydra-in-pane` unable to carry a TUI. Nine decisions graduate to grilling ticket 76; the build order is in research/75.
- [77 - A small model of Hydra's own](issues/77-a-local-model-for-the-mode.md): the router needs no LLM ($0, a classifier on record features); a generating model waits for a corpus worth fine-tuning, then QLoRA on a rented 3090 and Modal at scale to zero; from scratch at low bit width is $5,700 to $67,000 and does not pay; nothing has been exported yet because the allowlist file does not exist.
- [76 - The nine decisions under Master of Puppets](issues/76-master-of-puppets-decisions.md): A to C, I and J resolved 2026-09-24 in the morning, D to H the same evening. The goal stays in its own log, verdicts stay there too, `hy` asks for a provider among those with a measured interactive gate, the bridge may tail `claude`'s transcript, the session revises window budgets with a history. Build tickets 108 to 111 and 129 to 132.
- [81 - Jev, and judge seats that differ on one model](issues/81-jev-and-distinct-judges.md): resolved 2026-09-26 after research by `hydra run`. A degraded panel, declared and approved per goal, may fill a missing seat (amends 76-I for the mode only); Jev is probed on committed diffs as an advisory signal, never a vote. Build tickets 133 and 134.
- [78 - Claude Code reads its own tool results under `~/.claude/projects/`](issues/78-claude-reads-its-own-tool-results-outside-the-worktree.md): resolved 2026-09-24. A read-only carve-out on `claude` for the run's own project slug, recorded in `run_start.scopes`, unless a setting moves the directory. Build ticket 123.
- [79 - An unhooked row keeps its capability](issues/79-an-unhooked-row-keeps-its-capability.md): resolved 2026-09-24. Unhooked becomes a flag beside the real capability; the gate still skips the row, the replay files a finding when the policy denies the capability. Cursor and codex rows both. Build ticket 124.
- [80 - Where the walk over this machine's records belongs](issues/80-where-the-live-corpus-walk-belongs.md): resolved 2026-09-24. The live walks move to `hydra doctor --walk`, the fixture walks stay in `cargo test`, pre-push runs both. Build tickets 125 and 126.

- [82 - A judge that needs subagents inside the seatbelt](issues/82-a-judge-that-needs-subagents-inside-the-seatbelt.md): resolved 2026-09-28 with the human in six questions. A gated interactive session asks its own launcher, outside the seatbelt, for seats; a shape in protected config #6 launches alone, anything else waits for a human-typed `hydra approve`; seats that read get a bounded read-only shell under a write-denying seatbelt. Amends 63 and 64. Build tickets 138 to 141.
- [83 - A seat that runs longer than the launcher waits](issues/83-a-seat-that-runs-longer-than-the-launcher-waits.md): resolved 2026-09-29 by the session under the human's delegation, after a panel of three vendors. Nothing changes until the first live panel measures how long `claude` holds a call open.
- [84 - A seat that never hands off has no place on the record](issues/84-a-seat-that-never-hands-off.md): resolved 2026-09-29 the same way. The seat gets its own `never_started` record, and the session's record gets no new line type. Build ticket 147.
- [85 - A cockpit for the orchestrating session](issues/85-a-cockpit-for-the-orchestrating-session.md): resolved 2026-09-29 by the session under the human's delegation. Step 1 is a pane that loops `puppets.py board` beside the chat; Herdr draws no table and takes no cell click; a policy editor that writes is a map ticket. Build ticket 148.
- [86 - A cockpit of Hydra's own, wrapped around the vendor's terminal](issues/86-a-cockpit-of-hydras-own.md): resolved 2026-09-30 at the human's request. Amends 85: `hydra cockpit` in Ratatui, cut one beside the chat with one write (approve), cut two hosts the vendor's pty as `pty.rs` already owns it. Build tickets 150 and 151.
- [87 - The apprentice loop](issues/87-the-apprentice-loop.md): resolved 2026-09-30 under the human's delegation. The local model works a bank of tasks about records nightly, verifiable ones grade themselves, a panel grades the open ones, and preference pairs per harness grow for a later LoRA. Build ticket 160.
- [88 - A harness of our own for the small models](issues/88-a-harness-of-our-own-for-the-minis.md): resolved 2026-09-30 by a panel of three under the human's delegation. pi is not routed; an own harness is a separate Rust child speaking `claude`'s dialect, deferred until a local model emits tool calls, which none does today (Ollama: gemma3 1B does not support tools).
- [89 - A human stop, and the outcome it leaves](issues/89-a-human-stop-and-its-outcome.md): resolved 2026-09-30 by a panel of three under the human's delegation. A tenth outcome, `stopped`, from a stop-note prefix in `run_end.error`; rung one never learns from it.
- [90 - The integrations in Rust](issues/90-the-integrations-in-rust.md): resolved 2026-09-30 by a panel of three under the human's delegation. A Cargo workspace, one crate per integration, `puppets` first, each proved by differential runs against the Python before the Python goes; the cockpit keeps a subprocess.
- [91 - A bad line inside a goal log](issues/91-a-bad-line-inside-a-goal-log.md): resolved 2026-09-30 by a panel of three under the human's delegation. Nobody skips: readers fail by name, writers refuse, `repair` moves the bad lines aside and voids earlier approvals; in the Python and the Rust before build 194.
- [92 - What the cockpit shows of a tool call's input](issues/92-what-the-cockpit-shows-of-a-tool-call.md): resolved 2026-10-01 by a panel of three under the human's delegation. The row shows the head of the call; a key shows the whole input and it leaves by itself; the pattern mask goes.
- [93 - Where the grader reads a goal once `puppets.py` is gone](issues/93-where-the-grader-reads-a-goal-once-puppets-py-is-gone.md): resolved 2026-10-01 by a panel of three under the human's delegation. The grader runs `hydra-puppets show`; `sheet.py` stays in Python until the grader's port; `puppets.py` goes once the labels match both ways.
- [94 - The grader in Rust: shared code, the reference Python, and what is not ported](issues/94-the-grader-in-rust-shared-code-the-reference-python-and-what-is-not-ported.md): resolved 2026-10-01 by a panel of three under the human's delegation. A shared crate `crates/pyjson`; numbers match CPython 3.14.7 to the byte, after a probe of `exp`, `log` and `log1p`; `jev_probe ask` is dropped and `sheet.py` goes into `hydra-grader`.
- [95 - The pieces left: what is ported, what is dropped, and where shared code lives](issues/95-the-pieces-left-what-is-ported-and-what-is-dropped.md): resolved 2026-10-01 by a panel of three under the human's delegation, the mascot left to the human. The shell stays shell; a shared crate first and loopback only for every model client; `bitnet_serve.py` dropped, `frontier.py` a subcommand; `gate.py` stays Python.
- [96 - Help that asks the session's own model](issues/96-help-that-asks-the-sessions-own-model.md): resolved 2026-10-03 by a panel of three under the human's delegation. A help file in the repository, a key that types the question unsent, a start line for both modes plus refusals that name the way out; voice later.
- [97 - The orchestrator costs more than its agents](issues/97-the-orchestrator-costs-more-than-its-agents.md): resolved 2026-10-03 by the same panel. Sonnet by default, a short document of its own outside the repository's instructions, one blocking wait.
- [98 - A gated session that launches what the human approves](issues/98-a-gated-session-that-launches-what-the-human-approves.md): resolved 2026-10-03 by the same panel. More shapes only, written by the human; the 600-second refusal stays; a narrower sandbox for the gated copilot.
- [100 - The orchestrator on any provider](issues/100-the-orchestrator-on-any-provider.md): resolved 2026-10-03 by a panel, unanimous. The ungated copilot on all four, a transcript reader per provider first, one document rendered per file name, the repository as an argument.
- [101 - A copilot that does the work itself](issues/101-a-copilot-that-does-the-work-itself.md): resolved 2026-10-03 by the same panel, 2 to 1. Edit tools disallowed in the copilot; its token count against its agents' shown beside it.
- [102 - The cockpit is for the one who orchestrates](issues/102-the-cockpit-is-for-the-one-who-orchestrates.md): resolved 2026-10-03, D6 by panel unanimous, D7 by the human. The gated chat leaves the cockpit; the row and the panel detail show model, role, question, judge and votes.
- [103 - How a gated `claude` run keeps mods out](issues/103-how-a-gated-claude-run-keeps-mods-out.md): resolved 2026-10-03 by the human. A managed settings file with `allowManagedModsOnly`, and Hydra refuses a run's writes to plugin paths.
- [104 - Herdr goes, and the run graph stays](issues/104-herdr-goes-and-the-run-graph-stays.md): resolved 2026-10-03 by a panel. Herdr frozen, the cockpit the one interface, the run graph a cockpit view.
- [105 - Agent text reaches the orchestrator marked](issues/105-agent-text-reaches-the-orchestrator-marked.md): resolved 2026-10-03 by a panel. Agent text reaches the orchestrator inside the boundary marker.
- [106 - The attack corpus becomes a benchmark](issues/106-the-attack-corpus-becomes-a-benchmark.md): resolved 2026-10-03 by a panel. The whole corpus, three repetitions, Wilson intervals, nightly over a week; tie-break by Fable and astra.
- [107 - Orchestrator cost with variance, and one entry point](issues/107-orchestrator-cost-with-variance-and-one-entry-point.md): resolved 2026-10-03 by a panel. A fixed goal set plus real sessions; Master of Puppets the one entry point.
- [109 - A System 1 of our own](issues/109-a-system-one-of-our-own.md): resolved 2026-10-04 by Fable research. Labels, not technique, are missing; a logistic head over embeddings first, JEPA later; Jev ranks diffs; one answer withheld.
- [111 - Which small local model calls tools](issues/111-which-small-model-calls-tools.md): resolved 2026-10-04 by measurement. Qwen 2.5 3B and Ministral 3B 10 of 10, Llama 3.2 3B 8, Granite 4 micro 7; map 88 reopens as map 112.
- [113 - Does the task cross into sft.jsonl](issues/113-does-the-task-cross-into-sft.md): resolved 2026-10-04 by a panel, 3 to 1. The whole task crosses as the first user turn, redacted, its paths counted toward eligibility.
- [117 - Three tiers of orchestration](issues/117-three-tiers-of-orchestration.md): resolved 2026-10-15 by the human on the panel's recommendation. No middle tier that refuses; an offline audit with a rubric first, Jev as a logged score at `tasks` import, defaults kept, promotion only after a diagnosed failure.
- [118 - A specialist orchestrator, not a generalist](issues/118-a-specialist-orchestrator-not-a-generalist.md): resolved 2026-10-15 by the human on the panel's recommendation. The example is a plan proposal with preference and outcome kept apart; the untuned 3B scored half of Opus in map 121, so round two waits for a counted corpus; D12 stays.
- [119 - Hydra on Linux has no seatbelt](issues/119-hydra-on-linux-has-no-seatbelt.md): resolved 2026-10-15 by the human on the panel's recommendation. Runs stay on the Mac until bubblewrap wraps a launch; `run_start.host` and `sandbox.kind` label the population; one drift table keyed by provider, OS and arch; Linux compile first.
- [120 - The floor and the run's own worktree path](issues/120-the-floor-and-the-runs-own-worktree-path.md): resolved 2026-10-15 by the human on the panel's recommendation. Premise false, the own path was already allowed; the refusal names its cause; quoted heredoc bodies and single quotes are data, newlines split commands, the refused command is recorded whole.
- [122 - Whose plan was it when the panel composed one](issues/122-whose-plan-was-it-when-the-panel-composed-one.md): resolved 2026-10-15 by the human on the panel's recommendation. Credit none; the composed plan gets its own `plan_composed` row with the outcome; no provenance from the judge prompt; proposers without a draft get `chosen: null`.
- [123 - The run's git pointers reach every repository it makes](issues/123-the-runs-git-pointers-reach-every-repository-it-makes.md): resolved 2026-10-15 by the human on the panel's recommendation. An accepted cost that stays; no shim; reap decides what a staged nested repository becomes, the human's word pending on commit-the-files versus refuse-by-name.
- [124 - What each provider and model is good for](issues/124-what-each-provider-and-model-is-good-for.md): resolved 2026-10-15 by an Opus research agent over 932 records. Opus for Rust with a suite, Sol for scripts, Flash for small read-only probes, Astra as closed-shape judge, Fable only when a panel splits; the table is in research/124.
- [125 - The new agy seats measured](issues/125-the-new-agy-seats-measured-on-what-we-already-know.md): resolved 2026-10-15 by measurement. Flash 12 of 12 facts and both false premises; GPT-OSS 120B 9 of 12, two invented, none caught, two launches in five lost to capacity. The agy seat stays Flash; GPT-OSS only for bounded code with tests.
- [129 - What stands between Hydra and a public repository](issues/129-what-stands-between-hydra-and-a-public-repository.md): resolved 2026-10-15 by the session's reading. A real secret scan, green CI, a release and the README rewrite come first; the differentiator is the derived outcome, which Omnigent, Scion, OpenCode and ghostex do not claim.
- 126, resolved 2026-10-15 by the human on the session's recommendation from panel 130. No carve-out in 7.7; a foreign host is counted apart and never replayed; Rungs refuses a mixed-host corpus until a Linux wrapper exists.
- 127, resolved 2026-10-15 by the human on the session's recommendation from panel 130. No shell exemption for the pinned snapshot now; revisit when `sandbox_kind` exists, own snapshot only, preamble untouched.
- 130, resolved 2026-10-15 by the human: the docket. The Linux job narrows honestly and is fixed on a real Linux; 316 additive; 326 refuses; 321 and 320 stand; publication in the panel's order; three tickets opened.
- [135 - What bubblewrap does with a missing deny path, and with the private git directory](issues/135-what-bwrap-does-with-paths-that-do-not-exist-and-the-private-git-dir.md): resolved 2026-10-27 by a four-seat panel the human delegated to. A sweep before every host git on a run checkout; missing deny paths are created empty, `{}` or covered by their parent; builds 366, 367 and 370.
- [108 - One list of secret classes](issues/108-one-list-of-secret-classes.md): resolved 2026-10-27 by a four-seat panel the human delegated to. B: the write path keeps five classes, one shared table in a leaf crate, the cockpit masks spans by the full list; builds 368 and 369.
- [136 - Does a run's process group die with the run?](issues/136-does-a-runs-process-group-die-with-the-run.md): resolved 2026-10-27 by a four-seat panel the human delegated to. Yes: SIGTERM, grace, SIGKILL before `run_end`; reap refuses while a process holds the checkout; Linux waits for 370; builds 372 and 373.
- 131 - withheld: a lock inside a private repository.
- [99 - The gated copilot can still write `~/.claude.json`](issues/99-the-gated-copilot-can-still-write-claude-json.md): resolved 2026-11-01 by map 134's panel. Measured that `claude` runs without writing it; deny both files; build 378.
- [134 - The public gate](issues/134-the-public-gate.md): resolved 2026-11-01 by a three-seat panel the human asked for. 99 fix before, 110 and 114 stated; the human's list before publication is in the answer.
- [137 - The lab's next arc](issues/137-the-lab-s-next-arc-cloud-experts-and-post-quantum.md): resolved 2026-11-04 by the human on a three-seat panel. Training only in the cloud, base plus adapters first, ML-DSA provenance first; from scratch is map 138, an own post-quantum system is map 139; builds 380 to 383.
- [140 - Two videos: weight editing and a sliced agent runtime](issues/140-two-videos-weight-editing-and-a-sliced-agent-runtime.md): resolved 2026-11-04 by the human on a three-seat panel. Weight editing becomes an arm of 386, Infinity lends ideas only, measure the waiting parents first; builds 389 and 390, map 141.

## Not yet specified

- **A mini agent as a peripheral controller, and what the record says about its state.** From `~/Downloads/Arquitecturas Modulares de SLMs.md`, read 2026-09-06. Its second architecture puts a 1B to 3B model behind an MCP server as a device driver confined to a state machine, escalating to the frontier model on failure. Under Hydra that is a server declared in `config/mcp.md`, admitted per policy, whose calls already land in the record as `mcp__` tool calls and are refused under every policy but `permissive`. What the record cannot carry is the controller's own state per step: its state-machine node, its retry count, whether it escalated. Contract 7 has ten event types and `thinking` belongs to the provider, so a controller-state event is a vocabulary change under contract 15. Sharp enough to ticket the day a controller exists to emit it, and loose until then. The lookup pane the idea asks for is a herdr plugin tailing the record and pushing state tokens, which needs no Hydra change; ticket 45 measured that half on herdr 0.8.2, where `herdr pane report-metadata --token NAME=VALUE` puts arbitrary key-value pairs on a pane from outside it and `pane get` reads them back. The document's first architecture, a mixture of LoRAs, keeps its state inside the inference server where nothing here can see it, and is beyond this map.
- **Langfuse.** Optional viewer that tails `~/hydra/runs/`, never the source of truth. Revisit once there are enough records to be worth browsing.
- **Multi-machine use.** Whether run records and skills follow you to another machine. Ticket 12 settled the auth half: it does not. Every credential is in this machine's Keychain or its environment, so you log in again per machine.
- **A second corpus from Ouroboros verdicts.** Graduated 2026-09-09. The condition it named, "waits until rung two has produced verdicts worth looking at", was met by pass `01a084d3-bdd3`, whose panel blocked a real patch on a correct critical finding. Ticket 54 measured where such a finding lives and resolved the shape half: ticket 52's parser already fixed it, and the missing piece is a reader. Ticket 55 carries the decisions that are left, and ticket 53 carries what the same pass revealed about the panel itself.
- **Who reads the human's verdict on a diff.** Map ticket 76-E, 2026-09-24. Master of Puppets
  records applied, fixed or rejected per task in the goal's log under `~/hydra/puppets/`, and
  `sheet.py` reads it for the ranking. Rung one reads only records, so it cannot learn which runs a
  human applied, and a line after `run_end` would break the closed-record rule. Same shape as the
  policy signal below: kept before anything reads it. Sharp enough to ticket the day rung one wants
  it, and the choice then is a contract 15 event, a sibling file the contract names, or leaving it.
- **Who reads the policy signal.** Rung one is barred from learning from gated runs, auth failures, MCP shortfalls and rate limits, and all of them are recorded in full. Ticket 27 already names a use for one class: a run that failed on too narrow a policy is a policy bug rather than a skill bug. Nothing reads that in v1. Breaks no seam, since D5 requires every field Ouroboros reads to be one the harness writes and not the converse. Same shape as the second-corpus patch: a signal worth keeping before anyone knows what reads it.
- **What a resumed run's usage is scoped to on `claude` and `cursor-agent`.** Ticket 16 measured
  `agy` reporting a whole conversation's tokens on a resumed run's `result` frame, so a run's own
  counts come from its step frames there. The other two report usage only on `result` and neither has
  been resumed under measurement. Nothing in v1 resumes a run except a human typing `hydra resume`,
  and usage crosses nowhere into export, so the only exposure today is a wrong figure in what
  `hydra run` returns.

- **A stall detector.** Liveness is a pid plus a process start time, so a run that is alive and emitting nothing looks exactly like a run working hard, and ticket 14's five failure classes have no entry for it. Message deltas are the cheap heartbeat and ticket 32 turns them off for reasons that hold on their own. `agy` keeps one by accident through `step_update` with `state: ACTIVE`, so the signal exists on one provider of three. Ticket 14's rule applies: Hydra has run zero times and a timeout invented now is a guess.
- **What ticket 13's harvest can actually read.** Ticket 34 went looking for corrections and found
  two problems that belong to harvest rather than to the correction event. Cursor stores chats in two
  formats, plain JSON in older ones and protobuf plus a `blobEncryptionKey` in newer ones, with
  ordering carried by a root blob's hash list rather than by file order. And `agy` omits steps from
  its own `transcript_full.jsonl`, proven by a `step_index` that jumps 1 to 3 across a denied call.
  Ticket 13 made harvest idempotent and never asked what it can parse. Ticket 33 then gave the same
  store a second consumer, since the resolved model id is only there, so how much a record can say
  about a Cursor run now depends on which of the two formats that run happened to write. Sharpening
  this needs a reading of harvest against both, which is more than one patch of fog and may be more
  than one ticket.

  Ticket 38 gave it two more jobs and filled the `claude` column of both. The contract's section
  7.2.1 has no producer for a harvested `cursor-agent` or `agy` run, so a reader there must treat a
  missing event type as unknown rather than as absent, which is a worse position than any recorded
  run is in. And a `claude` transcript turned out to carry eight record types no stream has, one of
  which is run content that entered through no tool call and now blocks export. Whether the other
  two vendors' stores carry an equivalent is unasked, and it decides how much of the interactive
  corpus is exportable at all.

- **Rung two's build details.** `docs/ouroboros-spec.md` section 9 holds four open items and three of
  them are these, which nothing else on this map covers. Whether the proposer needs a sixth
  delegation policy, or runs as `standard`. How a review prompt is fitted when a diff is large, which
  the reference implementation answers with a fit note, a path manifest fallback and a binary-file
  representation, having budgeted 920K tokens per reviewer. And tolerant parsing of a reviewer's
  response, which is the thirty lines nobody writes until a refusal is read as a pass. The first is
  sharp enough to ticket today and the other two are build work for a rung that ships later, so the
  patch waits for someone to decide whether the document is hand-offable without them.

- **What an interactive run gets from the launch sequence.** `docs/harness-spec.md` section 19.6,
  opened by ticket 38 and not covered elsewhere. An interactive run gets no worktree, and two of the
  three providers read their injected MCP config out of one, so on those two an interactive run gets
  no injected servers. Two loose ends follow. The fatal Cursor MCP pre-flight checks that every
  injected server is ready, and with none injected it is a promise about an empty set. And
  `CURSOR_DATA_DIR` points into the run's spawn directory, which is reaped with a worktree that an
  interactive run does not have. Neither breaks anything today, and both are the kind of thing a
  builder hits in the first hour and settles by guessing.

- **Enforcing export eligibility during a run.** Ticket 19's allowlist is audited from the record at export time. Ticket 26 proved all three providers hard-block a tool call from a config Hydra writes per run, but whether a block can be scoped to a path argument on all three is unmeasured. If it can, eligibility stops being an audit and becomes a property the run cannot violate.

## Out of scope

<!-- ruled beyond the destination; closed, never graduates -->

- **Fine-tuning.** Axolotl, Unsloth, preference-optimisation training runs, any weights. Its own effort once there is a corpus worth training on. The record contract must not block it (D12).
- **Cursor GUI automation.** Ruled out already in `~/HephaetusLAB 2.0/V3_CODE_ORCHESTRATION_RESEARCH.md`: fragile, steals the screen, unacceptable in a pipeline. Headless CLI and ACP only.
- **Changing Hermes.** No forks, no upstream contributions, no dependency (D13).
