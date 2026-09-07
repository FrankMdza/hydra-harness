# 15 - `scopes` and `floor_lifted` on the record

Type: build
Status: open
Blocked by: 14

## Goal

Land the code half of map ticket 42, decided on 2026-09-06. `run_start` gains `scopes`, the
absolute paths the gate was compiled against keyed by scope name, and `floor_lifted`, the boolean
harness spec 16 says every per-run human flag leaves on the record. The reader prefers both where
present, derives where absent, and reports a limit on the derived path.

Blocked by 14 because the key set is 14's: `skill_checkout` is not a scope until 14 lands, and the
paths this ticket stores are the ones 14's `Spawn` carries.

## Where the design says it

- Map ticket 42's `## Answer`, all seven decisions.
- Contract 5.1, the two new rows and the two paragraphs on `policy_sha` and the gate sha; 5.5 and
  5.6, absent-not-null on a harness run and an interactive one; 7.7, what the re-evaluation resolves
  against; 11, stored input rather than derived; 15, the rule for a new line 0 field.
- Harness spec 5, the test on the spawn directory; 7.5, what the script is compiled from.

## The slices

**The write.** `src/record/lines.rs` gains `Option<Scopes>` and `Option<bool>` on `RunStart`.
`src/run.rs` fills both from the same values it hands `policy::compile`, so the record and the gate
cannot disagree by construction. Keys: `worktree`, `skill_store`, `skill_checkout`, `corpus`,
`gate_script`. A key the run has no path for is absent. Both fields absent on a harness run; on an
interactive run `scopes` is absent and `floor_lifted` is null. `RUN_START_FIELDS` does not gain
either name, because that fails `every_record_on_this_machine_meets_the_contract` on every record
written before this.

**The read.** The three normalizers under `src/normalize/` read `scopes` off line 0 and rebuild only
the keys that are missing. The rebuild is what `src/normalize/claude.rs` does today. A rebuilt key
produces one limit finding per record, in the shape of `StalePolicy`, routed to `signals.limits`
in `src/ouroboros/corpus.rs` and never to `signals.disagreements`, because feeding the curator a
disagreement it did not earn is the failure ticket 42 measured. `floor_lifted` is read the same way
and defaults to false when absent.

**The no-repo run.** The launch scopes `worktree` at `<spawn dir>/no-worktree` and the reader falls
back to `cwd`, and the two disagree today on the same `Write`, measured under ticket 42. With
`scopes` on the record the reader takes the stored path and the divergence goes. Add the test that
shows it, because it is the one shape where the stored path and the rebuilt one differ on purpose.

**The tests.** `tests/record_contract.rs` checks a written record carries both fields with the
values `write_spawn_config` used. `tests/normalize.rs` gains the pair: a record with `scopes` whose
replay against a moved layout still derives allow, and the same record without `scopes` whose replay
reports the limit. `tests/curate.rs`'s `a_curate_runs_own_record_does_not_read_back_as_a_gate_that_leaked`
stays and should pass on the stored path rather than the rebuilt one.

## Done when

- A fresh record carries `scopes` and `floor_lifted`, and the contract test reads them back.
- A replay prefers the stored paths, and a record without them reports a limit rather than a
  disagreement.
- `IMPLEMENTATION.md`'s three ticket 42 paragraphs say built rather than pending: "Reading a record
  back" on the scopes and on `policy_sha`, the `--allow-floor-escape` paragraph, and the "Rough
  edges" entry on scope paths.
