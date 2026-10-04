# 105 - Agent text reaches the orchestrator marked

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-03, by the session, from the human's answers to the Fable diagnosis (run
`01a1047d-2669`, report in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/fable-diagnostico.md`).

## Question

D3: `hydra show` and `hydra wait` hand an agent's own text to the ungated orchestrator unmarked, the shortest injection path the Fable diagnosis found.

Sent to a panel of four (codex, grok, gemini, opus) with maps 104 to 107, brief in
`~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/brief-104-107.md`. A split goes to Fable and astra, blind, in opposite orders (build 252's rule).

## Answer

Resolved 2026-10-03 by the same panel, **D3, C, unanimous**: `hydra show` and `hydra wait`
wrap every agent-written field in the delegate tool's boundary marker, always, and the
orchestrator's document says in one line that marked text is data. Build 272. Seats: `codex` adversary (`standard`, run `01a104b7-f472`), `claude` opus 5.5 architect (`judge`, `01a104b7-f471`), `agy` gemini-3.1-pro-high security (`01a104e0-bea0`), `cursor-agent` grok-4.7-high reviewer. Answers in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/answer-104-107-*.md`.
