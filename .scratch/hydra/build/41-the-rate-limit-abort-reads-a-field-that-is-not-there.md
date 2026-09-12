# 41 - `rate_limit_abort` reads `raw.status` and the vendor nests it under `rate_limit_info`

Type: build
Status: open
Blocked by:

## What

`src/ouroboros/evolve.rs:697` reads `raw.status` off a `rate_limit_event` frame.
IMPLEMENTATION.md:2070 measured that `claude` nests it as `raw.rate_limit_info.status`, so
the abort never fires and a pass under a rate-limited `claude` spends a proposer before
finding out. Contract 8.5 carries the same wrong path; ouroboros 4.7 also says the check runs
before the review, and it runs after the proposer, so "the last claude run" is the proposer
itself. Fix the path, decide where the check belongs, and fix both documents in the same
change. Map ticket 59.
