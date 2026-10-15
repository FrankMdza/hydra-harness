# 328 - The ticket report counts build 312's pasted table as a ticket, and leaves 299 of 327 Outcomes `unsaid`

Type: build
Status: resolved
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

## Outcome

Built in this run.

01a13e30-e5af-7ef0-b31a-3ec9ddc6aabf

Outcome text for run ids and class stops at the first fenced block or a line
that starts with a pipe. 312's snapshot is a fenced TSV, so skipping pipe
rows alone would not have kept it out. 312's row then has no run: its prose
never named one. That assumption in this ticket was false. Line wraps fold
to spaces before a phrase is sought. Two example strings from the ticket
body occur in no Outcome. Tickets-only report on this tree. The snapshot
and the new patterns sit in the fence below, so they are not this ticket's
own attribution. The file-read of the unslop and rust-writing skills outside
the worktree was refused (hydra/out-of-scope).

```text
new phrases, each with a fixture quoting a real Outcome sentence:
applied as is: applied to master; applied as written; applied by the session;
applied unchanged; the session applied; applied it as it was; built in this run;
applied without changes
corrected: fixed by the session; with one change; with one sentence;
the session corrected; corrected there; adjusted by hand; fixes by the session;
added one change
absent from every Outcome: went to master as it came; needed a correction

before: unsaid 301 of 329
after, tickets-only, no --runs (policy refuses the records directory):
applied as is 131, corrected 34, finished by hand 13, rejected 9, unsaid 142 of 329

provider	model	tickets	runs	total_tokens	median_tokens	completed	failed	rest	applied as is	corrected	finished by hand	rejected
unavailable	unavailable	329	unavailable	unavailable	unavailable	unavailable	unavailable	unavailable	131	34	13	9

312	-	unavailable	unavailable	unavailable	rejected	rejected	unavailable
328 names only
01a13e30-e5af-7ef0-b31a-3ec9ddc6aabf
```
