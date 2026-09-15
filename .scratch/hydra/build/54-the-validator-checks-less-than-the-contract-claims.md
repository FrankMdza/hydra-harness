# 54 - The validator does not check three things the contract requires

Type: build
Status: open
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
