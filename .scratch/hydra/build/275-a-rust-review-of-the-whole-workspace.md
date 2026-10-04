# 275 - A Rust review of the whole workspace

Type: build
Status: claimed
Blocked by:

## What

The human's priority for the night of 2026-10-03: repository quality first, a Rust review, clean
code without changing behaviour, efficiency. The Fable diagnosis of the same day found
`src/cockpit/app.rs` at 3,794 lines and 547 commits in 33 days. This ticket reads; it changes no
code. Its report is the input the session splits into refactor tickets.

Read `src/` and `crates/*/src/` (not tests, not fixtures). For each finding give file and line,
what it is, why it matters, and the smallest change that fixes it, in these classes:

1. **Correctness risks**: `unwrap`/`expect` on input a user or a provider controls, integer casts
   that can truncate, panics reachable from a run, error context lost.
2. **Duplication**: the same logic written twice (two folds, two readers of one file, copies of a
   helper across crates); name both places.
3. **Structure**: files or functions too large to hold in one's head, with a proposed split along
   seams that already exist; modules that reach into another's internals.
4. **Efficiency**: work repeated per tick or per record that could be cached or read once
   (`hydra status` and the cockpit read ~800 records; measure nothing, but name the loop).
5. **Idiom**: places where idiomatic Rust (iterators, `?`, enums over strings, `Cow`, borrowing)
   would remove code, only where it removes code or a bug, not taste.

Rank the findings by value over risk. End with up to eight refactor tickets, each a self-contained
change a separate run can make with the suite as its net, none of which changes behaviour.

## Done when

- The report is the run's final answer; nothing in the worktree changes.

## Outcome
