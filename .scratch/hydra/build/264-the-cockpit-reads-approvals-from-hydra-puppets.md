# 264 - The cockpit reads approvals from `hydra-puppets`

Type: build
Status: claimed
Blocked by:

## What

Build 263 gave `src/cockpit/goal.rs` its own fold of `approval` events (voided when the table's
revision moves, consumed by a `run`, live or not). `crates/puppets/src/fold.rs` is the only
authority on that, and `hydra-puppets show <goal>` prints its result: `revision`, and
`approvals` with `kind`, `ref`, `revision`, `yes`, `consumed`, and `voided` once a later revision
voids one. Two folds drift.

Replace the cockpit's approval logic with a read of `hydra-puppets show <goal>`, at the cadence
and with the bounds `read_steps` already uses for `pending --json` (`PENDING_EVERY`,
`PENDING_BUDGET`, the process-group kill). Keep `table_approved`, `plan_approved` and each row's
`launch_approved` as fields the page reads, filled from that answer. When the helper is missing or
late, the page says the approvals are unknown rather than showing them false.

## Done when

- A test with a stub helper that prints a `show` answer: a voided table approval reads as not
  approved; a consumed launch approval reads as given. The old fold code is gone. fmt, clippy,
  `cargo test --lib`, `tests/cockpit.rs` and `tests/stop.rs` are clean.

## Outcome
