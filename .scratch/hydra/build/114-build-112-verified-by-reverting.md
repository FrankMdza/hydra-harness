# 114 - Build 112 verified by reverting, and its two sentences

Type: build
Status: claimed
Blocked by:

## What

Build ticket 112 (two residuals of 105 and 106, formerly numbered 108) was written by an agent
that hit its usage limit before it could report. Its diff is on master and its four tests pass,
but nobody has seen any of them fail. A green test only says the code does something; the check
that it catches what the ticket describes is to revert the fix, run the test, read the failure,
and restore. Do that for each of the four, and write the two sentences 112 owes
`IMPLEMENTATION.md`.

The pairing, from the session that applied the diff:

| Test | The fix it covers |
|---|---|
| `tests/cleanup.rs::an_alternates_file_written_while_reap_imports_brings_nothing_in` | `import_run_objects` in `src/worktree.rs`: reap packs from a copy of the run's store rather than the store in place, so an `objects/info/alternates` a still-running process writes during the import brings nothing in |
| `tests/harvest.rs::a_start_time_that_will_not_read_leaves_a_failure_record_naming_the_pid` | `src/interactive.rs`: when `procinfo::start_time` fails after `claude` has started, write a failure record carrying the pid instead of rolling the checkout back from under a live session |
| `tests/harvest.rs::a_record_that_cannot_be_written_still_waits_for_the_session` | same file: a first record write that fails does not stop the wait on the session |
| `tests/harvest.rs::a_recorded_start_carries_the_pid_and_its_start_time` | same file: the recorded start carries pid and start time |

`git log --oneline -3 -- src/interactive.rs src/worktree.rs tests/harvest.rs tests/cleanup.rs`
shows the commit; `git show <sha> -- src/worktree.rs src/interactive.rs` is the diff to read.

## Do

For each row: revert by hand only the function or branch the fix added (not the whole commit,
the tests must stay), run that one test with `cargo test --test cleanup <name>` or
`cargo test --test harvest <name>`, copy the assertion message it fails with, then `git checkout`
the file to restore it and run the test again green. If a test stays green with its fix
reverted, that is the finding: say so, and say what the test is measuring instead.

Then `IMPLEMENTATION.md`, two passages, nothing else in that file:

- In the section headed `## Git that Hydra runs in a run's checkout, since build ticket 95`,
  the paragraph that names build ticket 112's window between the scrub and `pack-objects`: one
  or two sentences saying what closed it and which test pins it.
- In the section headed `## The interactive mode, on `claude`, since build ticket 27`, where the
  text says build ticket 112 is to write a failure record carrying the pid: one or two sentences
  saying it does now and which tests pin it.

Use `grep -n "build ticket 112" IMPLEMENTATION.md` to find both; read at most 60 lines around
each.

## Done when

The report carries, per test, the literal failure message seen with the fix reverted and the
green result after restoring, plus the two `IMPLEMENTATION.md` diffs. `cargo test --test cleanup
--test harvest` and `cargo test --lib` green, `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` clean. No source change is expected; if a revert
shows a test that does not catch its fix, do not fix it here, report it.
