# 83 - `run_start` takes one struct rather than thirteen arguments

Type: build
Status: resolved
Blocked by: 81

## What

Panel finding F31. `run_start` at `src/run.rs:1474` takes thirteen positional arguments, two
adjacent pairs of `Option` among them, under `#[allow(clippy::too_many_arguments)]`. A swap of
`gate_sha` and `sandbox` compiles. Replace the arguments after `request` with one struct whose
fields are named, drop the `allow`, and move every caller, `tests/delegation.rs` included.
Behaviour does not change: the record bytes a run writes are identical before and after.

## Done when

No `too_many_arguments` allow on `run_start`; the full suite's record-writing tests green; clippy
and fmt clean. Blocked by 81 only because both edit `src/run.rs`. Edits `src/` and `tests/` only.

## Outcome

`run_start` takes `&RunStartRequest`, whose fields carry the names the positional arguments had;
the `too_many_arguments` allow is gone and all six callers moved, `tests/delegation.rs` and
`tests/resume.rs` included (the ticket named only the first). Record bytes unchanged:
`record_contract` 20, `resume` 9, `delegation` 14 green on master, and the full suite with an empty
`HYDRA_ROOT` green. Built by `codex` under `standard`, run 01a0cc3f-4438, 1.4M tokens, reading
nothing from the skill link it was told to skip (map 71).
