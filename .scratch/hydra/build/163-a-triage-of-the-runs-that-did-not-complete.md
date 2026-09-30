# 163 - A triage of the runs that did not complete

Type: build
Status: open
Blocked by:

## What

Phase 4.2 of the human's plan, what map ticket 85's research called the one real piece of
"Cortana": a script that names why each run that did not complete stopped. On 2026-09-29 the
dataset held 28 `gated`, 26 `rate_limited`, 12 `failed`, 2 `skill_collision` and 1 `crashed`
runs, and since build 147 a seat that never handed off is a `failed` too. Nothing tells them
apart by cause.

## Do

Tests first in `integrations/grader/tests/test_triage.py`, on synthetic records, each shown
failing before the change. Stdlib only.

1. `integrations/grader/triage.py run` reads the dataset and, for each row whose outcome is not
   `completed`, reads its record and writes one line to `$HYDRA_GRADER_HOME/triage.jsonl`:
   `run_id`, `outcome`, `cause`, `evidence` (the record line that decides, cut to 300
   characters), `triaged_at`. Causes, each with an exact rule you write and document: `quota`
   (a rate-limit frame), `gate` (the run ended on a refusal, or a refusal was the last tool
   event), `provider_crash` (the provider exited without `run_end` from the harvest, or a
   crash frame), `never_started` (the record shape of build 147), `stall` (no event for more than
   ten minutes before the end, the sixth class `docs/rungs-spec.md` names near line 350),
   `collision` (skill collision), `unknown`.
2. `triage.py table` prints the counts per outcome and cause, and per provider and cause.
3. `triage.py sample --n 20 --seed <s>` prints 20 runs with their cause and evidence, for a human
   to check. The plan's bar is 16 of 20 right.
4. `README.md`: a `## Triage` section with the rules.

## Done when

- The tests, one per cause at least, each failed before the change; the grader suite passes.
- The report gives the two commands for the real home and says the rules that felt weakest.
