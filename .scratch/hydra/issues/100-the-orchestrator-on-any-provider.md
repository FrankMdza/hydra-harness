# 100 - The orchestrator on any provider

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-03, by the session, from the human's words: "ya es hora de agregar la posibilidad
de escoger cualquiera de los otros proveedores para empezar a orquestar desde allí, y que todo
funcione igual en todos". He decided that it happens; this ticket is how.

## Question

The interactive mode both chat modes use is `claude` only (`src/interactive.rs`, harness spec
10), and the copilot's start script names one repository, the one it lives in. Which modes go to
all four providers (D1), how a non-`claude` chat is recorded (D2), where the copilot's
instructions come from (D3), and how a person points it at their own repository (D4).

Sent to a panel of three on 2026-10-03 with maps 101 and 102, brief in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/brief-100-102.md`.
Judging follows build 252's rule: a split goes to two judges of two vendors, blind, in opposite
orders.

## Answer

Resolved 2026-10-03 by a panel of three under the human's delegation, unanimous on all four.
Seats: `codex` adversary (`standard`, run `01a1038a-cdd1`), `cursor-agent` grok-4.7-high reviewer (`judge`, `01a1038b-e667`), `agy` gemini-3.1-pro-high security (`judge`, `01a1038c-6b2d`). Answers in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/answer-100-102-*.md`.

- **D1, A.** The ungated copilot goes to all four providers. The gated chat goes with map 102.
- **D2, A.** A transcript reader per provider, run at exit, as harness spec 10 says. A provider
  is not a copilot until its reader exists, because a record without usage cannot say whether the
  orchestrator costs less than its agents.
- **D3, A.** One source document, rendered under each provider's file name (`CLAUDE.md`,
  `AGENTS.md`, `GEMINI.md`) in the start directory.
- **D4, A.** The start script takes the repository as an argument and renders it into the
  document; the copilot still starts outside the repository.

Least sure, per the seats: D2 for `agy` and `cursor-agent`, whose transcript formats Hydra
does not control. Builds follow one provider at a time, `codex` first.
