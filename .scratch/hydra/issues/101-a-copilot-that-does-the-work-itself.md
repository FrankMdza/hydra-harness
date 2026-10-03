# 101 - A copilot that does the work itself

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-03, by the session, from map 97's measurement after build 249.

## Question

On 2026-10-03 the copilot (run `01a10299-2a61`, Sonnet, four hours) launched no agent, did the
work on the human's web repository itself and spent 21.2M tokens. Nothing stops a copilot from
skipping delegation. Words in its document, disallowed edit tools, a `claude` mod that holds each
edit and asks, or a live count of its tokens against its agents' (D5).

Sent to a panel of three on 2026-10-03 with maps 100 and 102, brief in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/brief-100-102.md`.

## Answer

Resolved 2026-10-03 by the same panel, 2 to 1. Seats: `codex` adversary (`standard`, run `01a1038a-cdd1`), `cursor-agent` grok-4.7-high reviewer (`judge`, `01a1038b-e667`), `agy` gemini-3.1-pro-high security (`judge`, `01a1038c-6b2d`). Answers in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/answer-100-102-*.md`.

**D5, B**: the copilot is launched with its file-edit tools disallowed, on every provider that
has such a switch; the shell stays. `codex` (p 0.46) and `agy` (p 0.75) chose it; `cursor-agent`
chose D, a live count of the copilot's tokens against its agents', and named what would change
its mind: one session with that count visible still ending above its agents and above 1M tokens.
B leaves the shell, so it lowers the odds rather than closing the path; the seats agree on that.
D costs little and map 102's D7 row carries tokens anyway, so it comes with it.
