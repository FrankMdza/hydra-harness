# Hydra

A router over three agent CLIs (`claude`, `cursor-agent`, `agy`), plus Ouroboros, a
self-improvement loop that reads what the router records.

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

One provider, one recorded run, one run record at `~/hydra/runs/<run-id>.jsonl`. No
gate yet, so the provider's own permission defaults apply and the record carries a
null policy.
