# Hydra: unified agent harness and Ouroboros

Label: wayfinder:map

## Destination

Three documents, implementable and hand-offable without further discovery:

1. **Harness spec.** How Hydra launches, feeds and records the four agent CLIs.
2. **Ouroboros spec.** Both rungs of self-improvement, with rung one shippable first.
3. **The run-record contract.** The one format both write against.

No production code while this map is open. Throwaway prototypes are fine.

## Notes

**Domain.** Hydra is a router, not an agent. One command picks a provider, injects the shared skills, launches that provider's own CLI as a child process on a pty it owns, and tees its structured output to a trajectory log. Ticket 02 settled that Hydra is the parent process rather than a client of a terminal; ticket 01 settled that it should speak ACP to the providers that have it. Three routable providers in v1: `claude` (2.1.257), `cursor-agent` (2026.08.31-4057e58), `agy` (1.1.23). `hermes` (0.14.0) was the fourth until ticket 22 unrouted it. Versions observed 2026-09-01 15:00. `cursor-agent` self-updated from 2025.10.02 at 12:10 that day and the update invalidated ticket 03's Cursor finding, so re-read the version before trusting any provider-specific claim. Ticket 18 replaced `gemini` (0.42.0) with `agy` as the Google provider; `gemini` stays specced as a cold fallback on an API key. A session can delegate to another provider mid-flight, so a Claude Code session acting as orchestrator can hand a small task to cursor-agent and get a structured result back.

**Skills each session should call.** `mattpocock-skills:grilling` and `mattpocock-skills:domain-modeling` by default. `mattpocock-skills:research` for research tickets, `mattpocock-skills:prototype` for prototype tickets. All prose follows `~/.claude/skills/unslop/SKILL.md`.

**Source material.** `~/Downloads/Harness CLI Unificado para Cursor, Gemini y Claude.md` is the originating research doc. Treat it as a starting point, not as settled: its model naming is stale in places and several of its claims are unverified here.

### Settled while charting

Constraints for every ticket below. Reopening one of these is a scope change, not a ticket.

- **D1** The destination is three documents, not running code.
- **D2** One repo at `~/hydra` holds the harness and Ouroboros, because they share a contract.
- **D3** Thin router first. No custom TUI in v1; the vendor CLIs keep their own. Ticket 13 found the price: a CLI keeping its own TUI cannot also speak ACP, so interactive runs record thin and are harvested from the vendor's own transcript at exit. A TUI is what would let ACP become the interactive path.
- **D4** Two auth planes, named separately. Agent-CLI plane: subprocess, per-provider OAuth, subscription pricing. Raw-model plane: LiteLLM, API keys, one OpenAI-shaped endpoint. Interactive work defaults to the agent-CLI plane. Corrected by ticket 18: plane names the auth mechanism and nothing else. Pricing and interactivity are a second axis, the recorded path against the interactive path, and it cuts across both planes. Google proved they come apart, since `agy` is an agent CLI on a subscription and `gemini` is an agent CLI billing per token on a key. Amended by ticket 12: v1 ships the agent-CLI plane only. The raw-model plane is one gated path, `GEMINI_API_KEY` to `gemini` behind `--allow-billed`, and the spec names LiteLLM as an extension point it does not build. Nothing on that plane has ever made a successful call. Ticket 22 emptied it further: the Copilot token only ever reached Hydra through hermes, so with hermes unrouted the raw plane is that one gated fallback and nothing else.
- **D5** The two systems converge on exactly two things: the run record and the skill store. Nothing else crosses.
- **D6** Agents never hold provider secrets. They ask the harness. Inherited from the HephaestusLab V3 rule. Made true in code by ticket 12: Hydra builds each child's environment from an allowlist, because `GEMINI_API_KEY` is exported in `~/.zshrc` and every child inherited it by default. Measured by ticket 21: the list is `HOME`, `USER`, `PATH`, `TERM`, plus `SHELL` and `TMPDIR` for `claude`. The allowlist is also an injection boundary, since `NODE_OPTIONS` and `PYTHONPATH` execute attacker-named code inside `cursor-agent` and hermes.
- **D7** The CLI is the real interface, MCP is a face over it. `hydra run --provider <p> --task "..." --parent <session-id>` returns JSON on stdout; a thin MCP wrapper gives Claude Code a typed `delegate` tool over the same path.
- **D8** The router core is Rust. A self-edit that does not compile never reaches review.
- **D9** One canonical `~/hydra/skills/`, symlinked into the Claude and Cursor locations, plugin-wrapped for Gemini. Corrected by ticket 03: the Cursor target is `~/.cursor/skills/`, not `~/.cursor/skills-cursor`, which is Cursor's own managed cache.
- **D10** Run records are plain JSONL on disk under `~/hydra/runs/`, and they are the source of truth. Nothing needs to be running for the harness to work. Clarified by ticket 09: one file per **run**, named by a UUIDv7, with a session being a tree of runs linked by `parent_run_id`.
- **D11** Spec both Ouroboros rungs, ship rung one. Rung one edits skills, prompts and config. Rung two edits Hydra's own source behind a sandbox, a test gate and a diff quorum.
- **D12** Training is out of scope. Lossless export to SFT and DPO formats stays a constraint on the record.
- **D13** Hydra is standalone. Hermes is worth copying, not depending on.
- **D14** ~~Hermes is a fourth routable provider.~~ Amended by ticket 22: hermes is not routable in v1. Ticket 08 found neither the xAI nor the Nous account exists, ticket 12 found the Copilot token exchange has never succeeded, and ticket 22 found every remaining credential is junk or empty. It is also the only provider Hydra cannot gate through the environment, since `~/.hermes/.env` loads with `override=True`. What Hydra keeps from Hermes is ticket 05's guardrail rules, which D15 already covers and which need no binary launched.
- **D15** Hydra gets its own curator, built on Hermes' guardrail rules rather than Hermes' code.
- **D16** Hydra never spends on a billed API key without asking. The AI Studio project behind `GEMINI_API_KEY` has billing enabled, so the raw-model plane is opt-in per run rather than a silent fallback. Settled by ticket 18. Ticket 12 gives it a mechanism: an `--allow-billed` flag typed per run, never inheritable, and the delegation policy cannot set it. Ticket 21 found that mechanism does not reach hermes, whose `~/.hermes/.env` loads with `override=True` and beats anything Hydra passes. The hole is latent rather than open: hermes holds no usable Google credential today, so nothing can be spent through it. Ticket 22 decides the gate, and `HERMES_HOME` is the lever, since it redirects both `auth.json` and `.env`.

## Decisions so far

<!-- one line per closed ticket, then zoom the link for detail -->

- [02 - Ghostty pane control](issues/02-ghostty-pane-control.md): Ghostty launches a command but exposes no PID and no exit status, by design. Hydra owns the pty per provider instead, since it must be the parent to tee anyway. Rendering splits off as a separate deferred question.
- [03 - Skill discovery rules](issues/03-skill-discovery-rules.md): one unchanged `SKILL.md` serves all four providers, so no wrapper is forced. D9's Cursor path was wrong and is corrected above. Whether Cursor and Gemini follow symlinks is still unconfirmed.
- [04 - LiteLLM auth modes](issues/04-litellm-auth-modes.md): virtual keys satisfy D6; Vertex service accounts and Bedrock IAM both work upstream; nothing carries a Claude subscription, and that is a terms limit Anthropic enforces.
- [06 - Verify the Ouroboros paper](issues/06-ouroboros-paper-verification.md): paper and benchmarks are real, mechanism is not what we assumed. The quorum is a union veto over responsive reviewers, the sandbox is worktrees, and 63.5% of self-edits get blocked.
- [05 - Hermes curator rules](issues/05-hermes-curator-rules.md): fifteen rules with their reasons, plus two Hermes bugs not to copy. Its model is told archiving is recoverable while the tool it calls runs `shutil.rmtree`, so the lesson is that a guarantee written in a prompt is not a guarantee.
- [07 - Rust TUI against Textual](issues/07-tui-framework-cost.md): Textual wins on streaming markdown and the diff viewer, where ratatui costs 2 to 3 weeks of hand-built work. Live pane attach is the one thing that could put a third channel between the two systems, which is now ticket 09's problem.
- [01 - Provider headless modes](issues/01-provider-headless-survey.md): drive ACP rather than print mode, because print mode leaves only five fields common to all four. Gemini's OAuth path is dead on this machine, which opens ticket 18. Ten named gaps become the record's optional fields, and Hydra must own run ids, timestamps and delegation lineage itself.
- [09 - The run record schema](issues/09-run-record-schema.md): one file per run, envelope plus verbatim raw frame, eleven normalized event types derived at read time. Usage carries a `usage_source`, so Cursor is `estimated` from a model-aware tokenizer rather than a fake zero. Redaction happens at export, which graduated ticket 19.
- [11 - The delegation contract](issues/11-delegation-contract.md): children spawn detached with blocking as a view over them, one git worktree each, permissions from a policy rather than a human, depth capped at 2. Results stay small on purpose, because delegation exists to save the orchestrator's context. Cursor cannot be delegated tool work under any policy narrower than fully permissive.
- [08 - Account and tier inventory](issues/08-account-inventory.md): one account per provider, so ticket 12 builds no rotation. Anthropic has no API key and no credit, and ticket 04 already ruled out carrying the subscription upstream, so it is agent-CLI only. The raw-model plane is one Google AI Studio key and one borrowed Copilot token. D14 corrected above.
- [18 - Which binary is the Google provider?](issues/18-google-provider-target.md): `agy` 1.1.23 grew `--output-format stream-json` during this session, so Google is Antigravity on the subscription with per-step token counts, and `gemini` on the key drops to a cold fallback nobody runs without approval. `agy` cannot be asked for permission, which puts it in Cursor's category.
- [21 - What environment each provider needs, and what it leaks](issues/21-provider-env-requirements.md): the allowlist is `HOME`, `USER`, `PATH`, `TERM`, plus `SHELL` and `TMPDIR` for `claude`. A wrong `HOME` silently logs out three of four providers, `CURSOR_AUTH_TOKEN` overwrites the stored login rather than overriding it, and the scrub turns out to be an injection boundary as much as a secret one. It also found the one thing the allowlist cannot do, which opened ticket 22.
- [22 - Where D16's gate sits for hermes](issues/22-hermes-spend-gate.md): nowhere, because hermes stops being routable in v1. It holds nothing that works and it is the one provider the environment allowlist cannot gate. The rule it sets outlives it: a provider Hydra cannot gate is either given a config root Hydra owns or is not routed, never launched with the gap recorded. The record gains `auth.gated`.
- [13 - Session launch and pane lifecycle](issues/13-session-launch-lifecycle.md): interactive and recorded are two launch modes, because a vendor TUI and ACP cannot share a process. Interactive runs are harvested from the vendor's own transcript at exit, and harvest is idempotent so any later `hydra status` can finish one. Liveness is derived from a pid and a process start time in the run file's opening envelope, and nothing appends to a run file except the process that owns it. Cleanup is always a command you type.
- [12 - The two auth planes, and where secrets live](issues/12-auth-planes.md): Hydra holds no provider credential, so D6 costs no code beyond an environment allowlist that stops children inheriting `GEMINI_API_KEY`. The raw-model plane does not ship in v1 and the one billed path needs a human to type `--allow-billed`. Auth failure is its own run outcome, and auth never happens inside a run.

## Not yet specified

- **The TUI.** Deferred by D3. Ticket 07 settled the framework: Textual, so the TUI is a separate Python process reading `~/hydra/runs/` while the core stays Rust. The interface itself, widget set, diff viewer, session switcher, stays unspecified during this map. `textual-diff-view` from Toad is the starting point when it is built. Ticket 13 gave it a second job: a TUI is what would make ACP the interactive path, which would end the thin-record split and make every run fully recorded as it happens.
- **Langfuse.** Optional viewer that tails `~/hydra/runs/`, never the source of truth. Revisit once there are enough records to be worth browsing.
- **MCP server config across providers.** Each provider has its own MCP config file. Whether Hydra unifies them, injects into them, or leaves them alone is unclear until the skill injection strategy closes.
- **Multi-machine use.** Whether run records and skills follow you to another machine. Ticket 12 settled the auth half: it does not. Every credential is in this machine's Keychain or its environment, so you log in again per machine.

## Out of scope

<!-- ruled beyond the destination; closed, never graduates -->

- **Fine-tuning.** Axolotl, Unsloth, preference-optimisation training runs, any weights. Its own effort once there is a corpus worth training on. The record contract must not block it (D12).
- **Cursor GUI automation.** Ruled out already in `~/HephaetusLAB 2.0/V3_CODE_ORCHESTRATION_RESEARCH.md`: fragile, steals the screen, unacceptable in a pipeline. Headless CLI and ACP only.
- **Changing Hermes.** No forks, no upstream contributions, no dependency (D13).
