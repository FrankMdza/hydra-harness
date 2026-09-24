# 125 - A streamed run that stops at exit 0 without a terminator is `failed`, and the walk says so

Type: build
Status: open
Blocked by:

## What

Map ticket 80, resolved 2026-09-24. Read its `## Answer` first.

The full suite on `4261426`, run 2026-09-24 at 14:24, is red on one assertion:

```
thread 'the_recorded_write_denials_are_the_only_gated_runs_on_this_machine' panicked at tests/outcome.rs:1425:13:
assertion `left != right` failed: /Users/USER/hydra/runs/01a0d484-2d8c-7443-b46e-0e4f151c47e0.jsonl: failed with exit 0
```

That record is the fourth panel's `codex` seat, launched at 17:43:48Z right after its quota
reset. Twenty lines: `run_start`, command items, one `agent_message`, and `run_end` with
`exit_code` 0, `error` null, `body_source` "stream". No `turn.completed`, no `turn.failed`, no
noise line, no quota text anywhere in the file. `codex` stopped 27 seconds in and said nothing.

`derive` in `src/outcome.rs` returns `failed` for it, which is the contract's reading of a
streamed run that stopped without a terminator, and the module doc says nothing on a streamed
run reads the exit status. The assertion near `tests/outcome.rs:1418` ("A provider that ran the
task and then died on it is `failed`") assumes a `failed` run exits non-zero, because the first
one was `agy` at exit 1. This record breaks the assumption, not the contract.

## Do

1. Narrow the assertion to what the contract says: a `failed` run with exit 0 is allowed when
   the stream carries no terminator (`run.result` is none). Keep the exit-0 refusal for a
   `failed` run that does have a terminator, if the code can reach that case at all; if it
   cannot, say so in a comment and drop the check. Rewrite the comment above it to name both
   records, `agy`'s from 2026-09-07 and this one.
2. Add a fixture twin under `tests/fixtures/records/codex@<the version on the record's
   run_start>/`, built the way `tests/fixture_corpus.rs` builds the others: a streamed `codex`
   run with items, an `agent_message`, no turn terminator, `run_end` exit 0. `every_fixture_derives_an_outcome`
   and the fixture half of the same walk read it and get `failed`. Update README.md's fixture
   count ("ten synthetic records", near line 229) and any other place that counts them
   (`grep -rn "ten synthetic\|ten fixtures" README.md docs IMPLEMENTATION.md tests`).
3. If reading the record makes you think it should derive something other than `failed`, for
   example `rate_limited` from the timing alone, stop and say so in `## Outcome`. That is a
   contract decision, and it goes on the map.

## Done when

- `cargo test --test outcome` passes on this machine, with the live corpus.
- With your assertion change reverted, the new fixture fails the fixture walk with the same
  `failed with exit 0` message. Paste the line into `## Outcome`.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --test outcome
  --test fixture_corpus --test normalize --test record_contract` pass.
