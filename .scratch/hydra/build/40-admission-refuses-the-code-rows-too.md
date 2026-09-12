# 40 - Admission refuses the code rows harness 15.2 names, not only `policies/` and `config/`

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-12. `PROTECTED_CODE_PATHS` in `src/ouroboros/evolve.rs` lists the code rows,
and `protected_path_hit` joins them to the repo root next to the six config paths it already
had, same case-insensitive `inside` check. The rows are `src/ouroboros/review.rs`,
`src/ouroboros/evolve.rs`, `src/policy/`, `src/record/` and `src/env.rs`. Spec 15.2 names its
rows by role, so the constant's doc comment says which file each role landed on.

Two tests in `tests/evolve.rs`. One plants each row as the lone path of a patch against a
scratch repo path and asserts the refusal names it, then checks a case-mangled spelling, an
absolute path under the repo, and a bad path in the middle of a clean patch. The other feeds
`src/run.rs`, `src/ouroboros/corpus.rs`, `src/ouroboros/curate.rs`, `src/policyholder.rs`,
`src/recorder.rs`, `src/environment.rs`, a test file and a doc, and asserts none is refused.
`cargo test --release --test evolve`: 10 passed.

What the ticket did not say: the hook-config writer, row 4, is not one file. `src/policy/`
compiles the gate and the hook command, but `write_spawn_config` in `src/run.rs` is what
writes `settings.json`, and `src/run.rs` is also the whole launch sequence. Refusing every
patch to it would close rung two on most of what it could usefully change, so it is not a
row. IMPLEMENTATION.md says so. Splitting the writer out of `run.rs` so it can be a row is
a decision about the launch sequence's shape, and belongs on the map if anyone wants it.
