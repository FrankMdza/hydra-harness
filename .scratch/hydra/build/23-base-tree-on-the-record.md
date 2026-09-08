# 23 - `base_tree` on the record

Type: build
Status: resolved
Blocked by:

## Goal

Map ticket 49 decided that the sha of the tree Hydra seeded a worktree from goes on the record,
in `run_start.worktree`, beside `path` and `branch`. The launcher already has it: `run.rs` reads
`seed_tree` off the worktree it created before the provider starts, and only the projection's
`diff_stat` ever printed it. Write it, read it back, and prefer it over recomputation.

## Where the design says it

- Map ticket 49's `## Answer`, second paragraph.
- Contract 5's `worktree` row, amended by the orchestrating session in the same change.
- `src/run.rs` around the `WorktreeRef` construction and the `diff_stat` match near the end;
  `src/worktree.rs::seed_tree`; `src/record/lines.rs::WorktreeRef`.

## The slices

**The field.** `WorktreeRef` gains `base_tree: Option<String>`, filled from the worktree's
`seed_tree` at launch. Null only when `write-tree` did not answer, which the existing `Option`
on `seed_tree` already models.

**The reader.** Where `run.rs` computes `diff_stat` for a record read back, the base comes from
`run_start.worktree.base_tree` when the record carries it, and from the seed index in the spawn
directory only when it does not. A record with neither has no `diff_stat`, as today. The
projection's `diff_stat.base_tree` keeps printing the same sha either way.

**The validator.** Whatever `src/record/` checks a written record against must accept the new
key on `worktree` and must not require it, so every record on this machine still validates.

**Tests.** `tests/record_contract.rs`: a record written through the stub carries
`worktree.base_tree` as a 40-hex sha equal to what `worktree::seed_tree` answers. `tests/status.rs`
or `tests/outcome.rs`: a synthetic record carrying `base_tree` derives a `diff_stat` with that
sha with no spawn directory on disk, and one without the field and without a spawn directory
derives none. The walk over this machine's records still passes.

## Done when

- A fresh recorded run's line 0 carries `worktree.base_tree`, checked on a real record.
- The two tests above pass and the suite is untouched.
- `IMPLEMENTATION.md`'s note on `base_tree` under the Herdr section says written since ticket 23.

## Outcome

Resolved 2026-09-07. `claude` on Sonnet 5, run 01a07f18-910f under `standard`: 10.6M tokens,
10.4M cache reads, 49k output. Six files, 205 insertions. `WorktreeRef` carries `base_tree`,
filled from the worktree's `seed_tree` at launch; `live::read_view` reads it back; the diff stat
in `run::result` takes the record's sha first and the seed index only when the record says
nothing; the writer's validator needed no change because it never enumerated `worktree`'s keys.
Three tests: a record written through the real seeding path carries the sha `write-tree`
recomputes against a second index, a synthetic record with the field derives a diff stat with no
spawn directory on disk, and one without the field and without a spawn directory derives none.

Checked on a real run after the rebuild, 01a07f27-1683 on haiku: line 0 carries
`worktree.base_tree` equal to the `diff_stat.base_tree` the projection printed. The corpus
walks pass with the new records on disk.
