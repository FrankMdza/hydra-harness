# 102 - The cockpit is for the one who orchestrates

Type: grilling
Status: claimed
Blocked by:
Opened: 2026-10-03, by the session, from the human's words: the UI only makes sense for the
orchestrator session; a single agent makes more sense as a CLI. He also asked to see, per seat,
the exact model and what it was asked. Reopens map 98's D10 by the human's leave.

## Question

Whether the gated chat session stays in the cockpit (D6), given that on 2026-10-03 one tried to
commit into the main `.git` and then told the person to commit by hand instead of `hydra reap`;
and what the cockpit shows per run and per panel (D7).

Sent to a panel of three on 2026-10-03 with maps 100 and 101, brief in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/brief-100-102.md`.

## Answer

Answered 2026-10-03 by the same panel. Seats: `codex` adversary (`standard`, run `01a1038a-cdd1`), `cursor-agent` grok-4.7-high reviewer (`judge`, `01a1038b-e667`), `agy` gemini-3.1-pro-high security (`judge`, `01a1038c-6b2d`). Answers in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/answer-100-102-*.md`.

- **D6, A, unanimous.** The gated chat leaves the cockpit; `hydra run --interactive --policy`
  stays on the command line.
- **D7, split, and the human's own words disagree with the majority.** `cursor-agent` and `agy`
  chose C (provider, exact model, policy, elapsed and tokens on the row, the task in the detail,
  panel detail left in the goal log); `codex` chose B (C plus, for a panel seat, its role and
  question, and for a panel, who judged, whether it split, and each vote). The human asked for B
  in his own words on 2026-10-03 ("no solo codex modelo astra... cómo está el panel"). Left to
  him; the ticket stays open on D7.
