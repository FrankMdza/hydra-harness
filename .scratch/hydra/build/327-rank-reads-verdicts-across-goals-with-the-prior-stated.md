# 327 - `sheet.rs::rank` reads verdicts across goals, with the prior it starts from stated

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 117's answer, question 3. `sheet.rs::rank`
(`crates/puppets/src/sheet.rs` line 333) takes the goal's own events as `verdicts`, so it adapts
within one goal and never across goals: every goal starts from the prior in the `priors` file,
and "the smallest model that passes" is a sentence about one goal's history. Opus's memo on map
117 named this as the thing missing before any promotion rule can mean anything. The report of
build 301 already reads every goal log; `rank` should read the same verdicts.

## What

`ranked_assignment` (line 452) feeds `rank` the current goal's events. Give it the verdicts of
every goal under the home's goal log directory as well, folded into one list, with the current
goal's own verdicts last so they weigh the same as today. Where `rank` reads a prior from the
`priors` file, it now logs on the goal log, once per `tasks` import, an event that states the
prior it started from and how many verdicts across how many goals it read, so the human can see
when the prior stopped mattering. The event is additive to the goal log's contract (not
protected config).

Do not change what `rank` returns when there is one goal and no other logs: the tests that pin
today's rankings stay green as they are.

## Do

1. Read `sheet.rs` whole, `fold.rs` for how a goal log is read, and build 301's `report` for
   how it walks every goal log (`crates/puppets/src/report/`); reuse that walk.
2. The cross-goal read, the event, and tests: two synthetic goal logs where the second goal's
   ranking changes because of the first goal's verdicts, and the event's text.
3. In the Outcome, the real rankings on master before and after for the task classes the five
   bench goals use (the session runs that; you report the synthetic numbers).

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the assignment table and `rank` are described (map 77's paragraph).

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
