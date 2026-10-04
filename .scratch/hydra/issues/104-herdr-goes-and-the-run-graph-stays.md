# 104 - Herdr goes, and the run graph stays

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-03, by the session, from the human's answers to the Fable diagnosis (run
`01a1047d-2669`, report in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/fable-diagnostico.md`).

## Question

D1 and D2: whether Herdr stays the interactive layer (map 44) and, if not, where the run graph the human uses to study runs lives.

Sent to a panel of four (codex, grok, gemini, opus) with maps 104 to 107, brief in
`~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/brief-104-107.md`. A split goes to Fable and astra, blind, in opposite orders (build 252's rule).

## Answer

Resolved 2026-10-03 by a panel of four. Seats: `codex` adversary (`standard`, run `01a104b7-f472`), `claude` opus 5.5 architect (`judge`, `01a104b7-f471`), `agy` gemini-3.1-pro-high security (`01a104e0-bea0`), `cursor-agent` grok-4.7-high reviewer. Answers in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/answer-104-107-*.md`.

- **D1, B, 3 to 1** (gemini chose C, delete): Herdr is frozen, not deleted; no new work goes into
  `integrations/herdr/`, the cockpit is the one interface, and what the human uses from the plugin
  comes into Hydra. Build tickets 225 to 230, the plugin's Rust port, are dropped.
- **D2, B, 3 to 1** (grok chose A, a `hydra graph` subcommand): the run graph becomes a view inside
  the cockpit on the selected run. Build 271.
