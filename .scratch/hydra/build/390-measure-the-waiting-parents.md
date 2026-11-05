# 390 - Measure the parents that wait on a child

Type: build
Status: open
Blocked by:

## Objective

Map 140, measure first. Wake on message (map 141) would free a recorded run that waits on a child
inside `delegate`, about 200 MB each. Nobody has counted how many such parents there are. Count
them before deciding anything.

## Scope

1. Threshold, written here before the run: if the peak number of parents waiting at once, times
   200 MB, is under 400 MB, wake on message is dropped and map 141 closes with that answer.
2. A tally over the records, read only, no change to `src/` beyond a test helper if one is needed:
   the recorded runs that called `delegate`, the peak waiting at once, and the RAM-seconds they held
   at 200 MB each. Interactive runs are excluded, since `src/resume.rs:50` refuses to resume them.
3. The tally runs on a fixture corpus first, two parents whose overlap is known, and a test asserts
   the figures. Then it runs on this machine's corpus by the session, never by a gated run.

## Done when

The table is under Outcome, with whether the threshold held. Map 141 is amended with it.

## Outcome
