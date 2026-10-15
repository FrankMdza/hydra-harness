# 335 - `hydra show --verdict` prints what the puppets goal logs say about a run

Type: build
Status: claimed
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

Measured by the session on the Mac after applying, 2026-10-15, on the first goal's `redact` run
(`01a0d588-ebc8`), the two lines after the projection:

```
goal ea69da1f-0114-40fd-8a1d-bed411894331  task redact  fixed  by: human  2026-09-24T22:32:58.038Z
judge gemma3:1b-it-qat  ok
```

A proposer run of the same goal (`01a0d57e-4cf7`) prints `no verdict on record` and
`judge rule  quota`. Applying this build beside 334 conflicted in `src/show.rs`; the session kept
335's file and put 334's three edits (`finding_json` takes the token and prints
`rejected_before_hook`) back by hand. Suite after both: 1443 passed.
