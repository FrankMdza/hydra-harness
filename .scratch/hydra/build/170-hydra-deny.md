# 170 - hydra deny

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-30. First run 01a0f2d1-1e75 (`claude` opus) stopped at the contract check: no
record line carries a denial. The session cut the record line from the ticket (a withdrawal writes
none either, and the judge's condition put who and when in the store) and relaunched. Second run
01a0f2d2-26a1 (`claude` opus) built it; the session applied it and added "d denies" to the
cockpit's key hint, which the run had left because the ticket said nothing else changes.

`hydra deny <id>`: one `rename` from pending to denied, so approve, deny and the launcher's
withdraw still cannot both win; then who (`USER` or `LOGNAME`, filtered to a login name's
alphabet, else `unknown`) and when are written into the denied file. The launcher answers the face
at once with the fixed `hydra/request-denied`, `is_error`, nothing on the record. Refused inside a
seatbelt with `hydra/deny-inside-seatbelt`. The cockpit's `d` then `y` denies. Five tests; fmt and
clippy clean, `--test seats` 30, `--test cockpit` 25, `--lib` 220 on master.

Not seen yet: the answer reaching a live model through the face, and `hydra deny` typed in a real
terminal. `hydra deny` does not check the session's socket as approve does: a denial with nobody
waiting launches nothing and the store keeps who. Reversal from the judge stands: 3 or more
resubmissions in 10 live denials moves to a denial with a reason.

