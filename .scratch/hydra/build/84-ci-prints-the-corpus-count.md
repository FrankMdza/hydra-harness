# 84 - CI prints how many records its corpus walks read

Type: build
Status: resolved
Blocked by:

## What

Panel finding F32. CI runs with `HYDRA_ROOT` pointed at an empty scratch root
(`.github/workflows/ci.yml:19`), so every corpus walk in `tests/record_contract.rs` passes on zero
records and green says only that the readers compile and the fixtures hold. The cheap half: the
walk prints the number of records it read, and CI writes that number to the job summary, so the
zero is visible. The fixture corpus that would close F32 stays a documented limit.

## Done when

A CI run's summary shows the count; locally the same test prints the real count. `IMPLEMENTATION.md`
and `SECURITY.md` or the README, wherever F32 is stated, say the zero is now printed.

## Outcome

The CI job gains a last step that runs `every_record_on_this_machine_meets_the_contract` alone with
`--nocapture` and appends its `checked N record(s)` line to `$GITHUB_STEP_SUMMARY`, or a sentence
saying no runs directory was found. The test already printed the count; nothing in `tests/` changed.
Measured locally against an empty `HYDRA_ROOT`: `checked 0 record(s) under <tmp>/runs`. The first CI
run after the push is the measurement on the runner. Done by the session by hand, no agent run.
