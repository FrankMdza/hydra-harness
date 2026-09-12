# 40 - Admission refuses the code rows harness 15.2 names, not only `policies/` and `config/`

Type: build
Status: open
Blocked by:

## What

`protected_path_hit` in `src/ouroboros/evolve.rs:447-456` refuses a rung-two patch that
names `policies/` or `config/*`, and nothing else. Harness spec 15.2 lists code rows the
same check is supposed to refuse: the review module, the deny list, the gate script, the
record module, the admission gate itself. Today a patch to `src/ouroboros/review.rs` reaches
the panel, and ouroboros spec 4.4's sentence that two judges approving such a patch "is not a
state this design can reach" is false. Map ticket 59 found it.

Add the rows, with a test in `tests/evolve.rs` that plants a one-line patch to each and reads
the refusal back before any judge is launched. Amend IMPLEMENTATION.md in the same change.
