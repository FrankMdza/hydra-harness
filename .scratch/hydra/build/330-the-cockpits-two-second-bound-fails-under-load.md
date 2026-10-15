# 330 - The cockpit's two-second bound fails under load: `a_pending_that_hangs_is_given_up_after_two_seconds`

Type: build
Status: open
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
