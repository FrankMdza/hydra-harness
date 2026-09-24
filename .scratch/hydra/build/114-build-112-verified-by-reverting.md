# 114 - Build 112 verified by reverting, its outcome bug, and its two sentences

Type: build
Status: resolved
Blocked by:

## What

Build ticket 112 (two residuals of 105 and 106, formerly numbered 108) was written by an agent
that hit its usage limit before it could report. Its diff is on master and its four tests pass,
but nobody has seen any of them fail. Then the fourth panel's `claude` reader
(`docs/reviews/2026-09-24-fourth-panel-claude.md`, weakness 1) read both sides of the interactive
half and found a bug the green tests do not see:

`record_then_wait` in `src/interactive.rs` (near lines 548 to 620) writes the pid-carrying
failure record as `RunEnd::harvested(None, Some(failure))`, which sets `body_source: "harvest"`
(`src/record/lines.rs` near 186). In `derive`, `src/outcome.rs` near lines 188 to 206, the
harvested arm with no result reads only `exit_code`, and `None` maps to `Completed`. That arm
never reads `run_end.error`. So the record of a launch that failed derives `completed`, source
`unknown`, and `rung_one_learns(Completed)` is true. The same arm turns the "session wrote no
transcript" record (`src/interactive.rs` near 698 to 718) into `completed`, `derived`, when the
exit code is 0. The test for the failure record asserts line counts and the error text and never
asks what outcome the record derives.

The pairing for the revert check, from the session that applied the diff:

| Test | The fix it covers |
|---|---|
| `tests/cleanup.rs::an_alternates_file_written_while_reap_imports_brings_nothing_in` | `import_run_objects` in `src/worktree.rs`: reap packs from a copy of the run's store rather than the store in place |
| `tests/harvest.rs::a_start_time_that_will_not_read_leaves_a_failure_record_naming_the_pid` | `src/interactive.rs`: when `procinfo::start_time` fails after `claude` has started, write a failure record carrying the pid |
| `tests/harvest.rs::a_record_that_cannot_be_written_still_waits_for_the_session` | same file: a first record write that fails does not stop the wait on the session |
| `tests/harvest.rs::a_recorded_start_carries_the_pid_and_its_start_time` | same file: the recorded start carries pid and start time |

`git log --oneline -3 -- src/interactive.rs src/worktree.rs tests/harvest.rs tests/cleanup.rs`
shows the commit; `git show <sha> -- src/worktree.rs src/interactive.rs` is the diff to read.

## Do

1. The revert check, first, on the code as it is. For each row: revert by hand only the function
   or branch the fix added (not the whole commit, the tests must stay), run that one test with
   `cargo test --test cleanup <name>` or `cargo test --test harvest <name>`, copy the assertion
   message it fails with, then `git checkout` the file to restore it and run the test again
   green. If a test stays green with its fix reverted, that is a finding: say so, and say what
   the test measures instead.
2. The outcome bug. In `src/outcome.rs`'s harvested arm, read `run_end.error` before
   `exit_code`: a harvested record whose `run_end` carries an error and no result is a failure,
   not `completed`. Read the contract's section 8 (`docs/run-record-contract.md`, `grep -n "^## 8"`)
   for which of the nine outcomes a launch that failed after the process started maps to, and
   say in the report which you chose and why. The same read must make the no-transcript record
   with exit 0 derive something rung one does not learn from, and the report says what.
3. Extend `a_start_time_that_will_not_read_leaves_a_failure_record_naming_the_pid` to assert the
   derived outcome, and add the same assertion to whichever test covers the no-transcript record.
   Both fail before step 2 with `completed`; check that they do and say so.
4. Stale numbers. `src/worktree.rs:434`, `src/interactive.rs:536`, `tests/cleanup.rs:1786`,
   `tests/harvest.rs:665` and `724` credit "build ticket 107". The ticket was 107, then 108, and
   is 112 now; 107 is the `hy` launcher. Make them say 112.
5. `IMPLEMENTATION.md`, two passages, nothing else in that file. In the section headed
   `## Git that Hydra runs in a run's checkout, since build ticket 95`, the paragraph that names
   build ticket 112's window between the scrub and `pack-objects`: it describes a scrub the code
   no longer does. Say reap copies the store (`src/worktree.rs` near 452 to 475) and packs the
   copy, that the window is closed, and which test pins it. In the section headed
   `## The interactive mode, on `claude`, since build ticket 27`, where the text says build ticket
   112 is to write a failure record carrying the pid: say it does, what it derives after step 2,
   and which tests pin it. `grep -n "build ticket 112" IMPLEMENTATION.md` finds both; read at most
   60 lines around each.
6. `SECURITY.md`, lines near 115 to 118, the same scrub sentence: describe the copy and say the
   window is closed. Nothing else in that file.

## Done when

The report carries, per test in the table, the literal failure message seen with the fix
reverted and the green result after restoring; the outcome each of the two interactive records
derives before and after step 2, with the contract section that justifies it; and the diffs of
steps 4 to 6. `cargo test --test cleanup --test harvest --test outcome` and `cargo test --lib`
green, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean. If step 2
turns out to need a decision the contract does not make, stop there, do steps 1 and 3 to 6, and
say so in the report.

## Outcome

Built by `cursor-agent` on `composer-2.5` under `standard`, run 01a0d3f5-50e5-7360, 5.61M
tokens, the most expensive composer run of the day and the one that earned it. Applied to
master by hand (`~/.claude/handoffs/2026-09-24-panel4/diff-114.patch`).

Step 1, the revert check, with the literal failures:

| Test | With its fix reverted |
|---|---|
| `an_alternates_file_written_while_reap_imports_brings_nothing_in` | `a store naming blobs it does not hold imported cleanly: 2` |
| `a_start_time_that_will_not_read_leaves_a_failure_record_naming_the_pid` | `record_then_wait returned before the session exited` |
| `a_record_that_cannot_be_written_still_waits_for_the_session` | `record_then_wait returned before the session exited` |
| `a_recorded_start_carries_the_pid_and_its_start_time` | `assertion left == right failed left: Null right: Number(<pid>)` |

All four green again after `git checkout`. Build 112 is verified.

Step 2, the bug the fourth panel's `claude` reader found: the harvested arm of `derive` in
`src/outcome.rs` now reads `run_end.error` before `exit_code`, so a harvested close with an
error and no body derives `failed`. The pid-carrying failure record and the no-transcript
record both derived `completed` before; both derive `failed`, `Source::Derived`, now. The
existing test asserts the outcome and a new one,
`a_session_that_wrote_no_transcript_derives_failed`, covers the path no test read. Both failed
before step 2 with `left: Completed, right: Failed`. The agent chose contract 8's `failed` ("the
task ran and failed"). The ticket had asked for an outcome rung one does not learn from, and
`failed` is in the learns column; the contract has no row for a failure on Hydra's side after
the process started, so `failed` is the honest row it has. Whether it wants one is a contract
question, not this ticket's.

Steps 4 to 6: "build ticket 107" is 112 in `src/worktree.rs`, `src/interactive.rs`,
`tests/cleanup.rs` and `tests/harvest.rs`; `IMPLEMENTATION.md`'s two passages and
`SECURITY.md`'s store bullet describe the copy into `reap-import`, say the window is closed and
name the tests. Both `IMPLEMENTATION.md` passages were in the git section; the interactive
heading never named 112.

What the agent found on the way: a run's shell carries `GIT_OBJECT_DIRECTORY` and
`GIT_ALTERNATE_OBJECT_DIRECTORIES`, and a test that runs `git init` inherits them and sees a
repository with no commits. That is why every composer run on 2026-09-24 reported three
`run::tests` failures as "pre-existing". The base task now says to unset both.

On master: lib 210, cleanup 46, gate 98, harvest 12, normalize 90, outcome 47; fmt and clippy
clean.

The full suite then caught what the six files the ticket names did not:
`tests/status.rs::a_dead_interactive_run_is_harvested_by_status_one` expects `completed` with
source `unknown` for a dead session harvested with its transcript, and the harvest writes an
error on that record too ("harvested after the run already exited, so the exit code was not
observed"). The agent's rule read any error as `failed`. The session narrowed it: an error
counts as a failure only when the body holds no message, which is the two records this ticket
is about and not the one status harvests. status 20, harvest 12, outcome 47, lib 210 after.
