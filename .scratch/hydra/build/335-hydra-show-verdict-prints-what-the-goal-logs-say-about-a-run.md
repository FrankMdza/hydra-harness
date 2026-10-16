# 335 - `hydra show --verdict` prints what the puppets goal logs say about a run

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 130's answer, question 9. Research 129 suggested
`hydra show --verdict` so a reader does not open the events to learn how a run was judged. Grok's
seat corrected the premise: the run record has no verdict field; verdicts live in the Master of
Puppets goal logs under `~/hydra/puppets/`, as `verdict` events keyed by run id, and in the judge
labels the grader writes. So the flag reads a different file than `show` reads today, and the
ticket says which before anything is built.

## What

`hydra show <prefix> --verdict` prints, after the normalized record's summary, one block from the
goal logs: the goal id, the task id, the verdict event's value (`applied`, `fixed`, `rejected`,
or whatever the log carries), who gave it (`by: human|panel|session`) and when. It reads every
goal log under the puppets root the way `crates/puppets` reads them (reuse its reader; `hydra` may
depend on `hydra-puppets` as a library if it does not already, or the reader moves to
`crates/records`; say which). A run with no verdict prints `no verdict on record`. The judge labels
(`judge-labels.jsonl`, read by `hydra-grader`) are printed as a second line when present, naming
the judge model and the label. Nothing is written. `completed` is not a verdict and the output
does not call it one.

## Do

1. The reader and where it lives; the flag in `src/show.rs` and `main.rs`.
2. `tests/show.rs`: a record with one verdict in a synthetic goal log, one with a judge label, one
   with neither.
3. `README.md`'s command table, one line; `IMPLEMENTATION.md` where `show` is described.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green, and the real
output for one judged run of this machine pasted in the Outcome.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

`hydra show <prefix> --verdict` prints the projection, then one block from the puppets goal
logs. The reader stayed in `crates/puppets`: `report::goal_ids` then `log::read_log`, exposed
as `read::verdict_for_run`. `hydra` depends on `hydra-puppets` as a library; the reader did
not move to `crates/records`. A matching `human_verdict` (or `verdict`) event keyed by run
id prints `goal <id>  task <id>  <value>  by: <who>  <ts>`. No event prints `no verdict on
record`. A later line from `grader/judge-labels.jsonl` names the judge model and the label.
Nothing is written. `completed` is not treated as a verdict.

Tests in `tests/show.rs`: a synthetic `human_verdict` of `applied`, a judge-label line only,
and neither. Printed blocks:

```
goal goal-one  task t1  applied  by: human  2026-10-15T12:00:00.000Z
```

```
no verdict on record
judge llama3.1  needs_correction
```

```
no verdict on record
```

Measured: `cargo fmt --all --check` exit 0 with empty stdout; clippy finished in 24.84 s;
nextest 1437 passed, 4 skipped, 45.446 s; doc tests 0 passed across eight crates. The three
new tests ran in 0.075 s, 0.069 s and 0.064 s. The `record_contract` corpus walk named in
hydra-self-edit as a worktree-only red did not fail here.

False: this run cannot read `~/hydra/puppets` or `~/hydra/runs` (relative paths only; the
gate refused tools that named paths outside the worktree, including `~/.claude/skills/unslop`
and the rust-writing skill). `HYDRA_PUPPETS_HOME` and `HYDRA_GRADER_HOME` were unset, so
there is no live judged-run paste from this machine. The blocks above are the synthetic
ones the suite printed.
