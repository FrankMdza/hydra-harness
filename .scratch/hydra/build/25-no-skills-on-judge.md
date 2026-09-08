# 25 - No skill injection on `judge`

Type: build
Status: resolved
Blocked by:

## Goal

Map ticket 47 decided that a `judge` run is not linked the skill snapshot. The policy allows
nothing, the diff it judges rides its prompt, and build ticket 16 measured `--restricted`
hiding every skill on the filesystem anyway, so the links were never reachable and the record's
`skills` group named a store for a run that could touch none of it. Make the record honest.

## Where the design says it

- Map ticket 47's `## Answer`. Harness spec 6.4, which gains the exception in the same change,
  written by the orchestrating session. Contract 5.3 on when `skills` is absent.
- `src/run.rs`, where `write_spawn_config` links the snapshot and where `curate` already reaches
  that code by policy name, the precedent.

## The slices

**The skip.** On `judge`, no snapshot is linked into the checkout, no collision check runs, and
the probe under `judge` is not asked about skills. The `skills` group is absent on the record,
the way it is absent on a run with no checkout, rather than present with nulls.

**The reader.** Whatever reads `skills` back, `normalize` and the record validator, already
handles absence for the no-checkout case; confirm that a `judge` record with a worktree and no
`skills` reads and validates.

**Tests.** `tests/spawn.rs` or `tests/skills.rs`: a `judge` spawn against a non-empty store links
nothing into its checkout and its record carries no `skills`. `tests/record_contract.rs`: such a
record validates. The existing test that a `standard` spawn links the snapshot stays green.

## Done when

- A `judge` run's line 0 has no `skills` key and its checkout has no linked snapshot.
- The three tests pass and the suite is untouched.
- `IMPLEMENTATION.md`'s `--restricted` section says `judge` is not injected, since ticket 25.

## Outcome

Resolved 2026-09-07. `claude` on Haiku 4.5, run 01a07f37-8a67 under `standard`, the first Rust
ticket given to the cheapest model: 4.7M tokens, 4.6M cache reads, 16k output. One guard on the
`injected` match in `run.rs`, `Some(worktree) if request.policy != "judge"`, and two tests in
`tests/skills.rs`: a `judge` spawn against a non-empty store links nothing and its record has no
`skills`, and a `standard` spawn beside it still does. It edited `IMPLEMENTATION.md` against its
instructions; that hunk was dropped and the orchestrating session wrote the note. What its run
left behind was more useful than its diff: five `Read` calls with `offset` passed as an array,
each refused by Claude Code's input validation before any hook ran, which the normalizer read
as five `Ungated` findings. The normalizer now knows that shape, in the same commit.
