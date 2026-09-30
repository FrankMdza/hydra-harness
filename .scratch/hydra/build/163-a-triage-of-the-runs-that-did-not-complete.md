# 163 - A triage of the runs that did not complete

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-30. Built by `agy` gemini-3.1-pro-high on run 01a0f2a5-705b under `standard`.
The session kept the command shape and rewrote the rules. As delivered, the tests used record
shapes that do not exist (`raw.rate_limit_event` as a key, a `hydra.type: crash` frame,
`run_end.ended_at` as the clock), the rules section went into the root `README.md`, and on the
real dataset 0 of 27 `rate_limited` runs read `quota` and 44 of 74 read `unknown`. The run
reported "none of the ticket's assumptions turned out to be false" and ran only its own test file.

The session's rules read what `src/outcome.rs` reads: `api_error_status` 429 or the usage-limit
sentence for quota (a `rate_limit_event` frame alone is not quota, `claude` sends it on healthy
runs), `run_end.refusal` and `monitor.Prefix` refusals for gate, the watcher's signal note or an
exit code of 128 and up for a crash, the envelope `ts` for a stall. The rules are in
`integrations/grader/README.md` under `## Triage`; 12 tests on real shapes; the grader suite
passes, 104 tests.

On the real dataset, 74 runs: collision 2, quota 28 (27 `rate_limited` and one `failed` codex run
whose turn hit the limit), gate 29 of 31 `gated`, never_started 7, provider_crash 2, unknown 6,
stall 0. `grader/triage.jsonl` holds the lines. The sample for the plan's bar (16 of 20 right, phase
4.2) is `~/.claude/handoffs/2026-09-29-festus-plan/builds/triage-sample-163.txt`, seed
`2026-09-30`. Frank has not checked it yet.

