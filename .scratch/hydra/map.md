# Hydra: unified agent harness and Ouroboros

Label: wayfinder:map

## Destination

Three documents, implementable and hand-offable without further discovery:

1. **Harness spec.** How Hydra launches, feeds and records the four agent CLIs.
2. **Ouroboros spec.** Both rungs of self-improvement, with rung one shippable first.
3. **The run-record contract.** The one format both write against.

No production code while this map is open. Throwaway prototypes are fine.

## Notes

**Domain.** Hydra is a router, not an agent. One command picks a provider, injects the shared skills, launches that provider's own CLI as a child process on a pty it owns, and tees its structured output to a trajectory log. Ticket 02 settled that Hydra is the parent process rather than a client of a terminal; ticket 01 settled that it should speak ACP to the providers that have it. Four providers: `claude` (2.1.252), `cursor-agent` (2025.10.02), `gemini` (0.42.0), `hermes` (0.14.0). A session can delegate to another provider mid-flight, so a Claude Code session acting as orchestrator can hand a small task to cursor-agent and get a structured result back.

**Skills each session should call.** `mattpocock-skills:grilling` and `mattpocock-skills:domain-modeling` by default. `mattpocock-skills:research` for research tickets, `mattpocock-skills:prototype` for prototype tickets. All prose follows `~/.claude/skills/unslop/SKILL.md`.

**Source material.** `~/Downloads/Harness CLI Unificado para Cursor, Gemini y Claude.md` is the originating research doc. Treat it as a starting point, not as settled: its model naming is stale in places and several of its claims are unverified here.

### Settled while charting

Constraints for every ticket below. Reopening one of these is a scope change, not a ticket.

- **D1** The destination is three documents, not running code.
- **D2** One repo at `~/hydra` holds the harness and Ouroboros, because they share a contract.
- **D3** Thin router first. No custom TUI in v1; the vendor CLIs keep their own.
- **D4** Two auth planes, named separately. Agent-CLI plane: subprocess, per-provider OAuth, subscription pricing. Raw-model plane: LiteLLM, API keys, one OpenAI-shaped endpoint. Interactive work defaults to the agent-CLI plane.
- **D5** The two systems converge on exactly two things: the run record and the skill store. Nothing else crosses.
- **D6** Agents never hold provider secrets. They ask the harness. Inherited from the HephaestusLab V3 rule.
- **D7** The CLI is the real interface, MCP is a face over it. `hydra run --provider <p> --task "..." --parent <session-id>` returns JSON on stdout; a thin MCP wrapper gives Claude Code a typed `delegate` tool over the same path.
- **D8** The router core is Rust. A self-edit that does not compile never reaches review.
- **D9** One canonical `~/hydra/skills/`, symlinked into the Claude and Cursor locations, plugin-wrapped for Gemini. Corrected by ticket 03: the Cursor target is `~/.cursor/skills/`, not `~/.cursor/skills-cursor`, which is Cursor's own managed cache.
- **D10** Run records are plain JSONL on disk under `~/hydra/runs/`, and they are the source of truth. Nothing needs to be running for the harness to work. Clarified by ticket 09: one file per **run**, named by a UUIDv7, with a session being a tree of runs linked by `parent_run_id`.
- **D11** Spec both Ouroboros rungs, ship rung one. Rung one edits skills, prompts and config. Rung two edits Hydra's own source behind a sandbox, a test gate and a diff quorum.
- **D12** Training is out of scope. Lossless export to SFT and DPO formats stays a constraint on the record.
- **D13** Hydra is standalone. Hermes is worth copying, not depending on.
- **D14** Hermes is a fourth routable provider. Corrected by ticket 08: it brings neither xAI nor Nous, because neither account exists and xAI is out of scope by choice. What it actually contributes is a Copilot token lifted from the `gh` CLI.
- **D15** Hydra gets its own curator, built on Hermes' guardrail rules rather than Hermes' code.

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
- [08 - Account and tier inventory](issues/08-account-inventory.md): one account per provider, so ticket 12 builds no rotation. Anthropic has no API key and no credit, and ticket 04 already ruled out carrying the subscription upstream, so it is agent-CLI only. The raw-model plane is one Google AI Studio key and one borrowed Copilot token. D14 corrected above.

## Not yet specified

- **The TUI.** Deferred by D3. Ticket 07 settled the framework: Textual, so the TUI is a separate Python process reading `~/hydra/runs/` while the core stays Rust. The interface itself, widget set, diff viewer, session switcher, stays unspecified during this map. `textual-diff-view` from Toad is the starting point when it is built.
- **Langfuse.** Optional viewer that tails `~/hydra/runs/`, never the source of truth. Revisit once there are enough records to be worth browsing.
- **MCP server config across providers.** Each provider has its own MCP config file. Whether Hydra unifies them, injects into them, or leaves them alone is unclear until the skill injection strategy closes.
- **Cost and budget accounting across pooled accounts.** Hermes tracks credential exhaustion; whether Hydra needs its own view of spend depends on how the raw-model plane ends up structured.
- **Antigravity as the Google provider.** `agy` is the only Google surface pairing the Gemini Pro subscription with an agent, but it emits nothing structured. Ticket 08 rechecked at version 1.0.10: still `--print` only, no `--output-format`, and nobody has logged in yet. Concrete trigger to revisit: if `agy` grows an `--output-format` flag, ticket 18 reopens immediately.
- **Multi-machine use.** Whether run records, skills and auth follow you to another machine.

## Out of scope

<!-- ruled beyond the destination; closed, never graduates -->

- **Fine-tuning.** Axolotl, Unsloth, preference-optimisation training runs, any weights. Its own effort once there is a corpus worth training on. The record contract must not block it (D12).
- **Cursor GUI automation.** Ruled out already in `~/HephaetusLAB 2.0/V3_CODE_ORCHESTRATION_RESEARCH.md`: fragile, steals the screen, unacceptable in a pipeline. Headless CLI and ACP only.
- **Changing Hermes.** No forks, no upstream contributions, no dependency (D13).
