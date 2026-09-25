# 132 - A `rate_limited` record revises that provider's window budget, and the file keeps the history

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d618-a6fd under `standard`, about 0.96M
tokens. The session applied the diff with two changes.

`sheet.py budget <run-id>` takes a `rate_limited` record, sums `raw_input * cache_read_weight +
output_tokens` over every run of the same provider and model whose span touches the window that
just closed, writes the sum into that row's `window_budget_tokens` in `vendors.json`, and appends
a `budget_history` entry: `previous`, `new`, `date`, `run_ids`, and `outside_spend_possible`,
always true, because Hydra cannot see spending outside its own runs. It prints one line with
provider, old and new value, the run ids and the flag. A second call on the same record changes
nothing. Claude is refused, since its cost is utilization, not a token budget. The corpus loading
moved into `corpus_records()`, which `build` also calls.

The session's changes. First, the agent filled a null `windows` with invented spans, five hours
for codex, Cursor and agy and seven days for claude. codex ran out on 2026-09-24 and came back
three days later, so five hours would sum the wrong runs and write a budget far too small, the
first of the two failure modes 76-H names. With no `windows` the command now refuses and asks for
one, or for `--since <ISO time>`, and the README says so. Every row in `vendors.json` has
`windows: null` today, so the first real revision needs one of the two. Second, the refactor
dropped two comments in the corpus loader (the uncommitted-line break and the `HYDRA_ROOT`
symlink), and they are back.

Tests: the agent's `test_budget_from_rate_limited` (three codex records, sum 2150, one history
entry, idempotent second call) now names `windows: "5h"` on its row. It failed before the change
with `AttributeError: ... revise_budget`. The session's `test_budget_refuses_without_a_window`
reads the refusal and then the `--since` path. On master the puppets suite passes, 114 tests.

The line the orchestrator skill (`~/.claude/skills/puppets/`, the human's) should gain, from the
agent: "When a run ends `rate_limited`, run `python3 integrations/puppets/sheet.py budget
<run-id>` and paste the one-line summary to the human before you continue ranking or assigning."
The skill was not edited.
