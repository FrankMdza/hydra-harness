# 354 - The fixture corpus carries a `codex` 0.160.0 record with a page open, transcribed from a recorded run

Type: build
Status: resolved
Size: haiku
Blocked by: 353
Opened: 2026-10-21, by the session. Run `01a160af-5a15` (`codex`, `gpt-6.1-sol`, `permissive`,
one turn, 33,834 input tokens of which 29,056 cached, 70 output) opened one page and answered
its title. Its record is the first on this machine with a `webrun` call, and it is the measured
source for build 353's reader. It cannot enter `tests/fixtures/records/` as it is: the corpus is
built, not copied (`tests/fixture_corpus.rs` lines 9 to 13), its guard refuses a live run id, and
`the_fixture_corpus_on_disk_is_what_the_builders_write` holds the disk to what the builders
write. The shape enters the way every other measured shape did, as a builder in `tests/common`
and a new directory `codex@0.160.0` beside `codex@0.154.0`, because records in the old shape stay
on disk (lines 15 to 19).

## What

A fixture record under `tests/fixtures/records/codex@0.160.0/` with a synthetic id under
`FIXTURE_RUN_PREFIX`, written by a new builder beside the codex ones in `tests/common`, whose
frames are these, measured on run `01a160af-5a15` and on the probe of build 338, in this order
between `run_start` and `run_end`:

```
{"type": "thread.started", "thread_id": "<synthetic>"}
{"type": "item.completed", "item": {"id": "item_0", "type": "error", "message": "`--dangerously-bypass-hook-trust` is enabled. Enabled hooks may run without review for this invocation."}}
{"type": "turn.started"}
{"type": "item.completed", "item": {"id": "item_2", "type": "agent_message", "text": "I'll open the page and read its title."}}
{"type": "item.started", "item": {"id": "exec-<synthetic>", "type": "web_search", "query": "", "action": {"type": "other"}}}
{"type": "item.completed", "item": {"id": "exec-<synthetic>", "type": "web_search", "query": "https://example.com", "action": {"type": "open_page", "url": "https://example.com"}, "results": [{"type": "text_result", "domain": "example.com", "ref_id": "turn0view0", "snippet": "Total lines: 1", "title": "Example Domain", "url": "https://example.com/"}]}}
{"type": "item.completed", "item": {"id": "item_4", "type": "agent_message", "text": "[Example Domain](https://example.com/)"}}
{"type": "turn.completed", "usage": {"input_tokens": 33834, "cached_input_tokens": 29056, "cache_write_input_tokens": 0, "output_tokens": 70, "reasoning_output_tokens": 0}}
```

The real run carried the hook-trust notice twice; one copy is enough for the shape. No hook
frame exists for the call, since codex writes none for an allow, and no `gate_refusal` row
exists either. The `run_start` and `run_end` lines follow what the codex builder already writes,
with `provider_version` `0.160.0` where the builder names the version.

## Contract

Input: the fixture walks of `record_contract.rs`, `normalize.rs` and `outcome.rs`. Output: the
new record meets the contract; the normalizer reads one `webrun` call with the URL under
`open[0].ref_id` and one result, which is build 353's reading, and no finding; the outcome is
`completed`. `the_fixture_corpus_holds_every_routed_provider_and_the_harness` and
`the_fixture_corpus_on_disk_is_what_the_builders_write` stay green, and
`no_fixture_carries_a_real_path_a_secret_or_a_live_run_id` passes over the new directory.

## Cases

Written first, in `tests/fixture_corpus.rs`: the corpus holds a `codex@0.160.0` directory with
exactly one record, and `normalize.rs`'s fixture walk reads a `webrun` call out of it. Red until
the builder and the directory exist.

## Do

1. Read `tests/fixture_corpus.rs` lines 1 to 60 and 478 to 540, the codex builder in
   `tests/common`, and build 353's Outcome for what an allowed `webrun` reads as.
2. The case, red. The builder and the `CODEX_0160` constant. Then
   `cargo test --test fixture_corpus -- --ignored write_the_fixture_corpus` and read the diff:
   only the new directory should appear.
3. The three fixture walks green, then the four checks, then `## Outcome` with the record's path
   and the walk's reading of it.

## Outcome

Built. The record is `tests/fixtures/records/codex@0.160.0/00000000-0000-7000-8000-000000000013.jsonl`.
`write_the_fixture_corpus` wrote it from the inline codex builders in `tests/fixture_corpus.rs`,
with the `CODEX_0160` constant naming the directory. The codex shapes were never builders in
`tests/common`, so the new one sits beside them in `fixtures()`, not in `tests/common`. The
frames are the ticket's, in its order, with one copy of the hook-trust notice and synthetic ids.
The record carries `provider_version` `codex-cli 0.160.0`, `policy` `permissive` and no host,
like fixture 11.

Walk reading. The normalizer reads one `webrun` call, id `exec-fixture-0160-webrun`, with
arguments `{"open": [{"ref_id": "https://example.com"}]}` and capability `network_read`. It reads
one result holding the page title. The decision is `allow` with `decider: hydra_policy`. The
outcome is `completed`. The findings are two, both about the fixture's shape. One is the scope
limit every synthetic record gets, because it carries no `scopes`. The other is `StalePolicy`,
because every fixture carries the fake policy sha. The new normalize case filters `StalePolicy`
and says why in a comment.

Measured. Before the directory existed on disk, both new cases failed for the reason the ticket
gives, `no codex@0.160.0 directory`, and the normalize case failed on `StalePolicy`. After
`write_the_fixture_corpus`, both pass. The rewrite changed no existing fixture: the only change
in `tests/fixtures` is the new directory.

Checks. `cargo fmt --all --check` passes after `cargo fmt --all` reflowed the new normalize
assertion. `cargo clippy --workspace --all-targets -- -D warnings` is clean.
`cargo nextest run --release --workspace --no-fail-fast` gives 1472 passed and 4 skipped.
`cargo test --release --workspace --doc` passes. The three fixture walks, `record_contract`,
`normalize` and `outcome`, pass, and so do the fixture corpus guards, including
`no_fixture_carries_a_real_path_a_secret_or_a_live_run_id`.

Deviations from the ticket. The fast loop the brief names, `-p hydra-apprentice`, does not run
these tests, which live in the `hydra` package, so the cases ran under `--workspace`. The
ticket asks for a builder in `tests/common`, and there is none for codex, so the shape is
inline. The ticket's "one copy is enough" is kept. The fixture uses id 13.

IMPLEMENTATION.md. The fixture paragraph now says thirteen records in six directories and names
`codex@0.160.0` and what its replay reads.
