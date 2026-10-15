# 330 - The cockpit's two-second bound fails under load: `a_pending_that_hangs_is_given_up_after_two_seconds`

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session. Twice today the full suite under `cargo nextest` came back
with exactly one red test, `hydra::cockpit a_pending_that_hangs_is_given_up_after_two_seconds`,
once at 2.304 s and once at 2.373 s, both times with four recorded runs live on an eight-core
machine with 790 MB of swap free. Alone it passes at 2.04 s, and the whole suite passed with it
twice on the same tree when the machine was quieter. A test that measures wall time against a
fixed bound reads the machine's load, not the code.

## What

Read the test in `src/cockpit/` and the code it drives: a `pending` listing that hangs is given
up after two seconds. The test should assert the mechanism, not the clock: that the give-up fires
and the cockpit draws without the listing, with the bound injected (a `Duration` the test sets to
100 ms, the production value staying two seconds), or with a generous assertion (the elapsed time
is at least the bound and at most several times it) if injection is not possible without
reshaping the cockpit. Build 304 did the same for the pty flush and build 306 for a stop race;
read their Outcomes for the style.

## Do

1. Find the test and the hang it drives.
2. Inject the bound or widen the assertion; the test must still fail if the give-up never fires.
3. Run the test under load: eight `yes > /dev/null &` workers for the duration of the run, then
   kill them. Paste the timing in the Outcome.

## Done means

The test green under eight `yes` workers five times in a row, the four checks green. Amend
`IMPLEMENTATION.md` where the cockpit's pending listing is described, if it is.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Resolved on 2026-10-15. The bound was already injectable: `Reader::with_pending_budget` came in
with build 264 and the stub tests already pass 30 seconds. The two hang tests passed the production
`PENDING_BUDGET` back in and read the wall clock against 4 seconds. Both now pass 100 ms.

### What changed

- `tests/cockpit.rs`: the test is renamed `a_pending_that_hangs_is_given_up_at_the_budget`,
  since it no longer waits two seconds. Its stub sleeps 30 s (`HANG`), the reader gets 100 ms
  (`SHORT_BUDGET`), and it asserts that the read took at least the budget and less than half the
  hang, that the steps are empty, that the error reads `hydra-puppets pending g2 did not answer
  in 100ms`, and that `pending` was asked at most once. A pin, `PENDING_BUDGET == 2s`, keeps
  the production value in the test. `ticket264_a_late_show_leaves_the_approvals_unknown` had the
  same 4-second wall-clock bound against the production budget and got the same change.
- `src/cockpit/snapshot.rs`: the give-up error formats the budget with `{:?}` in place of
  `as_secs()`, which would have printed `0s` for 100 ms. Production still reads `did not answer
  in 2s`.

### Measured

- Before, with eight `yes` workers, five runs of the two old tests: all green, 2.035 to 2.046 s
  and 2.254 to 2.397 s. The red did not reproduce on CPU load alone.
- The give-up disabled (budget 60 s against the 30 s hang): red at 30.847 s, panicking on the
  elapsed assertion. Restored afterwards.
- After, with eight `yes` workers, five runs in a row: all green, 0.228 to 0.256 s per test.
- Full suite: 1418 passed, 4 skipped, in 42.4 s.

### What was false

- The ticket reads the failure as the two-second bound. It cannot have been the elapsed assertion:
  that one allowed 4 s and the test went red at 2.304 and 2.373 s. Both reds keep the session's
  error text, which this run does not have. The likeliest assertion is `pending-calls` having one
  line. Under swap pressure the stub's `sh` can be killed at the budget before it writes its first
  line. That assertion is now `<= 1`. The error text already shows the helper was asked and
  killed.
- `IMPLEMENTATION.md` said the hang test "keeps the production two seconds". It no longer does,
  and that paragraph is amended.
- `a_helper_that_hangs_on_approve_is_waited_on_five_seconds` still checks `5 s <= waited < 7 s`
  against the wall clock, the same kind of bound. It has not gone red yet. It is left alone
  because this ticket does not name it.
