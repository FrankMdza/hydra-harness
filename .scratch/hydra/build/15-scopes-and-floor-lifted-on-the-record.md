# 15 - `scopes` and `floor_lifted` on the record

Type: build
Status: resolved
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

## What ticket 14 found, added 2026-09-06

**`parent_git_dir` is a sixth key.** The launch denies the parent repo's `.git` and asks
`git rev-parse --git-common-dir` from the parent for the path. The read side can only ask the same
question inside the record's worktree, and once `hydra reap` has taken the worktree the answer is
nothing, so every reaped run replays with that denial missing. It is the one scope nothing on the
record can recover, and the case for a stored path over a rebuilt one is sharpest here. Store it
beside the five above, absent on a run outside a repo.

**The no-repo divergence is gone already.** 14 made the reader fall back to
`<spawn dir>/no-worktree`, the launch's own placeholder, rather than to `cwd`, so the `Write` that
disagreed under ticket 42 agrees now. The stored path still wins on a moved layout, which is what
the test should show instead.

**The rebuild lives in one place.** `ReplaySpawn::of` in `src/normalize/mod.rs` is what the three
adapters call, so the read half of this ticket is one function rather than three.

## Outcome

Resolved 2026-09-07. Map ticket 42's seven decisions are code.

**The write.** `src/record/lines.rs`: `RunStart` gains `floor_lifted: Option<bool>`, serialized as
null when absent, and `scopes: Option<BTreeMap<String, String>>`, left off the line entirely when
absent. `src/policy/mod.rs`: `Spawn` gains `skill_store` and `corpus` as paths, so every scope it
names is an argument rather than a `Layout` lookup, and `Spawn::paths` answers all seven named
paths in one map. `compile`'s `scopes` block and `Spawn::recorded_scopes` are two views over that
map, filtered by `GATE_SCOPES` and `RECORDED_SCOPES`, so the script and the record cannot name
different paths. `resolved` split into `resolved_path`, which canonicalizes the longest existing
ancestor and keeps the case, and `casefold`, which is what 15.2's case-insensitive match needs; the
record stores the first and the gate compiles the second. `src/run.rs`: `write_spawn_config` returns
the paths on `SpawnFiles::scopes` and `run::run` puts them on line 0.

**`RUN_START_FIELDS` did not gain either name**, per the ticket. `scopes` joined `skills`, `mcp`
and `auth` in the absent-not-null group check in `src/record/validate.rs`.

**Which refusals carry `scopes`.** None of them, and the order in `run::run` is why. All three
built refusals fire before `write_spawn_config`, so no gate was compiled and no scope was resolved:
build ticket 11's policy narrowing check, which refuses before the provider version is even read,
and the skill collision of 6.1 item 5 and the MCP shortfall of item 6 with Cursor's pre-flight
beside it, which write `refused_line_zero`. All three carry `floor_lifted` and no `scopes`. The
launched path recomposes line 0 after the compile, so the two records `never_started` writes, the
`agy` gate-mount failure and a `detach` that failed, both carry the scopes.

**The read.** `RecordedGate` in `src/normalize/mod.rs` reads both fields off line 0, and the three
adapters hold one each, filled where they already read `cwd`. `ReplaySpawn::of` takes it and prefers
a stored path key by key; a key missing from a present `scopes` object is a run that had no such
path, so `skill_checkout` outside `curate` and `parent_git_dir` outside a repo rebuild nothing.
`ReplaySpawn::scope_limit` is one `Finding::Limit` per record naming what it stood in for, and it
adds that nothing answered for `parent_git_dir` when the git call comes back empty, which is ticket
14's reaped-worktree case. `src/ouroboros/corpus.rs` needed no change: `Finding::Limit` already
routes to `signals.limits`, and `tests/curate.rs::a_record_with_no_scopes_reaches_the_curator_as_a_limit`
pins that it never reaches `signals.disagreements`.

**A rebuilt scope downgrades the two findings that claim two sources differed.** Decision 5 says a
limit and never a disagreement, and the limit alone did not deliver it: `GateLeak` and
`Disagreement` both route to `signals.disagreements`, and both were still reachable on a record
whose paths this reader rebuilt. `ReplaySpawn::stood_in` is what `policy_moved` is for a moved
`policy_sha`, and the six finding sites across the three adapters ask it before they report either.
The cost, said out loud in `IMPLEMENTATION.md`: a record with no `scopes` can no longer report a
gate leak, so the corpus-wide leak check proves less than it did until records written by this
build fill the corpus. `tests/common::Record::gated` is the fixture that carries scopes, and the six
tests that want a leak or a disagreement use it.

**One home for the scope key names.** `Scope::as_str` is `const` now and `policy::keys` names all
seven from it, the two that are not `Scope` values included. `RECORDED_SCOPES`, `GATE_SCOPES`,
`Spawn::paths` and every `RecordedGate::path` call read from there rather than from a string
literal, which is what stops a typo in one of the three readers from silently unscoping a path.

**Tests, six new.** `tests/record_contract.rs::a_record_carries_the_paths_its_gate_was_compiled_against`
drives `write_spawn_config` for a `curate` run in a throwaway repo, writes the record, and checks
all six keys against the paths the launch used and against the compiled script's own `scopes`
block, which is the seam. `tests/normalize.rs` has the pair,
`the_scopes_on_the_record_beat_a_layout_that_moved` and
`a_record_with_no_scopes_is_rebuilt_and_reports_a_limit`, plus
`the_floor_lift_on_the_record_reaches_the_replay`. `tests/curate.rs`'s existing
`a_curate_runs_own_record_does_not_read_back_as_a_gate_that_leaked` passes on the stored path now.
`tests/record_contract.rs::the_validator_catches_null_scopes` is the sixth, beside the existing null
group check. The corpus walk in `tests/normalize.rs` asserts the limit lands at most once per
record, which is what tolerating it means: every record on this machine predates the field.

**Measured.** `cargo test`, 0 failed, 431 passed on the branch from a base of 425, and 457 from 451
rebased onto build tickets 16 and 11. The same six tests either way: `normalize` 61 to 64, `curate`
17 to 18, `record_contract` 16 to 18. Every other binary on the rebase is untouched by this ticket
(lib 147, `cleanup` 25, `delegation` 7, `detached` 6, `drift` 2, `face` 7, `gate` 58, `mcp` 14,
`outcome` 37, `pty_tee` 2, `skills` 19, `spawn` 16, `status` 12, `worktree` 5).
`cargo clippy --all-targets` silent.

**What the ticket assumed that turned out false.**

- **The reader could not prefer a stored `skill_store` or `corpus` without a change to `Spawn`.**
  Both came off `spawn.layout` inside `compile`, so a stored path for either would have been read
  and then ignored. They are `Spawn` fields now. The deny list still comes off `layout`, because
  nothing on the record carries it, so a replay of a record from another `~/hydra` scopes against
  the stored paths and denies against the reader's own. That is half the compile recovered rather
  than all of it, and it is the half a record can carry.
- **Eight of the suite's assertions of "no findings" had to move.** A record built by
  `tests/common` carries no `scopes`, which is what every record on this machine looks like, so
  each one now earns the limit. Seven in `tests/normalize.rs` read through
  `common::other_findings`, which filters that one finding and keeps the rest strict, and
  `tests/curate.rs`'s gate-leak test carries stored scopes instead, because it is about a run this
  build would write.
- **The limit alone did not deliver decision 5, and the review caught it.** Eight more fixtures had
  to carry scopes once a rebuilt path downgraded `GateLeak` and `Disagreement`, six in
  `tests/normalize.rs` and two in `tests/outcome.rs`, because a fixture without them can no longer
  reach either finding. That is the shape of the decision rather than a test problem: the reader is
  quieter on every record written before this build.
- **`floor_lifted` is worth its own test and the ticket did not name one.**
  `the_floor_lift_on_the_record_reaches_the_replay` is the pair, and it is the first thing in the
  tree that proves the sixth input to `policy::compile` survives a round trip.

**Docs.** Two sentences, both allowed by the orchestration plan, and each gained one key and
nothing else: contract 5.1's `scopes` key list and harness spec 7.5's "What the script is compiled
from" both name `parent_git_dir` now, which is ticket 14's finding and was not in ticket 42's
answer. That two of the six keys are denied paths rather than scopes is an observation about the
code, so it sits in `IMPLEMENTATION.md` instead. `IMPLEMENTATION.md`: the
`--allow-floor-escape` paragraph, the scopes paragraph in "Reading a record back" and the one on
`policy_sha`, the "Rough edges" entry on scope paths, and the `skill_checkout` sentence in the
ticket 14 section all say built. `src/policy/replay.rs`'s doc comment on `Replay::compile` said the
record carries no `floor_lifted`; it does now.
