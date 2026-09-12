# Hydra

A router over three agent CLIs (`claude`, `cursor-agent`, `agy`), plus Ouroboros, a
self-improvement loop that reads what the router records.

![hydra run on agy: the vendor reports SUCCESS, the record says gated](docs/demo/hydra.gif)

One command launches any of three vendor agents in its own git worktree, behind a
deny-by-default policy, and writes every frame to one JSONL record. The outcome on that record
is derived from what the tools did, not from the exit code. In the demo above the vendor exits
0 and reports `SUCCESS` on a run where the gate refused its only shell command. All three routed
vendors were measured doing this, which is the reason the record decides and the vendor does not.

Re-record the demo with `docs/demo/record.sh`; it needs `asciinema` and `agg` on the PATH and
spends one short `agy` run.

`docs/` holds the design: `harness-spec.md` for how Hydra launches, feeds and records
the providers, `run-record-contract.md` for the format it writes and Ouroboros reads,
`ouroboros-spec.md` for both rungs of self-improvement. Read them together.

`IMPLEMENTATION.md` is the gap between that design and the code. **Read it before
trusting a run record**, because the first slice is deliberately not spec-complete
and the records say so.

## Today

```
cargo build --release
./target/release/hydra run --provider claude --task "..."
```

One recorded run, one record at `~/hydra/runs/<run-id>.jsonl`. `--provider` takes `claude`,
`cursor-agent` or `agy`; `--policy` takes one of the five in `policies/` and defaults to
`standard`, which is reads, writes, shell and network inside the run's own worktree. All
eleven commands of the harness spec exist: `run`, `status`, `logs`, `show`, `resume`, `reap`,
`prune`, `curate`, `evolve`, `usage`, `export`. `hydra --help` lists them, and `hydra show`
takes a run-id prefix.
