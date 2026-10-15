# 336 - The walk counts a record from another host apart and never replays it, and Rungs refuses a corpus that mixes hosts

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 126's answer. Two machines will write records into one
corpus. The human decided: no carve-out in contract 7.7, a record is never replayed with its own
layout, and a record whose `run_start.host` is not the reader's is counted under its host and never
re-evaluated, apart from `GateMark::Foreign`, which keeps meaning a gate compiled from another build
on this machine. And Rungs refuses, at load, a corpus that mixes hosts, naming both, until a Linux
wrapper exists (build 318); no weight for hook-only records.

## What

After 316 lands `host` and `sandbox_kind`:

1. `src/normalize/mod.rs`: before `GateMark::of` compiles a gate for a replay, compare the record's
   `host` with this machine's. A different `os`, `arch` or `machine` is a new mark,
   `GateMark::OtherHost`, whose clause names the record's host; no gate is compiled for it. A
   record without `host` is treated as today.
2. `src/walk.rs` and `hydra doctor --walk`: one count per foreign host (`linux/x86_64/vm-1: N`),
   printed with the summary, never as a leak. **Build 316 left half of this:** `walk::hosts_of`
   counts every record's host keyed `machine os/arch`, `walk::Report::hosts` carries it, and
   `doctor --walk` prints `host <machine> <os>/<arch>: N record(s)` per host, this machine's
   included. What is left is telling the reader's own host apart and marking the rest foreign.
   `record::Host::here()` is the comparison side.
3. `src/rungs/corpus.rs` (or wherever the corpus is loaded for `curate` and `evolve`): if the
   records carry more than one distinct `host`, refuse before any reading with one message naming
   each host and its count, and say how to point `HYDRA_ROOT` at one of them.
4. Contract 7.7: one paragraph saying what the reader does with a record from another host, dated.

## Do

1. The mark and the count; `tests/doctor.rs` walks a fixture with a foreign host and reports it
   under its own count with no `permission_decision` derived.
2. `tests/curate.rs`: a two-host corpus is refused with both hosts in the message; a one-host corpus
   is read as before.
3. `IMPLEMENTATION.md` beside the `Foreign` paragraph.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green, and `hydra
doctor --walk` over this machine's records reporting zero foreign hosts.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Built `GateMark::OtherHost`, with a clause naming the recorded host. All four
provider adapters check the three host leaves before rebuilding scopes or compiling
either replay gate; Claude's interactive harvest also derives no permission decisions
for another host. `Foreign` retains its existing meaning. Unstamped records retain
their previous behavior. If the local hostname cannot be determined, stamped records
cannot be identified as local and are counted apart.

`Host::of`, `label` and `is_other` share identity parsing and comparison. Typed host
counts prevent display formatting from deciding corpus identity. The walk reads only
line zero for those counts, validates every record's contract, and excludes foreign
records from replay assertions. `Report::foreign_hosts` and doctor print per-host
foreign counts and their total. The shared Rungs loader used by curate and evolve
checks the whole corpus before normalization or liveness filtering, refuses multiple
distinct stamped hosts, and names every host and count with `HYDRA_ROOT` guidance
until build 318. Unstamped records invent no additional identity. No hook-only
weighting was added. Contract 7.7 and IMPLEMENTATION.md document the behavior.

Added four tests: three in tests/doctor.rs and one in tests/curate.rs, plus a host
stamping helper in tests/common/mod.rs. Measured four providers times three differing
leaves: 12 foreign cases, each with tool calls but zero permission decisions and no
gate-leak, disagreement or ungated findings. All four matching-host cases still
derive decisions; the local read-only case derives Deny and Gated. A foreign fixture
counts 1 host and 1 record, derives 0 decisions, reaches the walk's replay reader
0 times and changes 0 record bytes. A local doctor fixture reports 0 foreign hosts.
A foreign record with a broken sequence still fails contract validation. Rungs reads
2 records on one host, then refuses the 3-record corpus after a second host appears,
with counts 2 and 1 in the message.

What was false: the existing fixture expectations assumed the synthetic `stub-host`
record could still derive a local gate denial. Map 126's authorized contract changes
that behavior: the foreign success terminator derives Completed without replay.
Updated these existing tests explicitly for that contract, keeping local Deny/Gated
coverage in the new provider test and the existing synthetic tests:
- tests/cockpit.rs::runs_go_into_lanes_by_liveness_and_outcome
- tests/normalize.rs::the_fixture_write_denial_derives_to_a_deny_under_capability
  (renamed the_foreign_fixture_write_denial_is_not_replayed)
- tests/outcome.rs::every_fixture_derives_an_outcome
- tests/outcome.rs::the_fixture_write_denial_is_the_only_gated_fixture_on_the_stream
  (renamed the_foreign_fixture_write_denial_does_not_derive_gated)

The requested walk over this machine's live records cannot be measured inside this
run's worktree-only boundary; no live corpus was opened, and the driving session
must perform that measurement. The 0-foreign-host result above is a fixture result,
not a claim about the live corpus. No tool call was refused by the gate. Nothing was
committed; unrelated seeded changes were left alone.

Final checks, without environment removals:
- `cargo fmt --all --check`: no output, exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`:
  `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.66s`
- `cargo nextest run --release --workspace`:
  `Summary [  40.989s] 1447 tests run: 1447 passed, 4 skipped`
- `cargo test --release --workspace --doc`:
  `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

`cargo build` also passed (9.32 s); `git diff --check` passed. The initial workspace
run exposed the fixture expectation described above. Intermediate runs also caught
two errors in the new tests: the AGY fixture had no tool call, and changing the value
of schema_version did not trigger this validator's presence check. The final tests
use a synthetic AGY tool call and a broken sequence respectively.

Amended by the session on applying, 2026-10-15. The run's "false assumption" was a cost it paid
and should not have: fixture 1 is the one stream-gated fixture, the read-only claude run whose
Write denial `tests/outcome.rs`, `tests/normalize.rs` and `tests/cockpit.rs` rely on replaying,
and 316 had hung `stub-host` on it. Under this build's rule that denial stopped replaying, the
fixtures no longer derived `gated`, and the run loosened four assertions to match. The session
moved the hosted claude fixture from 1 to 6 (`HOSTED` in `tests/fixture_corpus.rs`, regenerated:
two files, one line each), restored the four assertions as they were, and the suite passes with
them: 1447 tests. Real walk with this binary over 964 records: `host laptop.local
macos/aarch64: 6 record(s)` (the runs launched after 316 landed), `foreign hosts: 0`.
