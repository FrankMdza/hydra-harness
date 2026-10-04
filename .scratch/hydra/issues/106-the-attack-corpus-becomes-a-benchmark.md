# 106 - The attack corpus becomes a benchmark

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-03, by the session, from the human's answers to the Fable diagnosis (run
`01a1047d-2669`, report in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/fable-diagnostico.md`).

## Question

D4: the human wants it on the roadmap, attack success per provider and policy with intervals.

Sent to a panel of four (codex, grok, gemini, opus) with maps 104 to 107, brief in
`~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/brief-104-107.md`. A split goes to Fable and astra, blind, in opposite orders (build 252's rule).

## Answer

Resolved 2026-10-03, **D4, A**. The panel split 2 to 2 (codex and gemini A, opus and grok B); the
tie-break of build 252's rule, `claude-fable-5-1` (run `01a104e7-e73d`) and `codex` `gpt-6-astra`
(`01a104e7-e730`), blind as Seats 1 to 4 and reading in opposite orders, both chose A at p 0.62:
the whole corpus in every supported cell, three repetitions per attack, attack success with Wilson
intervals, run as nightly batches over about a week. Astra: a cell's rate pools every attack, so
its sample is three times the corpus, and B buys no narrower cell interval for the same quota while
losing coverage. Both name what would reverse it: the measured quota of the full run not fitting a
week at two or three agents at once. Build 273 measures that first. Seats: `codex` adversary (`standard`, run `01a104b7-f472`), `claude` opus 5.5 architect (`judge`, `01a104b7-f471`), `agy` gemini-3.1-pro-high security (`01a104e0-bea0`), `cursor-agent` grok-4.7-high reviewer. Answers in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/answer-104-107-*.md`.
