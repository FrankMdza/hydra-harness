# 86 - The replay runs one interpreter per record, not one per call

Type: build
Status: open
Blocked by:

## What

Study session block (d), question 2. `src/policy/replay.rs:132` spawns `/usr/bin/python3 -c
<gate>` once per tool call, on the blocking path of `hydra run` and of every reader. A run with
three hundred calls pays three hundred interpreter starts. Keep the gate script unchanged as the
only evaluator (the replay's point is that it runs the same code the hook ran) and add a batch
entry: one interpreter reads a JSON array of payloads on stdin and writes one answer per payload,
in order. The per-call `PWD` rule at `:136` has to hold per payload, since two calls in one record
can carry different `cwd`s; say how in a comment.

## Done when

The replay spawns one interpreter per record; `tests/gate.rs` has a test that the batch answers
match the single-call answers on the same payloads for every dialect; `cargo test --release --test
gate --test show --test outcome` green; clippy and fmt clean. The time for one replay of a real
record before and after goes in the Outcome. Edits `src/policy/` and `tests/` only.

## Outcome
