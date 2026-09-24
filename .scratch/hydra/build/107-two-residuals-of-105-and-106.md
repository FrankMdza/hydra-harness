# 107 - Two residuals of build tickets 105 and 106

Type: build
Status: open
Blocked by:

## What

Both named in the Outcomes of 105 and 106.

1. Interactive mode: `procinfo::start_time` and the first record write can fail after `claude` has
   started. Rolling back there would pull the checkout from under a live session, so write a failure
   record carrying the pid instead.
2. Reap scrubs the run's store and then packs from it, and a process the run left running can write
   in that window. Pack from a copy of the store the run cannot reach, or stop reading the store in
   place some other way, and measure it.

## Done when

Each has a test that fails before, checked by reverting; the suite passes outside a seatbelt.

## Outcome
