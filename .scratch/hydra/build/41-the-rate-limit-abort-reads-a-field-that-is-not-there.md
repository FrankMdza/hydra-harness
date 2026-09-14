# 41 - `rate_limit_abort` reads `raw.status` and the vendor nests it under `rate_limit_info`

Type: build
Status: resolved
Blocked by:

## What

`src/ouroboros/evolve.rs:697` reads `raw.status` off a `rate_limit_event` frame.
IMPLEMENTATION.md:2070 measured that `claude` nests it as `raw.rate_limit_info.status`, so
the abort never fires and a pass under a rate-limited `claude` spends a proposer before
finding out. Contract 8.5 carries the same wrong path; ouroboros 4.7 also says the check runs
before the review, and it runs after the proposer, so "the last claude run" is the proposer
itself. Fix the path, decide where the check belongs, and fix both documents in the same
change. Map ticket 59.

## Outcome

Resolved 2026-09-12. `evolve::rate_limit_abort` is public now and reads
`raw.rate_limit_info.status`, falling back to `raw.status` so a frame in contract 8.5's old
shape still counts. It takes the last `rate_limit_event` frame on the newest `claude` record
rather than the first that matches, because a run warned early and allowed later is not
rate-limited. The check stays where it was, after the proposer and before any judge, and the
doc comment and the abort's reason both say that the last `claude` run it reads is the
proposer itself.

Ouroboros 4.7 and contract 8.5 already carried the nested path; both now say the code
matches since this date. IMPLEMENTATION.md's measurement of the frame and its rung two
section say the same.

Four tests in `tests/evolve.rs`, all on synthetic records under `Layout::at`, nothing
launched: rejected under `rate_limit_info` fires with a reason naming `rejected` and the
proposer; `allowed_warning` then `allowed` does not fire; a flat `status` at the top still
fires; a newer `cursor-agent` record is skipped over. `cargo test --test evolve` is 14 of 14.

Not done: the `resetsAt` staleness test 8.5 asks for. The abort still fires on the vendor's
enum alone, which is what 4.7 says it keys on.
