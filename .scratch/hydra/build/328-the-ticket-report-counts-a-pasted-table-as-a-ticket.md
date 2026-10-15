# 328 - The ticket report counts build 312's pasted table as a ticket, and leaves 299 of 327 Outcomes `unsaid`

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 124's findings. `hydra-puppets report tickets`
(build 312) reads every ticket's `## Outcome`, finds the run ids in it, and classifies the prose
into a few phrases (applied as is, corrected, rejected, unsaid). Build 312's own Outcome holds
the report's two tables pasted whole, so every model named in that table gains one ticket and
one `rejected` from a ticket that built nothing of theirs. And the phrase heuristic leaves 299 of
327 tickets `unsaid`, which is most of the table saying nothing.

## What

Two fixes in `crates/puppets/src/report/tickets.rs`:

1. A ticket's Outcome is read up to the first fenced block or Markdown table, or run ids are taken
   only from prose lines and never from table rows (a line starting with `|`). Either rule keeps
   312's pasted table out; say which you took and why in the Outcome. Build 312's row then names
   its own run and nothing else.
2. The `PHRASES` heuristic is widened from what the Outcomes actually say. Read thirty `unsaid`
   Outcomes, list the sentences that state the verdict ("applied", "went to master as it came",
   "the session fixed", "needed a correction", "rejected", "not applied", "built in this run"),
   and add the patterns that cover them, each with a test case quoting a real Outcome sentence.
   Report the new count per class and how many stay `unsaid`.

The session re-runs the report on master and pastes the new tables into this ticket's Outcome,
in prose-safe form (the run ids on their own lines), so this ticket does not repeat 312's
mistake.

## Do

1. Read `report/tickets.rs`, build 312's ticket and `crates/puppets/tests/tickets.rs`.
2. The two fixes with their tests; the fixture tickets under the tests gain one with a pasted
   table and one per new phrase.
3. The four checks green.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the ticket report is described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
