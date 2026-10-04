# 277 - Normalize the corpus once per export

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Refactor ticket 2 (finding 2): `hydra export` normalizes the corpus once, once more per allowlisted repo, and twice more per eligible run. Build one `BTreeMap<String, Run>` in `export_with_signoff` and thread it through `audit`, `collect_pairs` and `read_judge_pairs`. Output must be byte-identical. Net: `tests/export.rs`.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-03 by run `01a10517-84cc` (`codex` gpt-6.1-sol, `standard`, 0.58M tokens),
applied without changes.

Built: `export::read_corpus` reads and normalizes every record once; `audit_runs`, the export
loop, `collect_pairs_from_runs` and `read_judge_pairs_from_runs` take that corpus, and the public
`audit`, `collect_pairs` and `read_judge_pairs` keep their signatures by reading it themselves.

Measured by the session on the real corpus with this build and master's: `hydra export --dry-run`
gives the same 797 lines; a real `hydra export` takes 48 s against 101 s and both stop at the
same error, which build 285 now carries: a curate pass on `ouroboros/blocked/01a0814a-c730`
names a commit, `02a6c80`, that the object store no longer has.
