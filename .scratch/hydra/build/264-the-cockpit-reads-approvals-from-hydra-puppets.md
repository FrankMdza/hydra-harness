# 264 - The cockpit reads approvals from `hydra-puppets`

Type: build
Status: resolved
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

Built 2026-10-04 by an Opus run (`01a106e2-809d`), reviewed and applied by the session.

The cockpit's own approval fold in `src/cockpit/goal.rs` is gone. `read_steps` asks
`hydra-puppets show <goal>` beside `pending --json`, at the same cadence and with the same
two-second, process-group kill budget, and `fill_approvals` applies the helper's rules. The
three approval fields are `Option<bool>`; the page reads `unknown` when the helper is missing or
late, or reports a different revision than the cockpit's fold. Four tests in `tests/cockpit.rs`
(a voided table approval reads no, a consumed launch approval reads given, a late `show` and no
helper read unknown), three of them failing before.

**Checked by the session:** the real `hydra-puppets show` on the goal in `~/hydra/puppets` prints
`revision` 13 and nine approvals with `kind`, `ref`, `revision`, `yes`, `consumed`, and `voided`
only on a voided one, the shape the parser reads. Noted by the run: the older steps tests' stubs
answer only `pending`, so `show` leaves a parse error in `snapshot.errors` there; no test checks
those errors, so nothing fails, but a later one that does will see it.
