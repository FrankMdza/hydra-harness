# 125 - A streamed run that stops at exit 0 without a terminator is `failed`, and the walk says so

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d56c-a2b7-7491 under `standard`, about
1.02M tokens, 0.95M of them cache reads. The session read the diff and applied it with one
change.

The assertion in `tests/outcome.rs` now skips a `failed` run whose stream carries no
terminator, and keeps the exit-0 refusal for a `failed` run that has one. No measured `failed`
run with a terminator exits 0: `agy`'s timeout and codex's `turn.failed` both exit non-zero. The
comment names both records. `src/outcome.rs` did not change, because `derive` already returns
`failed` for this shape.

Fixture 11 under `tests/fixtures/records/codex@0.154.0/` is the 01a0d484 shape: two
`command_execution` items, one `agent_message`, no turn terminator, `run_end` exit 0. The live
record says `codex-cli 0.154.0`, so the directory label is right. The session changed the
fixture's items from `output` to `aggregated_output`, the field codex actually writes and the
one the reader tries first, and regenerated the file with `write_the_fixture_corpus`. README.md
and IMPLEMENTATION.md count eleven fixtures.

With the assertion change reverted, the fixture walk fails:

```
assertion `left != right` failed: .../tests/fixtures/records/codex@0.154.0/00000000-0000-7000-8000-000000000011.jsonl: failed with exit 0
```

On master with the live corpus, `cargo test --test outcome` passes, 47 of 47, and so do
`fixture_corpus` (3) and `record_contract` (21). fmt and clippy are clean. Inside the worktree
two diff-stat tests in `outcome.rs` failed on the same inherited `GIT_OBJECT_DIRECTORY` as
`run::tests`. On master they pass.
