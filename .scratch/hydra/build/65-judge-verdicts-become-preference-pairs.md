# 65 - Judge verdicts become preference pairs, withheld until a chosen half exists and signed off by a human

Type: build
Status: resolved
Blocked by:

## What

Map ticket 55, resolved 2026-09-21. A reader beside `src/export.rs`, not inside rung one's five
classes, walks the `evolve` passes in the corpus and emits one preference pair per blocked
proposer patch whose chosen half exists.

1. The rejected half is the proposer's diff on `rungs/blocked/<pass-id>` or the older
   `ouroboros/blocked/<pass-id>`. The chosen half is the diff a human merged for the same task,
   found by the pass id named in a later commit message on `master`, or the panel-approved patch
   on `rungs/<pass-id>` when the pass was offered. No chosen half, no pair: the reader reports it
   as withheld, with the pass id.
2. A pair leaves only when a sign-off manifest, `config/export-signoff.json`, lists its pass id,
   the judge run id and the finding item a human confirmed. The manifest is protected config the
   human writes; the reader refuses a pair not in it and says so. `gated` judge runs are read the
   same as `completed` ones.
3. `hydra export` gains the pairs in `preference.jsonl` beside the correction pairs, in the shape
   contract 16.3 gives, with a `source` field naming which reader produced each.

Tests in `tests/export.rs` against a scratch repo and synthetic records: a blocked pass with no
chosen half is withheld; one with a merged fix and a manifest entry exports; one with a merged
fix and no entry is refused; a `gated` judge is read. Launch no provider. Do not edit `docs/`,
`config/`, `IMPLEMENTATION.md` or `.scratch/`; the manifest's example file may be added under
`config/` as `export-signoff.json.example` only.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` and `cargo test --release --test export`. Do not run the whole suite.

## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c5dd-984e under `standard`, 5.1M
tokens, the largest of the day: `src/export_preference.rs` with the sign-off manifest, the corpus
walk over `evolve` passes, the blocked and chosen halves read off branches and a later `master`
commit naming the pass, `export_with_signoff` so a test can hand a scratch manifest, and four tests
in `tests/export.rs` on a scratch repo. One correction before master: the module had copied
`redact_value`, `redact_string`, `repo_names_to_redact` and `own_repo` out of `export.rs`, and two
copies of the secret prefix list is the drift the export exists to prevent; the four are
`pub(crate)` in `export.rs` now and the module imports them. `export` 8 of 8.

Two things the run reported as false assumptions, both right: `std::env::set_var` is unsafe on
this toolchain, so the manifest path is a parameter rather than an environment override in tests;
and a `judge` run with a refused `Read` derives `failed` on the synthetic record rather than
`gated`, so the reader parses the answer whatever the outcome. `config/export-signoff.json` is the
human's to write, and `config/README.md` says its shape.
