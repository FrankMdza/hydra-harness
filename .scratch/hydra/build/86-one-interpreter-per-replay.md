# 86 - The replay runs one interpreter per record, not one per call

Type: build
Status: resolved
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

`gate.py` gains `main_batch`, entered only with `batch` as the second argument, which reads a JSON
array and answers each payload through the same `check_dialect` and `decide_payload` the single-call
path uses. `replay.rs` has `decide_batch` and `decide_all`, which falls back to one `decide` per call
if the batch fails; the four normalizers call `decide_all` once per record. The ticket said edit only
`src/policy/` and `tests/`, and that was false: the loop that asks the gate lives in each of the four
`src/normalize/` adapters. The run's first version set `PWD` from the payload on the live single-call
path too, outside the `try`, so a `cwd` holding a NUL gave a traceback and an empty answer, which
`claude` and `codex` read as allow; the Opus review found it and an Opus subagent moved it into the
batch entry only, with a test each way. It was also ported on top of build 77 and its test helpers
moved into `tests/corpus/mod.rs` after build 79. Measured on master: `hydra show 01a0c924-a494`
10.8 s and 10.3 s before, 0.21 s and 0.16 s after, output identical under `cmp`; gate 93, normalize
81, outcome 42, show 3, spawn 30, lib 194 passed; clippy and fmt clean. Built by `cursor-agent` on
Composer 2.5 under `standard`, run 01a0cc32-5f53, 3.2M tokens.
