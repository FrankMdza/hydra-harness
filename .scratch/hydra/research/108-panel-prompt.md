# Decision panel for map ticket 108

You are one seat on a four-seat decision panel. Read only: do not edit, create or delete a file in the repository, and do not commit. Your seat name is in your task line.

The human is away and said the panel decides. Your answer counts as a vote, so commit to one option.

## Read

1. `.scratch/hydra/issues/108-one-list-of-secret-classes.md`, the question and its options A, B and C.
2. `src/record/secrets.rs`, the write-path sweep, and every reader of the `secret_flag` field: `src/export.rs`, `src/record/validate.rs`, `src/record/mod.rs`.
3. The export redactor in `src/export.rs` around the class list, and `crates/grader/src/redact.rs`.
4. The cockpit: find where it hides or masks a line, and whether `record::secrets::prefixes()` has a caller.
5. `docs/run-record-contract.md` section 13, and `SECURITY.md` where it names the redactor's classes.

## Answer

Pick A, B or C. For your choice say:

- What changes in a record written after the change, and whether a record written before it still validates. Section 13 calls `secret_flag` a cache that a rescan reproduces exactly; say whether your choice keeps that true.
- Whether export's `secret_flag` refusal would start refusing runs that export redacts today, and whether that is wanted.
- Where the one list lives so the write path, export, the grader and the cockpit cannot drift again, given that `crates/grader` is its own crate.
- A false positive cost: a class whose prefix appears in ordinary text, such as `-----BEGIN` or `ya29.`, and what it does to the cockpit and to export.

Then list the build tickets your answer needs, one line each, smallest first. Say which claims you checked by running something.

At most 700 words. End with `## Pending from me`, `## Changed` and `## Found`.
