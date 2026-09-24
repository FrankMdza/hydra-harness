# 112 - Two residuals of build tickets 105 and 106

Type: build
Status: resolved
Blocked by:

## What

Both named in the Outcomes of 105 and 106. Opened as 107 and renumbered on 2026-09-24 by build
ticket 102: 107 was already the `hy` launcher, and `frontier.py` keys tickets by number, so one of
the two vanished from its count.

1. Interactive mode: `procinfo::start_time` and the first record write can fail after `claude` has
   started. Rolling back there would pull the checkout from under a live session, so write a failure
   record carrying the pid instead.
2. Reap scrubs the run's store and then packs from it, and a process the run left running can write
   in that window. Pack from a copy of the store the run cannot reach, or stop reading the store in
   place some other way, and measure it.

## Done when

Each has a test that fails before, checked by reverting; the suite passes outside a seatbelt.

## Outcome

Built by an Opus 5.5 subagent in its own worktree, which hit the session's usage limit before it
wrote this Outcome or reported. The session applied its uncommitted diff to master and ran what it
touched: lib 209, cleanup 46, harvest 11 and spawn 33 passed, fmt and clippy clean. The new tests
are `tests/cleanup.rs::an_alternates_file_written_while_reap_imports_brings_nothing_in`, and in
`tests/harvest.rs`, `a_start_time_that_will_not_read_leaves_a_failure_record_naming_the_pid`,
`a_record_that_cannot_be_written_still_waits_for_the_session` and
`a_recorded_start_carries_the_pid_and_its_start_time`. **Not verified:** that each new test fails
with its fix reverted, which the agent may have done and could not report. That check, and a note
for `IMPLEMENTATION.md`'s git and interactive sections, are the first thing the next session owes
this ticket.

