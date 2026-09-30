# 170 - hydra deny

Type: build
Status: claimed
Blocked by:

## What

A seat request that matches no shape waits for the human (`src/approve.rs`, map 82, build 141).
Today the only no is the launcher's timeout: the request is renamed `<id>.withdrawn.json` after
its wait, and the session hears the same answer as a timeout. The panel of 2026-09-30 and its
judge (Fable) chose option A: `hydra deny <id>` answers at once with one fixed word, distinct from
the timeout, carrying no human-typed text, and the store keeps who denied and when. Reason: any
text the human types reaches the model as a tool result, and the model can read it as orders.
Reversal: if in 10 live denials the session resubmits the same request 3 or more times after a
plain denial, the next step adds a reason.

## Do

Read `src/approve.rs`, the approval half of `src/seats.rs` (grep `approval_required`, `withdrawn`),
`tests/seats.rs`, and the `seat_approval` line in `docs/run-record-contract.md` before code.

**Checked already, 2026-09-30, run 01a0f2d1-1e75 (`claude` opus):** no line shape carries a
denial. `seat_approval` holds only `request_id` (contract lines 121 to 123, enforced at
`src/record/validate.rs:321-326`), and a new line type is a contract change. Its proposal, a
`seat_denial` line shaped like `seat_approval`, is left for a map ticket. This ticket writes
nothing to the run record and needs no contract change.

1. `hydra deny <id>` renames `<id>.pending.json` to `<id>.denied.json` in one `rename`, as
   approve does, so the three writers (approve, deny, the launcher's withdraw) still cannot both
   win. It writes `denied_at` and who denied (the login name from the environment, as the human
   is the only one who types it) into the file, never any free text.
2. The launcher sees the rename and answers the face with a new fixed refusal constant,
   `hydra/request-denied`, distinct from `hydra/request-withdrawn`. It writes no line on the
   session's record: the contract has no line for it (see below), and a withdrawal writes none
   either. The store file is where who and when live.
3. Refuses inside a seatbelt with its own constant, as approve does; refuses an id that is not
   pending with the existing constants.
4. The cockpit's approval dialog gains `d`: deny the selected request after the same `y`
   confirmation approve uses. Nothing else in the cockpit changes.
5. `IMPLEMENTATION.md` and the command list in `README.md`.

Tests first, each shown failing before the change: deny renames and records who and when; approve
after deny is refused; deny after approve is refused; the launcher answers `hydra/request-denied`
within one poll of the rename, not at the timeout; deny inside a seatbelt is refused; the cockpit
key denies after confirmation.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, and the test files you
  touched pass.
