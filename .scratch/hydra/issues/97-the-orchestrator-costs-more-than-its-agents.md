# 97 - The orchestrator costs more than the agents it launches

Type: grilling
Status: resolved
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

Resolved 2026-10-03 by a panel of three under the human's delegation: `codex` (adversary,
`standard`), `cursor-agent` grok-4.7-high (reviewer, `judge`), `agy` gemini-3.1-pro-high
(security, `judge`). One brief for maps 96 to 98, ten decisions, none split three ways, so no
judge sat. Brief and answers in the human's decision log,
`~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/`.

The brief's ten decisions put five questions of this ticket as three; the other two (the
number to watch, the acceptable ratio) were left to the criteria line, "the orchestrator costs
less than the agents it launches on a typical request".

- **Default model (D5): Sonnet**, 2 to 1 (grok 0.54, agy 0.85; codex took Sonnet with reviews
  sent to Opus agents, 0.60). All three named this their least certain pick: the brief gave
  tokens, not each model's weight in the usage window. Grok: if D6 and D7 cut the rereading,
  Opus may meet the cost rule on its own.
- **What it reads at start (D6): a short orchestrator document of its own, launched from a
  directory without the repository's instructions file**, unanimous (0.86, 0.71, 0.90).
- **How it waits (D7): one blocking command per wait**, unanimous (0.80, 0.63, 0.75). Agy: D5
  without D7 still pays for every poll.

Reversed by: three Sonnet copilots that fail to launch and explain agents where Opus succeeded,
or a window that charges the two models within 10 percent (D5); the copilot still above the sum
of its agents once D6 and D7 land.

Carried by build tickets 248 (`hydra wait`) and 249 (the copilot's profile: Sonnet, its own
short document, launched outside the repository's instructions).

### Measured after build 249 (2026-10-03, the human's own session)

Not a comparison, because there was nothing to compare against. The copilot (run
`01a10299-2a61`, Sonnet, started by `suprema.sh` in `~/.hydra-copilot`) ran four hours, 11:29 to
15:33 Colombia time, on `~/another-private-repo`, and launched no `hydra run`. It did the work
itself: SPEC-011, the tests and commit `5b24f7b` on that repository's `main`. It spent 21,208,692
tokens, and 20,727,993 of them (97.7%) were cache reads. The opening message came from the
session and said "trabaja en ~/another-private-repo", which may have read as permission to do the
work directly, so this does not show the copilot refuses to delegate. It shows nothing stops it
from skipping delegation, and that an orchestrator doing the work bills at chat prices. That
question goes to a new map ticket, not to D5 or D6.

### Measured after builds 253 and 258 (2026-10-03, the human's second session)

The first real comparison. Copilot `01a10441-619f`, `claude` Opus (the human's rule since that day:
an orchestrator is only Opus or Fable, which sets D5 aside), no edit tools, 44 minutes, 13 prompts,
on `~/another-private-repo`. It launched five agents (`01a10445-e17a`, `01a10451-8bcc`,
`01a10452-8fdd`, `01a10459-ffc0`, `01a10462-419a`, all `claude` on the CLI's default model) and
waited with `hydra wait` four times and `hydra status` once.

| | total tokens | cache read | the rest |
|---|---|---|---|
| copilot | 12,525,194 | 12,218,710 | 306,484 |
| five agents | 10,608,193 | 10,232,419 | 375,774 |

Raw, the copilot is 1.18 times its agents. Outside cache reads it is below them (306k against
376k). Weighting cache reads at a tenth, the vendor's discount, it is about 1.53M against 1.40M,
1.09 times. So D6 and D7 (short document, `hydra wait`) worked: the copilot delegated and stopped
polling, against 21.2M and no agent on the earlier session. What is left above the agents is
cache re-reads of the copilot's own growing conversation, on the most expensive model. The
criterion "costs less than its agents on a typical request" is not met yet, narrowly.
