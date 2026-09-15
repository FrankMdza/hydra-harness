# 54 - The validator does not check three things the contract requires

Type: build
Status: resolved
Blocked by:

## What

`RUN_START_FIELDS` in `src/record/validate.rs` omits `floor_lifted`, `env_allowlist`, and the
four leaves inside `auth` (`mechanism`, `source`, `allow_billed`, `gated`); the `auth` group is
checked for non-null and its contents never. `floor_lifted` is missing from 27 records on this
machine and nothing noticed, which is the argument: the validator is what the contract says
makes a record checkable.

The care this needs: 27 existing records would start failing a walk that has to keep reading
them. Contract 15's rule for a field added later applies, so the check is presence-if-the-run
-is-new-enough, or a type check on the field when present plus a limit the reader reports.
Decide which, and say it in section 15 rather than in the code alone.

Found by the conformance audit, `docs/reviews/2026-09-14-record-contract-conformance.md`.

## Outcome

**The decision: type when present, never presence.** The ticket offered two arms and they are the
same arm once you measure. A presence check on a field added after the first records fails every
record written before it, and contract 15 already promised that adding a field costs nobody a
migration. Measured 2026-09-15 over 213 records: `gate_sha` absent on 205, `scopes` on 36,
`floor_lifted` on 27, `env_allowlist` and `auth` on the 8 harness runs where the question does not
apply. A presence check would have turned all of those red.

`LATER_RUN_START_FIELDS` in `src/record/validate.rs` is the third class beside the two that existed:
six fields, each with the shape it carries and a flag for whether null is an answer. `floor_lifted`
is the only one that sets the flag, because contract 5.5 has a harness run write it null while the
other five are absent in that case. `Shape` is five variants, and `Sha` is the only one that pins a
string's contents: 64 hex digits, which is what caught nothing today and will catch a `gate_sha`
written as a path or a truncated hash.

The `auth` leaves are checked where they were not. The group was tested for non-null and its
contents never, which is how `allow_billed` stayed a field nothing writes and the validator said
nothing for weeks. All four of contract 5.4 are required when `auth` is present, because three of
four is a record that cannot say who paid. All 213 records on this machine carry all four, non-null.

Said in section 15 rather than in the code alone, as the ticket asked, plus the sentence that the
two rules are one rule. Suite: `record_contract` 20 of 20, the corpus walk over 213 records in 0.44
seconds.
