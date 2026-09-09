# 39 - `hydra show` and `hydra reap` take a run-id prefix

Type: build
Status: resolved
Blocked by:

## What

Every command taking a run id took all 36 characters, because `Layout::run_file` builds the
record path straight from the string it is handed. A prefix missed the file and the command
reported the run as missing, so every session that read a record copied a uuid to do it.

Resolve a prefix against the run files. Exactly one match is that run; none or several is an
error naming which. A full uuid keeps working unchanged. One resolver, both callers.

## Why this one was typed rather than launched

Rung two proposed it twice, on passes `01a084d3-bdd3` and `01a08684-67f4`, and both branches
were blocked. The second patch was good and the block was wrong: map ticket 57 has the reason,
which is that `judge_task` never shows a judge the proposer's task, so `agy` blocked the
proposer for obeying a contract the task named. Rather than spend a third pass arguing with a
blind panel, the code was written by the orchestrator, from the two patches, on 2026-09-09. The
seven-day Claude window stood at 0.83 that morning and a pass costs about 3.5 points of it,
which decided it as much as the review did.

## Outcome

`src/resolver.rs`, 8 tests in `tests/resolver.rs`, two call sites, one contract moved.

**A full uuid is never looked up.** It passes through whether or not a record exists, because
`hydra reap <uuid>` on a run whose record was pruned is a real thing to type and predates this
ticket. Only a shorter string is resolved.

**A short string matching nothing is an error, and that moved a contract.**
`reaping_a_run_that_was_never_there_does_nothing_and_says_so` passed `"nobody"` and expected a
benign result. Under prefix resolution `"nobody"` is a typo rather than an absent run, and the
two deserve different answers: a typo that reports "nothing to do" looks like success. The test
keeps its name and its assertions and takes a well-formed uuid no record carries;
`reaping_a_prefix_that_names_no_run_is_an_error_rather_than_nothing_to_do` pins the new half.
The move is stated here and in the test's own comment rather than done quietly, which is the
distinction pass 7's proposer failed to draw when it made the same edit.

**`is_uuid` checks shape, not length.** Counting to 36 admits 36 characters of anything, which
is the hole pass `01a084d3-bdd3` walked through when it fed a length check an invented id to
make a test pass. Hyphens at 8, 13, 18 and 23, hex everywhere else, and a test that feeds it
three different 36-character strings that are not uuids.

**Ambiguity is sorted before it is reported**, so the same corpus prints the same list twice
running, and a test asserts that rather than trusting `read_dir` order.

**An absent corpus reports no match rather than a read error.** A machine with no
`~/hydra/runs/` gets the same sentence a populated one gives for a prefix nobody used.

Suite green at 23 test files, 26 `test result: ok` lines with `tests/resolver.rs` added,
zero failures, about ten minutes single-threaded.
`cargo fmt --check` and `cargo clippy -- -D warnings` clean. `cargo clippy --all-targets`
reports five lints in `tests/evolve.rs`, `tests/skills.rs` and `tests/export.rs` that predate
this ticket and that the admission gate does not run; none is in the files this ticket touched.
