# 327 - `sheet.rs::rank` reads verdicts across goals, with the prior it starts from stated

Type: build
Status: resolved
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

## Outcome

Built. `sheet::rank_goals` takes a list of goal logs instead of one, and `rank` is now
`rank_goals` over the one log it is handed, so `rank`'s signature and every pinned ranking
stay as they were. `rank_goals` also returns a `Standing` for each agent that was available
under the policy: its prior from `priors.json` and the verdicts counted for it.
`ranked_assignment` takes the logs and returns a `Ranking` (the assignment, the standings, the
priors path), or `None` when there are no sheets. `report::goal_ids` is the walk `report
builders` already made, pulled out so both use it. `assign` reads every other goal log under
the home, but only when `agents/` exists, puts its own log last, and after its assignments
appends one `rank_prior` event. The event carries `priors`, `goals`, `verdicts` and `rows`
(class, provider, model, prior, verdicts), plus a `text`. "Once per `tasks` import" is
implemented as "no `rank_prior` after the last `task` event". A log that cannot be read is
named on stderr and passed over. `IMPLEMENTATION.md` and `integrations/puppets/README.md` say
so, and the README's event table has the new row.

Tests: `sheet::tests::a_verdict_in_another_goal_moves_this_goals_ranking` and
`tests/rank_across_goals.rs`, which drives the built binary on two goal logs on disk.
`differential.rs` gains one normalization: it drops the `rank_prior` line from the Rust's log
before comparing it with the Python's, which never wrote one.

Measured, synthetic. In the unit test both agents ran `t1` in goal b and in goal a, each with
one applied verdict per goal. Goal b alone: sonnet Beta(6, 1) at 0.6813, composer Beta(5, 2) at
0.4897, under a `low` task's 0.5, so sonnet alone ranks. With goal a read too: composer
Beta(6, 2) at 0.5474, score 0.00183, ranks first; sonnet Beta(7, 1) at 0.7197, score 0.0278,
second. In the binary test `assign` gives `t1` the default table alone and `claude sonnet
(ranking)` with goal a's one verdict, and the event reads `rank read 1 verdicts across 2
goals, starting from the priors in <data>/priors.json: docs claude sonnet Beta(5, 1) and 1
verdicts; weird claude sonnet Beta(5, 1) and 0 verdicts`. The rankings on master for the five
bench goals' classes, before and after, are for the session to run; this run cannot read
`~/hydra/puppets`.

What the ticket assumed that was false:
- "Folded into one list" would double-count wrongly. Task ids repeat across goals (`t1` in
  every goal), and `rank` keys both the task's class and its one-verdict-per-task on that id,
  as does the sheet's `task_id`. One flat list would have let goal a's `t1` hide goal b's
  verdict on `t1`. The logs stay a list of lists, each keyed on its own.
- `rank` is not what imports tasks: `tasks` appends the task events and `assign` ranks. The
  event is logged by `assign`, once after each import.
- Line 333 and line 452 had drifted to 326 and 444; and the IMPLEMENTATION.md paragraph that
  cites map 77 is about the local model. The ranking paragraph the ticket means is the one on
  `sheet.py` and `rank`, and that is the one amended.
- The differential write test compares `assign`'s log byte for byte with the Python's, so any
  new event needs a named normalization there. The ticket did not mention it.
