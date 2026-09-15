# 51 - The corpus walk reads the real home and ignores `HYDRA_ROOT`

Type: build
Status: resolved
Blocked by:

## What

`tests/record_contract.rs:575` builds its path from `real_home()` and appends `hydra/runs`,
so the two corpus walks validate whatever is in the developer's own `~/hydra` whatever
`HYDRA_ROOT` says. `.github/workflows/ci.yml` sets `HYDRA_ROOT` and its comment claims "the
corpus walks read HYDRA_ROOT", which is false; CI passes because the runner's home has no
corpus, not because the variable works.

`CONTRIBUTING.md` tells a contributor to use `Layout::at` and never `Layout::discover()`, and
never mentions the variable. So a contributor's first full suite run reads their own records
and can fail on a record Hydra never wrote.

Make the walks honour `HYDRA_ROOT`, fix the CI comment, and say in `CONTRIBUTING.md` that the
suite reads a corpus and which one. Found by the cold-clone review.

## Outcome

`corpus_runs_dir` in `tests/common/mod.rs` is the one place that answers where the walks read:
`$HYDRA_ROOT/runs` when the variable is set, `~/hydra/runs` from the passwd entry when it is not.
`tests/record_contract.rs` gained `mod common;` and its walk goes through it, and `corpus()` already
did. `CONTRIBUTING.md` says which corpus the suite reads and how to point it elsewhere. The CI
comment needed no change: it claimed the walks read `HYDRA_ROOT` and that is true now.

**What the ticket did not see coming, and what it cost.** Making the walks honour the variable
turned four tests red on an empty corpus, because they measure a particular record rather than
checking every one. A walk holds on an empty corpus and says nothing; a test asserting "the three
read-only Write denials are still gated" asserts a fact about the machine. `corpus_or_skip` returns
early for those four, which is what the walks already did. Measured: with `HYDRA_ROOT` pointing at
an empty scratch root, `normalize`, `outcome` and `record_contract` pass 139 of 139 in under a
second, which is what CI will see. Without it they read the real 213 records and take six minutes.

**One more thing the run found, from outside the ticket.** The corpus grew a record the outcome
suite had no arm for: run `01a0a56e-c71f` was killed by the kernel at signal 9 on 2026-09-15, with
six runs live on an 8 GB machine, and `the_recorded_write_denials_are_the_only_gated_runs_on_this_machine`
expected every non-failed run to be `Completed` or `Gated`. `Crashed` is derived from the signal on
`run_end.error`, and the gate's per-call answer says nothing about a run stopped from outside, so
the test has that arm now beside the `Failed` one it already had.

Built by `cursor-agent` on `composer-2.5` under `standard`, run `01a0a56f-9a20`, 3 files and 30
insertions. The report named the four failures it caused and attributed them correctly, and did not
fix them; that half was finished here.
