# 42 - A `hydra curate` run writes `command: curate`, as contract 5.1 says

Type: build
Status: resolved
Blocked by:

## What

`src/ouroboros/curate.rs:127` writes `command: "run"` for the curator's own run. Contract
5.1 says `curate`, and says the point is that the asymmetry with rung two's harness record is
visible in the data. It is not: `grep '"command":"curate"' ~/hydra/runs/*.jsonl` finds
nothing after a real pass. One word in the request, one assertion in `tests/curate.rs`, and
a note in IMPLEMENTATION.md that records before this ticket carry `run`. Map ticket 59.

## Outcome

Resolved 2026-09-12. The launch a curate pass asks for is built by a new
`curate::request(run_id, brief, model, timeout)`, which writes `command: "curate"`. `curate`
itself just calls it. Every reader that switches on `command` was checked: `validate.rs`
accepts all four values already, `corpus.rs` keys on `provider`, `run.rs` keys on `evolve`
for the harness parent and `normalize/agy.rs` on `resume`. Nothing expected `run` for the
curator's record.

Two tests in `tests/curate.rs`, launching nothing: one reads the field off the request and
off line 0 of a record shaped from it, and runs the validator over it; the other writes a
curator record carrying `run`, the shape every record before this date has, and checks the
validator and the corpus still take it. `cargo test --test curate` is 20 of 20.

Contract 5.1 and IMPLEMENTATION.md say that curator records before 2026-09-12 carry `run`
and readers accept both.
