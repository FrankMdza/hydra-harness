# 132 - A `rate_limited` record revises that provider's window budget, and the file keeps the history

Type: build
Status: open
Blocked by:

## What

Map ticket 76-H, resolved 2026-09-24. Read its `## Answer, the rest` first, including the two
ways the human and the session found it can go wrong.

`integrations/puppets/sheet.py` computes cost as window share against `window_budget_tokens` from
`integrations/puppets/vendors.json`, a list of rows per provider and model. On 2026-09-24 every
row's `window_budget_tokens` is null, so every cost is null. codex went `rate_limited` twice that
day, on 01a0d484-2d8c and 01a0d57e-4cf7, with another session spending in the same window.

## Do

1. A `sheet.py budget` subcommand (or a function the orchestrator skill calls): given a
   `rate_limited` record, sum that provider's recorded spend in the window that just closed, using
   the same token formula `sheet.py` uses for window share, and write the result into the matching
   row's `window_budget_tokens`.
2. Every write appends to a `budget_history` list on that row: the previous value, the new one,
   the date, the run ids summed, and `outside_spend_possible`. That flag is true when a run of the
   same provider by another session, or any use outside Hydra, could have fallen in the window;
   when in doubt, true. One entry undoes the change.
3. It prints one line for the human: provider, old value, new value, the run ids, and the flag.
   The orchestrator skill (`~/.claude/skills/puppets/`, the human's) should relay that line. Put
   the sentence you would add to the skill in your report, and do not edit the skill.
4. `integrations/puppets/README.md`: the paragraph on cost says who revises budgets and how to
   undo one.

This is the other session's territory. Check `git log -3 -- integrations/puppets/` first.

## Done when

- A test under `integrations/puppets/tests/` builds three synthetic records (two completed, one
  `rate_limited`) in a scratch corpus. `budget` writes the expected sum, appends one history
  entry with the right run ids, and a second run on the same record changes nothing.
- The puppets suite passes (`python3 -m unittest discover integrations/puppets/tests`).
