# 42 - A `hydra curate` run writes `command: curate`, as contract 5.1 says

Type: build
Status: open
Blocked by:

## What

`src/ouroboros/curate.rs:127` writes `command: "run"` for the curator's own run. Contract
5.1 says `curate`, and says the point is that the asymmetry with rung two's harness record is
visible in the data. It is not: `grep '"command":"curate"' ~/hydra/runs/*.jsonl` finds
nothing after a real pass. One word in the request, one assertion in `tests/curate.rs`, and
a note in IMPLEMENTATION.md that records before this ticket carry `run`. Map ticket 59.
