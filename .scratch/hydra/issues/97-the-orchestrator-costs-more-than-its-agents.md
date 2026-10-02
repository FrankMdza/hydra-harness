# 97 - The orchestrator costs more than the agents it launches

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-02, by the session, from the human's words after the usability probes: "the
orchestrator is spending more tokens than the sub agents, which is the opposite of what we
wanted".

## What the session measured

Five copilot sessions on 2026-10-02, `hydra cockpit --session claude --model opus`, no policy,
each asked one thing that launched one to three agents. `hydra show`, `usage.counts`:

| Copilot run | Total tokens | Cache read | Cache write | Output | Agents it launched | Their tokens |
|---|---|---|---|---|---|---|
| `01a0fe2b-79e3` | 428,109 | 374,686 | 49,550 | 3,857 | none | 0 |
| `01a0fe63-06b7` | 901,222 | 809,383 | 83,756 | 8,051 | 2 sonnet, `read-only` | 75,649 and 67,804 |
| `01a0fe69-8bb8` | 1,773,786 | 1,656,257 | 103,685 | 13,792 | 1 sonnet, 2 opus `judge` | 122,078, 32,314, 27,840 |
| `01a0fe70-de74` | 1,539,712 | 1,443,672 | 84,386 | 11,606 | 1 sonnet, 1 composer-2.5 | 75,847 and 116,426 |

- Nine tenths of a copilot's total is cache read: the same context sent again on every turn.
  The context it starts with is this repo's `CLAUDE.md`, the human's global instructions and the
  memory index, about 50,000 tokens written to cache before the first word.
- The copilot polls. It launches with `hydra run`, then calls `hydra status` and `hydra show`
  until the agents end, and every call is a turn that reads the whole context again.
- It runs on opus. The agents ran on sonnet and cost a tenth to a twentieth.
- Not measured: what the same four requests cost with the copilot on sonnet, or from a directory
  with no `CLAUDE.md`. The fifth session was still live when this was written.

## Questions for the human

1. Which model is the copilot by default? `suprema.sh` takes it as an argument and defaults to
   opus.
2. What does the copilot read at start? (a) this repo's `CLAUDE.md` as today; (b) a short
   orchestrator document of its own, launched from a directory that is not `~/hydra`; (c) a
   skill loaded on demand.
3. How does it wait? (a) polling as today; (b) one blocking command, `hydra-puppets wait` or a
   `hydra run --timeout` long enough, so a wait is one turn; (c) the cockpit tells the chat when
   a run ends.
4. Is the number to watch tokens, or the share of the provider's window? Cache reads are billed
   at a fraction and count differently against a subscription window (map 76-H, build 132).
5. What ratio is acceptable: the copilot under the sum of its agents, or a fixed ceiling per
   request?

## Answer
