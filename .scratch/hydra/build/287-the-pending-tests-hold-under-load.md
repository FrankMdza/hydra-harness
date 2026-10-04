# 287 - The cockpit's pending tests hold under load

Type: build
Status: claimed
Blocked by:

## What

On 2026-10-04 a full `cargo test --workspace`, run while three agents were live, failed four
tests in `tests/cockpit.rs` that start a stub `hydra-puppets pending` and wait on it
(`exit_two_says_the_goal_moved_on`, `exit_three_says_already_approved_and_hides_the_step`,
`inside_a_seatbelt_a_on_a_step_runs_nothing`,
`pending_runs_on_the_shown_goal_at_most_every_ten_seconds`); alone they pass. Find what each
waits on (a wall-clock budget, `PENDING_BUDGET`, a sleep) and make it wait on the event instead,
or give the budget a test-only value large enough, without weakening what the test checks.

## Done when

- The four pass under load: run them while `yes > /dev/null` keeps every core busy, ten times in
  a row. fmt, clippy, `tests/cockpit.rs` clean.

## Outcome
