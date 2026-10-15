# 336 - The walk counts a record from another host apart and never replays it, and Rungs refuses a corpus that mixes hosts

Type: build
Status: open
Blocked by: 316
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
